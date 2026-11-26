"""Polish 2 gate 2 (§9.2): every header element within 1 px of the board.

usage: python mockup_match.py <app screenshot.png> <board> <board row tops> <app row tops>
           <board header x> <app header x> <out dir> [--rows N] [--mirror] [--width W] [--heights 72,96,...]

Compares the first N rows' elements. Non-text parts (keys, wells, thumbs, strip, badge) are
compared with the board's HTML boxes (docs/polish2/boxes/<board>.jsonl, the spec, P2-D7); text
(number, name, values, scale labels) by its ink box against the board's 1x PNG, both found by
the same rule (pixels > 28 levels from the row's background) inside the element's box ± 3 px.
Writes <out>/<board>.md (a table per element) and <out>/<board>.png (crops: board | app | diff).
Exit 1 if any element is off by more than 1 px.
"""
import json
import os
import sys

from PIL import Image, ImageChops, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
BG = (10, 10, 10)


def ink_box(img, box, thresh=28, bg=BG):
    x0, y0, x1, y1 = (int(round(v)) for v in box)
    px = img.load()
    xs, ys = [], []
    for y in range(max(0, y0), min(img.height, y1)):
        for x in range(max(0, x0), min(img.width, x1)):
            p = px[x, y]
            if max(abs(p[0] - bg[0]), abs(p[1] - bg[1]), abs(p[2] - bg[2])) > thresh:
                xs.append(x)
                ys.append(y)
    if not xs:
        return None
    return (min(xs), min(ys), max(xs) + 1, max(ys) + 1)


def main():
    a = sys.argv[1:]
    shot, board = a[0], a[1]
    btops = [float(v) for v in a[2].split(",")]
    atops = [float(v) for v in a[3].split(",")]
    bx, ax = float(a[4]), float(a[5])
    out = a[6]
    rows = int(a[a.index("--rows") + 1]) if "--rows" in a else len(btops)
    heights = [float(v) for v in a[a.index("--heights") + 1].split(",")] if "--heights" in a else [72.0] * len(btops)
    os.makedirs(out, exist_ok=True)
    boxes = [json.loads(l) for l in open(os.path.join(REPO, "docs", "polish2", "boxes", board + ".jsonl"), encoding="utf-8")]
    bimg = Image.open(os.path.join(REPO, "docs", "design", "polish2-ref", board + "@1x.png")).convert("RGB")
    aimg = Image.open(shot).convert("RGB")
    # The app screenshot includes the window frame: find the client's top (the top bar's #323232 line).
    px = aimg.load()
    ctop = next(y for y in range(10, 200) if px[700, y][:3] == (0x32, 0x32, 0x32))
    aimg = aimg.crop((1, ctop, aimg.width - 1, aimg.height - 1))
    report = ["| Row | Element | Board | App | Δ (x0, y0, x1, y1) | OK |", "|---|---|---|---|---|---|"]
    bad = 0
    crops = []
    for r in range(rows):
        top_b, top_a = btops[r], atops[r]
        # This row's boxes in the board: inside the header column and the row.
        h = heights[r]
        mine = [b for b in boxes if bx <= b["x"] < bx + 380 and top_b <= b["y"] < top_b + h and b["tag"] in ("DIV", "BUTTON", "SPAN")]
        elems = []
        for b in mine:
            st = b.get("style", "") or ""
            t = b.get("text")
            ry = b["y"] - top_b
            if ry < 0 or ry >= h:
                continue
            if b["tag"] == "BUTTON":
                elems.append(("key " + t, b, False))
            elif "role" in st or "height:4px;background:#050505" in st or ("width:4px;height:" in st and "#050505" in st):
                elems.append(("well", b, False))
            elif "width:8px;height:12px" in st:
                elems.append(("pan thumb", b, False))
            elif "width:12px;height:8px" in st:
                elems.append(("fader thumb", b, False))
            elif "align-self:stretch" in st and "width:16px" in st:
                elems.append(("strip", b, False))
            elif "border:1px solid #323232" in st:
                elems.append(("badge", b, False))
            elif t and ("italic 800" in st):
                elems.append(("number", b, True))
            elif t and "600 14px" in st:
                elems.append(("name", b, True))
            elif t and "600 10px" in st:
                elems.append(("value " + t, b, True))
            elif t and "600 9px" in st:
                elems.append(("scale " + t, b, True))
        for name, b, text in elems:
            box_b = (b["x"], b["y"], b["x"] + b["w"], b["y"] + b["h"])
            rel = (box_b[0] - bx, box_b[1] - top_b, box_b[2] - bx, box_b[3] - top_b)
            pad = 3
            win_a = (rel[0] + ax - pad, rel[1] + top_a - pad, rel[2] + ax + pad, rel[3] + top_a + pad)
            if text:
                ib = ink_box(bimg, (box_b[0] - pad, box_b[1] - pad, box_b[2] + pad, box_b[3] + pad))
                ia = ink_box(aimg, win_a)
                want = None if ib is None else (ib[0] - bx, ib[1] - top_b, ib[2] - bx, ib[3] - top_b)
            else:
                want = rel
                ia = ink_box(aimg, win_a, thresh=4)
            got = None if ia is None else (ia[0] - ax, ia[1] - top_a, ia[2] - ax, ia[3] - top_a)
            if want is None or got is None:
                report.append(f"| {r + 1} | {name} | {want} | {got} | — | **no ink** |")
                bad += 1
                continue
            d = tuple(round(g - w, 1) for g, w in zip(got, want))
            # Text: the left/top edges and the height matter (glyph widths differ by font
            # rasteriser by a pixel or two); boxes: every edge.
            check = (d[0], d[1], d[3]) if text else d
            ok = all(abs(v) <= 1.0 for v in check)
            bad += not ok
            report.append(f"| {r + 1} | {name} | {tuple(round(v, 1) for v in want)} | {got} | {d} | {'yes' if ok else '**NO**'} |")
        # Crops for the picture: the row in the board and in the app.
        rb = bimg.crop((int(bx), int(top_b), int(bx) + 380, int(top_b) + 73))
        ra = aimg.crop((int(ax), int(top_a), int(ax) + 380, int(top_a) + 73))
        if h > 72:
            rb = bimg.crop((int(bx), int(top_b), int(bx) + 380, int(top_b + h) + 1)).resize((380, 73))
            ra = aimg.crop((int(ax), int(top_a), int(ax) + 380, int(top_a + h) + 1)).resize((380, 73))
        crops.append((rb, ra))
    report.append("")
    report.append("GATE 2 (" + board + "): " + ("PASS" if bad == 0 else f"FAIL ({bad} elements off by more than 1 px)"))
    text = "\n".join(report)
    open(os.path.join(out, board + ".md"), "w", encoding="utf-8", newline="\n").write(text + "\n")
    print(text)
    k = 2
    sheet = Image.new("RGB", (380 * k * 3 + 16, len(crops) * 73 * k + (len(crops) - 1) * 8), (40, 0, 0))
    for i, (rb, ra) in enumerate(crops):
        y = i * (73 * k + 8)
        sheet.paste(rb.resize((380 * k, 73 * k), Image.NEAREST), (0, y))
        sheet.paste(ra.resize((380 * k, 73 * k), Image.NEAREST), (380 * k + 8, y))
        dif = ImageChops.difference(rb, ra).point(lambda v: min(255, v * 3))
        sheet.paste(dif.resize((380 * k, 73 * k), Image.NEAREST), (2 * 380 * k + 16, y))
    sheet.save(os.path.join(out, board + ".png"))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
