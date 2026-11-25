#!/usr/bin/env python3
"""为跟读判定扫描参数：scale / 绝对距离门 / 竞争容差。

场景：孩子要读目标字母 T，实际录音是探针 p（可能读成了别的字母）。
  正样本：p 属于 T（读对了）→ 应 Pass
  负样本：p 属于别的字母（读错了）→ 应 Retry
留一：p 自己的文件不进模板池。

判定规则：
  d_target = min(dtw(p, tpl) for tpl in templates[T])
  d_best, best_idx = min over all letters
  Pass iff d_target < ABS_MAX and d_target <= d_best * RATIO

输出：不同 (FEATURE, scale, ABS_MAX, RATIO) 下的 TP 率 / FP 率。
跑法: python tools/eval_params.py
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_letter_templates as B  # noqa: E402

BD = B.ROOT / "assets" / "audio" / "bd"
XDF = B.ROOT / "assets" / "audio" / "xdf"
ASR = B.ROOT / "tools" / "_asr"

SYL = {
    "b": "bo", "p": "po", "m": "mo", "f": "fo", "d": "de", "t": "te",
    "n": "ne", "l": "le", "g": "ge", "k": "ke", "h": "he",
    "j": "ji", "q": "qi", "x": "xi", "zh": "zhi", "ch": "chi",
    "sh": "shi", "r": "ri", "z": "zi", "c": "ci", "s": "si",
    "y": "yi", "w": "wu", "ü": "yu",
}


def sources(letter: str) -> list[Path]:
    out: list[Path] = []
    syl = SYL.get(letter, letter)
    for t in range(1, 5):
        p = BD / f"{syl}{t}.mp3"
        if p.exists() and p.stat().st_size > 1000:
            out.append(p)
    for grp in ("shengmu", "zimu", "danyun", "fuyun"):
        d = XDF / grp
        if not d.exists():
            continue
        for fname in (f"{letter}.mp3", f"{letter}_yisheng.mp3",
                      f"{letter}_ersheng.mp3", f"{letter}_sansheng.mp3",
                      f"{letter}_sisheng.mp3"):
            p = d / fname
            if p.exists() and p.stat().st_size > 1000:
                out.append(p)
    for p in ASR.glob(f"*_{letter}_*.wav"):
        out.append(p)
    return sorted(set(out))


def dtw(a: np.ndarray, b: np.ndarray) -> float:
    n, m = len(a), len(b)
    if n == 0 or m == 0:
        return float("inf")
    prev = np.full(m + 1, np.inf, dtype=np.float64)
    cur = np.full(m + 1, np.inf, dtype=np.float64)
    prev[0] = 0.0
    for i in range(1, n + 1):
        cur[0] = np.inf
        ai = a[i - 1]
        for j in range(1, m + 1):
            cur[j] = float(np.linalg.norm(ai - b[j - 1])) + min(prev[j], cur[j - 1], prev[j - 1])
        prev, cur = cur, prev
    return prev[m] / (n + m)


def cap(f: np.ndarray, k: int = 45) -> np.ndarray:
    if f.shape[0] > k:
        step = f.shape[0] / k
        f = f[[int(j * step) for j in range(k)]]
    return f


def delta(m: np.ndarray) -> np.ndarray:
    if len(m) < 2:
        return np.zeros_like(m)
    d = np.zeros_like(m)
    d[0] = m[1] - m[0]
    d[-1] = m[-1] - m[-2]
    if len(m) > 2:
        d[1:-1] = (m[2:] - m[:-2]) * 0.5
    return d


def raw_mfcc(p: Path) -> np.ndarray:
    return cap(B.mfcc(B.trim_silence(B.load_pcm(p))))


def make_variants(raw: dict[Path, np.ndarray]):
    stat = np.vstack([m[:, 1:] for m in raw.values() if len(m)])
    dlt = np.vstack([delta(m[:, 1:]) for m in raw.values() if len(m)])
    std = np.concatenate([stat.std(axis=0), dlt.std(axis=0)])
    std[std < 1e-3] = 1.0

    def feat(m: np.ndarray, kind: str) -> np.ndarray:
        if kind == "static":
            return m
        if kind == "static_noc0":
            return m[:, 1:]
        if kind == "delta":
            return np.hstack([m[:, 1:], delta(m[:, 1:])])
        if kind == "delta_cmvn":
            x = np.hstack([m[:, 1:], delta(m[:, 1:])])
            return (x - x.mean(axis=0, keepdims=True)) / std
        raise ValueError(kind)

    return {k: {p: feat(m, k) for p, m in raw.items() if len(m)}
            for k in ("static", "static_noc0", "delta", "delta_cmvn")}


def main() -> None:
    letters = [l for l, _ in B.LETTERS]
    pool = {l: sources(l) for l in letters}
    raw: dict[Path, np.ndarray] = {}
    for ps in pool.values():
        for p in ps:
            if p not in raw:
                try:
                    raw[p] = raw_mfcc(p)
                except Exception:  # noqa: BLE001
                    pass
    feats = make_variants(raw)
    print(f"letters={len(letters)} unique_audio={len(raw)}")

    # 预计算每个特征变体的距离矩阵，避免重复 DTW
    for kind, F in feats.items():
        ps = list(F)
        idx = {p: i for i, p in enumerate(ps)}
        n = len(ps)
        D = np.full((n, n), np.inf)
        for i in range(n):
            for j in range(i + 1, n):
                d = dtw(F[ps[i]], F[ps[j]])
                D[i, j] = D[j, i] = d

        # 探针 → 目标字母的距离数组
        positives: list[tuple[int, np.ndarray]] = []
        negatives: list[tuple[int, np.ndarray]] = []
        for l in letters:
            for p in pool[l]:
                if p not in idx:
                    continue
                row = D[idx[p]].copy()
                for q in pool[l]:
                    if q in idx and q != p:
                        row[idx[q]] = np.inf  # 留一：排除同源
                dvec = np.array([min((row[idx[q]] for q in pool[t] if q in idx and q != p),
                                     default=np.inf) for t in letters])
                positives.append((letters.index(l), dvec))
                for t in letters:
                    if t != l:
                        negatives.append((letters.index(t), dvec))

        print(f"\n=== {kind} ===  pos={len(positives)} neg={len(negatives)}")
        # 先看距离分布
        dp = np.array([d[t] for t, d in positives if np.isfinite(d[t])])
        dn = np.array([d[t] for t, d in negatives if np.isfinite(d[t])])
        def pct(a, q):
            return f"{np.percentile(a, q):.2f}" if len(a) else "n/a"
        print(f"  d_target(pos): p10={pct(dp,10)} p50={pct(dp,50)} p90={pct(dp,90)}")
        print(f"  d_target(neg): p10={pct(dn,10)} p50={pct(dn,50)} p90={pct(dn,90)}")

        best = None
        for abs_max in (3.0, 3.5, 4.0, 4.5, 5.0, 6.0, 8.0):
            for ratio in (1.0, 1.05, 1.1, 1.15, 1.2, 1.3, 1.5, 99.0):
                tp = sum(1 for t, d in positives if d[t] < abs_max and d[t] <= d.min() * ratio)
                fp = sum(1 for t, d in negatives if d[t] < abs_max and d[t] <= d.min() * ratio)
                tpr = tp / max(len(positives), 1)
                fpr = fp / max(len(negatives), 1)
                # 目标：TP 高、FP 低
                score = tpr - 2 * fpr
                if best is None or score > best[0]:
                    best = (score, abs_max, ratio, tpr, fpr)
        print(f"  最优 abs_max={best[1]:.1f} ratio={best[2]:.2f} "
              f"TPR={best[3]*100:.1f}% FPR={best[4]*100:.1f}%")

        # 打印几个代表组合
        for abs_max, ratio in ((4.0, 1.1), (4.5, 1.15), (5.0, 1.2), (6.0, 99.0)):
            tp = sum(1 for t, d in positives if d[t] < abs_max and d[t] <= d.min() * ratio)
            fp = sum(1 for t, d in negatives if d[t] < abs_max and d[t] <= d.min() * ratio)
            print(f"    abs={abs_max:.1f} ratio={ratio:>5.2f}: "
                  f"TPR={tp/max(len(positives),1)*100:5.1f}% FPR={fp/max(len(negatives),1)*100:5.1f}%")


if __name__ == "__main__":
    main()
