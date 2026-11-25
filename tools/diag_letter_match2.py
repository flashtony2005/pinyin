"""用真实探针数据评估「闭集辨认 + margin」是否可行。

探针来源（与模板不同录音，模拟真实跟读）：
  - bd/<音节><其他声调>.mp3   （同发音人，不同声调）
  - xdf/<组>/<字母>.mp3        （不同发音人）
判定目标：录音对 47 个模板打分，要求
  1) 目标字母排名第 1
  2) 目标与第二名有足够差距（margin）

跑法: python tools/diag_letter_match2.py
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_letter_templates as B  # noqa: E402

BD = B.ROOT / "assets" / "audio" / "bd"
XDF = B.ROOT / "assets" / "audio" / "xdf"


def dtw(a, b):
    n, m = len(a), len(b)
    if n == 0 or m == 0:
        return float("inf")
    prev = np.full(m + 1, np.inf)
    cur = np.full(m + 1, np.inf)
    prev[0] = 0.0
    for i in range(1, n + 1):
        cur[0] = np.inf
        for j in range(1, m + 1):
            cur[j] = float(np.linalg.norm(a[i - 1] - b[j - 1])) + min(prev[j], cur[j - 1], prev[j - 1])
        prev, cur = cur, prev
    return prev[m] / (n + m)


def cap(feats, max_frames=B.MAX_FRAMES):
    if feats.shape[0] > max_frames:
        step = feats.shape[0] / max_frames
        feats = feats[[int(j * step) for j in range(max_frames)]]
    return feats


def delta(m):
    """一阶差分（拼读动作的时间变化，对辅音区分帮助大）。"""
    if len(m) < 2:
        return np.zeros_like(m)
    d = np.zeros_like(m)
    d[0] = m[1] - m[0]
    d[-1] = m[-1] - m[-2]
    if len(m) > 2:
        d[1:-1] = (m[2:] - m[:-2]) * 0.5
    return d


VARIANTS = {
    "A 原始13维": lambda m: m,
    "B 去c0": lambda m: m[:, 1:],
    "C 去c0+delta": lambda m: np.hstack([m[:, 1:], delta(m[:, 1:])]),
}

# 探针：字母 -> (bd 声调文件前缀, xdf 路径)
XDF_MAP = {}


def build_xdf_map():
    for grp in ("zimu", "shengmu"):
        d = XDF / grp
        if not d.exists():
            continue
        for p in d.glob("*.mp3"):
            XDF_MAP.setdefault(p.stem, p)


def main():
    mask = B.load_baidu_mask()
    build_xdf_map()

    letters, tmpl_src = [], []
    for letter, _ in B.LETTERS:
        path, _info = B.pick_source(letter, mask)
        if path is None:
            continue
        letters.append(letter)
        tmpl_src.append(path)
    n = len(letters)

    # 模板特征（按变体）
    base = []
    for p in tmpl_src:
        base.append(cap(B.mfcc(B.trim_silence(B.load_pcm(p)))))
    base = np.array(base, dtype=object)

    # 探针
    probes = []  # (letter_index, feats, desc)
    for i, letter in enumerate(letters):
        syl = B.LETTER_SYLLABLES[i] if hasattr(B, "LETTER_SYLLABLES") else None
        # bd 其他声调
        cands = sorted(BD.glob(f"{letter}[1-4].mp3"))
        if not cands:
            # 字母本身就是音节（如 a/o/e）
            cands = sorted(BD.glob(f"{letter}[1-4].mp3"))
        for p in cands:
            if p.name in {Path(tmpl_src[i]).name}:
                continue
            probes.append((i, cap(B.mfcc(B.trim_silence(B.load_pcm(p)))), f"bd:{p.name}"))
        # xdf 不同发音人
        p = XDF_MAP.get(letter)
        if p is not None:
            probes.append((i, cap(B.mfcc(B.trim_silence(B.load_pcm(p)))), f"xdf:{p.name}"))

    print(f"templates={n}  probes={len(probes)}")

    for name, prep in VARIANTS.items():
        T = [prep(f) for f in base]
        # 白化：用全部模板帧估每个维度的标准差
        allf = np.vstack(T)
        std = allf.std(axis=0)
        std[std < 1e-3] = 1.0

        top1 = 0
        margins = []
        fails = []
        for li, pf, desc in probes:
            q = prep(pf) / std
            d = np.array([dtw(q, t / std) for t in T])
            order = np.argsort(d)
            best, second = order[0], order[1]
            if best == li:
                top1 += 1
                margins.append((d[second] - d[best]) / max(d[best], 1e-6))
            else:
                fails.append((letters[li], desc, letters[best], d[best], d[li]))

        acc = top1 / max(len(probes), 1)
        m = np.array(margins) if margins else np.array([0.0])
        print(f"\n=== {name} ===")
        print(f"  top1 命中: {top1}/{len(probes)} = {acc*100:.1f}%")
        if len(margins):
            print(f"  命中时相对 margin: p5={np.percentile(m,5):.3f} "
                  f"p25={np.percentile(m,25):.3f} p50={np.median(m):.3f}")
        if fails:
            print(f"  失败 {len(fails)} 例，前 5:")
            for f in fails[:5]:
                print(f"    {f[0]:>3} ({f[1]}) → 误判 {f[2]:>3}  d_best={f[3]:.2f} d_truth={f[4]:.2f}")


if __name__ == "__main__":
    main()
