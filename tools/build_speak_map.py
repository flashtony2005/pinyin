#!/usr/bin/env python3
"""Build pinyin -> representative Han character map for TTS.

macOS `say` reads raw pinyin text unreliably (e.g. "kou" -> "古", "ma" -> "Mom").
Feeding a single Chinese character pronounced as the target syllable is stable.
This script derives that map from the GB2312 character set via pypinyin,
preferring the first (level) tone, which is the teaching default for a bare syllable.

Output: tools/speak_map.json  {"pinyin": "representative character", ...}

Requires: pypinyin (`pip install pypinyin`).
"""
from __future__ import annotations

import json
import unicodedata
from pathlib import Path

from pypinyin import pinyin, Style

ROOT = Path(__file__).resolve().parent.parent
TABLE = ROOT / "tools" / "pinyin_table.json"
OUT = ROOT / "tools" / "speak_map.json"

TONE_MARKS = "āáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜü"


def strip_tone(s: str) -> str:
    return "".join(
        c for c in unicodedata.normalize("NFD", s) if unicodedata.category(c) != "Mn"
    ).replace("v", "ü")


def gb2312_chars() -> list[str]:
    chars = []
    # GB2312 0xB0-0xD7 is the Hanzi block; 0xA1-0xAF holds symbols/pinyin letters.
    for h in range(0xB0, 0xD8):
        for l in range(0xA1, 0xFF):
            try:
                chars.append(bytes([h, l]).decode("gb2312"))
            except UnicodeDecodeError:
                pass
    return chars


def main() -> None:
    table = json.loads(TABLE.read_text(encoding="utf-8"))
    targets = sorted(set(table["initials"] + table["finals"] + table["syllables"]))

    # Collect characters by (plain syllable, tone number). Tone 1 is preferred.
    by_syllable: dict[str, dict[int, list[str]]] = {}
    def is_han(c: str) -> bool:
        return "\u4e00" <= c <= "\u9fff"

    for c in gb2312_chars():
        if not is_han(c):
            continue
        toned = "".join(pinyin(c, style=Style.TONE)[0])
        plain = strip_tone(toned)
        if not plain:
            continue
        tone = 0
        for mark, num in zip("āáǎà", range(1, 5)):
            if mark in toned:
                tone = num
                break
        else:
            for mark, num in zip("ēéěè", range(1, 5)):
                if mark in toned:
                    tone = num
                    break
        # A neutral/missing tone mark still has a plain syllable; keep as tone 0.
        by_syllable.setdefault(plain, {}).setdefault(tone, []).append(c)

    # Hand-picked authoritative characters, keyed by target spelling. These cover
    # initials (read with their teaching "呼读音") and rare syllables absent from GB2312.
    manual: dict[str, str] = {
        # initials (呼读音, tone 1 where possible)
        "b": "玻", "p": "坡", "m": "摸", "f": "佛", "d": "得", "t": "特",
        "n": "讷", "l": "勒", "g": "哥", "k": "科", "h": "喝", "j": "基",
        "q": "欺", "x": "希", "zh": "知", "ch": "吃", "sh": "诗", "r": "日",
        "z": "资", "c": "次", "s": "思", "y": "衣", "w": "乌",
        # rare syllables
        "dei": "得", "den": "扽", "fo": "佛", "kei": "剋", "len": "冷",
        "lo": "咯", "lü": "驴", "lüe": "略", "nü": "女", "nüe": "虐",
        "shei": "谁", "yai": "呀", "yei": "诶", "yen": "烟", "yeng": "英",
        "yie": "椰", "zhei": "这",
        # teaching-spelling finals (pypinyin uses iou/uei/uen/ueng etc.)
        "eng": "鞥", "ie": "耶", "in": "因", "ing": "英", "iu": "优",
        "ong": "轰", "ui": "威", "un": "温",
        "ü": "迂", "üe": "约", "ün": "晕",
        # rare syllables absent from GB2312 Hanzi block
        "cen": "岑", "ei": "诶", "i": "衣", "nou": "耨", "u": "乌",
    }

    result: dict[str, str] = {}
    missing: list[str] = []
    for s in targets:
        if s in manual:
            result[s] = manual[s]
            continue
        tones = by_syllable.get(s, {})
        # Prefer tone 1, then 2, 3, 4, then neutral, choosing the most common char
        # (earlier GB2312 code point ~ more common for level-1 set).
        picked = None
        for tone in (1, 2, 3, 4, 0):
            if tones.get(tone):
                picked = tones[tone][0]
                break
        if picked:
            result[s] = picked
        else:
            missing.append(s)

    OUT.write_text(
        json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(f"speak map: {len(result)} entries -> {OUT}")
    if missing:
        print(f"WARNING: {len(missing)} syllables without a character: {missing}")
    for s in ["ou", "kou", "ba", "ma", "dou", "hao", "zhong", "a", "o", "e"]:
        print(f"  {s} -> {result.get(s)}")


if __name__ == "__main__":
    main()
