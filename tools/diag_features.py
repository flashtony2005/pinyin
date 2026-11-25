"""特征变体实验：找出能同时「读对通过 + 乱哼拒绝」的特征。

关键假设：逐句 CMN（减均值）把声母/韵母的静态频谱形状抹掉了，
而单音节区分恰恰靠静态频谱 → 去掉 CMN 应该拉开距离。
指标：闭集 top-1 准确率（读对应判给自己）+ 哼声距离。
"""
import sys, math
from pathlib import Path
import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
import build_letter_templates as B
import diag_honest as H

SR = B.SR


def dtw(a, b):
    n, m = len(a), len(b)
    if n == 0 or m == 0:
        return float("inf")
    prev = np.full(m + 1, np.inf)
    cur = np.full(m + 1, np.inf)
    prev[0] = 0.0
    for i in range(1, n + 1):
        cur[0] = np.inf
        ai = a[i - 1]
        d = np.linalg.norm(ai - b, axis=1)
        cur[1:] = d + np.minimum.reduce([prev[1:], cur[:-1], prev[:-1]])
        prev, cur = cur, prev
    return float(prev[m] / (n + m))


def prep_variants(ceps, std):
    """返回多个特征变体。ceps = raw 13维 MFCC（无 CMN）。"""
    out = {}
    base = ceps[:, 1:]  # c1..c12
    dlt = B.delta(base)
    # A: 当前 = CMN + 白化
    x = np.hstack([base, dlt])
    out["A_cmn_white"] = B.cap((x - x.mean(0)) / std)
    # B: 无 CMN，仅白化（保留静态频谱形状）
    out["B_nocmn_white"] = B.cap(x / std)
    # C: 静态 c1..c12，无 CMN，白化
    sstd = std[:12]
    out["C_static_white"] = B.cap(base / sstd)
    # D: 静态 + delta，无 CMN，用静态 std 统一缩放（不放大噪声 delta）
    out["D_static_delta"] = B.cap(np.hstack([base / sstd, dlt / sstd]))
    return out


def build():
    masks = B.load_baidu_mask()
    plans, all_ceps = [], []
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
    std = np.concatenate([stat.std(0), dlt.std(0)])
    std[std < 1e-3] = 1.0

    # 每个字母的模板特征（各变体）
    tmpl = {}
    for letter, samples in plans:
        per = {k: [] for k in ("A_cmn_white", "B_nocmn_white", "C_static_white", "D_static_delta")}
        for p, _ in samples:
            c = B.raw_ceps(B.trim_silence(B.load_pcm(p)))
            for k, f in prep_variants(c, std).items():
                per[k].append(f)
        tmpl[letter] = per

    # 独立读对探针
    probes = []
    for letter, samples in plans:
        in_tpl = H.template_paths(letter)
        syl = B.SYLLABLE_HINT.get(letter, letter)
        for tone in (1, 2, 3, 4):
            cand = B.BD / f"{syl}{tone}.mp3"
            if cand.exists() and cand not in in_tpl and cand.stat().st_size > 1000:
                try:
                    pcm = B.trim_silence(B.load_pcm(cand))
                    a, b = H.rust_voice_span(pcm)
                    if b <= a:
                        continue
                    c = B.raw_ceps(pcm[a:b])
                    probes.append((letter, prep_variants(c, std)))
                    break
                except Exception:
                    pass
    # 哼声
    t = np.arange(16000 * 8 // 10) / 16000.0
    hum = (0.5 * np.sin(2 * np.pi * 220 * t)).astype("float32")
    hum_v = prep_variants(B.raw_ceps(B.trim_silence(hum)), std)
    return list(tmpl.keys()), tmpl, probes, hum_v


def main():
    letters, tmpl, probes, hum_v = build()
    print(f"探针 {len(probes)} 个，字母 {len(letters)} 个\n")
    print(f"{'特征':<18} {'top1':>6} {'读对self_d':>10} {'读对rival_d':>11} {'ratio':>6} {'哼声min_d':>9}")
    print("-" * 68)
    for key in ("A_cmn_white", "B_nocmn_white", "C_static_white", "D_static_delta"):
        top1 = 0
        self_ds, rival_ds = [], []
        for letter, feats in probes:
            f = feats[key]
            if not len(f):
                continue
            ds = {l: min(dtw(f, t) for t in tmpl[l][key]) for l in letters}
            self_ds.append(ds[letter])
            rival_ds.append(min(v for l, v in ds.items() if l != letter))
            if min(ds, key=ds.get) == letter:
                top1 += 1
        hum_ds = [min(dtw(hum_v[key], t) for t in tmpl[l][key]) for l in letters]
        sd = np.array(self_ds)
        rd = np.array(rival_ds)
        ratio = np.median(sd / rd)
        print(f"{key:<18} {top1:>3}/{len(probes):<3} {np.median(sd):>10.2f} "
              f"{np.median(rd):>11.2f} {ratio:>6.2f} {min(hum_ds):>9.2f}")


if __name__ == "__main__":
    main()
