"""生成独立探针夹具：没被 collect_sources 收进模板池的同字母发音。

关键：build_letter_templates.py 的 collect_sources 会收 bd/{syl}{1,4,2}.mp3
（TONE_ORDER 前 3 个）+ xdf + _asr。本脚本对每个字母挑一个**没被收进去**的
样本（优先 bd 的第 3/4 声调），解码成 f32 供 Rust 集成测试用。

用法：python tools/build_indep_probes.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import build_letter_templates as B
import diag_honest as H

OUT = B.ROOT / "tools" / "probes_indep"


def main():
    OUT.mkdir(exist_ok=True)
    # 清掉旧夹具（避免残留同源探针污染统计）
    for p in OUT.glob("*.f32"):
        p.unlink()

    masks = B.load_baidu_mask()
    count = 0
    for idx, (letter, _cands) in enumerate(B.LETTERS):
        primary, _ = B.pick_source(letter, masks)
        if primary is None:
            continue
        in_tpl = {p for p, _ in B.collect_sources(letter, primary)}
        syl = B.SYLLABLE_HINT.get(letter, letter)

        # 按 3,2,4,1 顺序找没进模板的 bd 样本（前 3 个声调通常已被收）
        probe = None
        for tone in (3, 2, 4, 1):
            cand = B.BD / f"{syl}{tone}.mp3"
            if cand.exists() and cand not in in_tpl and cand.stat().st_size > 1000:
                probe = cand
                break
        if probe is None:
            # bd 全被收了，退而求其次：xdf 里没进模板的分组
            for grp in ("zimu", "danyun", "fuyun", "shengmu"):
                cand = B.XDF / grp / f"{letter}.mp3"
                if cand.exists() and cand not in in_tpl and cand.stat().st_size > 1000:
                    probe = cand
                    break
        if probe is None:
            print(f"  skip {letter}: 所有候选都在模板池里")
            continue

        pcm = B.trim_silence(B.load_pcm(probe))
        (OUT / f"{idx:02}_{letter}.f32").write_bytes(pcm.astype("<f4").tobytes())
        count += 1
        print(f"  {idx:02} {letter:<3} <- {probe.relative_to(B.ROOT)}")

    print(f"\nwrote {count} independent probes to {OUT}")


if __name__ == "__main__":
    main()
