"""Batch 4b §9.3: the fixups against the `fixups` board (its HTML boxes, rendered at 1x).

Measured the same way on the board and on the app (fixups_shot.ps1: the board's song with 12
tracks, browser closed, the selected clip on track 9):

  number    track 9's number "9": its ink's centre x and top in its row (the 48 px column)
  name      track 9's name "Pulse 1": its ink's left edge (12 px after the column)
  overhang  the display's top shadow: the darkening under the display's top edge, row by row,
            down a text-free column (the 48 px ease-out, VISUAL-SPEC §13 / G11)
  outline   the selected clip's #E6E6E6 outline: 1 px outside its border
  glow      the selected clip's glow: how much it lifts the lane 4 px outside the outline

The board's clip is "Hook A", the app's "Intro Lead": both are measured from their own border.
Header meters (G8, the pan grey and the clip mark) are checked by the unit tests and B4B-C1:
the board shows a level the app (meters frozen) doesn't.

usage: python mockup_match_fixups.py <app shot> <app shot, unselected> <out dir>"""
import os
import io
import sys

from PIL import Image, ImageDraw

sys.stdout.reconfigure(encoding="utf-8")

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.join(HERE, "..", "..", "design", "meeting-2026-11-26", "fixups@1x.png")
sys.path.insert(0, os.path.join(HERE, "..", "..", "polish2", "tools"))
from diff import client_top  # noqa: E402


def lum(p):
    return max(p[:3])


def client(path):
    im = Image.open(path).convert("RGB")
    t = client_top(im, 100)
    return im.crop((1, t, min(im.width - 1, 1441), im.height - 1))


def row_of_number(im, digit_ink_x0=28, digit_ink_x1=76):
    """The row whose number reads 9: the row top (its strip's top) is found from the strip's
    colour run at x 8; returns the tops of every row."""
    px = im.load()
    tops = []
    prev = None
    for y in range(136, im.height):
        p = px[8, y]
        coloured = max(p) - min(p) > 30 and max(p) > 60
        if coloured and not prev:
            tops.append(y)
        prev = coloured
    return tops


def number_ink(im, top):
    px = im.load()
    xs, ys = [], []
    for y in range(top + 4, top + 60):
        for x in range(20, 84):
            p = px[x, y]
            if 0x1C <= lum(p) <= 0x2C and max(p) - min(p) < 6 and lum(px[x, y]) > lum(px[x - 3, y]) + 4:
                xs.append(x)
                ys.append(y)
    return (round((min(xs) + max(xs)) / 2, 1), min(ys) - top) if xs else None


def name_ink(im, top):
    px = im.load()
    xs = [x for y in range(top + 12, top + 32) for x in range(80, 240) if lum(px[x, y]) >= 0xC0]
    return min(xs) if xs else None


def overhang(im, x=500, top=9):
    """The display's darkening under its top edge: lum per row from the inner top down 52 rows."""
    px = im.load()
    # The display's inner top: the first row of its #151515-ish body under its border.
    return [lum(px[x, top + i]) for i in range(0, 52)]


def outline_top(im, x, y_from, y_to):
    """On column x: the outline's row (lum >= 200, neutral) over the clip's top edge, the clip
    border's colour 1 px under it, and the glow 4 px above the outline (its lift over the lane
    20 px above)."""
    px = im.load()
    ys = [y for y in range(y_from, y_to) if lum(px[x, y]) >= 200 and max(px[x, y]) - min(px[x, y]) < 30]
    if not ys:
        return None
    oy = min(ys)
    return oy, px[x, oy + 1], lum(px[x, oy - 4]) - lum(px[x, oy - 20])


def outline(im, y, x_from, x_to):
    """On row y: the outline's x (lum >= 200) and the clip border's x right of it, and the glow
    4 px outside the outline (lum minus the lane far away)."""
    px = im.load()
    xs = [x for x in range(x_from, x_to) if lum(px[x, y]) >= 200 and max(px[x, y]) - min(px[x, y]) < 30]
    if not xs:
        return None
    ox = min(xs)
    glow = lum(px[ox - 4, y]) - lum(px[ox - 30, y])
    return ox, glow


def main():
    shot, shot_u, out = sys.argv[1], sys.argv[2], sys.argv[3]
    os.makedirs(out, exist_ok=True)
    board = Image.open(BOARD).convert("RGB")
    app = client(shot)
    app_u = client(shot_u)
    res = []
    # Board: track 9 is the first row (top 136).
    b_top = 136
    a_tops = row_of_number(app)
    # The app's row "9": the one whose number column holds the 9 (the 9th track is the 4th row
    # from the bottom of the visible rows here; take the row whose strip is the lime of track 9).
    pxa = app.load()
    # Track 9 is the last coloured row (the Main row's strip is black).
    a_top = a_tops[-1] if a_tops else None
    _ = pxa
    bn, an = number_ink(board, b_top), number_ink(app, a_top) if a_top else None
    res.append(("number: centre x, top in its row", bn, an, bn and an and max(abs(bn[0] - an[0]), abs(bn[1] - an[1]))))
    bm, am = name_ink(board, b_top), name_ink(app, a_top) if a_top else None
    res.append(("name: left edge", bm, am, bm and am and abs(bm - am)))
    bo, ao = overhang(board), overhang(app_u)
    d = max(abs(a - b) for a, b in zip(bo, ao))
    res.append(("overhang: lum per row (0-51 px under the display's top)", bo[::4], ao[::4], d))
    # Outline: board's Hook A on row 136 + 36 (its body), searched right of x 760; app's selected
    # clip on track 9's row, searched from the lanes' left.
    # The clips' top edges: the board's Hook A (x 900), the app's selected clip (x 500). Each
    # clip's top is 2 px under its row's top (top: 2px), so the outline is at row top + 1.
    bo_ = outline_top(board, 900, b_top - 4, b_top + 8)
    ao_ = outline_top(app, 500, (a_top or 0) - 4, (a_top or 0) + 8)
    res.append(("outline: its row under the row's top", bo_ and bo_[0] - b_top, ao_ and ao_[0] - a_top, bo_ and ao_ and abs((bo_[0] - b_top) - (ao_[0] - a_top))))
    res.append(("outline: the clip border 1 px under it (its colour)", bo_ and bo_[1], ao_ and ao_[1], None))
    res.append(("glow: lift 4 px above the outline (levels)", bo_ and bo_[2], ao_ and ao_[2], bo_ and ao_ and abs(bo_[2] - ao_[2])))
    lines = ["# The fixups vs the `fixups` board (Batch 4b §9.3)", "", "Tolerance: 1 px for positions; for the overhang, the largest luminance difference on any row "
             "(of 255: the gradient is the same if it stays within a couple of levels); for the glow, the lift in levels.", "",
             "| part | board | app | Δ |", "|---|---|---|---|"]
    for name, b, a, dd in res:
        lines.append(f"| {name} | {b} | {a} | {dd} |")
    open(os.path.join(out, "report.md"), "w", encoding="utf-8").write("\n".join(lines) + "\n")
    print("\n".join(lines))
    # The before/after picture is made by fixups_pictures (BATCH4B screenshots).


if __name__ == "__main__":
    main()
