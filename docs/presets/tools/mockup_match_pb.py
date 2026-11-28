"""The device panel against the `workflow-chip-synth` board (PRESETS_PROMPT.md §8.3).

Each feature is a patch of the board's render (docs/design/meeting-2026-11-26/
workflow-chip-synth@1x.png: the studio device's box at (224, 532), the locked one's at
(224, 960)) around a part the board draws: the header line, the ◀ ▶ keys, the preset field's
ends, the sections' corners and titles, the displays' corners, the knobs, the meter. The patch is
searched in our screenshot within ±8 px of the same device-local place by normalised
cross-correlation of the grey levels (brightness-invariant, so the locked device's 40 % parts
match too); the best match's offset is ours minus the board's. Pass: within 2 px (the prompt's
limit), and the match is real (correlation >= 0.5). Display colours: #E8DCC0 must appear
exactly in our studio displays (the locked ones are drawn at 40 %).

usage: python mockup_match_pb.py <ours studio.png> <ours locked.png> <out.md> [scale 100]
Ours are window frame captures (shot.ps1); our device box is found from the client area.
"""
import math
import sys as _s
_s.stdout.reconfigure(encoding="utf-8")
import sys

from PIL import Image

BOARD = "docs/design/meeting-2026-11-26/workflow-chip-synth@1x.png"
SEARCH = 8

# (name, x0, y0, x1, y1) in device-local px: parts the board draws, in both states unless marked.
COMMON = [
    ("header: colour tab and name", 8, 6, 120, 34),
    ("header: line under it (y only; with the sections' top edge)", 150, 34, 270, 58),
    ("◀ key", 274, 3, 308, 37),
    ("preset field, left end", 306, 3, 350, 37),
    ("preset field, right end", 540, 3, 574, 37),
    ("▶ key", 572, 3, 606, 37),
    ("OSCILLATOR corner", 6, 46, 22, 70),
    ("OSCILLATOR title", 20, 60, 110, 86),
    ("ENVELOPES corner", 274, 46, 290, 70),
    ("ENVELOPES title", 288, 60, 380, 86),
    ("OUTPUT corner", 686, 46, 702, 70),
    ("OUTPUT title", 700, 60, 760, 86),
    ("OSCILLATOR display, top-left", 18, 84, 60, 112),
    ("OSCILLATOR display, bottom-right", 216, 168, 262, 196),
    ("ENVELOPES display, top-left", 286, 84, 328, 112),
    ("ENVELOPES display, bottom-right corner", 652, 180, 680, 196),
    ("knob VIB DEPTH", 331, 194, 379, 242),
    ("knob VIB RATE", 456, 194, 504, 242),
    ("knob ARP SPEED", 581, 194, 629, 242),
    ("knob VOLUME", 713, 113, 791, 191),
    ("meter, top (unlit)", 808, 100, 832, 126),
]
STUDIO = [
    ("knob DUTY", 32, 194, 80, 242),
    ("knob UNISON", 116, 194, 164, 242),
    ("knob DETUNE", 200, 194, 248, 242),
    ("voice selector", 190, 58, 262, 86),
]
LOCKED = [
    ("knob DUTY (alone)", 116, 194, 164, 242),
]
def grey(img):
    w, h = img.size
    px = img.load()
    return [[0.299 * px[x, y][0] + 0.587 * px[x, y][1] + 0.114 * px[x, y][2] for x in range(w)] for y in range(h)]


def patch(g, ox, oy, box):
    x0, y0, x1, y1 = box
    return [g[oy + y][ox + x0:ox + x1] for y in range(y0, y1)]


def ncc(a, b):
    n = len(a) * len(a[0])
    ma = sum(map(sum, a)) / n
    mb = sum(map(sum, b)) / n
    num = da = db = 0.0
    for ra, rb in zip(a, b):
        for va, vb in zip(ra, rb):
            xa, xb = va - ma, vb - mb
            num += xa * xb
            da += xa * xa
            db += xb * xb
    return num / math.sqrt(da * db) if da > 0 and db > 0 else 0.0


def best(gb, bo, go, oo, box, y_only=False):
    ref = patch(gb, bo[0], bo[1], box)
    bestv, bestd = -2.0, (0, 0)
    for dy in range(-SEARCH, SEARCH + 1):
        for dx in ([0] if y_only else range(-SEARCH, SEARCH + 1)):
            c = ncc(ref, patch(go, oo[0] + dx, oo[1] + dy, box))
            if c > bestv:
                bestv, bestd = c, (dx, dy)
    return bestd, bestv


def warm(img, o):
    px = img.load()
    return sum(1 for y in range(o[1] + 90, o[1] + 186) for x in range(o[0] + 24, o[0] + 668) if px[x, y][:3] == (0xE8, 0xDC, 0xC0))


def main():
    studio, locked, out = sys.argv[1], sys.argv[2], sys.argv[3]
    scale = int(sys.argv[4]) if len(sys.argv) > 4 else 100
    S = scale / 100
    board = Image.open(BOARD).convert("RGB")
    gb = grey(board)
    rows, n_ok, n_all, worst = [], 0, 0, 0
    warms = {}
    for kind, path, bo, extra in (("studio", studio, (224, 532), STUDIO), ("locked", locked, (224, 960), LOCKED)):
        img = Image.open(path).convert("RGB")
        ct = img.height - round(920 * S)
        cl = (img.width - round(1440 * S)) // 2
        go = grey(img)
        # The device box: its top-left corner (the border's rounding and the pane around it)
        # matched first; every part is measured from there, as on the board.
        guess = (cl + 224, ct + 552)
        (cx, cy), cc = best(gb, bo, go, guess, (-6, -6, 18, 18))
        o = (guess[0] + cx, guess[1] + cy)
        rows.append(f"| {kind} | device box, top-left corner | (0, 0) | anchor: window ({o[0]}, {o[1]}) | {cc:.2f} | – |")
        warms[kind] = warm(img, o)
        for name, *box in COMMON + extra:
            (dx, dy), c = best(gb, bo, go, o, box, y_only="y only" in name)
            ok = abs(dx) <= 2 and abs(dy) <= 2 and c >= 0.5
            n_all += 1
            n_ok += ok
            worst = max(worst, abs(dx), abs(dy))
            rows.append(f"| {kind} | {name} | ({box[0]}, {box[1]}) | {dx:+d}, {dy:+d} | {c:.2f} | {'yes' if ok else '**no**'} |")
    colours = warms["studio"] > 0
    text = (
        f"# The device panel against the board ({scale} %)\n\n"
        "`docs/presets/tools/mockup_match_pb.py`: each board part (a patch of the board's render) found in our screenshot "
        "by normalised cross-correlation within ±8 px of the same device-local place; offset = ours − board's (x, y). "
        "Pass: within 2 px and correlation ≥ 0.5.\n\n"
        f"**{n_ok} of {n_all} within 2 px** (largest offset {worst} px). Display colour #E8DCC0 exact in our studio displays: "
        f"{'yes' if colours else '**no**'} ({warms['studio']} exact pixels; the locked ones are at 40 %: {warms['locked']}).\n\n"
        "| Device | Part | Board patch at | Offset (px) | Correlation | Within 2 px |\n|---|---|---|---|---|---|\n" + "\n".join(rows) + "\n"
    )
    open(out, "w", encoding="utf-8").write(text)
    print(text)


if __name__ == "__main__":
    main()
