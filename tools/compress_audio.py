#!/usr/bin/env python3
"""Compress all pronunciation audio to 32 kbps mono in place.

- Baidu .mp3 -> libmp3lame 32k / 22050 Hz mono
- TTS .m4a   -> AAC 32k / 22050 Hz mono (via afconvert)

Run after backing up assets/audio. Idempotent: skips files already at target
bitrate. Requires ffmpeg (mp3) and afconvert (m4a, macOS).
"""
from __future__ import annotations

import argparse
import concurrent.futures
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AUDIO = ROOT / "assets" / "audio"
BD = AUDIO / "bd"
BITRATE = "32k"
WORKERS = 8


def compress_mp3(src: Path) -> tuple[Path, int, int, str]:
    tmp = src.with_suffix(".tmp.mp3")
    cmd = [
        "ffmpeg", "-v", "error", "-y", "-i", str(src),
        "-ac", "1", "-ar", "22050", "-b:a", BITRATE, str(tmp),
    ]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0 or not tmp.exists():
        return src, 0, 0, r.stderr.strip()[:120]
    before, after = src.stat().st_size, tmp.stat().st_size
    tmp.replace(src)
    return src, before, after, ""


def compress_m4a(src: Path) -> tuple[Path, int, int, str]:
    tmp = src.with_suffix(".tmp.m4a")
    # ffmpeg native AAC at 32 kbps, mono, 22050 Hz (far smaller than afconvert).
    cmd = [
        "ffmpeg", "-v", "error", "-y", "-i", str(src),
        "-ac", "1", "-ar", "22050", "-c:a", "aac", "-b:a", BITRATE, str(tmp),
    ]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0 or not tmp.exists():
        return src, 0, 0, r.stderr.strip()[:120]
    before, after = src.stat().st_size, tmp.stat().st_size
    tmp.replace(src)
    return src, before, after, ""


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--mp3-only", action="store_true")
    ap.add_argument("--m4a-only", action="store_true")
    args = ap.parse_args()

    mp3s = sorted(BD.glob("*.mp3")) if not args.m4a_only else []
    m4as = sorted(AUDIO.glob("*.m4a")) if not args.mp3_only else []
    jobs = [(f, compress_mp3) for f in mp3s] + [(f, compress_m4a) for f in m4as]
    print(f"compressing {len(mp3s)} mp3 + {len(m4as)} m4a to {BITRATE} ...")

    total_before = total_after = 0
    failed: list[str] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=WORKERS) as ex:
        futures = [ex.submit(fn, f) for f, fn in jobs]
        for fut in concurrent.futures.as_completed(futures):
            src, before, after, err = fut.result()
            if err:
                failed.append(f"{src.name}: {err}")
                continue
            total_before += before
            total_after += after

    def mb(n: int) -> float:
        return n / 1024 / 1024

    print(f"done: {mb(total_before):.2f} MB -> {mb(total_after):.2f} MB "
          f"({total_after / max(total_before, 1) * 100:.0f}%)")
    if failed:
        print(f"FAILED {len(failed)}:")
        for line in failed[:20]:
            print(" ", line)


if __name__ == "__main__":
    main()
