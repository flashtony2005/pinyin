#!/usr/bin/env python3
"""把 b_ckpt.pt 的权重导成 Rust 端可直接 include_bytes! 的裸 f32 二进制 + 词表。

布局（小端 f32，顺序即拼接顺序）：
  conv1.w [c1,1,3,3] conv1.b[c1]
  conv2.w [c2,c1,3,3] conv2.b[c2]
  conv3.w [c3,c2,3,3] conv3.b[c3]
  conv4.w [c4,c3,8,1] conv4.b[c4]
  fc.w    [ncls,c4]   fc.b[ncls]
"""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import torch

import b_model as M
import b_export as E

P0 = Path(__file__).resolve().parent
ROOT = P0.parent.parent
SRC = ROOT / "src" / "data"


def main():
    ck = torch.load(P0 / "b_ckpt.pt", map_location="cpu", weights_only=False)
    vocab = ck["vocab"]
    model = M.TinyCTC(n_cls=len(vocab) + 1)
    model.load_state_dict(ck["model"])
    f = E.fold(model).eval()

    seq = [m for m in f.body if isinstance(m, torch.nn.Conv2d)]
    assert len(seq) == 3, len(seq)
    blocks = [seq[0], seq[1], seq[2], f.collapse]
    SRC.mkdir(parents=True, exist_ok=True)
    buf = bytearray()
    meta = []
    for c in blocks:
        w = c.weight.detach().numpy().astype(np.float32)
        b = c.bias.detach().numpy().astype(np.float32)
        buf += w.tobytes() + b.tobytes()
        meta.append(dict(w=list(w.shape), b=list(b.shape)))
    w = f.head.weight.detach().numpy().astype(np.float32)
    b = f.head.bias.detach().numpy().astype(np.float32)
    buf += w.tobytes() + b.tobytes()
    meta.append(dict(w=list(w.shape), b=list(b.shape)))

    out = SRC / "ctc_net.bin"
    out.write_bytes(bytes(buf))
    (SRC / "ctc_vocab.txt").write_text("\n".join(vocab) + "\n", encoding="utf-8")
    (P0 / "b_net_meta.json").write_text(
        json.dumps({"layers": meta, "n_cls": len(vocab) + 1, "vocab": len(vocab)},
                   ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"{out} {len(buf)/1e6:.3f}MB  词表 {len(vocab)}  类别 {len(vocab)+1}")
    for m in meta:
        print("  ", m)


if __name__ == "__main__":
    main()
