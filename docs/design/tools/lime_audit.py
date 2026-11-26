"""Lime audit (DESIGN_PASS_PROMPT.md §4.3).

usage: python lime_audit.py <dir with lime_scenes.ps1 shots> [report.md]

A pixel is "lime" when it is close in hue to the accent #C7D74E (yellow-green,
bright, blue low). Every lime pixel must fall in an allowed place:

- the switched-on states (VISUAL-SPEC.md §1): the selected view icon, the
  display's indicators cell, lit M/S glyphs, the loop brace (and its glow);
- the track colour, which is a user colour, not the accent: the colour strips,
  clips and their notes, the volume meters' cool-to-hot fill (its 44 px stop is
  #C7D74E by spec), the browser's channel-kind markers and the inspector's
  macro graphs (drawn in the lane colour). The demo's pulse colour #C9D24A is
  nearly the accent, so these regions are listed rather than told apart by hue.

Anything else is a violation: the playhead, focus, dial arcs, buttons, pills,
text. Coordinates are window-screenshot px (client + 1, + 51).
"""
import os
import sys

from PIL import Image

OX, OY = 1, 51  # window border, title + menu bar


def lime(p):
    r, g, b = p[:3]
    return g >= 120 and r >= 0.70 * g and r <= 1.08 * g and b <= 0.62 * g and g - b >= 60


def regions(browser):
    """Allowed regions (client px): (name, x0, y0, x1, y1)."""
    bx = 241 if browser else 0
    hx = bx  # the header column's left edge
    allowed = [
        ("selected view icon", 16, 18, 56, 58),
        ("display indicators", 399 + 17, 10, 399 + 17 + 72, 66),
        ("loop brace + glow", hx + 380, 76, 1440, 92),
        ("colour strips", hx, 104, hx + 16, 920),
        ("M/S keys", hx + 304, 104, hx + 376, 920),
        ("volume meters", hx + 240, 104, hx + 324, 920),
        ("lanes: clips and notes (track colour)", hx + 380, 104, 1440, 690),
        ("inspector macro graphs (lane colour)", 840, 690, 1440, 920),
    ]
    if browser:
        allowed.append(("browser list: kind markers (lane colour)", 0, 140, 240, 920))
    return allowed


def audit(path):
    im = Image.open(path).convert("RGB")
    w, h = im.size
    px = im.load()
    # lime_scenes.ps1 shoots with the browser open (a fresh settings.json).
    browser = True
    allow = regions(browser)
    hits, bad = {}, []
    for y in range(OY, h):
        for x in range(OX, w):
            if not lime(px[x, y]):
                continue
            cx, cy = x - OX, y - OY
            name = next((n for n, x0, y0, x1, y1 in allow if x0 <= cx < x1 and y0 <= cy < y1), None)
            if name:
                hits[name] = hits.get(name, 0) + 1
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
