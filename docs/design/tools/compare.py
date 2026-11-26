"""Design pass §4.1–4.3: measure the app against the Option D mockup.

usage: python compare.py <app window png> <mockup png (1440x400)> <out dir>

- Finds the app's client area (the top bar's #323232 top line) and crops
  1440 x 400 from it, the mockup's size.
- Measures every probe in BOTH images the same way: the bounding box of
  the pixels within `tol` of the probe's colour (or its predicate) inside
  the probe's search window (the mockup's box grown by `grow` px). The
  mockup's measured box is the reference; any edge more than 1 px away in
  the app is a deviation.
- Display alignment (§4.2): ink boxes of each cell's value and label; the
  ink centre's offset from the display's middle, and the values' baselines.
- Writes compare-topbar.png and compare-tracks.png (the app above the
  mockup), deviations.json and a markdown report (report.md).
"""
import json
import os
import sys

from PIL import Image

GREY = lambda c: lambda p: max(abs(p[0] - c), abs(p[1] - c), abs(p[2] - c))


def near(rgb, tol):
    r, g, b = rgb
    return lambda p: abs(p[0] - r) <= tol and abs(p[1] - g) <= tol and abs(p[2] - b) <= tol


def bright(th):
    """Ink of light text on dark: every channel above th, low saturation."""
    return lambda p: min(p) >= th and max(p) - min(p) <= 24


def bbox(img, win, pred):
    x0, y0, x1, y1 = win
    xs, ys = [], []
    px = img.load()
    for y in range(max(0, y0), min(img.height, y1)):
        for x in range(max(0, x0), min(img.width, x1)):
            if pred(px[x, y]):
                xs.append(x)
                ys.append(y)
    if not xs:
        return None
    return (min(xs), min(ys), max(xs) + 1, max(ys) + 1)


def hexrgb(h):
    return ((h >> 16) & 255, (h >> 8) & 255, h & 255)


# (name, mockup box x, y, w, h, predicate, grow). Boxes from the mockup's
# layout (docs/design/mockup-boxes.jsonl).
K, W, DIV, T = hexrgb(0x383838), hexrgb(0x323232), hexrgb(0x2A2A2A), 3
PROBES = [
    ("key Arrangement", 16, 18, 40, 40, near(K, T), 4),
    ("key Mixer", 64, 18, 40, 40, near(K, T), 4),
    ("divider left", 116, 24, 1, 28, near(W, T), 4),
    ("key Play", 129, 18, 40, 40, near(K, T), 4),
    ("key Stop", 177, 18, 40, 40, near(K, T), 4),
    ("key Record", 225, 18, 40, 40, near(K, T), 4),
    ("key Loop", 273, 18, 40, 40, near(K, T), 4),
    ("icon Arrangement", 27, 31, 18, 14, lambda p: p[1] > 150 and p[2] < 140, 4),
    ("icon Play", 143, 31, 12, 14, bright(170), 3),
    ("display", 399, 10, 560, 56, near(W, T), 4),
    ("TAP key", 971, 10, 56, 56, near(K, T), 4),
    ("TAP text", 986, 31, 25, 14, bright(150), 3),
    ("spectrum", 1113, 10, 216, 56, near(W, T), 4),
    ("cell divider 1", 496, 22, 1, 32, near(DIV, 2), 3),
    ("cell divider 2", 609, 22, 1, 32, near(DIV, 2), 3),
    ("cell divider 3", 706, 22, 1, 32, near(DIV, 2), 3),
    ("cell divider 4", 787, 22, 1, 32, near(DIV, 2), 3),
    ("cell divider 5", 868, 22, 1, 32, near(DIV, 2), 3),
    ("CPU/MEM labels", 1340, 21, 29, 33, near(hexrgb(0x808080), 30), 4),
    ("ruler bar line 2", 480, 76, 1, 27, near(hexrgb(0x3A3A3A), 3), 2),
    ("loop brace", 380, 76, 800, 4, lambda p: p[1] > 150 and p[2] < 120, 3),
    ("ruler hairline", 0, 103, 1440, 1, near(hexrgb(0x1E1E1E), 2), 1),
    ("TRACKS label", 16, 84, 50, 10, near(hexrgb(0x808080), 30), 4),
    ("row 1 hairline", 400, 176, 40, 1, near(hexrgb(0x1A1A1A), 1), 1),
    ("strip 1", 0, 104, 16, 72, lambda p: max(p) - min(p) > 30, 3),
    ("number 1", 33, 117, 18, 30, near(hexrgb(0x222222), 4), 4),
    ("name 1", 68, 121, 47, 11, bright(150), 4),
    ("badge 1", 68, 143, 53, 19, near(W, T), 3),
    ("preset 1", 128, 147, 72, 12, near(hexrgb(0x808080), 30), 4),
    ("M key 1", 312, 120, 24, 24, near(K, T), 3),
    ("S key 1", 344, 120, 24, 24, near(K, T), 3),
    ("volume thumb 1", 297, 152, 8, 12, near(hexrgb(0x4A4A4A), 3), 3),
    ("pan track 1", 328, 156, 40, 4, near(hexrgb(0x050505), 3), 1),
    ("pan detent 1", 347, 154, 1, 8, near(hexrgb(0x4A4A4A), 4), 2),
    ("M key 3", 312, 266, 24, 24, near(K, T), 3),
    # The mockup's Triangle is its row 2; the app's is lane 3 (one row
    # lower: the app shows lanes 1-4, the mockup four chosen tracks).
    ("clip Bassline (row)", 382, 179, 796, 68, near(hexrgb(0x7F86E0), 40), 3, 73),
    ("clip Walk Down (row)", 1182, 179, 258, 68, near(hexrgb(0x7F86E0), 40), 3, 73),
    ("clip Intro Lead header", 383, 107, 394, 16, near(hexrgb(0xC9D24A), 24), 3),
    ("strip 3 (row)", 0, 177, 16, 72, lambda p: max(p) - min(p) > 30, 3, 73),
]


def measure(img, probe, app=False):
    name, x, y, w, h, pred, grow = probe[:7]
    if app and len(probe) > 7:
        y += probe[7]
    win = (x - grow, y - grow, x + w + grow, y + h + grow)
    if pred is None:
        # A clip: its header's lane colour (the brightest saturated run).
        pred = lambda p: max(p) - min(p) > 60 and max(p) > 150
    return bbox(img, win, pred)


def client_crop(img):
    """The app's client area: the first row whose pixel at x 700 is the top
    bar's #323232 line."""
    for y in range(10, 120):
        if img.getpixel((700, y))[:3] == (0x32, 0x32, 0x32):
            return img.crop((1, y, 1441, y + 400)), y
    raise SystemExit("no top bar line found")


def display_alignment(img, disp_box):
    """Ink boxes of every cell's value and label (§4.2)."""
    x0, y0, x1, y1 = disp_box
    mid = (y0 + y1) / 2.0
    # Cell column ranges, from the dividers (measured) plus the edges.
    divs = []
    px = img.load()
    for x in range(x0 + 2, x1 - 2):
        if all(abs(px[x, y][0] - 0x2A) <= 2 for y in range(int(mid) + 4, int(mid) + 14)):
            divs.append(x)
    edges = [x0 + 1] + divs + [x1 - 1]
    cells = []
    for a, b in zip(edges, edges[1:]):
        if b - a < 30:
            continue
        # Values: any grey well above the well's #151515 (the overhang
        # darkens their tops, so no high threshold), or lime.
        val = bbox(img, (a + 1, y0 + 2, b, int(mid) + 4), lambda p: (min(p) > 44 and max(p) - min(p) < 30) or (p[1] > 90 and p[1] - p[2] > 40))
        lab = bbox(img, (a + 1, int(mid) + 4, b, y1 - 2), lambda p: (max(abs(c - 0x80) for c in p) < 40 and max(p) - min(p) < 12) or (p[1] > 140 and p[2] < 110))
        if val and lab:
            top, bottom = val[1], lab[3]
            cells.append({"x": a, "value_ink": val, "label_ink": lab, "ink_centre_offset": round((top + bottom) / 2.0 - mid, 2), "value_bottom": val[3]})
    return cells


def main():
    app_png, mock_png, out = sys.argv[1:4]
    os.makedirs(out, exist_ok=True)
    app, top = client_crop(Image.open(app_png).convert("RGB"))
    mock = Image.open(mock_png).convert("RGB")
    rows, devs = [], []
    for p in PROBES:
        m = measure(mock, p)
        a = measure(app, p, True)
        if a and len(p) > 7:
            a = (a[0], a[1] - p[7], a[2], a[3] - p[7])
        if m is None or a is None:
            rows.append((p[0], m, a, "not found"))
            devs.append({"probe": p[0], "mockup": m, "app": a, "issue": "not found"})
            continue
        d = [a[i] - m[i] for i in range(4)]
        worst = max(abs(v) for v in d)
        rows.append((p[0], m, a, d))
        if worst > 1:
            devs.append({"probe": p[0], "mockup": m, "app": a, "delta_ltrb": d})
    md = ["| Element | Mockup (l, t, r, b) | App | Δ (l, t, r, b) |", "|---|---|---|---|"]
    for name, m, a, d in rows:
        md.append(f"| {name} | {m} | {a} | {d} |")

    disp = measure(app, PROBES[9])
    disp_m = measure(mock, PROBES[9])
    align = {"app": display_alignment(app, disp), "mockup": display_alignment(mock, disp_m)}
    md.append("")
    md.append("Display alignment (ink centre offset from the display's middle, px; value ink bottom):")
    for k in ("app", "mockup"):
        md.append(f"- {k}: " + ", ".join(f"{c['ink_centre_offset']:+.2f} (bottom {c['value_bottom']})" for c in align[k]))
    with open(os.path.join(out, "report.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(md) + "\n")
    with open(os.path.join(out, "deviations.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump({"client_top": top, "deviations": devs, "alignment": align}, f, indent=1)

    # The comparison images: the app above the mockup.
    def stack(box, name):
        a = app.crop(box)
        m = mock.crop(box)
        gap = 8
        c = Image.new("RGB", (a.width, a.height * 2 + gap), (64, 64, 64))
        c.paste(a, (0, 0))
        c.paste(m, (0, a.height + gap))
        c.save(os.path.join(out, name))

    stack((0, 0, 1440, 104), "compare-topbar.png")
    stack((0, 76, 1440, 400), "compare-tracks.png")
    app.save(os.path.join(out, "app-1440x400.png"))
    print("\n".join(md))
    print(f"\n{len(devs)} deviations over 1 px")


if __name__ == "__main__":
    main()
