"""The visual regression gate for Batch 4a (BATCH4A_PROMPT.md §9.2), polish 2's method
(docs/polish2/tools/diff.py: client area, > 2 levels per channel), with this batch's allowed
regions (client logical px):

- the badges (and the preset name beside them, which moves with the badge's width);
- the locked header state §5.3 requires: the dB value (a lock glyph), the fader's thumb and the
  pan (dimmed) in the control block;
- the TRACKS cell (the channel budget);
- the new Main row: found in each "after" shot by its #2E2E2E divider at x 700; from 16 px above
  it (its shadow, and the lanes' scrollbar, which moves up with the lanes' bottom) to its bottom;
- the menus and dialogs this batch adds (none in these states), Project Settings and the status
  bar's problem area (hidden here).

usage: python diff_b4a.py <before dir> <after dir> <scale %> <out dir> [rows_top]
"""
import os
import sys

from PIL import Image, ImageChops

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "polish2", "tools"))
from diff import client_top, crop  # noqa: E402

ROWS_TOP = 104  # the Overview hidden
PITCH = 73
HL = 241  # the header column's left with the browser open
W = 380


def header_regions(hl, rows=8, top=ROWS_TOP):
    regs = [("TRACKS cell", hl, top - 28, hl + W, top)]
    ctrl = hl + W - 132
    for i in range(rows):
        y = top + PITCH * i
        regs.append((f"badge {i + 1}", hl + 68, y + 36, hl + W - 144, y + 62))
        regs.append((f"locked controls {i + 1}", ctrl, y, ctrl + 120, y + 72))
    return regs


def main_row(img, scale):
    """The Main row's divider (y, client logical px) in the 'after' shot."""
    x = int(700 * scale / 100)
    px = img.load()
    for y in range(img.height - 1, 0, -1):
        if px[x, y][:3] == (0x2E, 0x2E, 0x2E):
            return y / (scale / 100)
    return None


def main():
    before, after, scale, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
    os.makedirs(out, exist_ok=True)
    k = scale / 100
    names = sorted(n[:-4] for n in os.listdir(after) if n.endswith(".png") and os.path.exists(os.path.join(before, n)))
    top_b = client_top(Image.open(os.path.join(before, "stopped.png")).convert("RGB"), scale)
    top_a = client_top(Image.open(os.path.join(after, "stopped.png")).convert("RGB"), scale)
    rows = ["| State | Changed px | Outside the allowed regions | Changed regions (client px) |", "|---|---|---|---|"]
    bad_total = 0
    for name in names:
        a, _ = crop(Image.open(os.path.join(before, name + ".png")).convert("RGB"), top_b)
        b, _ = crop(Image.open(os.path.join(after, name + ".png")).convert("RGB"), top_a)
        w, h = min(a.width, b.width), min(a.height, b.height)
        a, b = a.crop((0, 0, w, h)), b.crop((0, 0, w, h))
        hl = 0 if name == "browser_closed" else HL
        regs = header_regions(hl)
        my = main_row(b, scale)
        if my is not None:
            regs.append(("Main row", hl, my - 16, 1440, my + 74))
        if name == "settings":
            regs.append(("Settings panel", 380, 90, 1060, 900))
        regs = [(n, int(x0 * k), int(y0 * k), int(x1 * k), int(y1 * k)) for n, x0, y0, x1, y1 in regs]
        d = ImageChops.difference(a, b).load()
        out_img = Image.blend(b, Image.new("RGB", b.size, (0, 0, 0)), 0.6)
        o = out_img.load()
        changed = bad = 0
        hit = {}
        for y in range(h):
            for x in range(w):
                p = d[x, y]
                if max(p) <= 2:
                    continue
                changed += 1
                reg = next((r for r in regs if r[1] <= x < r[3] and r[2] <= y < r[4]), None)
                if reg is None:
                    bad += 1
                    o[x, y] = (255, 0, 255)
                    reg = ("NOT ALLOWED",)
                else:
                    o[x, y] = (255, 40, 40)
                hit[reg[0]] = hit.get(reg[0], 0) + 1
        out_img.save(os.path.join(out, name + "_diff.png"))
        bad_total += bad
        rows.append(f"| {name} | {changed} | {bad} | {', '.join(f'{n}: {c}' for n, c in sorted(hit.items()))} |")
    report = "\n".join(rows) + f"\n\n**{'PASS' if bad_total == 0 else 'FAIL'}**: {bad_total} px outside the allowed regions.\n"
    open(os.path.join(out, f"report-{scale}.md"), "w", encoding="utf-8").write(report)
    print(report)
    sys.exit(1 if bad_total else 0)


if __name__ == "__main__":
    main()
