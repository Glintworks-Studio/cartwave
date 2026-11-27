"""Batch 4b gate 7: the lime audit (VISUAL-SPEC §13: lime only for toggle on-states and the
centre display; never selection, meters, instrument displays or track identity).

The scenes (lime_scenes_b4b.ps1) use projects whose track colours hold no lime, so every lime
pixel is the interface's. Allowed, by region (client px at 100 %, 1440 wide):
  the view keys       x 10..112,  y 8..68   (the shown view's key is lit: a toggle on-state)
  the centre display  x 396..964, y 6..72   (its indicators: play, loop, the values' accents)
  the loop            the Overview's loop line (y 74..82) and the ruler's brace (y 104..136),
                      right of the headers, while the loop is on (the fixtures' loop is on)
Everything else is a violation. The same lime test as the design pass and polish 2.

usage: python lime_audit_b4b.py <scene dir> [report.md]"""
import os
import sys

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "polish2", "tools"))
from diff import client_top  # noqa: E402

sys.stdout.reconfigure(encoding="utf-8")


def lime(p):
    r, g, b = p[:3]
    return g >= 120 and r >= 0.70 * g and r <= 1.08 * g and b <= 0.62 * g and g - b >= 60


def allowed(name, x, y):
    if 10 <= x <= 112 and 8 <= y <= 68:
        return "view keys"
    if 396 <= x <= 964 and 6 <= y <= 72:
        return "display"
    if 74 <= y <= 82 and x >= 380:
        return "Overview loop line (loop on)"
    if 104 <= y <= 136 and x >= 380:
        return "loop brace (loop on)"
    return None


def main():
    src = sys.argv[1]
    rep = sys.argv[2] if len(sys.argv) > 2 else os.path.join(src, "lime_report.md")
    lines = ["# Lime audit, Batch 4b (gate 7)", "", "| scene | lime px allowed | violations | where |", "|---|---|---|---|"]
    total = 0
    for f in sorted(os.listdir(src)):
        if not f.endswith(".png") or f.startswith("_"):
            continue
        im = Image.open(os.path.join(src, f)).convert("RGB")
        t = client_top(im, 100)
        px = im.load()
        ok = bad = 0
        where = {}
        for y in range(t, im.height - 1):
            for x in range(1, min(im.width - 1, 1441)):
                if lime(px[x, y]):
                    cx, cy = x - 1, y - t
                    if allowed(f, cx, cy):
                        ok += 1
                    else:
                        bad += 1
                        key = (cx // 40 * 40, cy // 40 * 40)
                        where[key] = where.get(key, 0) + 1
        total += bad
        top = ", ".join(f"({k[0]},{k[1]}):{v}" for k, v in sorted(where.items(), key=lambda kv: -kv[1])[:6])
        lines.append(f"| {f[:-4]} | {ok} | {bad} | {top or '-'} |")
    lines += ["", f"**Violations: {total}**"]
    text = "\n".join(lines) + "\n"
    open(rep, "w", encoding="utf-8").write(text)
    print(text)


if __name__ == "__main__":
    main()
