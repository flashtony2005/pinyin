#!/usr/bin/env python3
"""路线 B —— 训练 + 评测（单音节、固定 2s、1329 类判别）。

  python tools/_p0/b_train.py --epochs 30 --aug 4

评测口径：
  val@clean   同源（bd）留出样本、无增广 —— 拟合度
  cross-voice 另一音源（xdf 韵母表/整体认读）同音节 —— 跨说话人泛化（关键指标）
  m>0         目标类概率 > 次优类概率，即"能判为对"
"""
from __future__ import annotations

import argparse
import os
import time
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn

import b_data as D
import b_model as M

torch.set_num_threads(max(1, (os.cpu_count() or 8)))
P0 = Path(__file__).resolve().parent
CKPT = P0 / "b_ckpt.pt"

# 跟读闯关用到的字母/韵母音节（与 src/data/letter_templates.rs 的 LETTER_SYLLABLES 对齐）
LETTER_SYL = set("""bo po mo fo fu de te ta ne na le ge ke he ji qi xi zhi chi shi ri re
zi ci si yi wu a o e ai ei wei ui ao ou you iu ye ie yue ve er an en yin in wen un
yun vn ang eng ying ing weng ong""".split())


# ---------------------------------------------------------------- 增广
def aug_wave(x: np.ndarray, rng: np.random.Generator) -> np.ndarray:
    x = np.asarray(x, dtype=np.float32).copy()
    if x.size < 400:
        x = np.pad(x, (0, 400 - x.size))
    # 变速：同时改 F0 / 共振峰 / 时长 —— 模拟儿童与成人声道的差异
    r = rng.uniform(0.78, 1.30)
    n = max(200, int(x.size / r))
    x = np.interp(np.linspace(0, x.size - 1, n), np.arange(x.size), x).astype(np.float32)
    x *= np.float32(10 ** (rng.uniform(-9, 6) / 20))
    if rng.random() < 0.5:                                     # 短混响
        ir_len = int(rng.integers(80, 480))
        t = np.arange(ir_len) / D.SR
        ir = np.exp(-t / rng.uniform(0.02, 0.12)).astype(np.float32)
        ir[0] = 1.0
        ir /= (np.linalg.norm(ir) + 1e-9)
        x = np.convolve(x, ir).astype(np.float32)
    if rng.random() < 0.7:                                     # 加噪
        snr = rng.uniform(12, 35)
        p = float(np.mean(x ** 2)) + 1e-12
        noise = rng.standard_normal(x.size).astype(np.float32)
        noise *= np.sqrt(p / (10 ** (snr / 10)) / (float(np.mean(noise ** 2)) + 1e-12))
        x = x + noise
    out = np.zeros(D.N_SAMP, dtype=np.float32)
    if x.size >= out.size:
        s = int(rng.integers(0, x.size - out.size + 1))
        out[:] = x[s:s + out.size]
    else:
        off = int(rng.integers(0, out.size - x.size + 1))
        out[off:off + x.size] = x[:out.size - off]
    return out


def mel_warp_matrix(alpha: float) -> np.ndarray:
    """mel 轴按 alpha 缩放（VTLP 的 mel 域近似，用于声道长度扰动）。"""
    idx = np.arange(D.N_MELS, dtype=np.float32)
    src = (idx - D.N_MELS / 2) / alpha + D.N_MELS / 2
    W = np.zeros((D.N_MELS, D.N_MELS), dtype=np.float32)
    for i, s in enumerate(src):
        s = min(max(s, 0.0), D.N_MELS - 1.0)
        j = int(np.floor(s))
        k = min(j + 1, D.N_MELS - 1)
        w = s - j
        W[i, j] += 1 - w
        W[i, k] += w
    return W


WARP_CACHE = {round(a / 100, 2): mel_warp_matrix(a / 100) for a in range(85, 116)}
FEAT_MODE = "frame"


def feat_of(pcm: np.ndarray, rng: np.random.Generator | None) -> np.ndarray:
    f, rms = D.logmel(pcm)
    mask = D.active_mask(rms)
    if rng is not None:
        W = WARP_CACHE[round(float(rng.uniform(0.85, 1.15)), 2)]
        f = f @ W.T
    f = D.cmvn(f, mask) if FEAT_MODE == "cmvn" else D.norm_frames(f, mask)
    f = f.copy()
    f[~mask] = 0.0
    if rng is not None:
        for _ in range(2):                                     # SpecAugment
            w = int(rng.integers(0, 7))
            if w:
                a = int(rng.integers(0, D.N_MELS - w))
                f[:, a:a + w] = 0.0
        for _ in range(2):
            w = int(rng.integers(0, 17))
            if w:
                a = int(rng.integers(0, D.N_FRAMES - w))
                f[a:a + w, :] = 0.0
    return np.stack([f, D.delta(f)])


# ---------------------------------------------------------------- 语料
def _pad2s(wav: np.ndarray) -> np.ndarray:
    out = np.zeros(D.N_SAMP, dtype=np.float32)
    wav = wav[:D.N_SAMP]
    off = max(0, (D.N_SAMP - wav.size) // 2)
    out[off:off + wav.size] = wav
    return out


def to_tensor(feats: list[np.ndarray]) -> np.ndarray:
    """[N,C,198,64] -> [N,C,64,198]（C = static + delta）"""
    return np.stack(feats).transpose(0, 1, 3, 2)


def stack_feats(pcm, meta, idxs, aug_per: int, seed: int):
    rng = np.random.default_rng(seed)
    feats, ys = [], []
    for i in idxs:
        wav = pcm[i, :meta[i]["n"]]
        if aug_per <= 0:
            feats.append(feat_of(_pad2s(wav), None))
            ys.append(i)
            continue
        for k in range(aug_per):
            use_aug = (k > 0) or (rng.random() < 0.85)
            feats.append(feat_of(aug_wave(wav, rng) if use_aug else _pad2s(wav),
                                 rng if use_aug else None))
            ys.append(i)
    return to_tensor(feats), np.array(ys)


def load_extra(d: Path) -> list[tuple[str, np.ndarray]]:
    """读取 <音节+声调>.wav（16k 单声道 wav），返回 (key, pcm)。"""
    import soundfile as sf
    out: list[tuple[str, np.ndarray]] = []
    if not d.is_dir():
        print(f"额外录音目录不存在：{d}")
        return out
    for p in sorted(list(d.glob("*.wav")) + list(d.glob("*.WAV"))):
        key = p.stem.lower()
        try:
            x, sr = sf.read(str(p), dtype="float32", always_2d=False)
            if x.ndim > 1:
                x = x.mean(axis=1)
            if sr != D.SR:
                from math import gcd
                from scipy.signal import resample_poly
                g = gcd(int(sr), D.SR)
                x = resample_poly(x, D.SR // g, int(sr) // g).astype(np.float32)
            out.append((key, D.trim(np.ascontiguousarray(x, dtype=np.float32))))
        except Exception as e:      # noqa: BLE001
            print(f"  跳过 {p.name}: {e}")
    return out


def evaluate(model, feats, labels):
    model.eval()
    outs = []
    with torch.no_grad():
        for s in range(0, len(feats), 64):
            outs.append(model(torch.from_numpy(feats[s:s + 64])).numpy())
    logit = np.concatenate(outs)
    logit = logit - logit.max(axis=1, keepdims=True)
    p = np.exp(logit)
    p /= p.sum(axis=1, keepdims=True)
    pred = p.argmax(axis=1)
    margins = np.array([p[i, t] - np.delete(p[i], t).max() for i, t in enumerate(labels)])
    return pred, p, margins


def main():
    global FEAT_MODE
    ap = argparse.ArgumentParser()
    ap.add_argument("--epochs", type=int, default=30)
    ap.add_argument("--aug", type=int, default=4)
    ap.add_argument("--bs", type=int, default=48)
    ap.add_argument("--lr", type=float, default=3e-3)
    ap.add_argument("--val-holdout", type=int, default=60)
    ap.add_argument("--ch", type=str, default="24,48,96,128")
    ap.add_argument("--scope", type=str, default="letters", choices=["letters", "all"])
    ap.add_argument("--feat", type=str, default="frame", choices=["frame", "cmvn"])
    ap.add_argument("--tag", type=str, default="")
    ap.add_argument("--init", type=str, default="", help="从已有 ckpt 继续训练（微调）")
    ap.add_argument("--extra", type=str, default="", help="额外录音目录：<音节+声调>.wav")
    a = ap.parse_args()
    ch = tuple(int(x) for x in a.ch.split(","))
    FEAT_MODE = a.feat

    pcm, lengths, meta = D.load_cache()
    if a.extra:
        extra = load_extra(Path(a.extra))
        if extra:
            add_pcm = np.zeros((len(extra), D.CAP), dtype=np.int16)
            add_len = np.zeros(len(extra), dtype=np.int32)
            for k, (key, wav) in enumerate(extra):
                w = wav[:D.CAP]
                add_pcm[k, :w.size] = (np.clip(w, -1, 1) * 32767).astype(np.int16)
                add_len[k] = w.size
                meta.append({"key": key, "syl": key[:-1], "tone": int(key[-1]) if key[-1].isdigit() else 0,
                             "src": "extra", "path": str(a.extra)})
            pcm = np.vstack([pcm, add_pcm])
            lengths = np.concatenate([lengths, add_len])
            print(f"额外录音 {len(extra)} 条")
    for i, m in enumerate(meta):
        m["n"] = int(lengths[i])
    keys = {m["key"] for m in meta}
    if a.scope == "letters":
        keys = {k for k in keys if k[:-1] in LETTER_SYL}
    vocab = sorted(keys)
    vindex = {k: i for i, k in enumerate(vocab)}
    print(f"vocab {len(vocab)} 样本 {len(meta)} scope={a.scope}")

    bd_idx = [i for i, m in enumerate(meta) if m["src"] in ("bd", "extra") and m["key"] in vindex]
    xdf_idx = [i for i, m in enumerate(meta) if m["src"].startswith("xdf") and m["key"] in vindex]
    rng = np.random.default_rng(7)
    perm = rng.permutation(bd_idx)
    va_idx = sorted(perm[:a.val_holdout].tolist())
    tr_idx = sorted(perm[a.val_holdout:].tolist())
    if a.val_holdout <= 0:
        va_idx = tr_idx
    print(f"train {len(tr_idx)}  val {len(va_idx)}  cross-voice(xdf) {len(xdf_idx)}")

    vx = to_tensor([feat_of(_pad2s(pcm[i, :meta[i]['n']]), None) for i in va_idx])
    vy = np.array([vindex[meta[i]['key']] for i in va_idx])
    cx = to_tensor([feat_of(_pad2s(pcm[i, :meta[i]['n']]), None) for i in xdf_idx])
    cy = np.array([vindex[meta[i]['key']] for i in xdf_idx])

    model = M.TinyNet(n_cls=len(vocab), ch=ch)
    if a.init:
        init_ck = torch.load(P0 / (f"b_ckpt_{a.init}.pt" if not a.init.endswith(".pt") else a.init),
                             map_location="cpu", weights_only=False)
        sd = init_ck["model"]
        if len(init_ck["vocab"]) == len(vocab):
            model.load_state_dict(sd)
            print(f"从 {a.init} 继续训练")
        else:
            missing = model.load_state_dict(sd, strict=False)
            print(f"从 {a.init} 部分加载：{missing}")
    nparam = sum(p.numel() for p in model.parameters())
    print(f"params {nparam/1e6:.2f}M (fp32 {nparam*4/1e6:.1f}MB) ch={ch}")
    opt = torch.optim.AdamW(model.parameters(), lr=a.lr, weight_decay=1e-2)
    nb_per_ep = (len(tr_idx) * a.aug + a.bs - 1) // a.bs
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, max_lr=a.lr,
                                                total_steps=a.epochs * nb_per_ep + 8, pct_start=0.2)
    crit = nn.CrossEntropyLoss(label_smoothing=0.05)

    ckpt_path = P0 / (f"b_ckpt_{a.tag}.pt" if a.tag else "b_ckpt.pt")
    best = -1.0
    for ep in range(1, a.epochs + 1):
        t0 = time.time()
        feats, ys = stack_feats(pcm, meta, tr_idx, a.aug, ep * 13)
        labels = np.array([vindex[meta[i]['key']] for i in ys])
        order = np.random.default_rng(ep).permutation(len(feats))
        model.train()
        tot, nb, corr = 0.0, 0, 0
        for s in range(0, len(order), a.bs):
            b = order[s:s + a.bs]
            if len(b) < 4:
                continue
            xb = torch.from_numpy(feats[b])
            yb = torch.from_numpy(labels[b]).long()
            logit = model(xb, yb)
            loss = crit(logit, yb)
            opt.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 5.0)
            opt.step()
            if sched.last_epoch < sched.total_steps - 1:
                sched.step()
            tot += float(loss)
            nb += 1
            corr += int((logit.argmax(1) == yb).sum())

        pv, pvp, mv = evaluate(model, vx, vy)
        pc, pcp, mc = evaluate(model, cx, cy)
        acc_v = float((pv == vy).mean())
        acc_c = float((pc == cy).mean()) if len(cy) else 0.0
        print(f"ep{ep:02d} loss {tot/max(nb,1):.3f} train-acc {corr/max(1,len(order))*100:4.1f}%  "
              f"val {acc_v*100:5.1f}%(m>0 {float((mv>0).mean())*100:4.1f}%)  "
              f"cross {acc_c*100:5.1f}%(m>0 {float((mc>0).mean())*100:4.1f}%)  {time.time()-t0:.0f}s",
              flush=True)
        score = acc_c * 0.6 + acc_v * 0.4
        if score > best:
            best = score
            torch.save({"model": model.state_dict(), "vocab": vocab, "ch": ch,
                        "feat": a.feat, "scope": a.scope}, ckpt_path)
            print("   ↑ saved", flush=True)

    print("done, best", round(best, 4))


if __name__ == "__main__":
    main()
