"""诚实评估：用没进模板池的独立样本复现三道门 + 综合分。

模板自己评自己 = 41/47 虚高。这里对每个字母找 1 个「没被 collect_sources
选中、但确实是这个字母」的 bd 样本当探针，复现 Rust letter_match.rs 的
完整判定，打印 verdict / 音色分 / 声调分 / 综合分，定位"过不了"。

用法：python tools/diag_honest.py
"""
import sys, math
from pathlib import Path
import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
import build_letter_templates as B

ROOT = B.ROOT
BD = B.BD


def template_paths(letter):
    """复现 build 脚本 main() 里每个字母实际选中的模板文件集合。"""
    masks = B.load_baidu_mask()
    primary, _ = B.pick_source(letter, masks)
    return {p for p, _ in B.collect_sources(letter, primary)}


def rust_voice_span(pcm, sr=16000):
    win = max(1, sr * 20 // 1000)
    if len(pcm) < win * 3:
        return 0, len(pcm)
    n = len(pcm) // win
    e = np.array([np.sqrt(np.mean(pcm[i*win:(i+1)*win]**2)) for i in range(n)])
    peak = float(e.max())
    if peak < 0.012:
        return 0, 0
    thresh = max(peak * 0.18, 0.006)
    idx = np.where(e >= thresh)[0]
    if len(idx) == 0:
        return 0, 0
    a, b = int(idx[0]), int(idx[-1])
    return max(0, a-1)*win, min(n, b+2)*win


def zcr(pcm):
    if len(pcm) < 2:
        return 0.0
    return float(np.mean((pcm[:-1] < 0) != (pcm[1:] < 0)))


def dtw(a, b):
    n, m = len(a), len(b)
    if n == 0 or m == 0:
        return float("inf")
    prev = np.full(m+1, np.inf)
    cur = np.full(m+1, np.inf)
    prev[0] = 0.0
    for i in range(1, n+1):
        cur[0] = np.inf
        ai = a[i-1]
        d = np.linalg.norm(ai - b, axis=1)
        cur[1:] = d + np.minimum.reduce([prev[1:], cur[:-1], prev[:-1]])
        prev, cur = cur, prev
    return float(prev[m] / (n+m))


def main():
    # 复现全局 std（和 build main 第一遍一致）
    masks = B.load_baidu_mask()
    all_ceps = []
    plans = []
    for letter, _ in B.LETTERS:
        primary, _ = B.pick_source(letter, masks)
        if primary is None:
            continue
        samples = B.collect_sources(letter, primary)
        plans.append((letter, samples))
        for p, _ in samples:
            try:
                all_ceps.append(B.raw_ceps(B.trim_silence(B.load_pcm(p))))
            except Exception:
                pass
    stat = np.vstack([c[:, 1:] for c in all_ceps if len(c)])
    dlt = np.vstack([B.delta(c[:, 1:]) for c in all_ceps if len(c)])
    std = np.concatenate([stat.std(axis=0), dlt.std(axis=0)])
    std[std < 1e-3] = 1.0

    # 预计算所有字母的模板特征（cap 到 45 帧，和生成脚本一致）
    tmpl_feats = []
    for letter, samples in plans:
        per = []
        for p, _ in samples:
            c = B.raw_ceps(B.trim_silence(B.load_pcm(p)))
            per.append(B.cap(B.enhance(c, std)))
        tmpl_feats.append(per)

    ABS_MAX = 5.0
    MARGIN = 0.85
    SCALE = 6.0

    print(f"{'字母':>3} {'探针':<18} {'d_tgt':>6} {'d_riv':>6} {'音色':>5} {'声调':>5} {'综合':>5} {'verdict':<8} {'卡门'}")
    print("-" * 88)
    stats = {"pass": 0, "abs": 0, "rival": 0, "total": 0, "tone_zero": 0}
    overall_list = []
    for idx, (letter, samples) in enumerate(plans):
        syl = B.SYLLABLE_HINT.get(letter, letter)
        in_tpl = template_paths(letter)
        # 找没进模板的 bd/{syl}{tone}.mp3
        probe = None
        for tone in (1, 2, 3, 4):
            cand = BD / f"{syl}{tone}.mp3"
            if cand.exists() and cand not in in_tpl and cand.stat().st_size > 1000:
                probe = cand
                break
        if probe is None:
            continue
        try:
            pcm = B.trim_silence(B.load_pcm(probe))
        except Exception:
            continue
        a, b = rust_voice_span(pcm)
        if b <= a:
            continue
        voiced = pcm[a:b]
        c = B.raw_ceps(voiced)
        frames = B.cap(B.enhance(c, std))
        if len(frames) == 0 or zcr(voiced) > 0.35:
            continue

        d_tgt = min(dtw(frames, t) for t in tmpl_feats[idx])
        d_riv = min(
            min(dtw(frames, t) for t in tmpl_feats[j])
            for j in range(len(tmpl_feats)) if j != idx
        )
        timbre = 1.0 / (1.0 + d_tgt / SCALE)
        tone = 0.0  # 稍后单独算
        overall = timbre * 0.7 + tone * 0.3

        if not math.isfinite(d_tgt) or d_tgt > ABS_MAX:
            verdict, gate = "Retry", "绝对门"
            stats["abs"] += 1
        elif d_tgt > d_riv * MARGIN:
            verdict, gate = "Retry", "竞争门"
            stats["rival"] += 1
        else:
            verdict, gate = "Pass", "-"
            stats["pass"] += 1
        stats["total"] += 1
        overall_list.append(overall)
        print(f"{letter:>3} {probe.name:<18} {d_tgt:6.2f} {d_riv:6.2f} "
              f"{timbre:5.2f} {tone:5.2f} {overall:5.2f} {verdict:<8} {gate}")

    print("-" * 88)
    print(f"独立样本通过 {stats['pass']}/{stats['total']}  "
          f"绝对门拒 {stats['abs']}  竞争门拒 {stats['rival']}")
    if overall_list:
        print(f"综合分(min/med/max): {min(overall_list):.2f}/{np.median(overall_list):.2f}/{max(overall_list):.2f}")


if __name__ == "__main__":
    main()
