#!/usr/bin/env python3
"""单条录音打分：看它像哪个音节、目标音节差多少、能否判过。

用法:
  python tools/_p0/b_score.py final bo1 record.wav [more.wav ...]
  python tools/_p0/b_score.py final bo1            # 不给文件则用 bd 里 bo1 的标准音

输出 top5 + 目标余弦 + margin + 判定（阈值 0.06）。
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

import b_data as D
from b_eval import load, cosines

TH = 0.06


def read(path: str) -> np.ndarray:
    p = Path(path)
    if p.suffix.lower() == ".wav":
        return D.trim(D.decode(p))
    return D.trim(D.decode(p))


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return
    tag, key = sys.argv[1], sys.argv[2].lower()
    files = sys.argv[3:]
    model, vocab, feat = load(tag)
    if key not in vocab:
        print(f"词表里没有 {key}；相近的：{[v for v in vocab if v[:-1] == key[:-1]]}")
        return
    ti = vocab.index(key)
    if not files:
        pcm, lengths, meta = D.load_cache()
        hits = [i for i, m in enumerate(meta) if m["key"] == key]
        if not hits:
            print("语料里没有这条标准音")
            return
        waves = [pcm[i, :lengths[i]] for i in hits]
        names = [f"{meta[i]['src']}/{key}.mp3" for i in hits]
    else:
        waves = [read(f) for f in files]
        names = files

    cos = cosines(model, waves)
    for n, c in zip(names, cos):
        order = np.argsort(-c)[:5]
        others = np.delete(c, ti).max()
        margin = float(c[ti] - others)
        print(f"\n{n}  ({len(waves[0])/D.SR*1000:.0f}ms)")
        print("  top5:", ", ".join(f"{vocab[i]}:{c[i]:.3f}" for i in order))
        print(f"  目标 {key} 余弦 {c[ti]:.3f}  次优 {others:.3f}  margin {margin:+.3f}  "
              f"→ {'通过' if margin >= TH else '再试一次'}")


if __name__ == "__main__":
    main()
