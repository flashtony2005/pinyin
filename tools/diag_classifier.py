#!/usr/bin/env python3
"""决定性实验：分段时域/频谱特征 + 监督分类能否区分声母。

背景：纯 MFCC 全局 DTW 距离分不开读对/读错（实测 1.89 vs 1.92）。
本实验验证：保留时间结构的分段特征 + 分类器，闭集 top1 能到多少。

诚实原则：用 GroupKFold 按音节分组，同一音节的不同声调不会同时出现在
train 和 test，避免"自己评自己"的假高分。
"""
from __future__ import annotations

import re
import sys
from collections import defaultdict
from pathlib import Path

import numpy as np
from sklearn.ensemble import ExtraTreesClassifier
from sklearn.model_selection import GroupKFold, cross_val_score

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_letter_templates as B

SR = 16_000
N_SEG = 6

# 声母标签提取（zh/ch/sh 优先；a/o/e/i/u/ü/er 为零声母）
INITIALS = ["b","p","m","f","d","t","n","l","g","k","h","j","q","x",
            "zh","ch","sh","r","z","c","s","y","w"]
DIGRAPHS = ("zh", "ch", "sh")


def label_of(syllable: str) -> str | None:
    s = syllable.lower().replace("v", "ü")
    for d in DIGRAPHS:
        if s.startswith(d):
            return d
    if s and s[0] in INITIALS:
        return s[0]
    return "zero"  # 零声母韵母


def features(pcm: np.ndarray) -> np.ndarray | None:
    v = B.trim_silence(pcm)
    if len(v) < SR * 0.15:
        return None
    # 全局幅度归一化（说话人无关）
    peak = float(np.max(np.abs(v)))
    if peak < 1e-5:
        return None
    v = v / peak

    # 逐帧 MFCC(12, 去 c0)
    ceps = B.raw_ceps(v)[:, 1:]
    if len(ceps) < N_SEG:
        return None

    # 频谱/时域逐帧量
    N, HOP = 512, 160
    win = np.hamming(N)
    cent, zcr, flat, roll = [], [], [], []
    for st in range(0, len(v) - N + 1, HOP):
        seg = v[st:st + N]
        S = np.abs(np.fft.rfft(seg * win)) + 1e-9
        p = S / S.sum()
        freq = np.fft.rfftfreq(N, 1 / SR)
        cent.append(float((p * freq).sum()))
        zcr.append(float(np.mean(np.diff(np.signbit(seg)).astype(float))))
        flat.append(float(np.exp(np.log(S).mean()) / S.mean()))
        cum = np.cumsum(p)
        roll.append(float(freq[np.searchsorted(cum, 0.85)]))
    if len(cent) < N_SEG:
        return None
    frame_aux = np.array([cent, zcr, flat, roll]).T  # T x 4

    # 对齐帧数（ceps 和 aux 帧移相同，取公共长度）
    T = min(len(ceps), len(frame_aux))
    mat = np.hstack([ceps[:T], frame_aux[:T]])  # T x 16

    # 分 N_SEG 段，每段取均值（保留时间结构 —— 声母在开头！）
    out = []
    edges = np.linspace(0, T, N_SEG + 1).astype(int)
    for i in range(N_SEG):
        seg = mat[edges[i]:edges[i + 1]]
        if len(seg) == 0:
            seg = mat[-1:]
        out.extend(seg.mean(axis=0).tolist())
    # 全局能量包络形状（4 点）
    e = np.array([np.sqrt(np.mean(v[j:j+HOP**2//HOP]**2))
                  for j in range(0, len(v)-N+1, N)])
    if len(e) >= 4:
        eq = np.interp(np.linspace(0, len(e)-1, 4), np.arange(len(e)), e)
        eq = eq / (eq.mean() + 1e-9)
        out.extend(eq.tolist())
    return np.array(out, dtype=np.float32)


def main() -> None:
    X, y, groups = [], [], []
    for p in sorted(B.BD.glob("*.mp3")):
        m = re.match(r"^([a-z]+)(\d)$", p.stem)
        if not m:
            continue
        lab = label_of(m.group(1))
        if lab is None or lab == "zero":
            continue
        if p.stat().st_size < 1000:
            continue
        try:
            f = features(B.load_pcm(p))
        except Exception:
            continue
        if f is None:
            continue
        X.append(f)
        y.append(lab)
        groups.append(m.group(1))  # 按音节分组
    X = np.array(X)
    y = np.array(y)
    groups = np.array(groups)
    print(f"样本数={len(X)} 特征维={X.shape[1]} 类别={len(set(y))}")
    print("类别分布:", {k: int((y == k).sum()) for k in sorted(set(y))})

    clf = ExtraTreesClassifier(n_estimators=200, min_samples_leaf=2,
                               random_state=42, n_jobs=-1)
    cv = GroupKFold(n_splits=5)
    scores = cross_val_score(clf, X, y, groups=groups, cv=cv, scoring="accuracy")
    print(f"\n[分段特征] GroupKFold top1 准确率: {scores.mean():.3%} ± {scores.std():.3%}")
    print("  各折:", " ".join(f"{s:.3f}" for s in scores))

    # 对照：只用全局均值（旧方法等价，抹掉时间结构）
    Xg = np.array([np.concatenate([row[i*16:(i+1)*16] for i in range(1)]) for row in X])
    # 真实对照：所有段平均 = 全局 MFCC 统计
    Xg = np.array([row[:N_SEG*16].reshape(N_SEG, 16).mean(axis=0) for row in X])
    sg = cross_val_score(clf, Xg, y, groups=groups, cv=cv, scoring="accuracy")
    print(f"[全局平均特征] GroupKFold top1 准确率: {sg.mean():.3%} ± {sg.std():.3%}")


if __name__ == "__main__":
    main()
