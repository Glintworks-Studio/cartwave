"""Batch 4b §9.3: the mixer strips and the Main strip against the `mixer` board, within 1 px.

The board (docs/design/meeting-2026-11-26/mixer@1x.png, the HTML's boxes at 1 CSS px = 1 px) and the
app's mixer (mixer_match_shot.ps1: fx_mixer scrolled to the board's tracks 8-16, the area under the
76 px top bar cropped to 1680 × 640) are measured with the same probes, at the same coordinates.
Meters are left out (they show levels: the board's are made up, the app's are at rest); everything
else is geometry:

  panel     the strip's #2A2A2A border: left, right, top, bottom
  bar       the name plate's colour bar: top row and height
  plate     the plate's recess: left edge
  name      the name's ink: left, top, bottom          number   the number's ink: right edge
  knob      the knob's cap (the bright brushed metal): bounding box
  keys      M and S: their top sheens' x extents
  clip      the clip mark's well (#1A1A1A): box
  glass     the meter glass (#050505): left, right, top, bottom
  slot      the fader slot: left, right, top, bottom
  cap       the cap's white centre line: y and x extent (at the track's dB)
  ticks     the scale's #4A4A4A ticks: their rows (+6, 0, 6, 12, 24, 36, inf)
  peak      the PEAK readout's well: box             voices  VOICES / the channel: centre x, top
  readout   the dB readout's well: box

usage: python mockup_match_mixer.py <app shot> <out dir> [scale%]
(writes report.md and mixer-mockup-match.png)"""
import os
import sys

from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.join(HERE, "..", "..", "design", "meeting-2026-11-26", "mixer@1x.png")
sys.path.insert(0, os.path.join(HERE, "..", "..", "polish2", "tools"))
from diff import client_top  # noqa: E402

STRIPS = [24 + 160 * k for k in range(9)]
MAIN_X = 1480
TOP = 60


def lum(p):
    return max(p[:3])


def near(p, c, tol):
    return all(abs(a - b) <= tol for a, b in zip(p[:3], c))


class Img:
    def __init__(self, im):
        self.im = im
        self.px = im.load()
        self.w, self.h = im.size

    def get(self, x, y):
        x, y = int(round(x)), int(round(y))
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.px[x, y]
        return (0, 0, 0)

    def run(self, y, x0, x1, pred):
        """First and last x on row y in [x0, x1) where pred holds."""
        xs = [x for x in range(int(x0), int(x1)) if pred(self.get(x, y))]
        return (min(xs), max(xs)) if xs else None

    def col(self, x, y0, y1, pred):
        ys = [y for y in range(int(y0), int(y1)) if pred(self.get(x, y))]
        return (min(ys), max(ys)) if ys else None

    def col_run(self, x, seed, pred):
        """The contiguous run of rows around `seed` on column x where pred holds."""
        if not pred(self.get(x, seed)):
            return None
        a = b = seed
        while pred(self.get(x, a - 1)):
            a -= 1
        while pred(self.get(x, b + 1)):
            b += 1
        return (a, b)

    def box(self, x0, y0, x1, y1, pred):
        xs, ys = [], []
        for y in range(int(y0), int(y1)):
            for x in range(int(x0), int(x1)):
                if pred(self.get(x, y)):
                    xs.append(x)
                    ys.append(y)
        return (min(xs), min(ys), max(xs), max(ys)) if xs else None


def measure(I, x0, main):
    """Strip at x0 (its left), y from TOP. Values relative to the strip's origin."""
    y0 = TOP
    w = 176 if main else 152
    m = {}
    rel = lambda v, base: None if v is None else tuple(round(a - base, 1) for a in v)  # noqa: E731
    border = lambda p: near(p, (42, 42, 42), 7)  # noqa: E731
    # The panel's border: on a row through the middle, the leftmost and rightmost border pixels;
    # on a column through the plate gap, the top and bottom.
    lr = I.run(y0 + 300, x0 - 3, x0 + 6, border)
    rr = I.run(y0 + 300, x0 + w - 6, x0 + w + 3, border)
    tb = I.col(x0 + 4, y0 - 3, y0 + 6, border)
    bb = I.col(x0 + 4, y0 + 546, y0 + 555, border)
    m["panel"] = (lr and lr[0] - x0, rr and rr[1] - x0, tb and tb[0] - y0, bb and bb[1] - y0)
    # The plate's colour bar (tracks: saturated; Main: the dark gradient, skipped).
    if not main:
        c = I.col(x0 + 76, y0 + 2, y0 + 20, lambda p: lum(p) >= 0x50)
        m["bar"] = c and (c[0] - y0, c[1] - c[0] + 1)
    plate_l = I.run(y0 + 40, x0 + 3, x0 + 16, lambda p: lum(p) <= 0x16)
    m["plate"] = plate_l and (plate_l[0] - x0,)
    nb = I.box(x0 + 30 if not main else x0 + 12, y0 + 14, x0 + w - 10, y0 + 32, lambda p: lum(p) >= 0xC0)
    m["name"] = nb and (nb[0] - x0, nb[1] - y0, nb[3] - y0)
    if not main:
        num = I.box(x0 + 14, y0 + 14, x0 + 40, y0 + 32, lambda p: 0x2C <= lum(p) <= 0x50 and max(p[:3]) - min(p[:3]) < 8)
        m["number"] = num and (num[2] - x0,)
        knob = I.box(x0 + 60, y0 + 57, x0 + 92, y0 + 89, lambda p: lum(p) >= 0x46)
        m["knob"] = knob and (knob[0] - x0, knob[1] - y0, knob[2] - x0, knob[3] - y0)
        # The keys' bodies (24 px, a darker body and a #101010 border on the lighter panel): the
        # dark run across each key's middle row. (Their 1 px top sheen is a hairline: at 150 % it
        # falls between the logical rows a 1x comparison samples.)
        def sheen_at(kx):
            return I.run(y0 + 130, x0 + kx - 4, x0 + kx + 28, lambda p: lum(p) <= 0x1A)
        sheen = [sheen_at(kx) for kx in (48, 80)]
        m["keys"] = tuple((s[0] - x0, s[1] - x0) if s else None for s in sheen)
        # The clip mark's well: #1A1A1A with a 1 px black inner top shadow, on the panel (~#1C);
        # found by its darker top row (the inset shadow) and its columns.
        clip = I.box(x0 + 98, y0 + 146, x0 + 124, y0 + 152, lambda p: lum(p) <= 0x0C)
        m["clip"] = clip and (clip[0] - x0, clip[1] - y0, clip[2] - x0, clip[3] - y0)
    gx, gw = (105, 30) if main else (97, 22)
    dark5 = lambda p: lum(p) <= 0x08  # noqa: E731
    g_lr = I.run(y0 + 400, x0 + gx - 4, x0 + gx + gw + 4, lambda p: lum(p) <= 0x0C)
    # The top and bottom from a column at the glass's right, clear of the diagonal reflection.
    g_tb = I.col(x0 + gx + gw - 1, y0 + 155, y0 + 440, lambda p: lum(p) <= 0x0C)
    # Its sides and bottom (its top rows carry the lit segments' glow and the reflection).
    m["glass"] = (g_lr and g_lr[0] - x0, g_lr and g_lr[1] - x0, g_tb and g_tb[1] - y0)
    sx = 72 if main else 64
    # The slot is darker than the panel beside it (a locked strip's at 45 %): its run is
    # found against the panel's own level on that row / column.
    yr = y0 + 428
    ref = lum(I.get(x0 + sx - 6, yr))
    s_lr = I.run(yr, x0 + sx - 4, x0 + sx + 10, lambda p: lum(p) <= ref - 6)
    refc = lum(I.get(x0 + sx - 6, y0 + 300))
    s_tb = I.col_run(x0 + sx + 3, y0 + 300, lambda p: lum(p) <= refc - 6)
    m["slot"] = (s_lr and s_lr[0] - x0, s_lr and s_lr[1] - x0, s_tb and s_tb[0] - y0, s_tb and s_tb[1] - y0)
    cx = 60 if main else 52
    line = I.box(x0 + cx + 2, y0 + 160, x0 + cx + 29, y0 + 432, lambda p: lum(p) >= 0x8C and max(p[:3]) - min(p[:3]) < 10)
    m["cap"] = line and (line[1] - y0, line[0] - x0, line[2] - x0)
    tx = 69 if main else 61
    # Tick rows, outside the cap (it covers the ticks where it sits).
    cap_rows = range(m["cap"][0] + y0 - 24, m["cap"][0] + y0 + 26) if m.get("cap") else range(0)
    rows = [y for y in range(y0 + 160, y0 + 432) if near(I.get(x0 + tx, y), (72, 72, 72), 10) and y not in cap_rows and near(I.get(x0 + tx - 2, y), (72, 72, 72), 10)]
    groups = []
    for y in rows:
        if not groups or y > groups[-1][-1] + 1:
            groups.append([y])
        else:
            groups[-1].append(y)
    m["ticks"] = tuple(g[0] - y0 for g in groups)
    if main:
        pk = I.run(y0 + 93, x0 + 40, x0 + 136, lambda p: lum(p) <= 0x10)
        pkt = I.col(x0 + 60, y0 + 74, y0 + 112, lambda p: lum(p) <= 0x10)
        m["peak"] = (pk and pk[0] - x0, pk and pk[1] - x0, pkt and pkt[0] - y0, pkt and pkt[1] - y0)
    else:
        pk = I.run(y0 + 450, x0 + 70, x0 + 146, lambda p: lum(p) <= 0x10)
        pkt = I.col(x0 + 80, y0 + 436, y0 + 464, lambda p: lum(p) <= 0x10)
        m["peak"] = (pk and pk[0] - x0, pk and pk[1] - x0, pkt and pkt[0] - y0, pkt and pkt[1] - y0)
        vb = I.box(x0 + 10, y0 + 470, x0 + w - 10, y0 + 488, lambda p: lum(p) >= 0x60)
        m["voices"] = vb and (round((vb[0] + vb[2]) / 2 - x0, 1), vb[1] - y0)
    if main:
        # Main's well and its panel's bottom are both near black: its text (centre, top, bottom).
        rt = I.box(x0 + 14, y0 + 514, x0 + w - 14, y0 + 540, lambda p: lum(p) >= 0x70)
        m["readout"] = rt and (round((rt[0] + rt[2]) / 2 - x0, 1), rt[1] - y0, rt[3] - y0)
    else:
        ro_lr = I.run(y0 + 520, x0 + 6, x0 + w - 4, lambda p: lum(p) <= 0x10)
        ro_tb = I.col(x0 + 20, y0 + 505, y0 + 548, lambda p: lum(p) <= 0x10)
        m["readout"] = (ro_lr and ro_lr[0] - x0, ro_lr and ro_lr[1] - x0, ro_tb and ro_tb[0] - y0, ro_tb and ro_tb[1] - y0)
    return m


def flat(v):
    if v is None:
        return [None]
    if isinstance(v, (int, float)):
        return [v]
    out = []
    for x in v:
        out += flat(x)
    return out


def main():
    shot, out = sys.argv[1], sys.argv[2]
    scale = int(sys.argv[3]) if len(sys.argv) > 3 else 100
    k = scale / 100
    os.makedirs(out, exist_ok=True)
    im = Image.open(shot).convert("RGB")
    t = client_top(im, 100)
    app = im.crop((1, t + round(76 * k), 1 + round(1680 * k), t + round(76 * k) + round(640 * k)))
    if k != 1:
        # Each logical pixel sampled at its centre (a filtering downscale would smear the 1 px
        # features: key sheens, the cap's line, the ticks).
        src = app.load()
        small = Image.new("RGB", (1680, 640))
        dst = small.load()
        for y in range(640):
            sy = min(app.height - 1, int((y + 0.5) * k))
            for x in range(1680):
                dst[x, y] = src[min(app.width - 1, int((x + 0.5) * k)), sy]
        app = small
    board = Image.open(BOARD).convert("RGB")
    B, A = Img(board), Img(app)
    lines = ["# The mixer vs the `mixer` board (Batch 4b §9.3)", "",
             f"App: `{os.path.basename(shot)}` at {scale} % (the area under the top bar{', scaled to 1x' if k != 1 else ''}). "
             "Board: `mixer@1x.png`. Values: px from the strip's top-left. Tolerance 1 px.", ""]
    total = fails = 0
    names = [f"strip {8 + i}" for i in range(9)] + ["Main"]
    for name, x0 in zip(names, STRIPS + [MAIN_X]):
        main_ = name == "Main"
        bm, am = measure(B, x0, main_), measure(A, x0, main_)
        lines += [f"## {name}", "", "| part | board | app | Δ | ok |", "|---|---|---|---|---|"]
        for key in bm:
            b, a = flat(bm[key]), flat(am.get(key))
            if key == "ticks":
                # As sets: each tick on one has one on the other within 1 px, outside both caps.
                cb = bm.get("cap") and bm["cap"][0]
                ca = am.get("cap") and am["cap"][0]
                # The cap (line ± 23) and its cast shadow (6 px down, σ 5) cover the ticks there.
                clear = lambda y: all(c is None or not (c - 26 <= y <= c + 45) for c in (cb, ca))  # noqa: E731
                bs, as_ = [y for y in b if clear(y)], [y for y in a if clear(y)]
                ok = bool(bs) and all(any(abs(x - y) <= 1 for y in as_) for x in bs) and all(any(abs(x - y) <= 1 for y in bs) for x in as_)
                total += 1
                fails += 0 if ok else 1
                lines.append(f"| ticks (outside the caps) | {tuple(bs)} | {tuple(as_)} | {'0-1' if ok else 'differ'} | {'yes' if ok else '**no**'} |")
                continue
            if None in b or None in a or len(a) != len(b):
                ok = False
                d = "missing"
            else:
                dd = max((abs(x - y) for x, y in zip(a, b)), default=0)
                ok = dd <= 1.0
                d = f"{dd:.1f}"
            total += 1
            fails += 0 if ok else 1
            lines.append(f"| {key} | {bm[key]} | {am.get(key)} | {d} | {'yes' if ok else '**no**'} |")
        lines.append("")
    lines.insert(4, f"**{total - fails} / {total} within 1 px.**\n")
    open(os.path.join(out, "report.md"), "w", encoding="utf-8").write("\n".join(lines) + "\n")
    # The side-by-side: board above, app below, 1 strip + Main enlarged.
    crop = lambda img, x: img.crop((x - 8, 52, x + 184, 620))  # noqa: E731
    parts = [crop(board, STRIPS[4]), crop(app, STRIPS[4]), crop(board, STRIPS[0]), crop(app, STRIPS[0]), crop(board, MAIN_X), crop(app, MAIN_X)]
    W = sum(p.width for p in parts) + 10 * 5
    canvas = Image.new("RGB", (W, 600), (24, 24, 24))
    x = 0
    dr = ImageDraw.Draw(canvas)
    for i, p in enumerate(parts):
        canvas.paste(p, (x, 24))
        dr.text((x + 4, 4), ("board " if i % 2 == 0 else "app ") + ["12 Lead Stack", "", "8 Pulse 1 (locked)", "", "Main", ""][i - i % 2], fill=(230, 230, 230))
        x += p.width + 10
    canvas.save(os.path.join(out, "mixer-mockup-match.png"))
    print(f"{total - fails}/{total} within 1 px")


if __name__ == "__main__":
    main()
