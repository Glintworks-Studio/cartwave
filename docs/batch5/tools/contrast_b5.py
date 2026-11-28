"""Batch 5: contrast of the new labels (BATCH5_PROMPT.md §8.6: >= 4.5 : 1), contrast_pb.py's method
with the automation lane headers' regions (lime_scenes_b5.ps1's scenes).

From the Presets batch: contrast of the new text (PRESETS_PROMPT.md §8.7): >= 4.5 : 1, the locked device's labels too.

Each region (client px at 100 %, in a lime_scenes_pb.ps1 screenshot) holds one kind of text; its
background is the region's most common colour and its ink the pixel farthest from it in
luminance (light text on dark, or dark text on the selected row's light grey). WCAG ratio.

usage: python contrast_pb.py <scene dir> [report.md]
"""
import os
import sys
from collections import Counter

from PIL import Image

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "polish2", "tools"))
from diff import client_top  # noqa: E402

# scene: [(what, x0, y0, x1, y1)] (device at (224, 552); browser from y 76)
REGIONS = {
    "auto_mode": [
        ("lane header label (Mixer > Volume)", 432, 214, 600, 230),
        ("lane header value", 432, 231, 520, 246),
    ],
    "auto_locked": [
        ("locked lane header label (Chip Synth > Duty)", 432, 214, 600, 230),
        ("locked lane header value", 432, 231, 520, 246),
    ],
    "auto_overridden": [
        ("overridden lane header value", 432, 231, 520, 246),
    ],
}


def lum(c):
    def ch(v):
        v /= 255.0
        return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    return 0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])


def ratio(img, top, box):
    x0, y0, x1, y1 = box
    px = [img.getpixel((x + 1, y + top))[:3] for y in range(y0, y1) for x in range(x0, x1)]
    bg = Counter(px).most_common(1)[0][0]
    lb = lum(bg)
    ink = max(px, key=lambda c: abs(lum(c) - lb))
    li = lum(ink)
    hi, lo = max(li, lb), min(li, lb)
    return (hi + 0.05) / (lo + 0.05), bg, ink


def main():
    src = sys.argv[1]
    rep = sys.argv[2] if len(sys.argv) > 2 else os.path.join(src, "contrast_report.md")
    rows, worst = [], 99.0
    for scene, regions in REGIONS.items():
        path = os.path.join(src, scene + ".png")
        if not os.path.exists(path):
            rows.append(f"| {scene} | (no screenshot) | – | – | – | **?** |")
            continue
        img = Image.open(path).convert("RGB")
        top = client_top(img, 100)
        for what, *box in regions:
            r, bg, ink = ratio(img, top, box)
            worst = min(worst, r)
            rows.append(f"| {scene} | {what} | #{bg[0]:02X}{bg[1]:02X}{bg[2]:02X} | #{ink[0]:02X}{ink[1]:02X}{ink[2]:02X} | {r:.2f} | {'yes' if r >= 4.5 else '**no**'} |")
    text = ("# Contrast of the new text (gate 7)\n\n`docs/presets/tools/contrast_pb.py` on the lime scenes: per region, the ink against the region's most common colour (WCAG).\n\n"
            f"**Lowest: {worst:.2f} : 1** ({'PASS' if worst >= 4.5 else 'FAIL'}: every region >= 4.5 : 1)\n\n"
            "| Scene | Text | Background | Ink | Ratio | >= 4.5 |\n|---|---|---|---|---|---|\n" + "\n".join(rows) + "\n")
    open(rep, "w", encoding="utf-8").write(text)
    print(text)


if __name__ == "__main__":
    main()
