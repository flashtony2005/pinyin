#!/usr/bin/env python3
"""Probe Baidu Chinese CDN for triple-syllable (三拼音节) whole-syllable audio.

Reuses the same CDN as download_baidu_audio.py:
  https://hanyu-word-pinyin-short.cdn.bcebos.com/<pinyin><tone>.mp3

If these exist, they can replace the current decomposed playback (j+i+a)
with a single real-voice sample (jia1.mp3), which is what native speakers say.
"""
from __future__ import annotations

import concurrent.futures
import json
import urllib.request
from pathlib import Path

CDN = "https://hanyu-word-pinyin-short.cdn.bcebos.com"
OUT = Path(__file__).resolve().parent / "baidu_triplet_tones.json"

# 83 standard triple syllables (hanyupinyin.cn), grouped by medial.
TRIPLETS = {
    "ia": "dia lia jia qia xia",
    "ua": "gua kua hua zhua chua shua",
    "uo": "duo tuo nuo luo guo kuo huo zhuo chuo shuo ruo zuo cuo suo",
    "uai": "guai kuai huai zhuai chuai shuai",
    "uan": "duan tuan nuan luan guan kuan huan juan quan xuan zhuan chuan shuan ruan zuan cuan suan",
    "ian": "bian pian mian dian tian nian lian jian qian xian",
    "iao": "biao piao miao diao tiao niao liao jiao qiao xiao",
    "iang": "niang liang jiang qiang xiang",
    "uang": "guang kuang huang zhuang chuang shuang",
    "iong": "jiong qiong xiong",
}
SYLS = sorted({s for grp in TRIPLETS.values() for s in grp.split()})


def url_for(s: str, tone: int) -> str:
    return f"{CDN}/{s.replace('ü', 'v')}{tone}.mp3"


def head(arg: tuple[str, int]) -> tuple[str, int, bool]:
    s, t = arg
    try:
        req = urllib.request.Request(url_for(s, t), method="HEAD")
        with urllib.request.urlopen(req, timeout=12) as r:
            return s, t, r.status == 200
    except Exception:
        return s, t, False


def main() -> None:
    args = [(s, t) for s in SYLS for t in (1, 2, 3, 4)]
    print(f"probing {len(SYLS)} triple syllables x 4 tones = {len(args)} URLs ...")
    found: dict[str, list[int]] = {s: [] for s in SYLS}
    with concurrent.futures.ThreadPoolExecutor(max_workers=16) as ex:
        for s, t, ok in ex.map(head, args):
            if ok:
                found[s].append(t)
    result = {s: sorted(v) for s, v in sorted(found.items()) if v}
    OUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    total = sum(len(v) for v in result.values())
    print(f"FOUND: {total}/{len(args)} tone-audios across {len(result)}/{len(SYLS)} syllables")
    print(f"manifest -> {OUT}")
    print("\n=== coverage by syllable ===")
    for s in SYLS:
        ts = result.get(s, [])
        mark = "".join(str(t) for t in ts) if ts else "-"
        print(f"  {s:8s} {mark}")


if __name__ == "__main__":
    main()
