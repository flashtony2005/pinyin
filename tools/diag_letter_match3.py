"""留一验证：每字母用「多份录音」当模板，看最近邻辨认的上限。

对比三种特征：
  A 原始 13 维 MFCC
  B 去 c0
  C 去 c0 + delta + 维度白化
并对每个探针统计「目标与最佳错误候选」的差距，用来看 margin 判定是否可行。
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_letter_templates as B  # noqa: E402

BD = B.ROOT / "assets" / "audio" / "bd"
XDF = B.ROOT / "assets" / "audio" / "xdf"


def dtw(a, b):
    n, m = len(a), len(b)
    prev = np.full(m + 1, np.inf)
    cur = np.full(m + 1, np.inf)
    prev[0] = 0.0
    for i in range(1, n + 1):
        cur[0] = np.inf
        for j in range(1, m + 1):
            cur[j] = float(np.linalg.norm(a[i - 1] - b[j - 1])) + min(prev[j], cur[j - 1], prev[j - 1])
        prev, cur = cur, prev
    return prev[m] / (n + m)


def cap(f, k=B.MAX_FRAMES):
    if f.shape[0] > k:
        step = f.shape[0] / k
        f = f[[int(j * step) for j in range(k)]]
    return f


def delta(m):
    d = np.zeros_like(m)
    if len(m) < 2:
        return d
    d[0] = m[1] - m[0]
    d[-1] = m[-1] - m[-2]
    if len(m) > 2:
        d[1:-1] = (m[2:] - m[:-2]) * 0.5
    return d


VARIANTS = {
    "A 原始13维": lambda m: m,
    "B 去c0": lambda m: m[:, 1:],
    "C 去c0+delta": lambda m: np.hstack([m[:, 1:], delta(m[:, 1:])]),
}


def main():
    # 收集每个字母所有可用录音
    pool = {}
    for letter, _ in B.LETTERS:
        files = []
        for p in sorted(BD.glob(f"{letter}[1-4].mp3")):
            files.append(p)
        for grp in ("zimu", "shengmu"):
            q = XDF / grp / f"{letter}.mp3"
            if q.exists():
                files.append(q)
        if files:
            pool[letter] = files

    letters = sorted(pool)
    print(f"letters with audio: {len(letters)}")
    total = sum(len(v) for v in pool.values())
    print(f"recordings: {total}")

    feats = {}
    for letter, files in pool.items():
        for p in files:
            try:
                feats[p] = cap(B.mfcc(B.trim_silence(B.load_pcm(p))))
            except Exception:
                pass

    for name, prep in VARIANTS.items():
        T = {l: [prep(feats[p]) for p in pool[l] if p in feats] for l in letters}
        T = {l: v for l, v in T.items() if v}
        allf = np.vstack([f for v in T.values() for f in v])
        std = allf.std(axis=0)
        std[std < 1e-3] = 1.0

        hits = 0
        tries = 0
        margins = []
        for l in T:
            for p in [q for q in pool[l] if q in feats]:
                # 留一：去掉这一条再辨认
                others = [f for q, f in zip([q for q in pool[l] if q in feats], T[l]) if q != p]
                if not others:
                    continue
                q = prep(feats[p]) / std
                best_d, best_l, second_d = np.inf, None, np.inf
                for cand, tmpls in T.items():
                    ds = [dtw(q, t / std) for t in tmpls if not (cand == l and len(others) == 0)]
                    if cand == l:
                        ds = [dtw(q, f / std) for f in others]
                    if not ds:
                        continue
                    m = min(ds)
                    if m < best_d:
                        best_d, best_l, second_d = m, cand, best_d
                    elif m < second_d:
                        second_d = m
                tries += 1
                if best_l == l:
                    hits += 1
                    margins.append((second_d - best_d) / max(best_d, 1e-6))

        print(f"\n=== {name} 多模板留一 ===")
        print(f"  top1: {hits}/{tries} = {hits/tries*100:.1f}%")
        if margins:
            m = np.array(margins)
            print(f"  命中 margin: p5={np.percentile(m,5):.3f} p25={np.percentile(m,25):.3f} p50={np.median(m):.3f}")


if __name__ == "__main__":
    main()
