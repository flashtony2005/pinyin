#!/usr/bin/env python3
"""Download real-voice pronunciation audio from Baidu Chinese CDN.

URL pattern: https://hanyu-word-pinyin-short.cdn.bcebos.com/<pinyin><tone>.mp3
where `pinyin` uses `v` for `ü` (lü -> lv) and tone is 1..4. Not every
syllable/tone combination exists (e.g. kou2 is 404), so availability is probed
first and cached in tools/baidu_tones.json.

Outputs:
  assets/audio/bd/<pinyin><tone>.mp3   (filename keeps the teaching spelling, e.g. lü4.mp3)
  tools/baidu_audio.json               {"pinyin": [available tones], ...}

Syllables with no Baidu audio fall back to the locally generated TTS .m4a
(see gen_audio.py). Re-run probe only when the syllable table changes.
"""
from __future__ import annotations

import concurrent.futures
import json
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CDN = "https://hanyu-word-pinyin-short.cdn.bcebos.com"
TABLE = ROOT / "tools" / "pinyin_table.json"
TONES = ROOT / "tools" / "baidu_tones.json"
OUT = ROOT / "assets" / "audio" / "bd"
MANIFEST = ROOT / "tools" / "baidu_audio.json"
WORKERS = 16
RETRIES = 3


def url_for(syllable: str, tone: int) -> str:
    return f"{CDN}/{syllable.replace('ü', 'v')}{tone}.mp3"


def filename_for(syllable: str, tone: int) -> str:
    return f"{syllable}{tone}.mp3"


def probe() -> dict[str, list[int]]:
    table = json.loads(TABLE.read_text(encoding="utf-8"))
    syls = sorted(set(table["initials"] + table["finals"] + table["syllables"]))
    args = [(s, t) for s in syls for t in (1, 2, 3, 4)]

    def head(arg: tuple[str, int]) -> tuple[str, int, bool]:
        s, t = arg
        try:
            req = urllib.request.Request(url_for(s, t), method="HEAD")
            with urllib.request.urlopen(req, timeout=10) as r:
                return s, t, r.status == 200
        except Exception:
            return s, t, False

    found: dict[str, set[int]] = {s: set() for s in syls}
    with concurrent.futures.ThreadPoolExecutor(max_workers=WORKERS) as ex:
        for s, t, ok in ex.map(head, args):
            if ok:
                found[s].add(t)
    result = {s: sorted(v) for s, v in found.items()}
    TONES.write_text(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def download(item: tuple[str, int]) -> tuple[str, int, bool, str]:
    s, t = item
    dst = OUT / filename_for(s, t)
    if dst.exists() and dst.stat().st_size > 1000:
        return s, t, True, "cached"
    url = url_for(s, t)
    last = ""
    for attempt in range(RETRIES):
        try:
            with urllib.request.urlopen(url, timeout=20) as r:
                data = r.read()
            if len(data) < 1000:
                raise ValueError(f"too small: {len(data)} bytes")
            dst.write_bytes(data)
            return s, t, True, f"{len(data)}B"
        except Exception as e:  # noqa: BLE001 - network retry loop
            last = str(e)
            if attempt + 1 < RETRIES:
                time.sleep(0.4 * (attempt + 1))
    return s, t, False, last


def main() -> None:
    import argparse

    ap = argparse.ArgumentParser()
    ap.add_argument("--reprobe", action="store_true", help="re-probe CDN availability")
    args = ap.parse_args()

    if args.reprobe or not TONES.exists():
        print("probing CDN availability ...")
        tones = probe()
    else:
        tones = json.loads(TONES.read_text(encoding="utf-8"))

    items = [(s, t) for s, ts in tones.items() for t in ts]
    OUT.mkdir(parents=True, exist_ok=True)
    print(f"downloading {len(items)} files with {WORKERS} workers ...")
    ok = 0
    failed: list[tuple[str, int, str]] = []
    start = time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=WORKERS) as ex:
        for s, t, success, info in ex.map(download, items):
            if success:
                ok += 1
            else:
                failed.append((s, t, info))
    elapsed = time.time() - start

    manifest = {s: ts for s, ts in sorted(tones.items()) if ts}
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"done: {ok}/{len(items)} files in {elapsed:.1f}s -> {OUT}")
    print(f"manifest: {len(manifest)} syllables -> {MANIFEST}")
    if failed:
        print(f"FAILED {len(failed)}:")
        for s, t, info in failed[:15]:
            print(f"  {s}{t}: {info}")


if __name__ == "__main__":
    main()
