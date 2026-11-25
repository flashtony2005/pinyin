"""P0 批量评测：Ear CTC 模型在项目 bd/ 真人单音节上的真实命中率。

回答：这个模型能不能当「读对/读错」的裁判？
指标：top1 命中率、top3 命中率、易混音对混淆情况。
"""
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tools" / "_p0"))

import build_letter_templates as B  # noqa: E402
from ear_model import EarScorer, compute_mel_spectrogram, SAMPLE_RATE  # noqa: E402

sc = EarScorer("tiny_int8.onnx", "vocab.json")
vocab = sc.vocab
valid = set(vocab)


def dist(pcm):
    feats, T = compute_mel_spectrogram(np.asarray(pcm, dtype=np.float32), SAMPLE_RATE)
    if T == 0:
        return None
    f = np.expand_dims(feats, 0).astype(np.float32)
    L = np.array([T], dtype=np.int64)
    lp = sc.log_softmax(sc.session.run(None, {"feats": f, "feat_lens": L})[0][0])
    pr = np.exp(lp)
    nb = 1.0 - pr[:, 0]
    sel = np.where(nb >= 0.05)[0]
    if len(sel) == 0:
        sel = np.array([int(np.argmax(nb))])
    cond = np.zeros(pr.shape[1], dtype=np.float64)
    for t in sel:
        den = max(float(pr[t, 1:].sum()), 1e-8)
        cond[1:] += pr[t, 1:] / den
    cond[1:] /= len(sel)
    return cond


files = sorted(B.BD.glob("*.mp3"))
pat = re.compile(r"^([a-z]+)([1-5])$")
top1_ok = top3_ok = total = skipped = 0
per_syl = defaultdict(lambda: [0, 0, 0])  # n, top1, top3
confusion = Counter()

for p in files:
    m = pat.match(p.stem)
    if not m or p.stem not in valid:
        skipped += 1
        continue
    target = p.stem
    try:
        pcm = B.load_pcm(p)
    except Exception:
        skipped += 1
        continue
    cond = dist(pcm)
    if cond is None:
        skipped += 1
        continue
    order = np.argsort(-cond)
    top1 = vocab[order[0]]
    top3 = [vocab[i] for i in order[:3]]
    total += 1
    per_syl[target][0] += 1
    if top1 == target:
        top1_ok += 1
        per_syl[target][1] += 1
    if target in top3:
        top3_ok += 1
        per_syl[target][2] += 1
    if top1 != target:
        confusion[(target, top1)] += 1

print(f"有效样本 {total}，跳过 {skipped}")
print(f"top1 命中率 {top1_ok}/{total} = {top1_ok/max(total,1):.1%}")
print(f"top3 命中率 {top3_ok}/{total} = {top3_ok/max(total,1):.1%}")

print("\n=== 各音节（样本≥3）===")
rows = [(s, v) for s, v in per_syl.items() if v[0] >= 3]
rows.sort(key=lambda x: -x[1][1] / x[1][0])
for s, (n, a, b) in rows[:25]:
    print(f"  {s:6s} n={n:2d} top1={a/n:5.0%} top3={b/n:5.0%}")
print("  ... 最差 10 个 ...")
for s, (n, a, b) in rows[-10:]:
    print(f"  {s:6s} n={n:2d} top1={a/n:5.0%} top3={b/n:5.0%}")

print("\n=== 最常见混淆（目标 → 听到）===")
for (t, h), c in confusion.most_common(20):
    print(f"  {t:6s} -> {h:6s}  {c}")

# 易混音对专项
print("\n=== 易混音对 ===")
for a, b in [("zi", "zhi"), ("ci", "chi"), ("si", "shi"), ("an", "ang"), ("en", "eng"),
             ("in", "ing"), ("bo", "po"), ("bo", "wo")]:
    na = sum(v[0] for s, v in per_syl.items() if s.rstrip("12345") == a)
    nb_ = sum(v[0] for s, v in per_syl.items() if s.rstrip("12345") == b)
    hit_a = sum(v[1] for s, v in per_syl.items() if s.rstrip("12345") == a)
    hit_b = sum(v[1] for s, v in per_syl.items() if s.rstrip("12345") == b)
    # 互相混成对方的比例
    a2b = sum(c for (t, h), c in confusion.items() if t.rstrip("12345") == a and h.rstrip("12345") == b)
    b2a = sum(c for (t, h), c in confusion.items() if t.rstrip("12345") == b and h.rstrip("12345") == a)
    print(f"  {a}/{b}: n={na}/{nb_} top1={hit_a}/{na} vs {hit_b}/{nb_}; 混成对方 {a}->{b}={a2b} {b}->{a}={b2a}")
