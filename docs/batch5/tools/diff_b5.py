"""Batch 5's visual regression diff (BATCH5_PROMPT.md §8.2): with Automation Mode off the app must
look as it did on main, except where the batch added something on purpose (labelled).

Nothing moved this time, so a pixel is **changed** when any channel differs from the baseline in
place by more than 2 levels. Changed pixels are grouped into regions (8 px cells, joined within 3
cells) and every region must lie inside a labelled box (labels.json: per state, or "*" for all, or
"*@150" per scale: [x0, y0, x1, y1, what] in client logical px at 100 %). The report lists every
region, its box, its pixel count and its label; an unlabelled region fails the state.

usage: python diff_b5.py <before dir> <after dir> <out dir> <scale 100|150> [labels.json]
Writes report-<scale>.md and a marked copy of each changed state (red: unlabelled, amber: labelled).
"""
import json
import os
import sys

from PIL import Image, ImageDraw

CELL = 8
GAP = 3


def near(p, q):
    return abs(p[0] - q[0]) <= 2 and abs(p[1] - q[1]) <= 2 and abs(p[2] - q[2]) <= 2


def changed(a, b, S):
    """Changed pixels of the window, without its frame: the 1 px border (its accent colour
    follows window focus, not the app) and the title bar."""
    w, h = min(a.width, b.width), min(a.height, b.height)
    pa, pb = a.load(), b.load()
    title = round(31 * S)
    return [(x, y) for y in range(title, h - 1) for x in range(1, w - 1) if not near(pa[x, y], pb[x, y])]


def regions(points):
    cells = {}
    for x, y in points:
        k = (x // CELL, y // CELL)
        cells[k] = cells.get(k, 0) + 1
    seen, res = set(), []
    for k in cells:
        if k in seen:
            continue
        stack, comp = [k], []
        seen.add(k)
        while stack:
            c = stack.pop()
            comp.append(c)
            for dx in range(-GAP, GAP + 1):
                for dy in range(-GAP, GAP + 1):
                    m = (c[0] + dx, c[1] + dy)
                    if m in cells and m not in seen:
                        seen.add(m)
                        stack.append(m)
        xs = [c[0] for c in comp]
        ys = [c[1] for c in comp]
        res.append(((min(xs) * CELL, min(ys) * CELL, (max(xs) + 1) * CELL, (max(ys) + 1) * CELL), sum(cells[c] for c in comp)))
    res.sort(key=lambda r: (r[0][1], r[0][0]))
    return res


def main():
    before, after, out, scale = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
    labels = json.load(open(sys.argv[5], encoding="utf-8")) if len(sys.argv) > 5 else {}
    S = scale / 100.0
    os.makedirs(out, exist_ok=True)
    lines = [f"# Visual regression at {scale} % (Batch 5, Automation Mode off, against main)", "",
             "Changed pixels in place (any channel > 2 levels; the window frame and title bar left out), grouped into regions; every region must be labelled "
             "(an intended change). Written by `docs/batch5/tools/diff_b5.py`.", "",
             "| State | Changed px | Regions (box in window px, pixels, label) |", "|---|---|---|"]
    bad = []
    for name in sorted(os.listdir(after)):
        if not name.endswith(".png") or not os.path.exists(os.path.join(before, name)):
            continue
        state = name[:-4]
        a = Image.open(os.path.join(before, name)).convert("RGB")
        b = Image.open(os.path.join(after, name)).convert("RGB")
        if a.size != b.size:
            lines.append(f"| {state} | size {a.size} vs {b.size} | **SIZE** |")
            bad.append(state)
            continue
        pts = changed(a, b, S)
        ct = b.height - round(920 * S)
        cl = (b.width - round(1440 * S)) // 2
        named = labels.get(state, []) + labels.get("*", []) + labels.get(f"*@{scale}", [])
        regs = []
        for box, n in regions(pts):
            x0, y0, x1, y1 = box
            lab = ""
            for lx0, ly0, lx1, ly1, what in named:
                if x0 >= cl + lx0 * S - CELL and y0 >= ct + ly0 * S - CELL and x1 <= cl + lx1 * S + CELL and y1 <= ct + ly1 * S + CELL:
                    lab = what
                    break
            regs.append((box, n, lab))
        if any(not r[2] for r in regs):
            bad.append(state)
        txt = "; ".join(f"{r[0]} {r[1]} px" + (f" ({r[2]})" if r[2] else " **UNLABELLED**") for r in regs) or "none"
        lines.append(f"| {state} | {len(pts)} | {txt} |")
        if pts:
            m = b.copy()
            d = ImageDraw.Draw(m)
            for box, n, lab in regs:
                d.rectangle(box, outline=(255, 200, 40) if lab else (255, 40, 40), width=2)
            m.save(os.path.join(out, f"{state}.marked.png"))
    lines += ["", f"**{'PASS' if not bad else 'FAIL'}**: " + ("every change is labelled" if not bad else "unlabelled changes in " + ", ".join(bad))]
    open(os.path.join(out, f"report-{scale}.md"), "w", encoding="utf-8").write("\n".join(lines) + "\n")
    print(lines[-1])


if __name__ == "__main__":
    main()
