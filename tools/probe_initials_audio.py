#!/usr/bin/env python3
"""Probe Baidu CDN for SINGLE-LETTER initial (声母) real-voice audio.

The 拼读 module currently falls back to local TTS <letter>.m4a for initials
because baidu_audio.json has no bare-initial entries. This script checks whether
the CDN actually serves e.g. b1.mp3 / d1.mp3, and if so reports sizes so we can
download + verify the pronunciation before switching sources.

CDN: https://hanyu-word-pinyin-short.cdn.bcebos.com/<pinyin><tone>.mp3
"""
from __future__ import annotations

import concurrent.futures
import json
import urllib.request
from pathlib import Path

CDN = "https://hanyu-word-pinyin-short.cdn.bcebos.com"
ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tools" / "baidu_initial_probe.json"

INITIALS = [
    "b", "p", "m", "f", "d", "t", "n", "l", "g", "k", "h",
    "j", "q", "x", "zh", "ch", "sh", "r", "z", "c", "s", "y", "w",
]
TONES = [1, 2, 3, 4]


def url_for(s: str, tone: int) -> str:
    return f"{CDN}/{s}{tone}.mp3"


def probe(arg: tuple[str, int]) -> tuple[str, int, int, str]:
    s, t = arg
    try:
        req = urllib.request.Request(url_for(s, t), method="GET")
        with urllib.request.urlopen(req, timeout=15) as r:
            data = r.read(4096)  # only need size probe
            total = r.headers.get("Content-Length")
            status = r.status
        return s, t, status, (str(total) if total else f">={len(data)}")
    except Exception as e:  # noqa: BLE001
        return s, t, 0, type(e).__name__


def main() -> None:
    args = [(s, t) for s in INITIALS for t in TONES]
    print(f"probing {len(INITIALS)} initials x {len(TONES)} tones = {len(args)} URLs ...")
    found: dict[str, list[int]] = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=12) as ex:
        for s, t, status, info in ex.map(probe, args):
            if status == 200:
                found.setdefault(s, []).append(t)
                print(f"  HIT  {s}{t}.mp3  status={status} size={info}")
            else:
                print(f"  miss {s}{t}.mp3  status={status} {info}")
    result = {s: sorted(v) for s, v in sorted(found.items()) if v}
    OUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"\nFOUND {sum(len(v) for v in result.values())}/{len(args)} "
          f"across {len(result)}/{len(INITIALS)} initials")
    print(f"manifest -> {OUT}")
    for s in INITIALS:
        ts = result.get(s, [])
        print(f"  {s:>2}: {''.join(map(str, ts)) if ts else '-'}")


if __name__ == "__main__":
    main()
