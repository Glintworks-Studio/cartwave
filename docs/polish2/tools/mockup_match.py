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
import math
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
    W = float(a[a.index("--width") + 1]) if "--width" in a else 380.0
    tag = a[a.index("--tag") + 1] if "--tag" in a else board
    heights = [float(v) for v in a[a.index("--heights") + 1].split(",")] if "--heights" in a else [72.0] * len(btops)
    os.makedirs(out, exist_ok=True)
    boxes = [json.loads(l) for l in open(os.path.join(REPO, "docs", "polish2", "boxes", board + ".jsonl"), encoding="utf-8")]
    bimg = Image.open(os.path.join(REPO, "docs", "design", "polish2-ref", board + "@1x.png")).convert("RGB")
    aimg = Image.open(shot).convert("RGB")
    # The app screenshot includes the window frame: find the client's top (the top bar's #323232 line).
    px = aimg.load()
    ctop = next(y for y in range(10, 200) if px[700, y][:3] == (0x32, 0x32, 0x32))
    aimg = aimg.crop((1, ctop, aimg.width - 1, aimg.height - 1))
    report = ["| Row | Element | Board box (row-relative) | Best match in the app | Offset (px) | OK |", "|---|---|---|---|---|---|"]
    bad = 0
    crops = []
    for r in range(rows):
        top_b, top_a = btops[r], atops[r]
        # This row's boxes in the board: inside the header column and the row.
        h = heights[r]
        mine = [b for b in boxes if bx <= b["x"] < bx + W and top_b <= b["y"] < top_b + h and b["tag"] in ("DIV", "BUTTON", "SPAN")]
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
            if name == "fader thumb":
                # The boards disagree on the thumb's place (P2-D8): checked against
                # §3's formula, the thumb's centre at round((1 - (dB+48)/54) * length).
                db = next((float(e[0].split()[1].replace("\u2212", "-")) for e in elems if e[0].startswith("value ") and "dB" in e[0]), None)
                L = h - 32
                want_cy = 8 + round((1 - (db + 48) / 54) * L) if db is not None else None
                col = aimg.crop((int(ax + rel[0] + 6), int(top_a), int(ax + rel[0] + 7), int(top_a + h)))
                ys = [y for y in range(col.height) if col.getpixel((0, y))[0] >= 0x9A]   # the thumb's #A0A0A0 inner top line
                got_cy = ys[0] - 1 + 4 if ys else None
                ok = want_cy is not None and got_cy is not None and abs(got_cy - want_cy) <= 1
                bad += not ok
                report.append(f"| {r + 1} | fader thumb (centre y, §3 formula) | {want_cy} | {got_cy} | {None if got_cy is None or want_cy is None else got_cy - want_cy} | {'yes' if ok else '**NO**'} |")
                continue
            pad = 3
            if name == "strip":
                # A solid block (lit with a glow on the board, unlit in the frozen app):
                # its edges along the row's middle, and its height.
                px = aimg.load()
                yc = int(top_a + h / 2)
                # Inside the header's own column and the row (neighbours: the browser's
                # divider, the next row's strip).
                xs = [x for x in range(int(ax), int(ax + W)) if max(abs(px[x, yc][i] - BG[i]) for i in range(3)) > 3 and abs(x - (ax + rel[0])) < 24]
                # The run that contains the strip's middle.
                mid = int(ax + rel[0] + 8)
                lo = mid
                while lo - 1 in xs:
                    lo -= 1
                hi = mid
                while hi + 1 in xs:
                    hi += 1
                col = [y for y in range(int(top_a), int(top_a + h)) if max(abs(px[mid, y][i] - BG[i]) for i in range(3)) > 3]
                got = (lo - ax, min(col) - top_a, hi + 1 - ax, max(col) + 1 - top_a) if col and mid in xs else None
                want = (rel[0], 0.0, rel[2], h)
                d = None if got is None else tuple(round(g - w, 1) for g, w in zip(got, want))
                # x edges within 1 px; it spans the row (plus its hairline below, drawn #1A1A1A).
                ok = d is not None and abs(d[0]) <= 1 and abs(d[2]) <= 1 and abs(d[1]) <= 1
                bad += not ok
                report.append(f"| {r + 1} | strip (edges at the row's middle) | {want} | {got} | {d} | {'yes' if ok else '**NO**'} |")
                continue
            if name == "well" and b["w"] == 4:
                # The fader's well (board meters and thumbs differ by design): its edges.
                L = h - 32
                x0, y0 = ax + rel[0], top_a + rel[1]
                px = aimg.load()
                dark = lambda x, y: max(px[int(x), int(y)]) <= 6
                cols = [x for x in range(int(x0) - 3, int(x0) + 7) if dark(x, y0 + L - 4)]
                rows = [y for y in range(int(y0) - 3, int(y0) + int(L) + 3) if dark(x0 + 1, y)]
                got = (min(cols) - ax, min(rows) - top_a, max(cols) + 1 - ax, max(rows) + 1 - top_a) if cols and rows else None
                want = (rel[0], 8.0, rel[0] + 4, 8.0 + L)
                d = None if got is None else tuple(round(g - w, 1) for g, w in zip(got, want))
                ok = d is not None and all(abs(v) <= 1 for v in d)
                bad += not ok
                report.append(f"| {r + 1} | fader well (x, from y 8, length h − 32) | {want} | {got} | {d} | {'yes' if ok else '**NO**'} |")
                continue

            def mask(img, x0, y0):
                w, hh = int(round(rel[2] - rel[0])) + 2 * pad, int(round(rel[3] - rel[1])) + 2 * pad
                # Round half up (Python's round() goes to even: 142.5 and 143.5 would
                # land on the same row and the offset search would skip one).
                ox, oy = int(math.floor(x0 + 0.5)) - pad, int(math.floor(y0 + 0.5)) - pad
                px = img.load()
                # The crop's own background: its most common colour.
                from collections import Counter
                cnt = Counter(px[X, Y] for Y in range(max(0, oy), min(img.height, oy + hh)) for X in range(max(0, ox), min(img.width, ox + w)))
                bgc = cnt.most_common(1)[0][0] if cnt else BG
                out = []
                for y in range(hh):
                    row = []
                    for x in range(w):
                        X, Y = ox + x, oy + y
                        if not (0 <= X < img.width and 0 <= Y < img.height):
                            row.append(None)
                        else:
                            p = px[X, Y]
                            row.append(max(abs(p[0] - bgc[0]), abs(p[1] - bgc[1]), abs(p[2] - bgc[2])) > 3)
                    out.append(row)
                return out
            mb = mask(bimg, box_b[0], box_b[1])
            # Only where the board has pixels and inside the header (its own column).
            def keep(x, y):
                return mb[y][x] is not None and 0 <= rel[0] - pad + x < W
            best = None
            for dy in range(-3, 4):
                for dx in range(-3, 4):
                    ma = mask(aimg, ax + rel[0] + dx, top_a + rel[1] + dy)
                    inter = union = 0
                    for y in range(len(mb)):
                        for x in range(len(mb[0])):
                            if not keep(x, y) or ma[y][x] is None:
                                continue
                            inter += mb[y][x] and ma[y][x]
                            union += mb[y][x] or ma[y][x]
                    iou = inter / union if union else 1.0
                    if best is None or iou > best[0] + 1e-9 or (abs(iou - best[0]) < 1e-9 and abs(dx) + abs(dy) < abs(best[1]) + abs(best[2])):
                        best = (iou, dx, dy)
            iou, dx, dy = best
            ok = abs(dx) <= 1 and abs(dy) <= 1 and iou >= 0.5
            bad += not ok
            report.append(f"| {r + 1} | {name} | {tuple(round(v, 1) for v in rel)} | offset ({dx}, {dy}), overlap {iou:.2f} | ({dx}, {dy}) | {'yes' if ok else '**NO**'} |")
        # Crops for the picture: the row in the board and in the app.
        rb = bimg.crop((int(bx), int(top_b), int(bx + W), int(top_b) + 73))
        ra = aimg.crop((int(ax), int(top_a), int(ax + W), int(top_a) + 73))
        if h > 72:
            rb = bimg.crop((int(bx), int(top_b), int(bx + W), int(top_b + h) + 1)).resize((int(W), 73))
            ra = aimg.crop((int(ax), int(top_a), int(ax + W), int(top_a + h) + 1)).resize((int(W), 73))
        crops.append((rb, ra))
    report.append("")
    report.append("GATE 2 (" + tag + "): " + ("PASS" if bad == 0 else f"FAIL ({bad} elements off by more than 1 px)"))
    text = "\n".join(report)
    open(os.path.join(out, tag + ".md"), "w", encoding="utf-8", newline="\n").write(text + "\n")
    print(text)
    k = 2
    cw = int(W)
    sheet = Image.new("RGB", (cw * k * 3 + 16, len(crops) * 73 * k + (len(crops) - 1) * 8), (40, 0, 0))
    for i, (rb, ra) in enumerate(crops):
        y = i * (73 * k + 8)
        sheet.paste(rb.resize((cw * k, 73 * k), Image.NEAREST), (0, y))
        sheet.paste(ra.resize((cw * k, 73 * k), Image.NEAREST), (cw * k + 8, y))
        dif = ImageChops.difference(rb, ra).point(lambda v: min(255, v * 3))
        sheet.paste(dif.resize((cw * k, 73 * k), Image.NEAREST), (2 * cw * k + 16, y))
    sheet.save(os.path.join(out, tag + ".png"))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
