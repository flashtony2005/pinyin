#!/usr/bin/env python3
"""决定性实验：用干净的呼读音节标签，测读对/乱读的距离分离度。

- 目标字母的呼读音（b→bo）样本 = 正样本（留一，排除模板自己）
- 其它字母的呼读音 = 乱读负样本
- 用与 Rust 一致的 24 维增强特征 + DTW
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_letter_templates as B
from diag_confusion import dtw

# 只取有 4 声调的干净呼读音节
PAIRS = {
    "b": "bo", "p": "po", "m": "mo", "z": "zi", "c": "ci", "s": "si",
    "zh": "zhi", "ch": "chi", "sh": "shi", "d": "de", "g": "ge", "k": "ke",
    "h": "he", "j": "ji", "q": "qi", "x": "xi", "y": "yi", "w": "wu",
    "a": "a", "o": "o", "e": "e", "an": "an", "ang": "ang", "en": "en",
}


def feat(path: Path) -> np.ndarray | None:
    pcm = B.trim_silence(B.load_pcm(path))
    if len(pcm) < B.SR * 0.15:
        return None
    ceps = B.raw_ceps(pcm)  # 含 c0，enhance 内部 drop
    if len(ceps) == 0:
        return None
    return B.cap(B.enhance(ceps, GSTD))


# 全局 STD
allc = []
for syl in PAIRS.values():
    for p in B.BD.glob(f"{syl}[1-4].mp3"):
        c = B.raw_ceps(B.trim_silence(B.load_pcm(p)))
        if len(c):
            allc.append(c[:, 1:])  # c1..c12
stack = np.vstack(allc)
dlt = np.diff(stack, axis=0)
GSTD = np.concatenate([stack.std(axis=0), dlt.std(axis=0)])
GSTD[GSTD < 1e-3] = 1.0

samples: dict[str, list[tuple[Path, np.ndarray]]] = {}
for letter, syl in PAIRS.items():
    ss = []
    for p in sorted(B.BD.glob(f"{syl}[1-4].mp3")):
        f = feat(p)
        if f is not None:
            ss.append((p, f))
    samples[letter] = ss

print(f"全局STD维度={len(GSTD)} 样本: " +
      " ".join(f"{k}={len(v)}" for k, v in samples.items()))

# 对每个目标字母：读对距离 vs 乱读距离
print(f"\n{'目标':>4} {'读对p50':>9} {'读对max':>9} {'乱读min':>9} {'分离?':>7}")
sep_count = 0
tot = 0
for letter, poss in samples.items():
    own, wrong = [], []
    for probe_p, pf in poss:
        # 读对：用同字母的其它样本当"模板"（留一）
        for tpl_p, tf in poss:
            if tpl_p == probe_p:
                continue
            own.append(dtw(pf, tf))
        # 乱读：其它字母的样本
        for other, oss in samples.items():
            if other == letter:
                continue
            for _, of in oss:
                wrong.append(dtw(pf, of))
    if not own or not wrong:
        continue
    tot += 1
    omax = max(own)
    wmin = min(wrong)
    sep = omax < wmin
    sep_count += int(sep)
    print(f"{letter:>4} {np.median(own):9.3f} {omax:9.3f} {wmin:9.3f} {'YES' if sep else 'no':>7}")

print(f"\n完全分离: {sep_count}/{tot}")
