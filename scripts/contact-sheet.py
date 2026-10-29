#!/usr/bin/env python3
"""Tile PNGs, each with its file name under it, into a few large images: contact sheets, for
looking over many new samples at once for a name or chat that samples/README.md doesn't allow.

    python scripts/contact-sheet.py <folder-or-png>... [--out <folder>] [--tile <px>] [--columns <n>] [--rows <n>]

A folder's PNGs are taken with those in its subfolders, in name order. A frame bigger than a tile
is shrunk to fit it; a smaller one is left as it is. Sheets go to `_scratch/contact/` at the repo's
root unless `--out` says otherwise, as `sheet-01.png`, `sheet-02.png`..., and their paths are
printed. Shrunk whole client areas can hide a name: give them a larger `--tile`.
"""

import argparse
import sys
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    sys.exit("contact-sheet.py needs Pillow: python -m pip install Pillow")

LABEL = 18
GAP = 8
BACKGROUND = (32, 32, 32)
TEXT = (230, 230, 230)


def pngs(args: list[str]) -> list[tuple[Path, str]]:
    """Each PNG to tile, with its label: its path under the folder given, or its name."""
    found = []
    for arg in args:
        path = Path(arg)
        if path.is_dir():
            found += [(png, png.relative_to(path).as_posix()) for png in sorted(path.rglob("*.png"))]
        elif path.suffix.lower() == ".png" and path.is_file():
            found.append((path, path.name))
        else:
            sys.exit(f"{arg}: not a folder or a PNG")
    return found


def font() -> ImageFont.ImageFont:
    try:
        return ImageFont.load_default(size=14)
    except TypeError:  # Pillow before 10.1 has one size.
        return ImageFont.load_default()


def sheet(tiles: list[tuple[Path, str]], tile: int, columns: int) -> Image.Image:
    shown = []
    for path, label in tiles:
        with Image.open(path) as img:
            img = img.convert("RGB")
        img.thumbnail((tile, tile))
        shown.append((img, label))
    # Each row as tall as its tallest frame, so a sheet of short crops stays short.
    rows = [shown[i : i + columns] for i in range(0, len(shown), columns)]
    heights = [max(img.height for img, _ in row) + LABEL + GAP for row in rows]
    out = Image.new("RGB", (columns * (tile + GAP) + GAP, sum(heights) + GAP), BACKGROUND)
    draw = ImageDraw.Draw(out)
    text = font()
    y = GAP
    for row, height in zip(rows, heights):
        for i, (img, label) in enumerate(row):
            x = GAP + i * (tile + GAP)
            out.paste(img, (x, y))
            # A label longer than the tile is cut from the left: the end of a name says the most.
            while len(label) > 1 and draw.textlength(label, font=text) > tile:
                label = "…" + label[2:]
            draw.text((x, y + img.height + 2), label, fill=TEXT, font=text)
        y += height
    return out


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("paths", nargs="+", help="folders of PNGs, or PNGs")
    parser.add_argument("--out", type=Path, default=root / "_scratch" / "contact", help="where the sheets go")
    parser.add_argument("--tile", type=int, default=480, help="the largest a frame is shown, in px (480)")
    parser.add_argument("--columns", type=int, default=6, help="tiles across a sheet (6)")
    parser.add_argument("--rows", type=int, default=6, help="tiles down a sheet (6)")
    args = parser.parse_args()

    found = pngs(args.paths)
    if not found:
        sys.exit("no PNGs found")
    args.out.mkdir(parents=True, exist_ok=True)
    per_sheet = args.columns * args.rows
    for n, start in enumerate(range(0, len(found), per_sheet), 1):
        path = args.out / f"sheet-{n:02}.png"
        sheet(found[start : start + per_sheet], args.tile, args.columns).save(path)
        print(path)
    print(f"{len(found)} PNGs on {n} sheets")


if __name__ == "__main__":
    main()
