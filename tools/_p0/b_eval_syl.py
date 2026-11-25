#!/usr/bin/env python3
"""路线 B —— 音节级（不看声调）评测。

跟读闯关只要求「读对这个字母的呼读音」，声调不评分；而模型词表是
`音节+声调`（189 类）。端上如果按精确 key 比对，孩子声调读偏就直接判错。
这里改成音节级口径：

  目标分 = 该音节所有声调变体里最高的余弦
  其它分 = 其它音节所有变体里最高的余弦
  margin = 目标分 − 其它分

用法: /f/venv-torch/Scripts/python.exe tools/_p0/b_eval_syl.py final
"""
from __future__ import annotations

import sys

import numpy as np

import b_eval as E
import b_data as D


def syl_of(key: str) -> str:
    return key[:-1] if key[-1].isdigit() else key


def margins_syl(cos: np.ndarray, vocab: list[str], targets: list[str]) -> np.ndarray:
    syls = [syl_of(k) for k in vocab]
    uniq = sorted(set(syls))
    sidx = {s: i for i, s in enumerate(uniq)}
    cls2syl = np.array([sidx[s] for s in syls])
    out = np.zeros(len(targets), dtype=np.float32)
    for i, t in enumerate(targets):
        c = cos[i]
        per = np.full(len(uniq), -np.inf, dtype=np.float32)
        np.maximum.at(per, cls2syl, c)
        ti = sidx[t]
        best_other = max(per[j] for j in range(len(uniq)) if j != ti)
        out[i] = per[ti] - best_other
    return out


def top1_syl(cos: np.ndarray, vocab: list[str]) -> list[str]:
    return [syl_of(vocab[i]) for i in cos.argmax(1)]


def main() -> None:
    tag = sys.argv[1] if len(sys.argv) > 1 else ""
    model, vocab, feat = E.load(tag)
    vi = {k: i for i, k in enumerate(vocab)}
    pcm, lengths, meta = D.load_cache()
    for i, m in enumerate(meta):
        m["n"] = int(lengths[i])

    bd = [i for i, m in enumerate(meta) if m["src"] == "bd" and m["key"] in vi]
    xdf = [i for i, m in enumerate(meta) if m["src"].startswith("xdf") and m["key"] in vi]
    print(f"checkpoint={tag or 'default'} feat={feat} 类别={len(vocab)} bd={len(bd)} xdf={len(xdf)}")

    tb = [syl_of(meta[i]["key"]) for i in bd]
    tx = [syl_of(meta[i]["key"]) for i in xdf]
    cos_bd = E.cosines(model, [pcm[i, : meta[i]["n"]] for i in bd])
    cos_x = E.cosines(model, [pcm[i, : meta[i]["n"]] for i in xdf])
    mb = margins_syl(cos_bd, vocab, tb)
    mx = margins_syl(cos_x, vocab, tx)

    pb = top1_syl(cos_bd, vocab)
    px = top1_syl(cos_x, vocab)
    print(f"音节级 top1：同源 {(np.array(pb) == np.array(tb)).mean() * 100:.1f}%  "
          f"跨音源 {(np.array(px) == np.array(tx)).mean() * 100:.1f}%")

    # 乱读：拿别的音节的录音要求判给目标
    rng = np.random.default_rng(0)
    pairs = []
    for _ in range(800):
        a, b = rng.choice(len(bd), 2, replace=False)
        ta, tb_ = tb[a], tb[b]
        if ta == tb_:
            continue
        pairs.append((tb_, bd[a]))          # 目标 = 另一个音节的说话意图，录音是 a
    cos_t = E.cosines(model, [pcm[i, : meta[i]["n"]] for _, i in pairs])
    mt = margins_syl(cos_t, vocab, [k for k, _ in pairs])

    print(f"\nmargin 中位：同源 {np.median(mb):.3f}  跨音源 {np.median(mx):.3f}  乱读 {np.median(mt):.3f}")
    print(f"{'阈值':>6} {'同源过':>8} {'跨音源过':>9} {'乱读误放':>9} {'top1命中即过(跨)':>16}")
    hit_x = np.array(px) == np.array(tx)
    for th in (-0.02, 0.0, 0.02, 0.04, 0.06, 0.08, 0.10, 0.15):
        acc_x = ((mx >= th) | hit_x).mean() * 100
        print(f"{th:6.2f} {(mb >= th).mean() * 100:7.1f}% {(mx >= th).mean() * 100:8.1f}% "
              f"{(mt >= th).mean() * 100:8.1f}% {acc_x:15.1f}%")

    # top1==目标 这条规则单独看误放行
    pt = top1_syl(cos_t, vocab)
    hit_false = np.array(pt) == np.array([k for k, _ in pairs])
    print(f"\n仅用 top1==目标：同源 {np.mean(np.array(pb) == np.array(tb)) * 100:.1f}%  "
          f"跨音源 {hit_x.mean() * 100:.1f}%  乱读误放 {hit_false.mean() * 100:.1f}%")

    miss = [(tx[i], px[i]) for i in range(len(tx)) if px[i] != tx[i]]
    print(f"跨音源音节级错例 {len(miss)}/{len(tx)}：", miss[:20])


if __name__ == "__main__":
    main()
