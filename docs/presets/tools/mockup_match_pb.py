"""The device panel against the `workflow-chip-synth` board (PRESETS_PROMPT.md §8.3).

The same feature finder runs on the board's render (docs/design/meeting-2026-11-26/
workflow-chip-synth@1x.png: the studio device at (224, 532), the locked one at (224, 960)) and
on our screenshot (the device found from its border), and reports every feature's position in
device-local px and the offset, ours minus the board's. Pass: within 2 px. Display colours:
the warm display colour #E8DCC0 must appear exactly in our waveform and envelope displays.

usage: python mockup_match_pb.py <ours studio.png> <ours locked.png> <out.md> [scale 100]
Ours are window frame captures (shot.ps1): client px + (cl, ct) found from the frame.
"""
import sys

from PIL import Image

BOARD = "docs/design/meeting-2026-11-26/workflow-chip-synth@1x.png"


def lum(c):
    return (0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]) / 255.0


def find_box(img, guess):
    """The device's top-left: where the border (#2A2A2A) begins, near `guess`."""
    gx, gy = guess
    px = img.load()
    best = None
    for dy in range(-30, 31):
        for dx in range(-30, 31):
            x, y = gx + dx, gy + dy
            # The border's top-left corner region: the pixel 10 px right of the corner on the
            # top edge is the border colour, the one above it is the pane's dark.
            c = px[x + 12, y]
            above = px[x + 12, y - 2]
            left = px[x - 2, y + 60]
            on_left = px[x, y + 60]
            score = abs(c[0] - 0x2A) + abs(on_left[0] - 0x2A) + max(0, above[0] - 0x14) + max(0, left[0] - 0x14)
            if best is None or score < best[0]:
                best = (score, x, y)
    return best[1], best[2]


def edge(img, o, y, x0, x1, darker=True):
    """The first x in [x0, x1) (device-local) where the luma steps by > 0.02 on row y."""
    px = img.load()
    prev = lum(px[o[0] + x0 - 1, o[1] + y])
    for x in range(x0, x1):
        c = lum(px[o[0] + x, o[1] + y])
        if abs(c - prev) > 0.02:
            return x
        prev = c
    return None


def vedge(img, o, x, y0, y1):
    px = img.load()
    prev = lum(px[o[0] + x, o[1] + y0 - 1])
    for y in range(y0, y1):
        c = lum(px[o[0] + x, o[1] + y])
        if abs(c - prev) > 0.02:
            return y
        prev = c
    return None


def knob_centre(img, o, cx, cy, r=26):
    """The centroid of the brushed cap's bright pixels around (cx, cy)."""
    px = img.load()
    sx = sy = n = 0
    for y in range(cy - r, cy + r):
        for x in range(cx - r, cx + r):
            c = px[o[0] + x, o[1] + y]
            if lum(c) > 0.42 and abs(c[0] - c[2]) < 12:
                sx += x
                sy += y
                n += 1
    return (sx / n, sy / n) if n > 30 else None


def features(img, o, locked):
    f = {}
    f["header line y"] = vedge(img, o, 120, 32, 46)
    f["◀ key left x"] = edge(img, o, 20, 266, 292)
    f["field left x"] = edge(img, o, 20, 306, 316)
    f["field right x"] = edge(img, o, 20, 562, 574)
    f["▶ key left x"] = edge(img, o, 20, 572, 582)
    for name, x0 in (("OSCILLATOR", 4), ("ENVELOPES", 272), ("OUTPUT", 684)):
        f[f"{name} left x"] = edge(img, o, 240, x0, x0 + 14)
        f[f"{name} top y"] = vedge(img, o, x0 + 40, 46, 58)
    # Knobs (the board's): studio Duty, Unison, Detune / locked Duty; the envelope row; Volume.
    knobs = {"VIB DEPTH": (355, 218), "VIB RATE": (480, 218), "ARP SPEED": (605, 218), "VOLUME": (746, 150)}
    if locked:
        knobs["DUTY"] = (140, 218)
    else:
        knobs.update({"DUTY": (56, 218), "UNISON": (140, 218), "DETUNE": (224, 218)})
    for k, (x, y) in knobs.items():
        c = knob_centre(img, o, x, y, 34 if k == "VOLUME" else 22)
        f[f"knob {k} x"] = None if c is None else c[0]
        f[f"knob {k} y"] = None if c is None else c[1]
    return f


def warm(img, o):
    """Exact #E8DCC0 pixels in the two displays (waveform, envelopes)."""
    px = img.load()
    n = 0
    for y in range(90, 186):
        for x in range(24, 668):
            if px[o[0] + x, o[1] + y][:3] == (0xE8, 0xDC, 0xC0):
                n += 1
    return n


def main():
    studio, locked, out = sys.argv[1], sys.argv[2], sys.argv[3]
    scale = int(sys.argv[4]) if len(sys.argv) > 4 else 100
    S = scale / 100
    board = Image.open(BOARD).convert("RGB")
    rows, worst, n_ok, n_all = [], 0.0, 0, 0
    warm_ours = {}
    for kind, path, bo in (("studio", studio, (224, 532)), ("locked", locked, (224, 960))):
        img = Image.open(path).convert("RGB")
        ct = img.height - round(920 * S)
        cl = (img.width - round(1440 * S)) // 2
        o = find_box(img, (cl + 224, ct + 552))
        bo = find_box(board, bo)
        fb, fo = features(board, bo, kind == "locked"), features(img, o, kind == "locked")
        warm_ours[kind] = warm(img, o)
        for k in fb:
            b, u = fb[k], fo[k]
            if b is None or u is None:
                rows.append(f"| {kind} | {k} | {b} | {u} | not found | **?** |")
                continue
            d = u - b
            n_all += 1
            ok = abs(d) <= 2
            n_ok += ok
            worst = max(worst, abs(d))
            rows.append(f"| {kind} | {k} | {b:.1f} | {u:.1f} | {d:+.1f} | {'yes' if ok else '**no**'} |")
        rows.append(f"| {kind} | device box (window px) | {bo} | {o} | – | – |")
    colours = all(v > 0 for v in warm_ours.values())
    text = (
        f"# The device panel against the board ({scale} %)\n\n"
        "`docs/presets/tools/mockup_match_pb.py`: one feature finder on the board's render and on our screenshot; "
        "positions in device-local px (the device box's top-left), offset = ours − board's. Pass: within 2 px.\n\n"
        f"**{n_ok} of {n_all} within 2 px** (largest {worst:.1f} px). Display colour #E8DCC0 exact in our displays: "
        f"{'yes' if colours else '**no**'} ({warm_ours} exact pixels).\n\n"
        "| Device | Feature | Board | Ours | Offset | Within 2 px |\n|---|---|---|---|---|---|\n" + "\n".join(rows) + "\n"
    )
    open(out, "w", encoding="utf-8").write(text)
    print(f"{n_ok} of {n_all} within 2 px (largest {worst:.1f}); warm colour {warm_ours}")


if __name__ == "__main__":
    main()
