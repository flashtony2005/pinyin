#!/usr/bin/env python3
"""Download triple-syllable whole-audio from Baidu CDN into assets/audio/bd/.

Uses the probe result tools/baidu_triplet_tones.json. Filenames match the
existing two-syllable convention: <syllable><tone>.mp3 (e.g. jia1.mp3).
Safe to re-run: files >1KB are skipped.
"""
from __future__ import annotations

import concurrent.futures
import json
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CDN = "https://hanyu-word-pinyin-short.cdn.bcebos.com"
MANIFEST = ROOT / "tools" / "baidu_triplet_tones.json"
OUT = ROOT / "assets" / "audio" / "bd"
WORKERS = 16
RETRIES = 3


def url_for(s: str, tone: int) -> str:
    return f"{CDN}/{s.replace('ü', 'v')}{tone}.mp3"


def download(item: tuple[str, int]) -> tuple[str, int, bool, str]:
    s, t = item
    dst = OUT / f"{s}{t}.mp3"
    if dst.exists() and dst.stat().st_size > 1000:
        return s, t, True, "cached"
    for attempt in range(RETRIES):
        try:
            with urllib.request.urlopen(url_for(s, t), timeout=20) as r:
                data = r.read()
            if len(data) < 1000:
                raise ValueError(f"too small: {len(data)}B")
            dst.write_bytes(data)
            return s, t, True, f"{len(data)}B"
        except Exception as e:  # noqa: BLE001 - network retry loop
            if attempt + 1 < RETRIES:
                time.sleep(0.4 * (attempt + 1))
            last = str(e)
    return s, t, False, last


def main() -> None:
    tones = json.loads(MANIFEST.read_text(encoding="utf-8"))
    items = [(s, t) for s, ts in tones.items() for t in ts]
    OUT.mkdir(parents=True, exist_ok=True)
    print(f"downloading {len(items)} triple-audios with {WORKERS} workers ...")
    ok = 0
    failed = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=WORKERS) as ex:
        for s, t, success, info in ex.map(download, items):
            if success:
                ok += 1
            else:
                failed.append((s, t, info))
    print(f"done: {ok}/{len(items)} -> {OUT}")
    if failed:
        print(f"FAILED {len(failed)}:")
        for f in failed[:15]:
            print(f"  {f[0]}{f[1]}: {f[2]}")


if __name__ == "__main__":
    main()
