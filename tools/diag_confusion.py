#!/usr/bin/env python3
"""易混淆音对区分度实验：为 letter_match 重构提供参数依据。

对每对易混淆字母 (A, B)，采样各自的真人录音，比较：
  - 类内距离：A 的不同录音之间
  - 类间距离：A 的录音 vs B 的录音
  - 分离比 = 类间最小 / 类内均值（越大越好）

并比较 4 种特征：原始13维 / 去c0 / 去c0+delta / 去c0+delta+CMVN。

跑法: python tools/diag_confusion.py
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_letter_templates as B  # noqa: E402

ROOT = B.ROOT
BD = ROOT / "assets" / "audio" / "bd"
XDF = ROOT / "assets" / "audio" / "xdf"
ASR = ROOT / "tools" / "_asr"

PAIRS = [
    ("b", "w"), ("b", "p"), ("d", "g"), ("t", "k"),
    ("z", "zh"), ("c", "ch"), ("s", "sh"), ("n", "l"),
    ("an", "ang"), ("en", "eng"), ("in", "ing"), ("o", "u"),
]

# 字母 -> 呼读音节（用于找 bd 声调文件）
SYL = {
    "b": "bo", "p": "po", "d": "de", "t": "te", "g": "ge", "k": "ke",
    "z": "zi", "zh": "zhi", "c": "ci", "ch": "chi", "s": "si", "sh": "shi",
    "n": "ne", "l": "le", "w": "wu",
}


def sources(letter: str) -> list[Path]:
    out = []
    syl = SYL.get(letter, letter)
    for t in range(1, 5):
        p = BD / f"{syl}{t}.mp3"
        if p.exists() and p.stat().st_size > 1000:
            out.append(p)
    for grp in ("shengmu", "zimu", "danyun", "fuyun"):
        d = XDF / grp
        if not d.exists():
            continue
        # 精确 stem：glob(f"{letter}*.mp3") 会把 zh 匹配进 z、ang 匹配进 an。
        for fname in (f"{letter}.mp3", f"{letter}_yisheng.mp3"):
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
            c = float(np.linalg.norm(ai - b[j - 1]))
            cur[j] = c + min(prev[j], cur[j - 1], prev[j - 1])
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


def cmvn(m: np.ndarray, std: np.ndarray) -> np.ndarray:
    return (m - m.mean(axis=0, keepdims=True)) / std


VARIANTS = {
    "A 原始13维": lambda m, std: m,
    "B 去c0": lambda m, std: m[:, 1:],
    "C 去c0+delta": lambda m, std: np.hstack([m[:, 1:], delta(m[:, 1:])]),
    "D 去c0+delta+CMVN": lambda m, std: cmvn(
        np.hstack([m[:, 1:], delta(m[:, 1:])]), std
    ),
}


def main() -> None:
    feats: dict[Path, np.ndarray] = {}
    pool: dict[str, list[Path]] = {}
    for a, b in PAIRS:
        for l in (a, b):
            if l in pool:
                continue
            pool[l] = sources(l)
            for p in pool[l]:
                if p in feats:
                    continue
                try:
                    feats[p] = cap(B.mfcc(B.trim_silence(B.load_pcm(p))))
                except Exception as e:  # noqa: BLE001
                    print(f"  skip {p.name}: {e}")

    allf = np.vstack([f for f in feats.values() if len(f)])
    # 用「去c0+delta」后的维度估全局 std（CMVN 用）
    stacked = np.hstack([m[:, 1:] for m in feats.values() if len(m)]
                        + [delta(m[:, 1:]) for m in feats.values() if len(m)]) if False else None
    # 全局 std：静态 12 维 + delta 12 维 = 24 维
    stat = np.vstack([m[:, 1:] for m in feats.values() if len(m)])
    dlt = np.vstack([delta(m[:, 1:]) for m in feats.values() if len(m)])
    std = np.concatenate([stat.std(axis=0), dlt.std(axis=0)])
    std[std < 1e-3] = 1.0

    for name, prep in VARIANTS.items():
        t = {p: prep(f, std) for p, f in feats.items() if len(f)}
        print(f"\n=== {name} ===")
        for a, b in PAIRS:
            fa = [t[p] for p in pool[a] if p in t]
            fb = [t[p] for p in pool[b] if p in t]
            if len(fa) < 2 or len(fb) < 1:
                print(f"  {a:>3}/{b:<3} 样本不足 ({len(fa)}/{len(fb)})")
                continue
            within = [dtw(fa[i], fa[j]) for i in range(len(fa)) for j in range(i + 1, len(fa))]
            across = [dtw(x, y) for x in fa for y in fb]
            wi = min(within) if within else 0.0
            wm = float(np.mean(within)) if within else 1.0
            am = min(across)
            ratio = am / max(wi, 1e-6)
            flag = "OK" if am > wm else "!!"
            print(f"  {flag} {a:>3}/{b:<3} n={len(fa)}/{len(fb)} "
                  f"within_min={wi:.2f} within_mean={wm:.2f} across_min={am:.2f} "
                  f"ratio={ratio:.2f}")


if __name__ == "__main__":
    main()
