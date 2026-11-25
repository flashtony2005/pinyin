"""四类探针的距离分布：读对 / 读错 / 哼声 / 噪声。
用独立样本（没进模板池），给出能同时满足「读对过 + 乱哼不过」的阈值。
"""
import sys, math
from pathlib import Path
import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
import build_letter_templates as B
import diag_honest as H
import diag_margin as M

plans, probes, D = M.build()
n = len(plans)

# 发探针：合成哼声和噪声，测它们对每个字母的距离
masks = B.load_baidu_mask()
all_ceps = []
for letter, _ in B.LETTERS:
    primary, _ = B.pick_source(letter, masks)
    if primary is None:
        continue
    for p, _ in B.collect_sources(letter, primary):
        try:
            all_ceps.append(B.raw_ceps(B.trim_silence(B.load_pcm(p))))
        except Exception:
            pass
stat = np.vstack([c[:, 1:] for c in all_ceps if len(c)])
dlt = np.vstack([B.delta(c[:, 1:]) for c in all_ceps if len(c)])
std = np.concatenate([stat.std(axis=0), dlt.std(axis=0)])
std[std < 1e-3] = 1.0

def feat(pcm):
    pcm = B.trim_silence(pcm)
    a, b = H.rust_voice_span(pcm)
    if b <= a:
        return None
    return B.cap(B.enhance(B.raw_ceps(pcm[a:b]), std))

def synth_hum(freq=220.0, ms=800):
    t = np.arange(int(16000*ms/1000))/16000.0
    return (0.5*np.sin(2*np.pi*freq*t)).astype("float32")

rng = np.random.default_rng(42)
def synth_noise(ms=400):
    return (rng.uniform(-0.5, 0.5, int(16000*ms/1000))).astype("float32")

hum_f = feat(synth_hum())
noise_f = feat(synth_noise())

# 重建每个字母的模板特征
tmpl = []
for letter, samples in plans:
    per = []
    for p, _ in samples:
        c = B.raw_ceps(B.trim_silence(B.load_pcm(p)))
        per.append(B.cap(B.enhance(c, std)))
    tmpl.append(per)

def dists_to_all(f):
    return np.array([min(H.dtw(f, t) for t in tmpl[i]) for i in range(n)])

hum_d = dists_to_all(hum_f)
noise_d = dists_to_all(noise_f)

print("哼声(220Hz) d_t 全字母: min={:.2f} p50={:.2f} max={:.2f}".format(
    hum_d.min(), np.median(hum_d), hum_d.max()))
print("白噪声       d_t 全字母: min={:.2f} p50={:.2f} max={:.2f}".format(
    noise_d.min(), np.median(noise_d), noise_d.max()))

# 独立探针：读对 vs 读错
own_d, wrong_d = [], []
for pi, (self_idx, _, _, _, _) in enumerate(probes):
    own_d.append(D[pi, self_idx])
    for j in range(n):
        if j != self_idx:
            wrong_d.append(D[pi, j])
own_d = np.array(own_d)
wrong_d = np.array(wrong_d)
print("读对         d_t        : min={:.2f} p50={:.2f} p90={:.2f} max={:.2f}".format(
    own_d.min(), np.median(own_d), np.percentile(own_d, 90), own_d.max()))
print("读错         d_t        : min={:.2f} p50={:.2f} p90={:.2f}".format(
    wrong_d.min(), np.median(wrong_d), np.percentile(wrong_d, 90)))

print("\n=== 绝对门扫描：读对通过 / 哼声拒绝 / 噪声拒绝 / 读错误放 ===")
for thr in (2.0, 2.5, 3.0, 3.5, 4.0, 5.0):
    tp = (own_d <= thr).sum()
    hum_ok = (hum_d > thr).sum()
    noise_ok = (noise_d > thr).sum()
    fp = (wrong_d <= thr).sum()
    print(f"  thr={thr:.1f}: 读对 {tp:>2}/{len(own_d)}  哼声拒 {hum_ok:>2}/{n}  "
          f"噪声拒 {noise_ok:>2}/{n}  读错误放 {fp:>4}/{len(wrong_d)} ({fp/len(wrong_d):.0%})")
