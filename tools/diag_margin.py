"""用距离矩阵扫描竞争门阈值：读对 vs 读错到底能不能分开。

读对探针 = 没进模板池的同字母 bd 样本（独立）
读错探针 = 任意字母的样本去评别的字母
ratio = d_target / d_rival  (<1 = 目标比所有对手都近，是好事)
"""
import sys, math
from pathlib import Path
import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
import build_letter_templates as B
import diag_honest as H

BD = B.BD


def build():
    masks = B.load_baidu_mask()
    plans = []
    all_ceps = []
    for letter, _ in B.LETTERS:
        primary, _ = B.pick_source(letter, masks)
        if primary is None:
            continue
        samples = B.collect_sources(letter, primary)
        plans.append((letter, samples))
        for p, _ in samples:
            try:
                all_ceps.append(B.raw_ceps(B.trim_silence(B.load_pcm(p))))
            except Exception:
                pass
    stat = np.vstack([c[:, 1:] for c in all_ceps if len(c)])
    dlt = np.vstack([B.delta(c[:, 1:]) for c in all_ceps if len(c)])
    std = np.concatenate([stat.std(axis=0), dlt.std(axis=0)])
    std[std < 1e-3] = 1.0

    tmpl = []
    for letter, samples in plans:
        per = []
        for p, _ in samples:
            c = B.raw_ceps(B.trim_silence(B.load_pcm(p)))
            per.append(B.cap(B.enhance(c, std)))
        tmpl.append(per)

    # 独立读对探针（同字母不同声调，没进模板池）
    probes = []  # (letter, is_own_probe, pcm_features)
    for idx, (letter, samples) in enumerate(plans):
        in_tpl = H.template_paths(letter)
        syl = B.SYLLABLE_HINT.get(letter, letter)
        for tone in (1, 2, 3, 4):
            cand = BD / f"{syl}{tone}.mp3"
            if cand.exists() and cand not in in_tpl and cand.stat().st_size > 1000:
                try:
                    pcm = B.trim_silence(B.load_pcm(cand))
                    a, b = H.rust_voice_span(pcm)
                    if b <= a:
                        continue
                    v = pcm[a:b]
                    if H.zcr(v) > 0.35:
                        continue
                    f = B.cap(B.enhance(B.raw_ceps(v), std))
                    if len(f):
                        probes.append((idx, True, f, letter, cand.name))
                        break
                except Exception:
                    pass

    # 距离矩阵 [probe][letter]
    D = np.full((len(probes), len(plans)), np.inf)
    for pi, (_, _, f, _, _) in enumerate(probes):
        for li in range(len(plans)):
            D[pi, li] = min(H.dtw(f, t) for t in tmpl[li])
    return plans, probes, D


def main():
    plans, probes, D = build()
    print(f"读对探针 {len(probes)} 个，字母 {len(plans)} 个")

    own_ratios, own_abs = [], []
    wrong_ratios, wrong_abs = [], []
    for pi, (self_idx, _, _, lname, pname) in enumerate(probes):
        dt = D[pi, self_idx]
        rivals = [D[pi, j] for j in range(len(plans)) if j != self_idx]
        dr = min(rivals)
        ratio = dt / dr if dr > 0 else float("inf")
        own_ratios.append(ratio)
        own_abs.append(dt)
        for j in range(len(plans)):
            if j == self_idx:
                continue
            dt_w = D[pi, j]
            rivals_w = [D[pi, k] for k in range(len(plans)) if k != j]
            dr_w = min(rivals_w)
            wrong_ratios.append(dt_w / dr_w if dr_w > 0 else float("inf"))
            wrong_abs.append(dt_w)

    own_ratios = np.array(own_ratios)
    wrong_ratios = np.array(wrong_ratios)
    print("\n读对 ratio=d_t/d_r (越小越好):")
    print(f"  p10={np.percentile(own_ratios,10):.2f} p50={np.percentile(own_ratios,50):.2f} "
          f"p90={np.percentile(own_ratios,90):.2f} max={own_ratios.max():.2f}")
    print("读错 ratio=d_t/d_r (越大越好, >1 表示对手更近):")
    print(f"  p10={np.percentile(wrong_ratios,10):.2f} p50={np.percentile(wrong_ratios,50):.2f} "
          f"p90={np.percentile(wrong_ratios,90):.2f} min={wrong_ratios.min():.2f}")

    own_abs = np.array(own_abs)
    wrong_abs = np.array(wrong_abs)
    print("\n绝对距离 d_t (竞争门无关):")
    print(f"  读对 p10={np.percentile(own_abs,10):.2f} p50={np.percentile(own_abs,50):.2f} "
          f"p90={np.percentile(own_abs,90):.2f}")
    print(f"  读错 p10={np.percentile(wrong_abs,10):.2f} p50={np.percentile(wrong_abs,50):.2f} "
          f"p90={np.percentile(wrong_abs,90):.2f}")
    print("\n绝对门扫描 (判读对条件: d_t <= threshold):")
    print(f"{'thr':>6} {'读对通过':>10} {'读错误放':>10} {'误放率':>8}")
    for m in (2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 6.0):
        tp = int((own_abs <= m).sum())
        fp = int((wrong_abs <= m).sum())
        print(f"{m:6.1f} {tp:>6}/{len(own_abs):<3} {fp:>6}/{len(wrong_abs):<4} {fp/len(wrong_abs):>7.1%}")

    print("\n竞争门阈值扫描 (判读对条件: ratio <= margin):")
    print(f"{'margin':>7} {'读对通过':>10} {'读错误放':>10} {'误放率':>8}")
    for m in (0.85, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.7, 2.0):
        tp = int((own_ratios <= m).sum())
        fp = int((wrong_ratios <= m).sum())
        print(f"{m:7.2f} {tp:>6}/{len(own_ratios):<3} {fp:>6}/{len(wrong_ratios):<4} {fp/len(wrong_ratios):>7.1%}")


if __name__ == "__main__":
    main()
