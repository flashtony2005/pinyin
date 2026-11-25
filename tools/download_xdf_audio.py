#!/usr/bin/env python3
"""Download the complete XDF (新东方) pinyin real-voice audio set.

Source: https://www.xdf.cn/zhuanti/suyang_pinyin/js/dict.js  (DICT_MAP)
  group 1 = 字母表(29) 2 = 声母表(23) 3 = 整体认读(64)
  4 = 单韵母 5 = 复韵母 6 = 前鼻韵母 7 = 后鼻韵母

Saves to assets/audio/xdf/<group>/<name>.mp3 and writes tools/xdf_audio.json
as the manifest {group: {pinyinWord: relative_path}}.
"""
from __future__ import annotations

import concurrent.futures
import json
import re
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "audio" / "xdf"
MANIFEST = ROOT / "tools" / "xdf_audio.json"
DICT = ROOT / "tools" / "xdf_dict.js"
DICT_URL = "https://www.xdf.cn/zhuanti/suyang_pinyin/js/dict.js"
UA = {"User-Agent": "Mozilla/5.0"}

GROUPS = {
    "1": "zimu",      # 字母表
    "2": "shengmu",   # 声母表
    "3": "zhengti",   # 整体认读
    "4": "danyun",    # 单韵母
    "5": "fuyun",     # 复韵母
    "6": "qianbi",    # 前鼻韵母
    "7": "houbi",     # 后鼻韵母
}


def fetch_text(url: str) -> str:
    return urllib.request.urlopen(
        urllib.request.Request(url, headers=UA), timeout=30
    ).read().decode("utf-8", "ignore")


def parse(dict_js: str) -> dict[str, list[tuple[str, str]]]:
    starts = [(m.group(1), m.start()) for m in re.finditer(r"'(\d+)'\s*:\s*\[", dict_js)]
    starts.append(("END", len(dict_js)))
    out: dict[str, list[tuple[str, str]]] = {}
    for i in range(len(starts) - 1):
        k, s = starts[i]
        e = starts[i + 1][1]
        rows = []
        for it in re.findall(r"\{[^{}]*\}", dict_js[s:e]):
            a = re.search(r'"audioUrl":\s*"([^"]+)"', it)
            p = re.search(r'"pinyinWord":\s*"([^"]+)"', it)
            if a and p:
                rows.append((p.group(1), a.group(1)))
        if rows:
            out[k] = rows
    return out


def download(task: tuple[str, str, str]) -> tuple[str, str, str, int]:
    group, word, url = task
    safe = url.rsplit("/", 1)[-1]
    dst = OUT / GROUPS[group] / safe
    if dst.exists() and dst.stat().st_size > 1000:
        return group, word, str(dst.relative_to(ROOT)), dst.stat().st_size
    try:
        data = urllib.request.urlopen(
            urllib.request.Request(url, headers=UA), timeout=30
        ).read()
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_bytes(data)
        return group, word, str(dst.relative_to(ROOT)), len(data)
    except Exception as e:  # noqa: BLE001
        return group, word, f"FAIL {type(e).__name__}", 0


def main() -> None:
    if DICT.exists():
        txt = DICT.read_text(encoding="utf-8")
    else:
        txt = fetch_text(DICT_URL)
        DICT.write_text(txt, encoding="utf-8")
    groups = parse(txt)
    tasks = [(g, w, u) for g, rows in groups.items() for w, u in rows]
    print(f"total {len(tasks)} audios across {len(groups)} groups")
    manifest: dict[str, dict[str, str]] = {GROUPS[g]: {} for g in groups}
    ok = fail = 0
    with concurrent.futures.ThreadPoolExecutor(max_workers=12) as ex:
        for group, word, path, size in ex.map(download, tasks):
            if size:
                manifest[GROUPS[group]][word] = path
                ok += 1
            else:
                fail += 1
                print(f"  FAIL {group} {word}: {path}")
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8")
    print(f"downloaded {ok}/{len(tasks)} (fail {fail})")
    for g, name in GROUPS.items():
        if g in groups:
            print(f"  {name:8s} {len(manifest[name])}/{len(groups[g])}")
    print(f"manifest -> {MANIFEST}")


if __name__ == "__main__":
    main()
