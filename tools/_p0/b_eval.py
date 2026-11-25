#!/usr/bin/env python3
"""路线 B —— 面向产品的评测：读对能过率 / 乱读误放行率 / 阈值扫描。

用法:
  python tools/_p0/b_eval.py cmvn
  python tools/_p0/b_eval.py frame

口径：
  读对 = 用「目标音节自己」的干净录音（同源 bd / 跨音源 xdf 各算一次）
  乱读 = 用「别的音节」的录音，却要求判给目标音节
  判定 = margin（目标余弦 − 次优余弦，次优排除目标）≥ 阈值
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import torch

import b_data as D
import b_model as M
import b_train as T

P0 = Path(__file__).resolve().parent


def load(tag: str):
    p = P0 / (f"b_ckpt_{tag}.pt" if tag else "b_ckpt.pt")
    ck = torch.load(p, map_location="cpu", weights_only=False)
    m = M.TinyNet(n_cls=len(ck["vocab"]), ch=tuple(ck["ch"]))
    m.load_state_dict(ck["model"])
    m.eval()
    T.FEAT_MODE = ck.get("feat", "frame")
    return m, ck["vocab"], ck.get("feat")


def cosines(model, pcm_list):
    feats = []
    for wav in pcm_list:
        f = T.feat_of(T._pad2s(wav), None)                 # [2,198,64]
        feats.append(f)
    x = torch.from_numpy(np.stack(feats).transpose(0, 1, 3, 2))  # [N,2,64,198]
    emb = model.embed(x)
    emb = emb / (emb.pow(2).sum(1, keepdim=True).sqrt() + 1e-6)
    w = model.weight.detach()
    w = w / (w.pow(2).sum(1, keepdim=True).sqrt() + 1e-6)
    with torch.no_grad():
        return (emb @ w.t()).numpy()


def margins(cos, targets):
    """目标类余弦 − 次优（排除目标）。"""
    out = np.zeros(len(targets), dtype=np.float32)
    for i, t in enumerate(targets):
        c = cos[i].copy()
        tv = c[t]
        c[t] = -np.inf
        out[i] = tv - c.max()
    return out


def main():
    tag = sys.argv[1] if len(sys.argv) > 1 else ""
    model, vocab, feat = load(tag)
    vi = {k: i for i, k in enumerate(vocab)}
    pcm, lengths, meta = D.load_cache()
    for i, m in enumerate(meta):
        m["n"] = int(lengths[i])

    bd = [i for i, m in enumerate(meta) if m["src"] == "bd" and m["key"] in vi]
    xdf = [i for i, m in enumerate(meta) if m["src"].startswith("xdf") and m["key"] in vi]
    print(f"checkpoint={tag or 'default'} feat={feat} 类别={len(vocab)} bd={len(bd)} xdf={len(xdf)}")

    # 读对
    cos_bd = cosines(model, [pcm[i, :meta[i]["n"]] for i in bd])
    mb = margins(cos_bd, [vi[meta[i]["key"]] for i in bd])
    cos_x = cosines(model, [pcm[i, :meta[i]["n"]] for i in xdf])
    mx = margins(cos_x, [vi[meta[i]["key"]] for i in xdf])

    # 乱读：拿别的音节的录音要求判给目标
    rng = np.random.default_rng(0)
    tpairs = []
    src = bd
    for _ in range(600):
        a, b = rng.choice(len(src), 2, replace=False)
        if meta[src[a]]["key"] == meta[src[b]]["key"]:
            continue
        tpairs.append((meta[src[b]]["key"], src[a]))     # 目标是 b 的类别，录音是 a
    cos_t = cosines(model, [pcm[i, :meta[i]["n"]] for _, i in tpairs])
    mt = margins(cos_t, [vi[k] for k, _ in tpairs])

    print(f"\nmargin 分布：bd 中位 {np.median(mb):.3f}  跨音源中位 {np.median(mx):.3f}  乱读中位 {np.median(mt):.3f}")
    print(f"{'阈值':>6} {'bd过':>7} {'跨音源过':>9} {'乱读误放':>9}")
    for th in (0.02, 0.04, 0.06, 0.08, 0.10, 0.12, 0.15, 0.20):
        print(f"{th:6.2f} {(mb>=th).mean()*100:6.1f}% {(mx>=th).mean()*100:8.1f}% {(mt>=th).mean()*100:8.1f}%")

    # 主要混淆对（跨音源）
    pred = cos_x.argmax(1)
    miss = [(meta[xdf[i]]["key"], vocab[pred[i]]) for i in range(len(xdf)) if pred[i] != vi[meta[xdf[i]]["key"]]]
    print(f"\n跨音源错例 {len(miss)}/{len(xdf)}：", miss[:18])


if __name__ == "__main__":
    main()
