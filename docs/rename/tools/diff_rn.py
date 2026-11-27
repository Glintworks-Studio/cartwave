"""The rename's visual regression diff (RENAME_PROMPT.md §3.4): only text that shows the
name may differ. Unlike the earlier gates' diffs, nothing is allowed up front: every
differing region of every state is listed (whole window, the title bar included), with its
bounding box in window pixels, so each one can be named in RENAME.md.

A pixel differs when any channel moves by more than 2 levels (diff.py's threshold). Regions:
the changed pixels gathered into 8 px cells, cells within 3 cells of each other joined.

usage: python diff_rn.py <before dir> <after dir> <out dir> [labels.json]
labels.json (optional): {"state": [[x0, y0, x1, y1, "what it is"], ...]}; a region inside a
labelled box takes its label, and the report says UNLABELLED for any other.
"""
import json
import os
import sys

from PIL import Image, ImageDraw

CELL = 8
GAP = 3


def regions(a, b):
    w, h = min(a.width, b.width), min(a.height, b.height)
    pa, pb = a.load(), b.load()
    cells = {}
    n = 0
    # A 1 px frame is left out (the window border's focus colour).
    for y in range(1, h - 1):
        for x in range(1, w - 1):
            p, q = pa[x, y], pb[x, y]
            if abs(p[0] - q[0]) > 2 or abs(p[1] - q[1]) > 2 or abs(p[2] - q[2]) > 2:
                n += 1
                k = (x // CELL, y // CELL)
                cells[k] = cells.get(k, 0) + 1
    seen, out = set(), []
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
        box = (min(xs) * CELL, min(ys) * CELL, (max(xs) + 1) * CELL, (max(ys) + 1) * CELL)
        out.append((box, sum(cells[c] for c in comp)))
    out.sort(key=lambda r: (r[0][1], r[0][0]))
    return n, out


def label_of(box, labels):
    for x0, y0, x1, y1, what in labels:
        if box[0] >= x0 and box[1] >= y0 and box[2] <= x1 and box[3] <= y1:
            return what
    return None


def main():
    before, after, out = sys.argv[1], sys.argv[2], sys.argv[3]
    labels = json.load(open(sys.argv[4], encoding="utf-8")) if len(sys.argv) > 4 else {}
    os.makedirs(out, exist_ok=True)
    rows = ["| State | Changed px | Regions (window px: x0,y0–x1,y1, changed px, what) |", "|---|---|---|"]
    unlabelled = 0
    for f in sorted(os.listdir(before)):
        if not f.endswith(".png") or not os.path.exists(os.path.join(after, f)):
            continue
        state = f[:-4]
        a = Image.open(os.path.join(before, f)).convert("RGB")
        b = Image.open(os.path.join(after, f)).convert("RGB")
        if a.size != b.size:
            rows.append(f"| {state} | size {a.size} vs {b.size} | **UNLABELLED** |")
            unlabelled += 1
            continue
        n, regs = regions(a, b)
        parts = []
        img = b.copy()
        d = ImageDraw.Draw(img)
        for box, px in regs:
            what = label_of(box, labels.get(state, []) + labels.get("*", []))
            if what is None:
                unlabelled += 1
                what = "**UNLABELLED**"
            parts.append(f"{box[0]},{box[1]}–{box[2]},{box[3]}: {px} px, {what}")
            d.rectangle(box, outline=(255, 0, 255) if what.startswith("**") else (255, 0, 0), width=2)
        if regs:
            img.save(os.path.join(out, f"{state}.diff.png"))
        rows.append(f"| {state} | {n} | {'; '.join(parts) if parts else 'none'} |")
    rows.append("")
    rows.append("**PASS**: every differing region is labelled." if unlabelled == 0 else f"**{unlabelled} unlabelled region(s)**: look at them.")
    text = "\n".join(rows) + "\n"
    open(os.path.join(out, "report.md"), "w", encoding="utf-8").write(text)
    print(text)
    sys.exit(0 if unlabelled == 0 else 1)


if __name__ == "__main__":
    main()
