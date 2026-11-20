"""Export the approved Erpin cutout as PNG/ICO and a Windows linker resource.

Requires Pillow and Windows SDK rc.exe (on PATH or supplied with --rc).
The checked-in .res lets normal Cargo builds remain fully offline and avoids
requiring an additional resource compiler step on every build.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import shutil
import subprocess

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "assets/ui/icons"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rc", default=shutil.which("rc.exe"),
                        help="Path to the Windows SDK resource compiler")
    args = parser.parse_args()
    if not args.rc:
        parser.error("Pass --rc with the Windows SDK rc.exe path")
    source = Image.open(ICONS / "erpin-source.png").convert("RGBA")
    if source.width != source.height or source.getchannel("A").getextrema() != (0, 255):
        raise ValueError("The source must be square with a transparent background")
    source.resize((256, 256), Image.Resampling.LANCZOS).save(ICONS / "erpin.png")
    source.save(ICONS / "erpin.ico", sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])
    source.save(ICONS / "favicon.ico", sizes=[(n, n) for n in (16, 32, 48)])
    for size in (16, 32):
        source.resize((size, size), Image.Resampling.LANCZOS).save(
            ICONS / f"favicon-{size}x{size}.png")
    subprocess.run([args.rc, "/nologo", "/fo", "erpin.res", "erpin.rc"],
                   cwd=ICONS, check=True)
    print(f"Exported app icons and favicons to {ICONS}")


if __name__ == "__main__":
    main()
