#!/usr/bin/env python3
"""路线 B —— 折 BN、导出端侧权重、生成 Rust 侧黄金校验数据。

产物：
  src/data/spell_net.bin    裸 f32（顺序见下）+ scale
  src/data/spell_vocab.txt  词表，每行 音节+声调
  src/data/golden.bin       2 条样本的 features 与 logits，供 Rust 单测逐位比对

权重顺序（小端 f32）：
  w1[c1,2,3,3] b1[c1] w2[c2,c1,3,3] b2[c2] w3[c3,c2,3,3] b3[c3]
  w4[c4,c3,8,1] b4[c4] wfc[emb,2*c4] bfc[emb] class_w[n_cls,emb] scale
"""
from __future__ import annotations

import copy
import json
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

import b_model as M

P0 = Path(__file__).resolve().parent
ROOT = P0.parent.parent
SRC = ROOT / "src" / "data"


def fold(model: M.TinyNet) -> M.TinyNet:
    """把 BN 折进前面的卷积（推理图只剩 conv+relu）。"""
    m = copy.deepcopy(model).eval()
    for i in range(len(m.body) - 1):
        if isinstance(m.body[i], torch.nn.Conv2d) and isinstance(m.body[i + 1], torch.nn.BatchNorm2d):
            conv, bn = m.body[i], m.body[i + 1]
            scale = bn.weight / torch.sqrt(bn.running_var + bn.eps)
            conv.weight.data *= scale.view(-1, 1, 1, 1)
            conv.bias = torch.nn.Parameter(bn.bias - bn.running_mean * scale)
            m.body[i + 1] = torch.nn.Identity()
    scale = m.bn.weight / torch.sqrt(m.bn.running_var + m.bn.eps)
    m.collapse.weight.data *= scale.view(-1, 1, 1, 1)
    m.collapse.bias = torch.nn.Parameter(m.bn.bias - m.bn.running_mean * scale)
    m.bn = torch.nn.Identity()
    # 类中心归一化后固化，推理只需点积
    w = m.weight.detach()
    m.weight.data = w / (w.pow(2).sum(1, keepdim=True).sqrt() + 1e-6)
    return m


def main(tag: str = "", n_golden: int = 2) -> None:
    ckpt = P0 / (f"b_ckpt_{tag}.pt" if tag else "b_ckpt.pt")
    ck = torch.load(ckpt, map_location="cpu", weights_only=False)
    vocab = ck["vocab"]
    model = M.TinyNet(n_cls=len(vocab), ch=tuple(ck["ch"]))
    model.load_state_dict(ck["model"])
    model.eval()
    f = fold(model)

    convs = [m for m in f.body if isinstance(m, torch.nn.Conv2d)]
    blocks = [*convs, f.collapse]
    SRC.mkdir(parents=True, exist_ok=True)
    buf = bytearray()
    for c in blocks:
        buf += c.weight.detach().numpy().astype(np.float32).tobytes()
        buf += c.bias.detach().numpy().astype(np.float32).tobytes()
    buf += f.fc.weight.detach().numpy().astype(np.float32).tobytes()
    buf += f.fc.bias.detach().numpy().astype(np.float32).tobytes()
    buf += f.weight.detach().numpy().astype(np.float32).tobytes()
    buf += np.float32(f.scale).tobytes()

    (SRC / "spell_net.bin").write_bytes(bytes(buf))
    (SRC / "spell_vocab.txt").write_text("\n".join(vocab) + "\n", encoding="utf-8")
    print(f"spell_net.bin {len(buf)/1e6:.3f}MB  n_cls={len(vocab)}  feat={ck.get('feat')}")

    # ---- 黄金数据（供 Rust 单测）----
    import b_data as D
    import b_train as T
    pcm, lengths, meta = D.load_cache()
    for i, m in enumerate(meta):
        m["n"] = int(lengths[i])
    T.FEAT_MODE = ck.get("feat", "frame")
    pick = [i for i, m in enumerate(meta) if m["key"] in vocab][:n_golden]
    feats, logits = [], []
    for i in pick:
        wav = T._pad2s(pcm[i, :meta[i]["n"]])
        ft = T.feat_of(wav, None)                       # [2,198,64]
        # 存成 [C][MELS][FRAMES]，与 Rust 的 ctc_feat::features 输出布局一致
        feats.append(ft.transpose(0, 2, 1).astype(np.float32))
        x = torch.from_numpy(ft.transpose(0, 2, 1)[None])  # [1,2,64,198]
        with torch.no_grad():
            logits.append(f(x).numpy()[0].astype(np.float32))
        print("  golden", meta[i]["key"])
    gb = bytearray()
    gb += np.int32(len(pick)).tobytes()
    gb += np.int32(len(vocab)).tobytes()
    for ft, lg in zip(feats, logits):
        gb += ft.tobytes() + lg.tobytes()
    (SRC / "golden.bin").write_bytes(bytes(gb))
    print(f"golden.bin {len(gb)} 字节；logits 尺度 {float(np.abs(logits[0]).max()):.1f}")

    # ---- 前端黄金数据：[n i32][pcm f32 x n][feat 2*64*198 f32] ----
    i0 = pick[0]
    raw = pcm[i0, :meta[i0]["n"]].astype(np.float32)
    fb = bytearray()
    fb += np.int32(raw.size).tobytes()
    fb += raw.tobytes()
    fb += feats[0].tobytes()                      # 已是 [C][MELS][FRAMES]
    (SRC / "golden_feat.bin").write_bytes(bytes(fb))
    print(f"golden_feat.bin {len(fb)} 字节（{meta[i0]['key']}，{raw.size} 采样点）")

    # ---- ONNX（可选，便于外部工具检查）----
    try:
        torch.onnx.export(f, (torch.randn(1, 2, 64, 198),), str(P0 / "b_tiny.onnx"),
                          input_names=["mel"], output_names=["logits"], opset_version=17,
                          do_constant_folding=True, dynamo=False)
        print("b_tiny.onnx 已导出")
    except Exception as e:      # noqa: BLE001
        print("onnx 导出跳过:", e)

    (P0 / "b_net_meta.json").write_text(json.dumps(
        {"n_cls": len(vocab), "ch": list(ck["ch"]), "feat": ck.get("feat"),
         "scope": ck.get("scope")}, ensure_ascii=False, indent=1), encoding="utf-8")


if __name__ == "__main__":
    import sys
    main(sys.argv[1] if len(sys.argv) > 1 else "")
