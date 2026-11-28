"""The Presets + Browser batch's visual regression diff (PRESETS_PROMPT.md §8.2).

Changes are allowed only in the browser, the bottom pane (the device panel and its toggle) and
the inspector (instrument controls removed). The browser is the board's 420 px (it was 240), so
the main area right of it moved 180 logical px to the right: a pixel counts as **unchanged** if
it equals the baseline's at the same place (the top bar, menus and modal panels didn't move) or
180 px to the left (the arrangement and the mixer moved, they didn't change). What is left is
grouped into regions (8 px cells, joined within 3 cells, as diff_rn.py) and each region is put
in the browser, the bottom pane, or OTHER. Every OTHER region is listed with its box, and named
when labels.json names it (an expected consequence, e.g. a row the prompt added to the shortcut
sheet, or the arrangement's right edge that the narrower lanes no longer reach).

A pixel differs when any channel moves by more than 2 levels.

usage: python diff_pb.py <before dir> <after dir> <out dir> <scale 100|150> [labels.json]
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
    """Pixels of b (after) that match a (before) neither in place nor 180 px to the left."""
    w, h = min(a.width, b.width), min(a.height, b.height)
    pa, pb = a.load(), b.load()
    dx = round(180 * S)
    raw = 0
    out = []
    for y in range(1, h - 1):
        for x in range(1, w - 1):
            q = pb[x, y]
            if near(pa[x, y], q):
                continue
            raw += 1
            if x - dx >= 0 and near(pa[x - dx, y], q):
                continue
            out.append((x, y))
    return raw, out


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
            for ddx in range(-GAP, GAP + 1):
                for ddy in range(-GAP, GAP + 1):
                    m = (c[0] + ddx, c[1] + ddy)
                    if m in cells and m not in seen:
                        seen.add(m)
                        stack.append(m)
        xs = [c[0] for c in comp]
        ys = [c[1] for c in comp]
        res.append(((min(xs) * CELL, min(ys) * CELL, (max(xs) + 1) * CELL, (max(ys) + 1) * CELL), sum(cells[c] for c in comp)))
    res.sort(key=lambda r: (r[0][1], r[0][0]))
    return res


MODALS = ("about", "settings", "look_feel", "settings_files", "shortcuts")


def panel_box(img):
    """The modal panel: the bounding box of its #141414 fill (rows with a long run of it)."""
    px = img.load()
    w, h = img.size
    rows = [y for y in range(h) if sum(1 for x in range(0, w, 4) if px[x, y][:3] == (0x14, 0x14, 0x14)) > 60]
    if not rows:
        return None
    y0, y1 = min(rows), max(rows)
    mid = (y0 + y1) // 2
    cols = [x for x in range(w) if px[x, mid][:3] == (0x14, 0x14, 0x14)]
    return (min(cols), y0, max(cols), y1) if cols else None


def main():
    before, after, out, scale = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
    labels = json.load(open(sys.argv[5])) if len(sys.argv) > 5 else {}
    S = scale / 100.0
    os.makedirs(out, exist_ok=True)
    lines = [f"# Visual regression at {scale} % (Presets + Browser batch)", "",
             "Each state: the pixels that changed in place, then those that match the baseline neither in place nor 180 px "
             "(the browser's growth) to the left, sorted into the allowed regions (the browser, the bottom pane) and OTHER, "
             "every OTHER region listed. Written by `docs/presets/tools/diff_pb.py`.", "",
             "| State | Changed in place (px) | Changed after the shift (px) | Browser | Bottom pane | OTHER regions |",
             "|---|---|---|---|---|---|"]
    detail = []
    bad_states = []
    for name in sorted(os.listdir(after)):
        if not name.endswith(".png") or not os.path.exists(os.path.join(before, name)):
            continue
        state = name[:-4]
        a = Image.open(os.path.join(before, name)).convert("RGB")
        b = Image.open(os.path.join(after, name)).convert("RGB")
        if a.size != b.size:
            lines.append(f"| {state} | size {a.size} vs {b.size} | – | – | – | **SIZE** |")
            bad_states.append(state)
            continue
        raw, pts = changed(a, b, S)
        # Window geometry: the client area is at the bottom of the frame capture.
        ct = b.height - round(920 * S)
        cl = (b.width - round(1440 * S)) // 2
        top = ct + round(76 * S)
        browser_r = cl + round(421 * S)
        # A selection shows the bottom pane (356 px; it was 232) with the Main row (72 px)
        # right above it: from where main's Main row began (client 616) down, the pane's growth.
        has_pane = state in ("inspector",)
        grow_top = ct + round((920 - 232 - 72) * S - (356 - 232) * S)
        # A modal panel is drawn in place over the moved app.
        panel = panel_box(b) if state in MODALS else None
        named = labels.get(state, []) + labels.get("*", []) + labels.get(f"*@{scale}", [])
        browser = pane = 0
        rest = []
        for x, y in pts:
            in_panel = panel and panel[0] <= x <= panel[2] and panel[1] <= y <= panel[3]
            if in_panel:
                rest.append((x, y))
            elif x < browser_r and y >= top:
                browser += 1
            elif has_pane and y >= grow_top:
                pane += 1
            else:
                rest.append((x, y))
        other = []
        for box, n in regions(rest):
            x0, y0, x1, y1 = box
            inside_panel = panel and x0 >= panel[0] - CELL and y0 >= panel[1] - CELL and x1 <= panel[2] + CELL and y1 <= panel[3] + CELL
            # A region that reaches past the panel's edge is the app behind it (its shadow).
            if panel and (y1 > panel[3] + 2 or y0 < panel[1] - 2 or x0 < panel[0] - 2 or x1 > panel[2] + 2):
                inside_panel = False
            lab = ""
            for lx0, ly0, lx1, ly1, what in named:
                if x0 >= cl + lx0 * S - CELL and y0 >= ct + ly0 * S - CELL and x1 <= cl + lx1 * S + CELL and y1 <= ct + ly1 * S + CELL:
                    lab = what
                    break
            if not lab and panel and inside_panel and min(abs(x0 - panel[0]), abs(x1 - panel[2])) <= 2 * CELL and min(abs(y0 - panel[1]), abs(y1 - panel[3])) <= 2 * CELL and n <= 64 * 2 * S * S:
                lab = "a corner of the panel (rounded: the moved app shows behind it)"
            if not lab and panel and not inside_panel:
                lab = "the dimmed app behind the panel: moved 180 px, with the panel's shadow over it"
            if not lab and state.startswith("menu_") and x0 < browser_r + round(120 * S) and y0 < ct + round(560 * S):
                lab = "the menu's dropdown (in place) and its shadow over the browser and the moved headers"
            other.append((box, n, lab))
        unl = [o for o in other if not o[2]]
        if unl:
            bad_states.append(state)
        ostr = "; ".join(f"{o[0]} {o[1]} px" + (f" ({o[2]})" if o[2] else " **UNLABELLED**") for o in other) or "none"
        lines.append(f"| {state} | {raw} | {len(pts)} | {browser} | {pane} | {ostr} |")
        # A marked copy: red boxes OTHER, blue the allowed regions.
        if pts:
            m = b.copy()
            d = ImageDraw.Draw(m)
            if browser:
                d.rectangle((cl, top, browser_r, b.height - 1), outline=(60, 140, 255), width=2)
            if pane:
                d.rectangle((cl, grow_top, b.width - 1, b.height - 1), outline=(60, 140, 255), width=2)
            for box, n, lab in other:
                d.rectangle(box, outline=(255, 200, 40) if lab else (255, 40, 40), width=2)
            m.save(os.path.join(out, f"{state}.marked.png"))
    lines += ["", f"**{'PASS' if not bad_states else 'FAIL'}**: " + ("every change is in the browser, the bottom pane or a named region" if not bad_states else "unlabelled changes in " + ", ".join(bad_states))]
    open(os.path.join(out, f"report-{scale}.md"), "w", encoding="utf-8").write("\n".join(lines + detail) + "\n")
    print("\n".join(lines[-1:]))


if __name__ == "__main__":
    main()
