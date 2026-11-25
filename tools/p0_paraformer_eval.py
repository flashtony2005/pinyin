"""P0：paraformer-zh-small 在项目 bd/ 单音节素材上的真实识别率。

判定逻辑：模型识别出汉字 → 转带调拼音 → 与目标音节（文件名）比较。
统计：整音节命中 / 仅声韵母命中 / 声调命中。
"""
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np
import sherpa_onnx
from pypinyin import Style, pinyin

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import build_letter_templates as B  # noqa: E402

D = ROOT / "tools" / "_p0" / "sherpa-onnx-paraformer-zh-small-2024-03-09"
rec = sherpa_onnx.OfflineRecognizer.from_paraformer(
    paraformer=str(D / "model.int8.onnx"),
    tokens=str(D / "tokens.txt"),
    num_threads=2,
    sample_rate=16000,
    feature_dim=80,
    decoding_method="greedy_search",
    provider="cpu",
)


def asr(pcm):
    s = rec.create_stream()
    s.accept_waveform(16000, np.asarray(pcm, dtype=np.float32))
    rec.decode_stream(s)
    return s.result.text.strip()


def to_py(text):
    if not text:
        return []
    return [x[0] for x in pinyin(text, style=Style.TONE3, neutral_tone_with_five=True)]


def base(tok):
    return tok[:-1] if tok and tok[-1].isdigit() else tok


pat = re.compile(r"^([a-z]+)([1-5])$")
files = [p for p in sorted(B.BD.glob("*.mp3")) if pat.match(p.stem)]

total = exact = base_ok = tone_ok = empty = 0
per = defaultdict(lambda: [0, 0, 0, 0])  # n, exact, base, tone
conf = Counter()

for p in files:
    target = p.stem
    try:
        pcm = B.load_pcm(p)
        text = asr(pcm)
    except Exception:
        continue
    got = to_py(text)
    total += 1
    t_base = base(target)
    t_tone = target[-1]
    if not got:
        empty += 1
        g = ""
        per[t_base][0] += 1
        conf[(t_base, "<空>")] += 1
        continue
    g0 = got[0]
    g_base = base(g0)
    g_tone = g0[-1] if g0 and g0[-1].isdigit() else ""
    per[t_base][0] += 1
    if g0 == target and len(got) == 1:
        exact += 1
        per[t_base][1] += 1
    if g_base == t_base and len(got) == 1:
        base_ok += 1
        per[t_base][2] += 1
        if g_tone == t_tone:
            tone_ok += 1
            per[t_base][3] += 1
    if g_base != t_base:
        conf[(t_base, g_base)] += 1

print(f"总样本 {total}，模型输出为空 {empty}")
print(f"整音节完全一致 {exact}/{total} = {exact/max(total,1):.1%}")
print(f"声韵母一致(不计调) {base_ok}/{total} = {base_ok/max(total,1):.1%}")
print(f"声韵调全对 {tone_ok}/{total} = {tone_ok/max(total,1):.1%}")

print("\n=== 各声母/韵母音节准确率（样本≥5，按整音节正确率）===")
rows = [(s, v) for s, v in per.items() if v[0] >= 5]
rows.sort(key=lambda x: -x[1][1] / x[1][0])
for s, (n, e, b, t) in rows[:20]:
    print(f"  {s:6s} n={n:3d} 整音节={e/n:5.0%} 声韵={b/n:5.0%} 全对={t/n:5.0%}")
print("  ... 最差 12 ...")
for s, (n, e, b, t) in rows[-12:]:
    print(f"  {s:6s} n={n:3d} 整音节={e/n:5.0%} 声韵={b/n:5.0%} 全对={t/n:5.0%}")

print("\n=== 易混音对专项 ===")
for a, b in [("zi", "zhi"), ("ci", "chi"), ("si", "shi"), ("an", "ang"),
             ("en", "eng"), ("bo", "po"), ("bo", "wo"), ("n", "l")]:
    na, nb_ = per[a][0], per[b][0]
    ea, eb = per[a][1], per[b][1]
    a2b = sum(c for (t, g), c in conf.items() if t == a and g == b)
    b2a = sum(c for (t, g), c in conf.items() if t == b and g == a)
    print(f"  {a}/{b}: n={na}/{nb_} 整音节={ea}/{na} vs {eb}/{nb_}; 互相混 {a}->{b}={a2b} {b}->{a}={b2a}")

print("\n=== 最常见混淆 ===")
for (t, g), c in conf.most_common(15):
    print(f"  {t:6s} -> {g:6s} {c}")
