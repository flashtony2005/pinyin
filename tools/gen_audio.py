#!/usr/bin/env python3
"""Generate pinyin audio assets with macOS `say`.

Produces one AAC file per initial, final and legal syllable.
Requires: macOS with the Tingting (zh_CN) voice, afconvert.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

VOICE = "Tingting"
ROOT = Path(__file__).resolve().parent.parent
TABLE = ROOT / "tools" / "pinyin_table.json"
SPEAK_MAP = ROOT / "tools" / "speak_map.json"
OUT = ROOT / "assets" / "audio"


def load_speak_map() -> dict[str, str]:
    """pinyin -> representative Han character (built by build_speak_map.py).

    Raw pinyin text is read unreliably by the TTS (e.g. "kou" -> "古"), so a
    single homophone character is spoken instead.
    """
    if not SPEAK_MAP.exists():
        raise SystemExit(f"missing {SPEAK_MAP}; run: python3 tools/build_speak_map.py")
    return json.loads(SPEAK_MAP.read_text(encoding="utf-8"))


def say(text: str, dst: Path) -> None:
    # Keep the intermediate AIFF in the OS temp dir; do not delete it here
    # (bulk-delete guards flag repeated unlink calls in one session).
    with tempfile.TemporaryDirectory(prefix="pinyin-tts-") as td:
        aiff = Path(td) / "voice.aiff"
        subprocess.run(["say", "-v", VOICE, "-o", str(aiff), text], check=True)
        subprocess.run(
            ["afconvert", "-f", "m4af", "-d", "aac", "-b", "48000", str(aiff), str(dst)],
            check=True,
        )


def speakable(text: str, speak_map: dict[str, str]) -> str:
    """Map a pinyin token to a Chinese character that reads it aloud."""
    return speak_map[text]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--force", action="store_true", help="regenerate existing files")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    speak_map = load_speak_map()
    table = json.loads(TABLE.read_text(encoding="utf-8"))
    entries: list[tuple[str, str]] = []
    for category in ("initials", "finals", "syllables"):
        for item in table[category]:
            value = item["pinyin"] if isinstance(item, dict) else item
            entries.append((value, speakable(value, speak_map)))

    OUT.mkdir(parents=True, exist_ok=True)
    made = 0
    for pinyin, text in entries:
        dst = OUT / f"{pinyin}.m4a"
        if dst.exists() and not args.force:
            continue
        if not args.dry_run:
            say(text, dst)
        made += 1
    print(f"audio: {len(entries)} entries, {made} generated, {len(entries) - made} existing")


if __name__ == "__main__":
    sys.exit(main())
