"""Lime audit for polish 2 (POLISH2_PROMPT.md §9.5), the design pass's method
(docs/design/tools/lime_audit.py) with the polish 2 layout: the Overview (76..108), the ruler
(108..136), rows from 136, the E1 header (M / S at x 256..312, the fader at 346..350 in the
header), the status bar along the bottom (no lime allowed there).

usage: python lime_audit_p2.py <dir with lime_scenes_p2.ps1 shots> [report.md]

Allowed: the switched-on states (selected view icon, the display's indicators, lit M/S glyphs,
the loop brace and its glow), the Overview's loop line **only while the loop is on** (shots
whose name contains "loop_off" get no allowance there), and the track colour (strips, clips and
notes in the lanes and in the Overview, the volume meters' cool-to-hot fill, the browser's kind
markers, the inspector's macro graphs). Everything else is a violation.
"""
import os
import sys

from PIL import Image

OX, OY = 1, 51


def lime(p):
    r, g, b = p[:3]
    return g >= 120 and r >= 0.70 * g and r <= 1.08 * g and b <= 0.62 * g and g - b >= 60


def regions(name, right):
    hx = 1060 if right else 241          # the header's left edge
    lx0, lx1 = (241, 1060) if right else (621, 1440)   # the lanes
    if "browser_right" in name:          # headers left at 0, lanes 380..1199, browser 1200..
        hx, lx0, lx1 = 0, 380, 1199
    allowed = [
        ("selected view icon", 16, 18, 56, 58),
        ("display indicators", 399 + 17, 10, 399 + 17 + 72, 66),
        ("loop brace + glow", lx0, 106, lx1, 124),
        ("Overview clips (track colour)", lx0, 79, lx1, 107),
        ("colour strips (+ spill)", hx, 136, hx + 40, 870),
        ("colour strips by the clips", hx + 340, 136, hx + 380, 870),
        ("M/S keys", hx + 256, 136, hx + 312, 870),
        ("volume meters", hx + 340, 136, hx + 356, 870),
        ("lanes: clips and notes (track colour)", lx0, 136, lx1, 690),
        ("inspector macro graphs (lane colour)", 840, 690, 1440, 896),
    ]
    if "loop_off" not in name:
        allowed.append(("Overview loop line (loop on)", lx0, 76, lx1, 79))
    bx = 1200 if "browser_right" in name else 0
    allowed.append(("browser list: kind markers (lane colour)", bx, 140, bx + 240, 896))
    return allowed


def audit(path):
    name = os.path.basename(path)
    im = Image.open(path).convert("RGB")
    w, h = im.size
    px = im.load()
    allow = regions(name, name.startswith("right"))
    hits, bad = {}, []
    for y in range(OY, h):
        for x in range(OX, w):
            if not lime(px[x, y]):
                continue
            cx, cy = x - OX, y - OY
            n = next((n for n, x0, y0, x1, y1 in allow if x0 <= cx < x1 and y0 <= cy < y1), None)
            if n:
                hits[n] = hits.get(n, 0) + 1
            else:
                bad.append((cx, cy))
    return hits, bad


def main():
    d = sys.argv[1]
    out = sys.argv[2] if len(sys.argv) > 2 else None
    lines = ["| Screenshot | Lime px in allowed places | Outside (violations) |", "|---|---|---|"]
    total_bad = 0
    for f in sorted(os.listdir(d)):
        if not f.endswith(".png"):
            continue
        hits, bad = audit(os.path.join(d, f))
        total_bad += len(bad)
        where = ""
        if bad:
            xs, ys = [p[0] for p in bad], [p[1] for p in bad]
            where = f" in ({min(xs)}, {min(ys)})–({max(xs)}, {max(ys)})"
        lines.append(f"| {f} | " + ", ".join(f"{k}: {v}" for k, v in sorted(hits.items())) + f" | {len(bad)}{where} |")
    lines.append("")
    lines.append(f"Violations: {total_bad}")
    text = "\n".join(lines)
    print(text)
    if out:
        open(out, "w", encoding="utf-8", newline="\n").write(text + "\n")
    sys.exit(1 if total_bad else 0)


if __name__ == "__main__":
    main()
