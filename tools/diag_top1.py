#!/usr/bin/env python3
"""公平口径：干净呼读音节，留一，算闭集 top1 命中率。

每个样本当 probe：
  d_self  = 到同字母其它样本的最小距离
  d_rival = 到其它字母所有样本的最小距离
命中 = d_self < d_rival。

只有这个指标才回答"竞争门/闭集判定能不能用"。
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import diag_calibrated as C
from diag_confusion import dtw

# 预计算所有特征
feats: dict[str, list[tuple[str, np.ndarray]]] = {}
for letter, ss in C.samples.items():
    feats[letter] = [(p.name, f) for p, f in ss]

print(f"类别数={len(feats)} 总样本={sum(len(v) for v in feats.values())}")

hit = 0
total = 0
wrong_cases = []
# 每个 probe
for target, samples in feats.items():
    for pname, pf in samples:
        d_self = min((dtw(pf, tf) for tn, tf in samples if tn != pname), default=float("inf"))
        d_rival = float("inf")
        rival_letter = None
        for other, oss in feats.items():
            if other == target:
                continue
            for _, of in oss:
                dv = dtw(pf, of)
                if dv < d_rival:
                    d_rival = dv
                    rival_letter = other
        if not np.isfinite(d_self):
            continue
        total += 1
        if d_self < d_rival:
            hit += 1
        else:
            wrong_cases.append((target, pname, rival_letter, round(d_self, 2), round(d_rival, 2)))

print(f"\n闭集 top1 命中率: {hit}/{total} = {hit/total:.1%}")
print("\n误判样本 (目标/探针/被认成/d_self/d_rival):")
for w in wrong_cases[:20]:
    print("  ", w)
