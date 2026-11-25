#!/usr/bin/env python3
"""路线 B —— 固定 2 秒输入的极简 CNN + ArcFace 度量头（tract 可运行）。

为什么用 ArcFace 而不是普通 softmax：每个音节只有 1 条标准音，1355 类 softmax
在 1-shot 下收敛极慢（实测 8 轮 train-acc 仍 <1%）。加性角度间隔把"同音节不同
增广"压成一簇、把异音节推开，收敛快得多，且推理时可直接用余弦相似度 + 间隔
(margin) 做"乱读拒识"判据。

输入 [B,2,64,198]（static + delta 双通道 log-mel，逐帧去均值，静音置零）
输出 [B,n_cls] logits（= s * cos，s 为尺度因子）
算子：Conv2d / Relu / MaxPool2d / ReduceMean / ReduceMax / Concat / Gemm
     / Pow / ReduceSum / Sqrt / Div
"""
from __future__ import annotations

import torch
import torch.nn as nn
import torch.nn.functional as F


class TinyNet(nn.Module):
    def __init__(self, n_mels: int = 64, n_cls: int = 1330, emb: int = 128,
                 ch: tuple[int, int, int, int] = (24, 48, 96, 128),
                 scale: float = 30.0, margin: float = 0.25):
        super().__init__()
        c1, c2, c3, c4 = ch
        self.body = nn.Sequential(
            nn.Conv2d(2, c1, 3, padding=1, bias=False), nn.BatchNorm2d(c1), nn.ReLU(True),
            nn.MaxPool2d(2),                                   # 32 x 99
            nn.Conv2d(c1, c2, 3, padding=1, bias=False), nn.BatchNorm2d(c2), nn.ReLU(True),
            nn.MaxPool2d(2),                                   # 16 x 49
            nn.Conv2d(c2, c3, 3, padding=1, bias=False), nn.BatchNorm2d(c3), nn.ReLU(True),
            nn.MaxPool2d((2, 1)),                              # 8 x 49
        )
        self.collapse = nn.Conv2d(c3, c4, (n_mels // 8, 1), bias=False)   # 1 x 49
        self.bn = nn.BatchNorm2d(c4)
        self.fc = nn.Linear(2 * c4, emb)
        self.weight = nn.Parameter(torch.randn(n_cls, emb) * 0.02)        # 类中心
        self.scale = scale
        self.margin = margin
        self.n_cls = n_cls
        self.emb = emb

    def embed(self, x):
        h = torch.relu(self.bn(self.collapse(self.body(x))))   # [B,c4,1,T]
        feat = torch.cat([h.mean(dim=(2, 3)), h.amax(dim=(2, 3))], dim=1)
        return self.fc(feat)

    def forward(self, x, y=None):
        e = self.embed(x)
        e = e / (e.pow(2).sum(1, keepdim=True).sqrt() + 1e-6)
        w = self.weight / (self.weight.pow(2).sum(1, keepdim=True).sqrt() + 1e-6)
        cos = e @ w.t()
        if self.training and y is not None:
            th = torch.acos(cos.clamp(-1 + 1e-6, 1 - 1e-6))
            onehot = F.one_hot(y, self.n_cls).bool()
            cos = torch.where(onehot, torch.cos(th + self.margin), cos)
        return cos * self.scale


TinyCTC = TinyNet


if __name__ == "__main__":
    m = TinyNet()
    n = sum(p.numel() for p in m.parameters())
    print(f"params {n/1e6:.2f}M  fp32 {n*4/1e6:.2f}MB  int8 {n/1e6:.2f}MB")
    print("out", tuple(m(torch.zeros(2, 2, 64, 198)).shape))
