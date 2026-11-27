"""Batch 4b §3.2 / §9.3 (G5): the Main row against the `main-track` board, within 1 px.

The board is HTML (1 CSS px = 1 logical px); its 1x render
(docs/design/meeting-2026-11-26/main-track@1x.png) is that HTML's boxes, so the board and the app's
shots (main_shots.ps1) are measured with the same detectors:

  name      "Main": ink >= #B0B0B0 in the name column
  badge     the OUT badge's #323232 ring (columns / rows with a long run of it)
  speaker   the speaker glyph's ink inside the ring (its first 14 px)
  out       "OUT": the ink after the glyph
  device    the device name's ink: left edge and top (the text differs: "MOTU M2" / "No output")
  peak_l    "PEAK" ink in column A
  peak_v    the PEAK value's ink: centre x and top (the text differs: "−1.2 dB" / "−inf")
  well      the master fader's #050505 well: x and top (the board's meter fill covers the rest)
  db        the dB value's ink ("−3.0 dB" in both)
  strip     the black strip's width (16) and its #4A4A4A outer edge's rows
  row       the divider (#2E2E2E) and the row's height to the status bar (72)

Positions are compared in the board's frame: each part is anchored to its column (the name column,
the control block, the strip), which the header layouts move or mirror as a whole (polish 2 §5.3).

Checks:
  1. Every part equals the board's within 1 logical px (150 % shots are divided by 1.5), in every
     layout and at 520 px.
  2. In every layout the Main row lines up with track 1 as on the board: the strip's outer edge,
     the name's and the badge's left edges, the fader's well, the dB value's centre, and PEAK
     centred in column A (track 1's pan value centre).

usage: python mockup_match_main.py <shots dir> [scale%]   (writes report.md and
main-mockup-match.png there)"""
import os
import sys

from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.join(HERE, "..", "..", "design", "meeting-2026-11-26", "main-track@1x.png")
sys.path.insert(0, os.path.join(HERE, "..", "..", "polish2", "tools"))
from diff import client_top  # noqa: E402

LAYOUTS = {"left_outside": (False, False, 380), "left_byclips": (False, True, 380), "right_byclips": (True, True, 380),
           "right_outside": (True, False, 380), "left_outside_520": (False, False, 520)}
PARTS = ["name", "badge", "speaker", "out", "device", "peak_l", "peak_v", "well", "db", "strip", "row"]


def rgb(h):
    return tuple(int(h[i:i + 2], 16) for i in (1, 3, 5))


def near(p, c, tol=3):
    return all(abs(a - b) <= tol for a, b in zip(p[:3], c))


def lum(p):
    return max(p[:3])


class Shot:
    def __init__(self, im):
        self.im = im
        self.px = im.load()

    def box(self, box, pred):
        x0, y0, x1, y1 = [int(round(v)) for v in box]
        x0, y0, x1, y1 = max(0, x0), max(0, y0), min(self.im.width, x1), min(self.im.height, y1)
        xs, ys = [], []
        for y in range(y0, y1):
            for x in range(x0, x1):
                if pred(self.px[x, y]):
                    xs.append(x)
                    ys.append(y)
        if not xs:
            return None
        return [min(xs), min(ys), max(xs), max(ys)]

    def ring(self, box, colour, min_run):
        """A 1 px ring of `colour` in `box`: the outermost columns and rows that hold a run."""
        x0, y0, x1, y1 = [int(round(v)) for v in box]
        cols = [x for x in range(x0, x1) if sum(near(self.px[x, y], colour, 6) for y in range(y0, y1)) >= min_run]
        rows = [y for y in range(y0, y1) if sum(near(self.px[x, y], colour, 6) for x in range(x0, x1)) >= min_run]
        if not cols or not rows:
            return None
        # The ring's right edge: the last column before its top edge's straight run ends (text
        # after the ring can look like a column of the colour).
        yt = min(rows)
        x = min(cols) + 6
        while x < x1 and any(near(self.px[x, yy], colour, 12) for yy in (yt, yt + 1)):
            x += 1
        right = max(c for c in cols if c <= x + 6)
        return [min(cols), yt, right, max(rows)]


def divider(sh, x):
    for y in range(sh.im.height - 1, 0, -1):
        if near(sh.px[x, y], rgb("#2E2E2E"), 2):
            return y
    return None


def frames(w, mirrored):
    """Header-relative left edges (logical) of the name column, the control block and the strip
    (strip-first frame: name 68, block 248 at 380 px; mirrored as a whole)."""
    name_w = w - 68 - 144
    if mirrored:
        return {"name": w - 68 - name_w, "block": 12, "strip": w - 16}
    return {"name": 68, "block": w - 12 - 120, "strip": 0}


def measure(sh, hx, w, mirrored, k, top):
    """Boxes (device px) of the Main row whose body starts at y `top`; the header's left edge `hx`."""
    S = lambda v: v * k  # noqa: E731
    f = frames(w, mirrored)
    nx = hx + S(f["name"])
    bx = hx + S(f["block"])
    m = {}
    m["name"] = sh.box((nx, top + S(8), nx + S(w - 212), top + S(36)), lambda p: lum(p) >= 0xB0)
    ring = sh.ring((nx - S(2), top + S(36), nx + S(72), top + S(62)), rgb("#323232"), int(8 * k))
    m["badge"] = ring
    if ring:
        ix0, iy0, ix1, iy1 = ring[0] + 1, ring[1] + 1, ring[2], ring[3]
        m["speaker"] = sh.box((ix0, iy0, ix0 + S(14), iy1), lambda p: lum(p) >= 0x70)
        sp = m["speaker"]
        m["out"] = sh.box(((sp[2] + 2) if sp else ix0, iy0, ix1, iy1), lambda p: lum(p) >= 0x70)
        m["device"] = sh.box((ring[2] + 2, top + S(40), nx + S(w - 212), top + S(60)), lambda p: 0x40 <= lum(p) <= 0x9A)
    ca = bx + S(8)
    m["peak_l"] = sh.box((ca, top + S(16), ca + S(56), top + S(36)), lambda p: lum(p) >= 0x58)
    m["peak_v"] = sh.box((ca, top + S(36), ca + S(56), top + S(54)), lambda p: lum(p) >= 0x58)
    cb = ca + S(72)
    m["well"] = sh.box((cb, top + S(4), cb + S(40), top + S(14)), lambda p: near(p, rgb("#050505"), 2))
    m["db"] = sh.box((cb, top + S(50), cb + S(40), top + S(68)), lambda p: lum(p) >= 0x58)
    sx = hx + S(f["strip"])
    m["strip"] = sh.box((sx - S(2), top + S(4), sx + S(18), top + S(60)), lambda p: near(p, rgb("#4A4A4A"), 3))
    return m, f


def track1(sh, hx, w, mirrored, k, rows_top):
    """Track 1's strip edge, name, badge, well, dB value and pan value boxes (device px)."""
    S = lambda v: v * k  # noqa: E731
    f = frames(w, mirrored)
    nx, bx = hx + S(f["name"]), hx + S(f["block"])
    t = {}
    t["name"] = sh.box((nx, rows_top + S(8), nx + S(w - 212), rows_top + S(36)), lambda p: lum(p) >= 0xB0)
    t["badge"] = sh.ring((nx - S(2), rows_top + S(36), nx + S(90), rows_top + S(62)), rgb("#323232"), int(8 * k))
    ca = bx + S(8)
    t["pan_v"] = sh.box((ca, rows_top + S(40), ca + S(56), rows_top + S(56)), lambda p: lum(p) >= 0x58)
    cb = ca + S(72)
    t["well"] = sh.box((cb, rows_top + S(2), cb + S(40), rows_top + S(60)), lambda p: near(p, rgb("#050505"), 2))
    t["db"] = sh.box((cb, rows_top + S(50), cb + S(40), rows_top + S(68)), lambda p: lum(p) >= 0x58)
    return t


def to_frame(key, b, hx, top, k, f):
    """A part's box (device px) in the board's frame (logical): its column's offset removed and
    the board's column offset added (name 68, block 248, strip 0)."""
    if b is None:
        return None
    anchor = {"name": "name", "badge": "name", "speaker": "name", "out": "name", "device": "name",
              "peak_l": "block", "peak_v": "block", "well": "block", "db": "block", "strip": "strip"}[key]
    board = {"name": 68, "block": 248, "strip": 0}[anchor]
    xs = lambda x: round((x - hx) / k - f[anchor] + board, 2)  # noqa: E731
    ys = lambda y: round((y - top) / k, 2)  # noqa: E731
    return [xs(b[0]), ys(b[1]), xs(b[2]), ys(b[3])]


def compare(key, b, a):
    """(pairs to compare) for part `key` (board box b, app box a, in the board's frame)."""
    c = lambda q: (q[0] + q[2]) / 2  # noqa: E731
    # Text: its anchor (a left-aligned text's left edge, a centred one's centre) and its top and
    # bottom; the ink's width follows the rasteriser and the scale, not the layout.
    if key in ("name", "out"):
        return [(b[0], a[0]), (b[1], a[1]), (b[3], a[3])]
    if key in ("peak_l", "db"):
        return [(c(b), c(a)), (b[1], a[1]), (b[3], a[3])]
    if key == "device":
        return [(b[0], a[0]), (b[1], a[1])]
    if key == "peak_v":
        return [(c(b), c(a)), (b[1], a[1])]
    if key == "well":
        return [(b[0], a[0]), (b[2], a[2]), (b[1], a[1])]
    if key == "strip":
        # The edge's column depends on the side (outer edge); its rows are the strip's.
        return [(b[1], a[1]), (b[3], a[3])]
    return [(b[0], a[0]), (b[1], a[1]), (b[2], a[2]), (b[3], a[3])]


def main():
    d = sys.argv[1]
    scale = int(sys.argv[2]) if len(sys.argv) > 2 else 100
    k = scale / 100
    out = ["# Main row vs the `main-track` board (Batch 4b §3.2, §9.3)", "",
           f"Shots: `{os.path.basename(os.path.normpath(d))}` at {scale} %. Board: `main-track@1x.png` (its HTML "
           "boxes). Tolerance 1 logical px. Coordinates: the board's frame (each part's column moved to the board's "
           "place), y from the Main row's top.", ""]
    total = fails = 0
    bsh = Shot(Image.open(BOARD).convert("RGB"))
    bdiv = divider(bsh, 1300)
    btop = bdiv + 1
    bm, bf = measure(bsh, 0, 380, False, 1.0, btop)
    brel = {key: to_frame(key, b, 0, btop, 1.0, bf) for key, b in bm.items()}
    bstatus = next(y for y in range(btop, bsh.im.height) if near(bsh.px[1300, y], rgb("#1E1E1E"), 1))
    brow_h = bstatus - btop
    side = {}
    for name, (right, byclips, w) in LAYOUTS.items():
        path = os.path.join(d, name + ".png")
        if not os.path.exists(path):
            continue
        im = Image.open(path).convert("RGB")
        t = client_top(im, 100)
        im = im.crop((1, t, im.width - 1, im.height - 1))
        sh = Shot(im)
        W = round(w * k)
        hx = im.width - W if right else 0
        lane_x = (hx - round(60 * k)) if right else (W + round(60 * k))
        div = divider(sh, lane_x)
        top = div + 1
        mirrored = right != byclips
        m, f = measure(sh, hx, w, mirrored, k, top)
        status = next((y for y in range(top, im.height) if near(sh.px[lane_x, y], rgb("#1E1E1E"), 1)), None)
        out += [f"## {name}", "", "| part | board | app | Δ | ok |", "|---|---|---|---|---|"]
        for key in PARTS:
            if key == "row":
                h = (status - top) / k if status else None
                ok = h is not None and abs(h - brow_h) <= 1
                total += 1
                fails += 0 if ok else 1
                out.append(f"| row height | {brow_h} | {h} | {abs((h or 0) - brow_h):.2f} | {'yes' if ok else '**no**'} |")
                continue
            b, a = brel.get(key), to_frame(key, m.get(key), hx, top, k, f)
            total += 1
            if b is None or a is None:
                fails += 1
                out.append(f"| {key} | {b} | {a} | missing | **no** |")
                continue
            delta = max(abs(x - y) for x, y in compare(key, b, a))
            ok = delta <= 1.0
            fails += 0 if ok else 1
            out.append(f"| {key} | {b} | {a} | {delta:.2f} | {'yes' if ok else '**no**'} |")
        # The lane: its fill and bar lines (the board's #0D0D0D and #222222), on a row through the
        # middle of the lane, away from the playhead.
        lx0, lx1 = (0, hx - round(4 * k)) if right else (hx + W + round(4 * k), im.width)
        ly = top + round(36 * k)
        lane = [sh.px[x, ly] for x in range(lx0, lx1)]
        fill_ok = sum(near(p, rgb("#0D0D0D"), 1) for p in lane) > 0.8 * len(lane)
        bars_ok = any(near(p, rgb("#222222"), 2) for p in lane)
        beats = any(near(p, rgb("#1D1D1D"), 1) or near(p, rgb("#262626"), 1) for p in lane)
        for label, ok in [("lane fill #0D0D0D", fill_ok), ("lane bar lines #222222", bars_ok), ("no beat lines in the lane", not beats)]:
            total += 1
            fails += 0 if ok else 1
            out.append(f"| {label} | | | | {'yes' if ok else '**no**'} |")
        # 2. Alignment with track 1 (the row under the ruler: find its strip's first coloured row).
        sx = hx + round(frames(w, mirrored)["strip"] * k) + round(8 * k)
        # Track 1's top: the first row under the ruler where its strip's colour starts.
        rows_top = next((y for y in range(round(100 * k), top) if lum(sh.px[min(sx, im.width - 1), y]) > 0x60 and lum(sh.px[min(sx, im.width - 1), y - 1]) <= 0x30), None)
        tr = track1(sh, hx, w, mirrored, k, rows_top) if rows_top is not None else {}
        cen = lambda q: (q[0] + q[2]) / 2  # noqa: E731
        pairs = [("name left", m.get("name"), tr.get("name"), lambda q: q[0]),
                 ("badge left", m.get("badge"), tr.get("badge"), lambda q: q[0]),
                 ("well x", m.get("well"), tr.get("well"), lambda q: q[0]),
                 ("dB centre", m.get("db"), tr.get("db"), cen),
                 ("PEAK centre = pan value centre", m.get("peak_l"), tr.get("pan_v"), cen)]
        out += ["", "| alignment with track 1 | Main | track 1 | Δ | ok |", "|---|---|---|---|---|"]
        for label, a, b, fn in pairs:
            total += 1
            if a is None or b is None:
                fails += 1
                out.append(f"| {label} | {a} | {b} | missing | **no** |")
                continue
            delta = abs(fn(a) - fn(b)) / k
            ok = delta <= 1.0
            fails += 0 if ok else 1
            out.append(f"| {label} | {fn(a) / k:.1f} | {fn(b) / k:.1f} | {delta:.2f} | {'yes' if ok else '**no**'} |")
        out.append("")
        side[name] = (im, m, top)
    out.append(f"**{total - fails} / {total} within 1 px.**")
    open(os.path.join(d, "report.md"), "w", encoding="utf-8").write("\n".join(out) + "\n")

    if "left_outside" in side:
        im, m, top = side["left_outside"]
        h = 80
        cb = bsh.im.crop((0, btop - 4, 1440, btop - 4 + h))
        ca = im.crop((0, top - round(4 * k), round(1440 * k), top - round(4 * k) + round(h * k))).resize(cb.size)
        canvas = Image.new("RGB", (1440, 2 * h + 28), (24, 24, 24))
        canvas.paste(cb, (0, 0))
        canvas.paste(ca, (0, h + 28))
        dr = ImageDraw.Draw(canvas)
        dr.text((8, h + 8), "above: the main-track board (red boxes); below: the app, headers left / strip outside (blue)", fill=(230, 230, 230))
        for key, b in bm.items():
            if b:
                dr.rectangle((b[0], b[1] - btop + 4, b[2], b[3] - btop + 4), outline=(255, 90, 90))
        for key, b in m.items():
            if b:
                dr.rectangle((b[0] / k, (b[1] - top) / k + 4 + h + 28, b[2] / k, (b[3] - top) / k + 4 + h + 28), outline=(90, 190, 255))
        canvas.save(os.path.join(d, "main-mockup-match.png"))
    print(f"{total - fails}/{total} within 1 px")
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
