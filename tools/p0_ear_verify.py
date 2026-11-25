"""P0 验证：Ear（CTC 音节级）模型能否真正区分「读对」和「读错」。

不训练、不调参，只回答一个问题：
  用项目现成的 bo/po/zi/zhi/an/ang 真人音，
  模型算出的目标音节概率 pTarget，读对和读错能不能拉开？

用例覆盖用户反馈过的全部场景：b 读成 w、平翘舌、前后鼻音、声调。
"""
import sys
import numpy as np
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tools" / "_p0"))

import build_letter_templates as B  # noqa: E402
from ear_model import EarScorer, compute_mel_spectrogram, SAMPLE_RATE  # noqa: E402

MODEL = ROOT / "tools" / "_p0" / "tiny_int8.onnx"
VOCAB = ROOT / "tools" / "_p0" / "vocab.json"
scorer = EarScorer(str(MODEL), str(VOCAB))
print(f"vocab={len(scorer.vocab)} blank={scorer.blank_id}")


def evaluate(pcm, target_tok, trim=True):
    pcm = np.asarray(pcm, dtype=np.float32)
    if trim:
        pcm = B.trim_silence(pcm)
    feats, T = compute_mel_spectrogram(pcm, SAMPLE_RATE, max_seconds=5.0)
    if T == 0:
        return None
    feats = np.expand_dims(feats, 0).astype(np.float32)
    lens = np.array([T], dtype=np.int64)
    logits = scorer.session.run(None, {"feats": feats, "feat_lens": lens})[0]
    log_probs = scorer.log_softmax(logits[0])
    tid = scorer.token_to_id.get(target_tok, scorer.unk_id)
    r = scorer.evaluate_syllable(log_probs, target_tok, tid)
    return r, T


# (音频文件, 目标音节, 期望)
CASES = [
    # 读对
    ("bo1.mp3", "bo1", "correct"),
    ("bo2.mp3", "bo2", "correct"),
    ("po1.mp3", "po1", "correct"),
    ("mo1.mp3", "mo1", "correct"),
    ("zi1.mp3", "zi1", "correct"),
    ("zhi1.mp3", "zhi1", "correct"),
    ("chi1.mp3", "chi1", "correct"),
    ("shi1.mp3", "shi1", "correct"),
    ("an1.mp3", "an1", "correct"),
    ("ang1.mp3", "ang1", "correct"),
    # 读错：声母
    ("bo1.mp3", "po1", "wrong"),
    ("po1.mp3", "bo1", "wrong"),
    ("bo1.mp3", "wo1", "wrong"),
    ("wo1.mp3", "bo1", "wrong"),
    ("mo1.mp3", "bo1", "wrong"),
    ("bo1.mp3", "mo1", "wrong"),
    # 读错：平翘舌
    ("zhi1.mp3", "zi1", "wrong"),
    ("zi1.mp3", "zhi1", "wrong"),
    ("ci1.mp3", "zi1", "wrong"),
    ("shi1.mp3", "si1", "wrong"),
    # 读错：前后鼻音
    ("ang1.mp3", "an1", "wrong"),
    ("an1.mp3", "ang1", "wrong"),
    # 读错：声调
    ("bo1.mp3", "bo2", "wrong"),
    ("bo2.mp3", "bo1", "wrong"),
]

rows = []
for name, target, expect in CASES:
    p = ROOT / "assets" / "audio" / "bd" / name
    if not p.exists():
        print(f"{name:10s} -> {target:5s}  缺素材")
        continue
    out = evaluate(B.load_pcm(p), target)
    if out is None:
        print(f"{name:10s} -> {target:5s}  音频太短")
        continue
    r, T = out
    rows.append((name, target, expect, r))
    print(
        f"{name:10s} -> {target:5s}  top1={r['top1']:5s} "
        f"pTar={r['pTarget']:.3f} pTop1={r['pTop1']:.3f} "
        f"margin={r['targetMargin']:+.3f} {r['status']:7s} "
        f"err={str(r['errorType']):9s} T={T}"
    )

print("\n=== 汇总 ===")
pos = [r["pTarget"] for n, t, e, r in rows if e == "correct"]
neg = [r["pTarget"] for n, t, e, r in rows if e == "wrong"]
judged_ok = sum(
    1 for n, t, e, r in rows if (e == "correct") == (r["status"] == "correct")
)
print(f"读对 pTarget: n={len(pos)} min={min(pos):.3f} mean={np.mean(pos):.3f}")
print(f"读错 pTarget: n={len(neg)} max={max(neg):.3f} mean={np.mean(neg):.3f}")
print(f"状态判定与期望一致: {judged_ok}/{len(rows)}")
wrong_but_pass = [
    (n, t, r["top1"], r["pTarget"]) for n, t, e, r in rows
    if e == "wrong" and r["status"] == "correct"
]
print(f"读错却判 correct（假阳性）: {len(wrong_but_pass)}")
for x in wrong_but_pass:
    print("   ", x)
right_but_reject = [
    (n, t, r["top1"], r["pTarget"]) for n, t, e, r in rows
    if e == "correct" and r["status"] != "correct"
]
print(f"读对却未判 correct（假阴性）: {len(right_but_reject)}")
for x in right_but_reject:
    print("   ", x)
