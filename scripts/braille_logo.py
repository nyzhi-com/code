#!/usr/bin/env python3
"""Convert a PNG image to Unicode braille dot patterns for TUI rendering.

Each braille character encodes a 2x4 dot grid (U+2800 to U+28FF).
A 128x128 icon resized to 60x60 yields ~30 cols x 15 rows.

Usage:
    python3 scripts/braille_logo.py /path/to/icon.png [--width 30] [--threshold 40]
"""

import argparse
import sys

try:
    from PIL import Image
except ImportError:
    print("pip install Pillow", file=sys.stderr)
    sys.exit(1)

BRAILLE_BASE = 0x2800

# Braille dot positions -> bit offsets
# Dot layout:    Bit values:
#   1 4            0x01 0x08
#   2 5            0x02 0x10
#   3 6            0x04 0x20
#   7 8            0x40 0x80
DOT_MAP = [
    (0, 0, 0x01), (1, 0, 0x02), (2, 0, 0x04), (0, 1, 0x08),
    (1, 1, 0x10), (2, 1, 0x20), (3, 0, 0x40), (3, 1, 0x80),
]


def image_to_braille(path: str, target_cols: int = 30, threshold: int = 40) -> str:
    img = Image.open(path).convert("RGBA")

    px_w = target_cols * 2
    ratio = img.height / img.width
    px_h = int(px_w * ratio)
    px_h = px_h + (4 - px_h % 4) % 4

    img = img.resize((px_w, px_h), Image.LANCZOS)
    pixels = img.load()

    rows = px_h // 4
    cols = px_w // 2
    lines = []

    for row in range(rows):
        line = []
        for col in range(cols):
            code = 0
            for dy, dx, bit in DOT_MAP:
                x = col * 2 + dx
                y = row * 4 + dy
                if x < px_w and y < px_h:
                    r, g, b, a = pixels[x, y]
                    if a > threshold:
                        code |= bit
            line.append(chr(BRAILLE_BASE + code))
        # Strip trailing blank braille chars
        text = "".join(line).rstrip(chr(BRAILLE_BASE))
        lines.append(text)

    # Strip leading/trailing empty lines
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="PNG to braille converter")
    parser.add_argument("image", help="Path to PNG image")
    parser.add_argument("--width", type=int, default=30, help="Target width in braille columns")
    parser.add_argument("--threshold", type=int, default=40, help="Alpha threshold (0-255)")
    parser.add_argument("--rust", action="store_true", help="Output as Rust const string")
    args = parser.parse_args()

    braille = image_to_braille(args.image, args.width, args.threshold)

    if args.rust:
        escaped = braille.replace("\\", "\\\\").replace('"', '\\"')
        print(f'pub const LOGO_BRAILLE: &str = "\\')
        for line in escaped.split("\n"):
            print(f"{line}\\n\\")
        print('";')
    else:
        print(braille)


if __name__ == "__main__":
    main()
