#!/usr/bin/env python3
"""Generate ENHANCED acoustic templates for the "read-along challenge" mode.

每字母收集最多 3 个不同来源的真人样本（去重、精确 stem 匹配），提取：
  - 24 维声学特征 = MFCC(c1..c12) + 一阶差分 + 逐句 CMN + 全局方差白化
  - 20 点相对音分轮廓（声调用，任务3）

产物 src/data/letter_templates.rs：
  LETTERS / LETTER_SYLLABLES / LETTER_TONES   -- 展示与播放
  LETTER_TEMPLATES[letter][sample][frame][24] -- 声学模板
  MFCC_STD[24]                                -- 全局白化系数（运行时必须一致）
  LETTER_F0[letter][sample][20]               -- 声调轮廓（cents，相对本句中位）

⚠ 任何特征参数变更必须同步 src/dtw.rs，否则分数全乱。

Run: pip install soundfile numpy scipy imageio_ffmpeg
"""
from __future__ import annotations

import json
import math
import subprocess
import time
import urllib.request
from pathlib import Path

import imageio_ffmpeg
import numpy as np
import soundfile as sf
from scipy import signal

FFMPEG = imageio_ffmpeg.get_ffmpeg_exe()

ROOT = Path(__file__).resolve().parent.parent
CDN = "https://hanyu-word-pinyin-short.cdn.bcebos.com"
AUDIO = ROOT / "assets" / "audio"
BD = AUDIO / "bd"
XDF = AUDIO / "xdf"
ASR = ROOT / "tools" / "_asr"
OUT = ROOT / "src" / "data" / "letter_templates.rs"

BAIDU_JSONS = [ROOT / "tools" / "baidu_audio.json",
               ROOT / "tools" / "baidu_triplet_tones.json"]

# (letter glyph, candidate syllables). ü written v for CDN.
LETTERS: list[tuple[str, list[str]]] = [
    ("b", ["bo"]), ("p", ["po"]), ("m", ["mo"]), ("f", ["fo", "fu"]),
    ("d", ["de"]), ("t", ["te", "ta"]), ("n", ["ne", "na"]), ("l", ["le"]),
    ("g", ["ge"]), ("k", ["ke"]), ("h", ["he"]),
    ("j", ["ji"]), ("q", ["qi"]), ("x", ["xi"]),
    ("zh", ["zhi"]), ("ch", ["chi"]), ("sh", ["shi"]), ("r", ["ri", "re"]),
    ("z", ["zi"]), ("c", ["ci"]), ("s", ["si"]),
    ("y", ["yi"]), ("w", ["wu"]),
    ("a", ["a"]), ("o", ["o"]), ("e", ["e"]),
    ("i", ["yi"]), ("u", ["wu"]), ("ü", ["yu", "v"]),
    ("ai", ["ai"]), ("ei", ["ei"]), ("ui", ["wei", "ui"]),
    ("ao", ["ao"]), ("ou", ["ou"]),
    ("iu", ["you", "iu"]), ("ie", ["ye", "ie"]),
    ("üe", ["yue", "ve"]), ("er", ["er"]),
    ("an", ["an"]), ("en", ["en"]), ("in", ["yin", "in"]),
    ("un", ["wen", "un"]), ("ün", ["yun", "vn"]),
    ("ang", ["ang"]), ("eng", ["eng"]),
    ("ing", ["ying", "ing"]), ("ong", ["weng", "ong"]),
]

TONE_ORDER = [1, 4, 2, 3]

# MFCC config - MUST match src/dtw.rs MfccConfig::default()
SR = 16_000
N_MELS = 26
N_CEPS = 13
FRAME = 400
HOP = 160
FFT = 512
PRE = 0.97
MAX_FRAMES = 45
FEATURE_DIM = 24       # (13-1) static + (13-1) delta
F0_POINTS = 20
MAX_TEMPLATES = 3
FRAME_MS = 20

# 声母呼读音等映射（与 lib.rs initial_audio() 一致）
INITIAL_AUDIO = {
    "d": "bd/de2.mp3", "t": "bd/te4.mp3", "r": "bd/ri4.mp3",
    **{k: f"xdf/shengmu/{k}.mp3" for k in (
        "b", "p", "m", "f", "n", "l", "g", "k", "h",
        "j", "q", "x", "zh", "ch", "sh", "z", "c", "s", "y", "w",
    )},
}

# 字母 -> 呼读音节，用于从 bd/ 找同字母其它声调样本
SYLLABLE_HINT = {
    "b": "bo", "p": "po", "m": "mo", "f": "fo", "d": "de", "t": "te",
    "n": "ne", "l": "le", "g": "ge", "k": "ke", "h": "he",
    "j": "ji", "q": "qi", "x": "xi", "zh": "zhi", "ch": "chi",
    "sh": "shi", "r": "ri", "z": "zi", "c": "ci", "s": "si",
    "y": "yi", "w": "wu", "ü": "yu", "üe": "yue", "ün": "yun",
}


def load_baidu_mask() -> dict[str, int]:
    mask: dict[str, int] = {}
    for path in BAIDU_JSONS:
        data = json.loads(path.read_text(encoding="utf-8"))
        for syl, tones in data.items():
            m = 0
            for t in tones:
                if 1 <= t <= 4:
                    m |= 1 << (t - 1)
            mask[syl] = mask.get(syl, 0) | m
    return mask


def url_for(s: str, tone: int) -> str:
    return f"{CDN}/{s}{tone}.mp3"


def ensure_audio(item: tuple[str, list[str]]) -> tuple[str, Path | None, str, int]:
    letter, candidates = item
    for syl in candidates:
        for tone in TONE_ORDER:
            dst = BD / f"{syl}{tone}.mp3"
            if dst.exists() and dst.stat().st_size > 1000:
                return letter, dst, "cached", tone
    last = "no candidate"
    for syl in candidates:
        for tone in TONE_ORDER:
            dst = BD / f"{syl}{tone}.mp3"
            for attempt in range(2):
                try:
                    with urllib.request.urlopen(url_for(syl, tone), timeout=15) as r:
                        data = r.read()
                    if len(data) < 1000:
                        raise ValueError(f"too small: {len(data)}B")
                    dst.write_bytes(data)
                    return letter, dst, f"{syl}{tone} {len(data)}B", tone
                except Exception as e:  # noqa: BLE001 - network retry
                    last = str(e)
                    if attempt + 1 < 2:
                        time.sleep(0.3)
    return letter, None, last, 1


def pick_source(letter: str, mask_map: dict[str, int]) -> tuple[Path | None, str]:
    """Exactly what speak(letter, ...) plays -- must be template #1."""
    if letter in INITIAL_AUDIO:
        p = AUDIO / INITIAL_AUDIO[letter]
        if p.exists() and p.stat().st_size > 1000:
            return p, INITIAL_AUDIO[letter]
        return None, f"missing {INITIAL_AUDIO[letter]}"
    m = mask_map.get(letter, 0)
    if m & 0b0001:
        p = BD / f"{letter}1.mp3"
        if p.exists() and p.stat().st_size > 1000:
            return p, f"{letter}1.mp3 (baidu mask={m})"
    p = AUDIO / f"{letter}.m4a"
    if p.exists() and p.stat().st_size > 1000:
        return p, f"{letter}.m4a (local TTS, mask={m})"
    return None, f"no source for {letter} (mask={m})"


def collect_sources(letter: str, primary: Path | None) -> list[tuple[Path, str]]:
    """最多 MAX_TEMPLATES 个样本：primary 优先，再补不同来源/声调。

    ⚠ 用精确文件名匹配，glob(f"{letter}*.mp3") 会把 zh 匹配进 z、ang 进 an。
    """
    seen: set[Path] = set()
    out: list[tuple[Path, str]] = []

    def add(p: Path | None, tag: str) -> None:
        if p is None or p in seen or not p.exists() or p.stat().st_size <= 1000:
            return
        if len(out) >= MAX_TEMPLATES:
            return
        seen.add(p)
        out.append((p, tag))

    add(primary, "playback")

    syl = SYLLABLE_HINT.get(letter, letter)
    for tone in TONE_ORDER:
        add(BD / f"{syl}{tone}.mp3", f"bd:{syl}{tone}")
    for letter_key in (letter, syl):
        for grp in ("shengmu", "zimu", "danyun", "fuyun"):
            add(XDF / grp / f"{letter_key}.mp3", f"xdf:{grp}/{letter_key}")
    for p in sorted(ASR.glob(f"*_{letter}_*.wav")):
        add(p, f"asr:{p.stem}")
    return out


def load_pcm(path: Path) -> np.ndarray:
    if path.suffix.lower() == ".m4a":
        raw = subprocess.run(
            [FFMPEG, "-v", "error", "-i", str(path), "-f", "f32le",
             "-ac", "1", "-ar", str(SR), "-"],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True,
        ).stdout
        return np.frombuffer(raw, dtype="<f4").copy()
    data, sr = sf.read(str(path), dtype="float32")
    if data.ndim > 1:
        data = data.mean(axis=1)
    if sr != SR:
        g = math.gcd(SR, sr)
        data = signal.resample_poly(data, SR // g, sr // g).astype("float32")
    return data


def trim_silence(pcm: np.ndarray) -> np.ndarray:
    win = int(SR * FRAME_MS / 1000)
    if len(pcm) < win * 3:
        return pcm
    n = len(pcm) // win
    energy = np.array([
        np.sqrt(np.mean(pcm[i * win:(i + 1) * win] ** 2)) for i in range(n)
    ])
    peak = float(energy.max())
    if peak <= 1e-6:
        return pcm
    thresh = max(peak * 0.12, 1e-5)
    active = np.where(energy >= thresh)[0]
    if len(active) == 0 or active[-1] - active[0] < 2:
        return pcm
    a = max(0, active[0] - 1) * win
    b = min(n, active[-1] + 2) * win
    return pcm[a:b]


def hz_to_mel(hz: float) -> float:
    return 2595.0 * math.log10(1.0 + hz / 700.0)


def mel_to_hz(mel: float) -> float:
    return 700.0 * (10.0 ** (mel / 2595.0) - 1.0)


def mel_filterbank() -> np.ndarray:
    low, high = hz_to_mel(60.0), hz_to_mel(SR / 2)
    pts = []
    for i in range(N_MELS + 2):
        mel = low + (high - low) * i / (N_MELS + 1)
        hz = mel_to_hz(mel)
        bin_ = math.floor((FFT + 1) * hz / SR)
        pts.append(min(bin_, FFT // 2))
    bank = np.zeros((N_MELS, FFT // 2 + 1), dtype="float32")
    for m in range(1, N_MELS + 1):
        left, center, right = pts[m - 1], pts[m], pts[m + 1]
        for k in range(left, center):
            if center > left:
                bank[m - 1, k] = (k - left) / (center - left)
        for k in range(center, right):
            if right > center:
                bank[m - 1, k] = (right - k) / (right - center)
    return bank


def dct_matrix() -> np.ndarray:
    mat = np.zeros((N_CEPS, N_MELS), dtype="float32")
    s0 = math.sqrt(1.0 / N_MELS)
    for c in range(N_CEPS):
        scale = s0 if c == 0 else math.sqrt(2.0 / N_MELS)
        for m in range(N_MELS):
            mat[c, m] = scale * math.cos(math.pi * c * (m + 0.5) / N_MELS)
    return mat


BANK = mel_filterbank()
DCT = dct_matrix()
WIN = np.array(
    [0.54 - 0.46 * math.cos(2 * math.pi * i / (FRAME - 1)) for i in range(FRAME)],
    dtype="float32",
)


def raw_ceps(pcm: np.ndarray) -> np.ndarray:
    """13 维 MFCC，不做 CMN。ft dexact 镜像 dtw.rs 的 raw 阶段。"""
    if len(pcm) < FRAME:
        return np.zeros((0, N_CEPS), dtype="float32")
    pre = np.empty_like(pcm)
    pre[0] = pcm[0]
    pre[1:] = pcm[1:] - PRE * pcm[:-1]
    frames = []
    start = 0
    while start + FRAME <= len(pre):
        seg = pre[start:start + FRAME] * WIN
        spec = np.fft.rfft(seg, n=FFT)
        power = (spec.real ** 2 + spec.imag ** 2).astype("float32")
        log_mel = np.log(np.maximum(power @ BANK.T, 1e-10))
        frames.append(DCT @ log_mel)
        start += HOP
    return np.stack(frames) if frames else np.zeros((0, N_CEPS), dtype="float32")


def delta(m: np.ndarray) -> np.ndarray:
    if len(m) < 2:
        return np.zeros_like(m)
    d = np.zeros_like(m)
    d[0] = m[1] - m[0]
    d[-1] = m[-1] - m[-2]
    if len(m) > 2:
        d[1:-1] = (m[2:] - m[:-2]) * 0.5
    return d


def enhance(ceps: np.ndarray, std: np.ndarray | None) -> np.ndarray:
    """c1..c12 + delta + 逐句 CMN + (可选)全局白化 → 24 维。"""
    if len(ceps) == 0:
        return np.zeros((0, FEATURE_DIM), dtype="float32")
    base = ceps[:, 1:]                 # drop c0 (energy)
    x = np.hstack([base, delta(base)]) # 24 dims
    x = x - x.mean(axis=0, keepdims=True)  # per-utterance CMN
    if std is not None:
        x = x / std
    return x.astype("float32")


def cap(f: np.ndarray, k: int = MAX_FRAMES) -> np.ndarray:
    if f.shape[0] > k:
        step = f.shape[0] / k
        f = f[[int(j * step) for j in range(k)]]
    return f


def f0_contour(pcm: np.ndarray) -> np.ndarray:
    """NSDF 自相关提 F0 → 相对音分 → 重采样 F0_POINTS 点。与 audio_tone.rs 同参。"""
    frame, hop = 512, 160
    min_lag, max_lag = int(SR / 450), int(SR / 70)
    f0: list[float] = []
    for start in range(0, max(0, len(pcm) - frame + 1), hop):
        seg = pcm[start:start + frame]
        mean = float(seg.mean())
        energy = float(np.mean((seg - mean) ** 2))
        if math.sqrt(energy) < 0.012:
            f0.append(0.0)
            continue
        w = np.array([0.54 - 0.46 * math.cos(2 * math.pi * i / (frame - 1))
                      for i in range(frame)], dtype="float32")
        buf = (seg - mean) * w
        denom = float(np.dot(buf, buf))
        if denom < 1e-9:
            f0.append(0.0)
            continue
        best, best_v = min_lag, -1.0
        for lag in range(min_lag, max_lag + 1):
            num = float(np.dot(buf[:-lag], buf[lag:]))
            v = num / math.sqrt(max(denom * float(np.dot(buf[lag:], buf[lag:])), 1e-9))
            if v > best_v:
                best, best_v = lag, v
        if best_v < 0.55:
            f0.append(0.0)
            continue
        y1 = float(np.dot(buf[:-max(best - 1, 1)], buf[max(best - 1, 1):])) / denom if best > 1 else 0.0
        y3 = float(np.dot(buf[:-(best + 1)], buf[best + 1:])) / denom if best + 1 < len(buf) else 0.0
        den = y1 - 2 * best_v + y3
        delta_lag = 0.5 * (y1 - y3) / den if abs(den) > 1e-9 else 0.0
        lag_f = best + max(-1.0, min(1.0, delta_lag))
        f0.append(SR / lag_f if lag_f > 0 else 0.0)

    voiced = sorted(v for v in f0 if v > 0)
    out = np.zeros(F0_POINTS, dtype="float32")
    if not voiced:
        return out
    base = voiced[len(voiced) // 2]
    cents = np.array([1200.0 * math.log2(v / base) if v > 0 else np.nan for v in f0])
    valid = [float(v) for v in cents if math.isfinite(v)]
    if not valid:
        return out
    idx = np.linspace(0, len(valid) - 1, F0_POINTS).astype(int)
    return np.array([valid[i] for i in idx], dtype="float32")


def fmt_matrix(m: np.ndarray, indent: str = "        ", decimals: int = 4) -> str:
    if m.shape[0] == 0:
        return "&[]"
    lines = []
    for row in m:
        vals = ",".join(f"{float(v):.{decimals}f}" for v in row)
        lines.append(f"{indent}&[{vals}]")
    return "&[\n" + ",\n".join(lines) + f"\n{indent[:-4]}]"


def fmt_vec(v: np.ndarray, indent: str = "    ", decimals: int = 4) -> str:
    return "&[" + ",".join(f"{float(x):.{decimals}f}" for x in v) + "]"


def main() -> None:
    masks = load_baidu_mask()
    # 第一遍：确定每个字母的样本来源
    plan: list[tuple[str, str, str, list[tuple[Path, str]]]] = []
    all_ceps: list[np.ndarray] = []
    for letter, _cands in LETTERS:
        primary, info = pick_source(letter, masks)
        if primary is None:
            print(f"  FAIL {letter}: {info}")
            continue
        samples = collect_sources(letter, primary)
        plan.append((letter, letter, "1", samples))
        for p, _tag in samples:
            try:
                all_ceps.append(raw_ceps(trim_silence(load_pcm(p))))
            except Exception as e:  # noqa: BLE001
                print(f"  skip {p.name}: {e}")

    # 全局 std：所有样本、所有帧、无 CMN 的 c1..c12+delta
    stat = np.vstack([c[:, 1:] for c in all_ceps if len(c)])
    dlt = np.vstack([delta(c[:, 1:]) for c in all_ceps if len(c)])
    std = np.concatenate([stat.std(axis=0), dlt.std(axis=0)])
    std[std < 1e-3] = 1.0
    print(f"global std computed from {len(stat)} frames, dim={len(std)}")

    headers = [
        "// Generated by tools/build_letter_templates.py -- do not edit by hand.",
        f"// {FEATURE_DIM}-dim enhanced MFCC (c1..c12 + delta + CMN + whitening).",
        "// Params MUST mirror src/dtw.rs MfccConfig::default() + MFCC_STD.",
        "",
    ]
    decl: list[str] = []
    decl += ["/// Letter glyphs.", "pub const LETTERS: &[&str] = &["]
    decl += [f'    "{l}",' for l, _, _, _ in plan]
    decl.append("];")
    decl += ["/// Playback token == glyph.", "pub const LETTER_SYLLABLES: &[&str] = &["]
    decl += [f'    "{l}",' for l, _, _, _ in plan]
    decl.append("];")
    decl += ["/// Default tone (呼读音 = first tone).", "pub const LETTER_TONES: &[u8] = &["]
    decl += [f"    {t}," for _, _, t, _ in plan]
    decl.append("];")
    decl += [f"/// Global whitening std ({FEATURE_DIM} dims).", f"pub const MFCC_STD: &[f32; {FEATURE_DIM}] = &"]
    decl.append(fmt_vec(std, "    ") + ";")
    decl.append("")

    decl += ["/// Acoustic templates [letter][sample][frame][24].",
             "pub static LETTER_TEMPLATES: &[&[&[&[f32]]]] = &["]
    for letter, _, _, samples in plan:
        decl.append(f"    // {letter}: {len(samples)} samples")
        decl.append("    &[")
        for p, tag in samples:
            ceps = raw_ceps(trim_silence(load_pcm(p)))
            feat = cap(enhance(ceps, std))
            decl.append(f"        // {tag}")
            decl.append("        " + fmt_matrix(feat) + ",")
        decl.append("    ],")
    decl.append("];")
    decl.append("")

    decl += [f"/// F0 contour templates [letter][sample][{F0_POINTS}] (cents).",
             f"pub static LETTER_F0: &[&[&[f32; {F0_POINTS}]]] = &["]
    for letter, _, _, samples in plan:
        decl.append(f"    // {letter}")
        decl.append("    &[")
        for p, tag in samples:
            c = f0_contour(trim_silence(load_pcm(p)))
            decl.append("        " + fmt_vec(c, "        ") + ",")
        decl.append("    ],")
    decl.append("];")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(headers + decl) + "\n", encoding="utf-8")
    print(f"wrote {OUT} ({len(plan)} letters, {OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
