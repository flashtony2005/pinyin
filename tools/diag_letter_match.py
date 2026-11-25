"""诊断现有跟读识别：47 个模板两两打分，看阈值 0.52 是否形同虚设。

跑法: python tools/diag_letter_match.py
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_letter_templates as B  # noqa: E402


def dtw(a, b):
    n, m = len(a), len(b)
    if n == 0 or m == 0:
        return float("inf")
    prev = np.full(m + 1, np.inf, dtype=np.float64)
    cur = np.full(m + 1, np.inf, dtype=np.float64)
    prev[0] = 0.0
    for i in range(1, n + 1):
        cur[0] = np.inf
        for j in range(1, m + 1):
            c = float(np.linalg.norm(a[i - 1] - b[j - 1]))
            cur[j] = c + min(prev[j], cur[j - 1], prev[j - 1])
        prev, cur = cur, prev
    return prev[m] / (n + m)


def sim_current(a, b, scale=45.0):
    """现有实现：1/(1+d/scale)"""
    return 1.0 / (1.0 + dtw(a, b) / scale)


def drop_c0(m):
    return m[:, 1:]


def norm_frames(m):
    """每帧 L2 归一化（去掉整体音量），再按维度方差白化。"""
    x = m[:, 1:].copy()  # 去掉 c0（能量）
    n = np.linalg.norm(x, axis=1, keepdims=True)
    x = x / np.maximum(n, 1e-6)
    return x


def main():
    mask = B.load_baidu_mask()
    items = []
    for letter, _cands in B.LETTERS:
        path, info = B.pick_source(letter, mask)
        if path is None:
            print(f"  MISS {letter}: {info}")
            continue
        pcm = B.trim_silence(B.load_pcm(path))
        feats = B.mfcc(pcm)
        if feats.shape[0] > B.MAX_FRAMES:
            step = feats.shape[0] / B.MAX_FRAMES
            idx = [int(j * step) for j in range(B.MAX_FRAMES)]
            feats = feats[idx]
        items.append((letter, feats))

    letters = [l for l, _ in items]
    print(f"templates: {len(items)}")

    metrics = {
        "current(c0+scale45)": lambda m: m,
        "drop_c0": drop_c0,
        "drop_c0+frame_norm": norm_frames,
    }

    for name, prep in metrics.items():
        feats = [prep(f) for _, f in items]
        n = len(feats)
        S = np.zeros((n, n))
        for i in range(n):
            for j in range(n):
                S[i, j] = sim_current(feats[i], feats[j])
        off = S[~np.eye(n, dtype=bool)]
        best_wrong = np.array([max(S[i, j] for j in range(n) if j != i) for i in range(n)])
        # 相似度 = 1/(1+d/45)，反解出 d，看不同 scale 下的分数
        print(f"\n=== {name} ===")
        print(f"  off-diagonal sim: mean={off.mean():.3f} p50={np.median(off):.3f} "
              f"p95={np.percentile(off,95):.3f} max={off.max():.3f}")
        print(f"  best-wrong per target: mean={best_wrong.mean():.3f} "
              f"min={best_wrong.min():.3f} max={best_wrong.max():.3f}")
        worst = int(np.argmax(best_wrong))
        print(f"  最容易误判的目标: {letters[worst]} 被误判成 "
              f"{letters[int(np.argmax(S[worst]))]} sim={best_wrong[worst]:.3f}")
        # 现有阈值 0.52 下，多少个"字母读成另一个字母"会被判对
        falsely = int((best_wrong >= 0.52).sum())
        print(f"  阈值 0.52 下会把「读错字母」判对: {falsely}/{n} 个目标")

        # 换算原始距离，看分离度
        d = lambda s: 45.0 * (1.0 - s) / s if s > 0 else float("inf")
        ds = np.array([dtw(feats[i], feats[j]) for i in range(n) for j in range(n) if i != j])
        print(f"  raw d: mean={ds.mean():.2f} p5={np.percentile(ds,5):.2f} "
              f"min={ds.min():.2f} max={ds.max():.2f}")


if __name__ == "__main__":
    main()
