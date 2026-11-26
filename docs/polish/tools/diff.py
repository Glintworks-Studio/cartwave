"""The visual regression gate (POLISH_PROMPT.md §4.1).

usage: python diff.py <before dir> <after dir> <scale %> <out dir>

Compares the client area of every screenshot pair (the window frame is the OS's: its
1 px border changes with focus). A pixel "differs" when any channel moves by more than 2
levels. Pixels may differ only inside the regions this pass changes (below, client
logical px, scaled); anything else fails. Writes, per pair, an image with the changed
pixels in red (allowed) or magenta (not allowed) over a dimmed copy of the "after" shot,
and a markdown report.
"""
import os
import sys

from PIL import Image, ImageChops

FRAME = (1, 51)  # window border, title bar + menu bar (screenshot px at any scale: the frame isn't scaled... see crop())

# Regions this pass may change (client logical px): name -> (x0, y0, x1, y1).
DISPLAY = ("display well", 397, 8, 961, 68)
BROWSER_HEAD = ("browser header", 0, 76, 241, 112)
ALLOWED = {
    "*": [DISPLAY],
    "stopped": [BROWSER_HEAD],
    "playing": [BROWSER_HEAD],
    "loop_off": [BROWSER_HEAD],
    "solo_mute": [BROWSER_HEAD],
    "inspector": [BROWSER_HEAD],
    "piano_roll": [BROWSER_HEAD],
    "settings": [BROWSER_HEAD, ("Settings panel", 380, 90, 1060, 900)],
    # The Edit menu's popup opens under the menu bar (y < 0 in client px).
    "menu_edit": [BROWSER_HEAD, ("menu popup", 0, -24, 440, 760)],
}


def crop(img, scale):
    """The client area. The title bar and menu bar heights scale with the DPI the
    window was created at, not the app's interface scale, so the client top is found:
    the top bar's #323232 top line in the column at x = 700 logical."""
    x = int(700 * scale / 100)
    px = img.load()
    for y in range(10, 200):
        if px[x, y][:3] == (0x32, 0x32, 0x32):
            return img.crop((1, y, img.width - 1, img.height - 1)), y
    raise SystemExit("no client top found")


def main():
    before, after, scale, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
    os.makedirs(out, exist_ok=True)
    k = scale / 100.0
    rows = ["| State | Changed px | Outside the allowed regions | Changed regions (bbox, client px) |", "|---|---|---|---|"]
    failed = 0
    for f in sorted(os.listdir(before)):
        if not f.endswith(".png") or not os.path.exists(os.path.join(after, f)):
            continue
        name = f[:-4]
        a, ya = crop(Image.open(os.path.join(before, f)).convert("RGB"), scale)
        b, yb = crop(Image.open(os.path.join(after, f)).convert("RGB"), scale)
        w, h = min(a.width, b.width), min(a.height, b.height)
        # The menu popup sits above the client top: compare from the screenshot's top
        # for that state (same window placement in both).
        d = ImageChops.difference(a.crop((0, 0, w, h)), b.crop((0, 0, w, h)))
        regs = [(n, int(x0 * k), int(y0 * k), int(x1 * k), int(y1 * k)) for n, x0, y0, x1, y1 in ALLOWED["*"] + ALLOWED.get(name, [])]
        # Windows 11 rounds the window's bottom corners: those pixels show
        # whatever is behind the window (the OS's, not the app's).
        c = int(12 * k)
        regs += [("window corner (OS)", 0, h - c, c, h), ("window corner (OS)", w - c, h - c, w, h)]
        px = d.load()
        mark = Image.eval(b.crop((0, 0, w, h)), lambda v: v // 3)
        mp = mark.load()
        changed, bad = 0, 0
        boxes = {}
        for y in range(h):
            for x in range(w):
                if max(px[x, y]) > 2:
                    changed += 1
                    reg = next((r[0] for r in regs if r[1] <= x < r[3] and r[2] <= y < r[4]), None)
                    if reg is None:
                        bad += 1
                        mp[x, y] = (255, 0, 255)
                        reg = "NOT ALLOWED"
                    else:
                        mp[x, y] = (255, 60, 60)
                    bx = boxes.get(reg, [x, y, x, y])
                    boxes[reg] = [min(bx[0], x), min(bx[1], y), max(bx[2], x), max(bx[3], y)]
        failed += bad > 0
        mark.save(os.path.join(out, f"{name}.png"))
        desc = "; ".join(f"{r}: {tuple(v)}" for r, v in boxes.items()) or "—"
        rows.append(f"| {name} @ {scale}% | {changed} | {bad} | {desc} |")
    rows.append("")
    rows.append("GATE: " + ("PASS" if failed == 0 else f"FAIL ({failed} states)"))
    text = "\n".join(rows)
    open(os.path.join(out, "report.md"), "w", encoding="utf-8", newline="\n").write(text + "\n")
    print(text)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
