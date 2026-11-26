"""Lime audit for the colour popup and the palette reference sheet (polish §4.3).

usage: python lime_palette.py <palette-reference-sheet.png> <color-grid-raw.png>

The usual audit (docs/design/tools/lime_audit.py) flags a whole yellow-green hue band,
which the palette's own greens and yellows fall into by design. Here a pixel counts as
lime only when it is within OKLab dE 0.04 of the toggle lime #C7D74E itself. On the
sheet, every such pixel must belong to the tile of the legacy pulse colour #C9D24A (tile
18: the logged exception); in the popup, to its cell (row 2, column 4) or to the app's
own lime places outside the popup (the selected view icon, the loop indicator, the loop
brace).
"""
import math
import sys

from PIL import Image


def lin(c):
    c /= 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def oklab(p):
    r, g, b = (lin(v) for v in p[:3])
    l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b) ** (1 / 3)
    m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b) ** (1 / 3)
    s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b) ** (1 / 3)
    return (0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
            1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
            0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s)


LIME = oklab((0xC7, 0xD7, 0x4E))


def is_lime(p):
    q = oklab(p)
    return math.dist(q, LIME) < 0.04


def scan(path, allowed):
    im = Image.open(path).convert("RGB")
    px = im.load()
    inside, outside = 0, []
    cache = {}
    for y in range(im.height):
        for x in range(im.width):
            p = px[x, y]
            v = cache.get(p)
            if v is None:
                v = cache[p] = is_lime(p)
            if v:
                if any(x0 <= x < x1 and y0 <= y < y1 for x0, y0, x1, y1 in allowed):
                    inside += 1
                else:
                    outside.append((x, y))
    return inside, outside


def main():
    sheet, popup = sys.argv[1], sys.argv[2]
    # Sheet tiles: 7 per row, 176 x 96 from (12, 12); tile 18 is index 17.
    t = 17
    tile = (12 + (t % 7) * 176, 12 + (t // 7) * 96, 12 + (t % 7 + 1) * 176, 12 + (t // 7 + 1) * 96)
    i1, o1 = scan(sheet, [tile])
    print(f"| Reference sheet | {i1} lime px, all in tile 18 (#C9D24A, the legacy pulse) | {len(o1)} elsewhere |")
    # Popup (window px of color-grid-raw.png): the pulse cell and the app's lime places.
    allowed = [
        (705, 377, 729, 401),          # the grid's #C9D24A cell (row 2, column 4) and its ring
        (16, 69, 58, 111),             # the selected view icon (client y + 51)
        (420, 60, 480, 95),            # the display's loop indicator
        (621, 126, 1442, 140),         # the loop brace and its glow
        (622, 155, 1442, 230),         # Pulse 1's clips (#C9D24A) in the lanes
        (622, 228, 1442, 300),         # Pulse 2's clips
        (242, 155, 258, 300),          # the pulse strips
        (8, 190, 236, 940),            # the browser's kind markers (the pulse colour)
    ]
    i2, o2 = scan(popup, allowed)
    where = f" at {o2[:3]}" if o2 else ""
    print(f"| Colour grid popup | {i2} lime px in the allowed places | {len(o2)} elsewhere{where} |")
    sys.exit(1 if o1 or o2 else 0)


if __name__ == "__main__":
    main()
