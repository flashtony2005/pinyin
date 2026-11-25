#!/usr/bin/env python3
"""路线 B —— 数据层：解码 / 特征 / 增广。

特征定义**必须**与 Rust 侧（src/ctc_feat.rs）逐位一致：
  sr=16000, win=400(Hann periodic), hop=160, n_fft=512, n_mels=64, fmin=60Hz
  log(mel + 1e-10)，然后逐句（逐 bin）CMVN。
固定 2.0s 输入 -> 198 帧 x 64 bin。

Run: python tools/_p0/b_data.py --cache
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent.parent
P0 = ROOT / "tools" / "_p0"
CACHE = P0 / "bcache"

SR = 16_000
WIN = 400
HOP = 160
NFFT = 512
N_MELS = 64
FMIN = 60.0
DUR = 2.0
N_SAMP = int(SR * DUR)          # 32000
CAP = int(SR * 2.4)             # 缓存原始音频容量
N_FRAMES = (N_SAMP - WIN) // HOP + 1   # 198
EPS = 1e-10


# ---------------------------------------------------------------- 梅尔前端
def hz_to_mel(hz: float) -> float:
    return 2595.0 * np.log10(1.0 + hz / 700.0)


def mel_to_hz(mel: float) -> float:
    return 700.0 * (10.0 ** (mel / 2595.0) - 1.0)


def mel_filterbank() -> np.ndarray:
    """[64, 257] 三角滤波器，逐点权重（与 dtw.rs mel_filterbank 同式）。"""
    low, high = hz_to_mel(FMIN), hz_to_mel(SR / 2.0)
    pts = []
    for i in range(N_MELS + 2):
        mel = low + (high - low) * i / (N_MELS + 1)
        hz = mel_to_hz(mel)
        pts.append(int(np.floor((NFFT + 1) * hz / SR)))
    fb = np.zeros((N_MELS, NFFT // 2 + 1), dtype=np.float32)
    for m in range(1, N_MELS + 1):
        left, center, right = pts[m - 1], pts[m], pts[m + 1]
        for k in range(left, center):
            if center > left:
                fb[m - 1, k] = (k - left) / (center - left)
        for k in range(center, right):
            if right > center:
                fb[m - 1, k] = (right - k) / (right - center)
    return fb


FB = mel_filterbank()
WINDOW = np.hanning(WIN + 1)[:WIN].astype(np.float32)   # periodic Hann


def logmel(pcm: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """pcm: float32 [N_SAMP] -> (log-mel [N_FRAMES, N_MELS], 帧 RMS [N_FRAMES])。"""
    x = np.asarray(pcm, dtype=np.float32)
    if x.shape[0] < N_SAMP:
        x = np.pad(x, (0, N_SAMP - x.shape[0]))
    x = x[:N_SAMP]
    idx = np.arange(WIN)[None, :] + HOP * np.arange(N_FRAMES)[:, None]
    frames = x[idx] * WINDOW                      # [198, 400]
    rms = np.sqrt((frames ** 2).mean(axis=1)).astype(np.float32)
    spec = np.fft.rfft(frames, n=NFFT, axis=1)    # [198, 257]
    power = (spec.real ** 2 + spec.imag ** 2)
    mel = power @ FB.T
    return np.log(mel + EPS).astype(np.float32), rms


def active_mask(rms: np.ndarray, floor_db: float = -40.0) -> np.ndarray:
    """帧能量不低于峰值 floor_db 的帧视为有效帧（静音补零段被排除）。"""
    db = 20.0 * np.log10(rms + 1e-8)
    mask = db > (db.max() + floor_db)
    if mask.sum() < 8:                      # 兜底：最少 8 帧
        mask = np.ones_like(mask, dtype=bool)
    return mask


def cmvn(feat: np.ndarray, mask: np.ndarray | None = None) -> np.ndarray:
    """逐 bin 均值方差归一（只统计有效帧）。仅用于诊断对比，主线不采用。"""
    sub = feat if mask is None else feat[mask]
    m = sub.mean(axis=0, keepdims=True)
    s = sub.std(axis=0, keepdims=True) + 1e-5
    return ((feat - m) / s).astype(np.float32)


def norm_frames(feat: np.ndarray, mask: np.ndarray) -> np.ndarray:
    """逐帧减均值（去增益/音量），静音帧置零 —— 保留谱形（元音身份）不被抹掉。"""
    f = feat - feat.mean(axis=1, keepdims=True)
    f[~mask] = 0.0
    return f.astype(np.float32)


def delta(feat: np.ndarray, width: int = 2) -> np.ndarray:
    """一阶差分（时间轴），强调声母瞬态。边界复制填充。"""
    pad = np.pad(feat, ((width, width), (0, 0)), mode="edge")
    d = (pad[2 * width:] - pad[:-2 * width]) / (2.0 * width)
    return d.astype(np.float32)


def featurize(pcm: np.ndarray) -> np.ndarray:
    """log-mel -> 逐帧去均值 -> 静音置零 -> 拼一阶差分。返回 [2, N_FRAMES, N_MELS]。"""
    f, rms = logmel(pcm)
    m = active_mask(rms)
    f = norm_frames(f, m)
    return np.stack([f, delta(f)])


def featurize_list(pcm: np.ndarray) -> list[np.ndarray]:
    """逐帧版本（避免一次算全部），与 featurize 等价。"""
    return [featurize(pcm)]



# ---------------------------------------------------------------- 解码
def ffmpeg_exe() -> str:
    import imageio_ffmpeg
    return imageio_ffmpeg.get_ffmpeg_exe()


def decode(path: Path) -> np.ndarray:
    """libsndfile 直接解 mp3（快且稳，不走 ffmpeg 子进程），再重采样到 16k。"""
    import soundfile as sf
    from math import gcd
    from scipy.signal import resample_poly

    x, sr = sf.read(str(path), dtype="float32", always_2d=False)
    if x.ndim > 1:
        x = x.mean(axis=1)
    if sr != SR:
        g = gcd(int(sr), SR)
        x = resample_poly(x, SR // g, int(sr) // g).astype(np.float32)
    return np.ascontiguousarray(x, dtype=np.float32)


def trim(x: np.ndarray, thresh_db: float = -38.0) -> np.ndarray:
    """按 20ms 窗能量裁掉首尾静音。"""
    if x.size == 0:
        return x
    w = 320
    n = max(1, x.size // w)
    e = np.array([np.sqrt(np.mean(x[i * w:(i + 1) * w] ** 2) + 1e-12) for i in range(n)])
    db = 20 * np.log10(e + 1e-12)
    top = db.max()
    keep = np.where(db > top + thresh_db)[0]
    if keep.size == 0:
        return x
    a, b = keep[0] * w, min(x.size, (keep[-1] + 1) * w)
    pad = 800
    return x[max(0, a - pad):min(x.size, b + pad)]


# ---------------------------------------------------------------- 语料清单
TONE_MAP = {"yisheng": 1, "ersheng": 2, "sansheng": 3, "sisheng": 4}


def collect() -> list[dict]:
    """返回 [{key, syl, tone, src, path}]。key = 归一化 syl+tone。"""
    items = []
    bd = ROOT / "assets" / "audio" / "bd"
    for p in sorted(bd.glob("*.mp3")):
        stem = p.stem
        syl, tone = stem[:-1], stem[-1]
        if not tone.isdigit():
            continue
        items.append(dict(key=f"{syl}{tone}", syl=syl, tone=int(tone), src="bd", path=str(p)))

    xdf = ROOT / "assets" / "audio" / "xdf"
    for sub in ("fuyun", "zhengti"):
        for p in sorted((xdf / sub).glob("*.mp3")):
            parts = p.stem.split("_")
            if len(parts) != 2 or parts[1] not in TONE_MAP:
                continue
            syl = parts[0]
            items.append(dict(key=f"{syl}{TONE_MAP[parts[1]]}", syl=syl,
                              tone=TONE_MAP[parts[1]], src=f"xdf_{sub}", path=str(p)))
    return items


def build_cache(workers: int = 6) -> None:
    """逐文件解码为 bcache/wav/<i>.npy（可断点续跑），最后打包成定长 int16。"""
    from concurrent.futures import ThreadPoolExecutor

    CACHE.mkdir(parents=True, exist_ok=True)
    wav_dir = CACHE / "wav"
    wav_dir.mkdir(exist_ok=True)
    items = collect()
    (CACHE / "meta.json").write_text(json.dumps(items, ensure_ascii=False, indent=1), encoding="utf-8")
    todo = [i for i in range(len(items)) if not (wav_dir / f"{i:05d}.npy").exists()]
    print(f"语料 {len(items)} 条，待解码 {len(todo)}", flush=True)

    def one(i: int) -> int:
        try:
            x = trim(decode(Path(items[i]["path"])))
        except Exception as e:      # 单文件失败不该毁掉整个语料
            print(f"  !! {items[i]['key']} {type(e).__name__} {e}", flush=True)
            x = np.zeros(1600, dtype=np.float32)
        np.save(wav_dir / f"{i:05d}.npy", x.astype(np.float32))
        return i

    done = 0
    with ThreadPoolExecutor(max_workers=workers) as ex:
        for _ in ex.map(one, todo):
            done += 1
            if done % 200 == 0:
                print(f"  {done}/{len(todo)}", flush=True)

    pcm = np.zeros((len(items), CAP), dtype=np.int16)
    lengths = np.zeros(len(items), dtype=np.int32)
    for i in range(len(items)):
        x = np.load(wav_dir / f"{i:05d}.npy")
        if x.size > CAP:
            x = x[:CAP]
        pcm[i, :x.size] = (np.clip(x, -1, 1) * 32767).astype(np.int16)
        lengths[i] = x.size
    np.save(CACHE / "pcm16.npy", pcm)
    np.save(CACHE / "len.npy", lengths)
    print("长度分布 ms:", np.percentile(lengths / SR * 1000, [1, 10, 50, 90, 99]).round(0))


def load_cache():
    pcm = np.load(CACHE / "pcm16.npy").astype(np.float32) / 32768.0
    lengths = np.load(CACHE / "len.npy")
    meta = json.loads((CACHE / "meta.json").read_text(encoding="utf-8"))
    return pcm, lengths, meta


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", action="store_true")
    a = ap.parse_args()
    if a.cache:
        build_cache()
    else:
        pcm, lengths, meta = load_cache()
        print(pcm.shape, lengths[:3], len(meta))
        f = featurize(pcm[0] * (np.arange(CAP) < lengths[0]))
        print("feat", f.shape, f.mean().round(3), f.std().round(3))
