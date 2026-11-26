"""Assembles docs/screens/polish2/*.png from shots_p2.ps1's raw screenshots (POLISH2_PROMPT §10).

usage: python compose_p2.py <shots dir> [regression diff dir] [mockup-match dir]"""
import glob
import os
import sys

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
OUT = os.path.join(REPO, "docs", "screens", "polish2")
FONT = os.path.join(REPO, "assets", "fonts")


def font(px):
    for f in glob.glob(os.path.join(FONT, "**", "*.ttf"), recursive=True):
        if "Inter" in os.path.basename(f) and ("SemiBold" in f or "Medium" in f):
            return ImageFont.truetype(f, px)
    return ImageFont.load_default()


def client(path):
    """The window screenshot's client area (frame and title/menu bars cut)."""
    im = Image.open(path).convert("RGB")
    px = im.load()
    top = next(y for y in range(10, 200) if px[700, y][:3] == (0x32, 0x32, 0x32))
    return im.crop((1, top, im.width - 1, im.height - 1))


def stack(items, gap=16, bg=(24, 24, 24), label_h=28):
    f = font(15)
    w = max(im.width for _, im in items)
    h = sum(im.height + label_h for _, im in items) + gap * (len(items) - 1)
    out = Image.new("RGB", (w, h), bg)
    d = ImageDraw.Draw(out)
    y = 0
    for label, im in items:
        d.text((8, y + 6), label, fill=(220, 220, 220), font=f)
        out.paste(im, (0, y + label_h))
        y += im.height + label_h + gap
    return out


def save(name, im):
    os.makedirs(OUT, exist_ok=True)
    p = os.path.join(OUT, name)
    im.save(p)
    print(p)


def main():
    shots = sys.argv[1]
    reg = sys.argv[2] if len(sys.argv) > 2 else None
    mm = sys.argv[3] if len(sys.argv) > 3 else None
    c = lambda n: client(os.path.join(shots, n + ".png"))
    # The header area: browser + headers + the first lanes.
    head = lambda im, h=400: im.crop((0, 0, 1060, h))
    save("header-e1.png", head(c("e1"), 420))
    save("row-heights.png", stack([("72 / 144 / 240 px (the board's rows)", head(c("rows_board"), 500)),
                                   ("72 / 96 / 144 / 240 px", head(c("rows"), 700))]))
    save("header-widths.png", stack([("380 px (minimum)", head(c("w380"), 340)), ("520 px", head(c("w520"), 340)),
                                     ("600 px (maximum)", head(c("w600"), 340)),
                                     ("380 px on the right", c("w380_right").crop((380, 0, 1440, 340)))]))
    save("layouts-4.png", stack([("Left · strip outside (default)", head(c("lay_lo"), 330)),
                                 ("Left · strip by the clips", head(c("lay_lc"), 330)),
                                 ("Right · strip by the clips", c("lay_rc").crop((380, 0, 1440, 330))),
                                 ("Right · strip outside", c("lay_ro").crop((380, 0, 1440, 330)))]))
    st = [("A message ('Saved') with the problem icon, and hover help (the fader)", c("status_msg_help")),
          ("Idle text with the persistent problem's icon", c("status_idle")),
          ("A click on the icon lists the problems", c("status_problems"))]
    save("status-bar.png", stack([(l, im.crop((0, im.height - 24 - (200 if "lists" in l else 0), 1440, im.height))) for l, im in st]))
    save("overview-on-off.png", stack([("Overview on (default)", c("e1_overview").crop((0, 0, 1440, 420))),
                                       ("Overview off (View ▸ Show Overview, Ctrl+Alt+O)", c("e1").crop((0, 0, 1440, 420)))]))
    frames = sorted(glob.glob(os.path.join(shots, "ruler_zoom_*.png")))
    if frames:
        save("ruler-zoom.png", stack([(f"frame {i}: dragging down on the ruler ({12 * i} px)", client(f).crop((600, 60, 1440, 260))) for i, f in enumerate(frames)], gap=6))
    # Strip before / after: the palette sheets' first tiles (both drawn by the app's own code).
    before = os.path.join(REPO, "docs", "screens", "polish", "palette-reference-sheet.png")
    after = os.path.join(OUT, "palette-reference-sheet.png")
    if os.path.exists(before) and os.path.exists(after):
        b, a = Image.open(before).convert("RGB"), Image.open(after).convert("RGB")
        w, h = min(b.width, a.width, 1200), min(b.height, a.height, 360)
        save("strip-before-after.png", stack([("Before (polish pass): white .35 / .70, glow .55, spill .35", b.crop((0, 0, w, h))),
                                              ("After (polish 2): white .28 / .55, glow .40, spill .25", a.crop((0, 0, w, h)))]))
    if reg and os.path.isdir(reg):
        items = [(os.path.basename(p)[:-4], Image.open(p).convert("RGB").resize((720, 460))) for p in sorted(glob.glob(os.path.join(reg, "*.png")))]
        if items:
            cols = 3
            rows = (len(items) + cols - 1) // cols
            sheet = Image.new("RGB", (cols * 728, rows * 496), (24, 24, 24))
            d = ImageDraw.Draw(sheet)
            for i, (n, im) in enumerate(items):
                x, y = (i % cols) * 728, (i // cols) * 496
                d.text((x + 6, y + 6), n, fill=(230, 230, 230), font=font(15))
                sheet.paste(im, (x, y + 32))
            save("regression-diff.png", sheet)
    if mm and os.path.isdir(mm):
        items = [(os.path.basename(p)[:-4], Image.open(p).convert("RGB")) for p in sorted(glob.glob(os.path.join(mm, "*.png")))]
        if items:
            save("mockup-match.png", stack(items))


if __name__ == "__main__":
    main()
