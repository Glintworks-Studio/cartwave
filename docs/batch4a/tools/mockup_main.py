"""Batch 4a §9.3, the Main row "within 1 px" check. The CEO's B4a-main-track board was not
provided (RUNLOG B4A-D1), so the Main row is measured against the spec it was built from: the
prompt's §7 colours and the E1 header grid (VISUAL-SPEC.md §4, §4b) that the track rows follow
and that the polish 2 gate matched to its boards. In each layout shot (main_shots.ps1):

- the divider row is #2E2E2E across the header column and the lanes; the row below is 72 px;
- the lane is #0D0D0D;
- the strip sits where track 1's strip sits (x within 1 px), its outer edge #4A4A4A, dark near the
  outer edge and lighter (#2A..#3A) near the inner one: the main_strip gradient;
- "Main", the OUT badge and the fader's well sit at track 1's offsets (name ink, badge ring, well)
  within 1 px.

usage: python mockup_main.py <shots dir>   (writes report.md there)"""
import os
import sys

from PIL import Image

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "polish2", "tools"))
from diff import client_top  # noqa: E402

ROWS_TOP = 136  # Overview shown
LAYOUTS = {"left_outside": (False, False, 380), "left_byclips": (False, True, 380), "right_byclips": (True, True, 380), "right_outside": (True, False, 380), "left_outside_520": (False, False, 520)}
results = []


def near(p, hexv, tol=3):
    c = tuple(int(hexv[i:i + 2], 16) for i in (1, 3, 5))
    return all(abs(a - b) <= tol for a, b in zip(p[:3], c))


def find(img, box, pred):
    x0, y0, x1, y1 = box
    px = img.load()
    hits = [(x, y) for y in range(max(0, y0), min(img.height, y1)) for x in range(max(0, x0), min(img.width, x1)) if pred(px[x, y])]
    if not hits:
        return None
    return min(h[0] for h in hits), min(h[1] for h in hits), max(h[0] for h in hits), max(h[1] for h in hits)


def check(name, cond, msg):
    results.append((name, bool(cond), msg))


def main():
    d = sys.argv[1]
    for name, (right, byclips, w) in LAYOUTS.items():
        path = os.path.join(d, name + ".png")
        if not os.path.exists(path):
            continue
        im = Image.open(path).convert("RGB")
        top = client_top(im, 100)
        im = im.crop((1, top, im.width - 1, im.height - 1))
        px = im.load()
        hl = 1440 - w if right else 241
        lanes_x = 1300 if not right else 600
        # The divider: scan up from the bottom at a lanes column.
        my = next((y for y in range(im.height - 1, 0, -1) if near(px[lanes_x, y], "#2E2E2E", 2)), None)
        check(name, my is not None, f"divider #2E2E2E found in the lanes (y {my})")
        if my is None:
            continue
        hdr_div = near(px[hl + 200, my], "#2E2E2E", 2)
        check(name, hdr_div, f"divider across the header column at the same y ({px[hl + 200, my]})")
        # 72 px row under it, then the status bar's hairline.
        lane_col = next((x for x in range(lanes_x, lanes_x + 40) if near(px[x, my + 30], "#0D0D0D", 2)), None)
        check(name, lane_col is not None, f"the lane is #0D0D0D ({px[lanes_x, my + 30]})")
        if lane_col is not None:
            bottom = next((y for y in range(my + 1, im.height) if not near(px[lane_col, y], "#0D0D0D", 2)), im.height)
            check(name, abs((bottom - my - 1) - 72) <= 1, f"the row is 72 px under the divider ({bottom - my - 1})")
        row_top, main_top = ROWS_TOP, my + 1
        mirrored = right != byclips
        strip_rel = (w - 1 - 16) if mirrored else 0
        # Track 1's strip and the Main strip: the same columns.
        def strip_cols(t):
            y = t + 36
            cols = [x for x in range(max(0, hl + strip_rel - 4), min(im.width, hl + strip_rel + 22)) if max(px[x, y]) - min(px[x, y]) > 25 or (t == main_top and px[x, y][0] < 0x50 and px[x, y][0] != px[hl + 120, y][0])]
            return cols
        tcol = [x for x in range(max(0, hl + strip_rel - 4), min(im.width, hl + strip_rel + 22)) if max(px[x, row_top + 36]) - min(px[x, row_top + 36]) > 25]
        outer = hl + strip_rel + (15 if mirrored and not right else 0)
        edge_x = next((x for x in range(max(0, hl + strip_rel - 2), min(im.width, hl + strip_rel + 18)) if near(px[x, main_top + 36], "#4A4A4A", 3)), None)
        check(name, edge_x is not None and tcol and abs(edge_x - (min(tcol) if (edge_x - min(tcol)) < 8 else max(tcol))) <= 1, f"the strip's #4A4A4A edge at x {edge_x} (track 1's strip x {tcol[:1]}…{tcol[-1:]})")
        if edge_x is not None:
            inner = edge_x + 14 if edge_x - (hl + strip_rel) < 8 else edge_x - 14
            a, b = px[edge_x + (2 if inner > edge_x else -2), main_top + 36], px[inner, main_top + 36]
            # The inner end is #363636 under the 6% white inner glow (~#3D).
            check(name, max(a) < 0x14 and 0x30 <= b[0] <= 0x44, f"main_strip: dark at the outer edge {a}, #363636 (+6% glow) at the inner {b}")
        # Name ink, badge ring, fader well: the Main row's offsets against track 1's.
        nx = hl + (w - 1 - 68 - 200 if mirrored else 68)
        def ink(t, x0, x1, y0, y1):
            return find(im, (x0, t + y0, x1, t + y1), lambda p: p[0] > 0xB0)
        t_name, m_name = ink(row_top, hl, hl + w, 8, 32), ink(main_top, hl, hl + w, 8, 32)
        if t_name and m_name:
            check(name, abs((t_name[0]) - (m_name[0])) <= 1 and abs((t_name[1] - row_top) - (m_name[1] - main_top)) <= 1, f"'Main' ink at track 1's name offset ({m_name[0]},{m_name[1] - main_top} vs {t_name[0]},{t_name[1] - row_top})")
        else:
            check(name, False, f"name ink found ({t_name}, {m_name})")
        ring = lambda p: near(p, "#383838", 4)
        # The name column only (the M / S keys' borders are #383838 too).
        nx0, nx1 = (hl + 140, hl + w - 20) if mirrored else (hl + 20, hl + w - 140)
        t_badge = find(im, (nx0, row_top + 36, nx1, row_top + 62), ring)
        m_badge = find(im, (nx0, main_top + 36, nx1, main_top + 62), ring)
        if t_badge and m_badge:
            check(name, abs(t_badge[0] - m_badge[0]) <= 1 and abs((t_badge[1] - row_top) - (m_badge[1] - main_top)) <= 1, f"OUT badge at the badge's place ({m_badge[0]},{m_badge[1] - main_top} vs {t_badge[0]},{t_badge[1] - row_top})")
        else:
            check(name, False, f"badge rings found ({t_badge}, {m_badge})")
        well = lambda p: near(p, "#050505", 2)
        t_well = find(im, (hl, row_top + 6, hl + w, row_top + 30), well)
        m_well = find(im, (hl, main_top + 6, hl + w, main_top + 30), well)
        if t_well and m_well:
            # Track 1 is a studio track with a fader; the Main fader's well.
            def well_x(t):
                cnt = {}
                for y in range(t + 8, t + 40):
                    for x in range(hl, hl + w):
                        if well(px[x, y]):
                            cnt[x] = cnt.get(x, 0) + 1
                return [x for x, c in cnt.items() if c >= 10]
            tw, mw = well_x(row_top), well_x(main_top)
            check(name, tw and mw and abs(max(tw) - max(mw)) <= 1, f"the fader's well at track 1's x ({mw[-1:]} vs {tw[-1:]})")
        else:
            check(name, False, f"fader wells found ({t_well}, {m_well})")
    lines = ["| Layout | Check | Result |", "|---|---|---|"] + [f"| {n} | {m} | {'PASS' if ok else 'FAIL'} |" for n, ok, m in results]
    ok = all(r[1] for r in results) and results
    rep = "\n".join(lines) + f"\n\n**{'PASS' if ok else 'FAIL'}** ({sum(r[1] for r in results)}/{len(results)})\n"
    open(os.path.join(d, "report.md"), "w", encoding="utf-8").write(rep)
    sys.stdout.reconfigure(encoding="utf-8")
    print(rep)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
