"""Gradient banding, before and after dithering (DESIGN_PASS_PROMPT.md §3.8).

usage: python banding.py <raw dir> <out png> [report.md]

The measure: along a column through a vertical gradient, the run lengths of
identical pixels. An undithered gradient from #191919 to #0A0A0A over 76 px
has 15 grey levels, so it shows ~5 px flat bands with a visible step between
them; dithering breaks every band into a fine mix of the two neighbouring
levels, so runs shrink to 1–2 px. Reported per gradient: the longest run
(the widest visible band), the mean run, and the share of rows that sit in a
run of 4 px or more ("banded rows").

Screenshots (window px: client + (1, 51)), from snap.ps1:
- band-prerun.png     main 23b5da9, a clip selected (no dithering anywhere)
- band-dither.png     this build, a clip selected
- band-nodither.png   this build with CHIPDAW_NO_DITHER=1 (procedural gradients undithered;
                      the stylesheet's are dithered by the vizia fork either way)
- band-idle-*.png     the same pair, nothing selected (the canvas under the lanes shows)
"""
import os
import sys

from PIL import Image

OX, OY = 1, 51


def runs(img, x, y0, y1):
    px = img.load()
    col = [px[x, y][:3] for y in range(y0, y1)]
    out, n = [], 1
    for a, b in zip(col, col[1:]):
        if a == b:
            n += 1
        else:
            out.append(n)
            n = 1
    out.append(n)
    return out, col


def stats(img, x, y0, y1, xs=5):
    """Over `xs` neighbouring columns (dither noise differs per column)."""
    longest, total, banded, rows, levels = 0, 0, 0, 0, set()
    for dx in range(xs):
        r, col = runs(img, x + dx, y0, y1)
        longest = max(longest, max(r))
        total += len(r)
        banded += sum(n for n in r if n >= 4)
        rows += sum(r)
        levels.update(col)
    return {"longest": longest, "mean": round(rows / total, 2), "banded": round(100.0 * banded / rows, 1), "levels": len(levels)}


# (label, image, client x, client y0, y1)
CASES = [
    ("Top bar #191919→#0A0A0A (stylesheet)", [("before: main 23b5da9", "band-prerun.png", 360, 2, 58), ("after: this build", "band-dither.png", 360, 2, 74)]),
    ("Bottom panel #121212→#0A0A0A (stylesheet)", [("before: main 23b5da9", "band-prerun.png", 500, 670, 898), ("after: this build", "band-dither.png", 500, 690, 918)]),
    ("Canvas under the lanes #191919→#0A0A0A (drawn)", [("dither off", "band-idle-nodither.png", 1300, 692, 918), ("dither on", "band-idle-dither.png", 1300, 692, 918)]),
    ("Clip body, lane colour 22%→10% (drawn)", [("dither off", "band-idle-nodither.png", 700, 126, 172), ("dither on", "band-idle-dither.png", 700, 126, 172)]),
    ("Key body #161616→#424242 (drawn)", [("dither off", "band-idle-nodither.png", 36, 21, 56), ("dither on", "band-idle-dither.png", 36, 21, 56)]),
]


def stretch(img, lo=6, hi=34):
    """Contrast-stretch dark greys so bands are visible in the report image."""
    lut = [max(0, min(255, int((v - lo) * 255 / (hi - lo)))) for v in range(256)]
    return img.point(lut * 3)


def main():
    raw, out_png = sys.argv[1], sys.argv[2]
    report = sys.argv[3] if len(sys.argv) > 3 else None
    lines = ["| Gradient | Shot | Longest run (px) | Mean run (px) | Banded rows (runs ≥ 4 px) | Distinct colours |", "|---|---|---|---|---|---|"]
    tiles = []
    for label, shots in CASES:
        for name, f, x, y0, y1 in shots:
            img = Image.open(os.path.join(raw, f)).convert("RGB")
            s = stats(img, x + OX, y0 + OY, y1 + OY)
            lines.append(f"| {label} | {name} | {s['longest']} | {s['mean']} | {s['banded']}% | {s['levels']} |")
            crop = img.crop((x + OX - 30, y0 + OY, x + OX + 30, y1 + OY))
            tiles.append((f"{label}: {name}", stretch(crop).resize((crop.width * 3, crop.height * 3), Image.NEAREST)))
    text = "\n".join(lines)
    print(text)
    if report:
        open(report, "w", encoding="utf-8", newline="\n").write(text + "\n")
    # The composite: each gradient's pair side by side, contrast-stretched ×3.
    w = max(t[1].width for t in tiles) * 2 + 24
    h = sum(max(tiles[i][1].height, tiles[i + 1][1].height) + 12 for i in range(0, len(tiles), 2)) + 12
    sheet = Image.new("RGB", (w, h), (48, 48, 48))
    y = 12
    for i in range(0, len(tiles), 2):
        a, b = tiles[i][1], tiles[i + 1][1]
        sheet.paste(a, (8, y))
        sheet.paste(b, (16 + a.width, y))
        y += max(a.height, b.height) + 12
    sheet.save(out_png)


if __name__ == "__main__":
    main()
