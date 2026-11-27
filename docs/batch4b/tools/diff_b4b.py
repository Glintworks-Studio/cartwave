"""The visual regression gate for Batch 4b (BATCH4B_PROMPT.md §9.2), batch 4a's method
(diff_b4a.py: polish 2's regress.ps1 -New states, client area, > 2 levels per channel), with this
batch's allowed regions (client logical px), each a change the batch was asked for:

- the view keys (the shown view's key lit, the Mixer key enabled: B4B-D20);
- the display (its overhang eases out over 48 px: G11);
- each header's meter column and its clip mark (the pan grey: G8);
- the Main row (matched to the `main-track` board: §3.2), found by its divider as in 4a;
- the selected clip in the `inspector` state (the neutral outline and glow: G12), and the
  piano roll's selected notes (`piano_roll`);
- the browser's kind markers and preset dots (Pulse in Sky, not lime: G15, B4B-D13);
- the baseline's tooltip over the inspector (`inspector`: no tooltips, G9);
- the View menu (`menu_view`: the new Mixer item); the Settings panel (its rows move with K2's
  inset focus ring only when focused: not here, so not allowed).

usage: python diff_b4b.py <before dir> <after dir> <scale %> <out dir>
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
    regs = [("view keys", 10, 8, 112, 68), ("display (overhang)", 396, 6, 964, 72)]
    for i in range(rows):
        y = top + PITCH * i
        regs.append((f"meter {i + 1}", hl + W - 36, y, hl + W - 20, y + 72))
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
        if name in ("settings", "look_feel"):
            # A modal's backdrop dims the divider: the Main row is where it is
            # in the plain state.
            my = main_row(crop(Image.open(os.path.join(after, "stopped.png")).convert("RGB"), top_a)[0], scale)
        if my is not None:
            regs.append(("Main row", hl, my - 16, 1440, my + 74))
        # The lanes are 73 px shorter: their overlay scrollbar (at the lanes'
        # right edge) appears or changes size.
        if hl:
            # The browser's kind markers: Pulse ones are Sky, not lime (G15, B4B-D13).
            regs.append(("browser kind markers", 8, 136, 20, 1000))
            regs.append(("browser preset dots", 176, 136, 204, 1000))
        if name == "inspector":
            # The baseline shows a tooltip where the mouse is parked; 4b has none (G9).
            regs.append(("the baseline's tooltip (G9)", 960, 670, 1300, 760))
        if name in ("inspector", "piano_roll"):
            regs.append(("selected clip (outline, glow)", hl + W, ROWS_TOP - 12, 1440, ROWS_TOP + PITCH + 12))
        if name == "piano_roll":
            regs.append(("piano roll (selected notes)", 0, 560, 1440, 900))
        if name == "menu_view":
            regs.append(("View menu", 0, 0, 420, 520))
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
