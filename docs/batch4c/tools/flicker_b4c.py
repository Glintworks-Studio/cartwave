"""K1 (Batch 4c, from batch 4b's flicker.py): counts the frames of menu_flicker_b4c.ps1's capture
where the menu bar flashes. Besides 4b's light test it flags a *misdrawn* bar: the menu labels'
part of the bar (its left 360 px, which a right/bottom/corner drag doesn't move) differing from
the steady bar (the most common frame there) by more than 12 levels on average, e.g. the text
missing or a light patch too small for the light test.

The capture is a band from the title bar's middle to the client area's top: its lower part is the
menu bar. The bar's background (the median of its rows, so its text doesn't count) is dark
(muda's dark menu bar, about #2B2B2B or darker); a frame where that median turns light (above
#808080) is a flash. Also reported: frames where the bar is missing or misdrawn (the band's
lower rows equal the title bar's colour), which a resize could cause.

usage: python flicker.py <dir>   (writes flicker.json there and prints a summary)"""
import glob
import json
import os
import statistics
import sys

from PIL import Image

d = sys.argv[1]
frames = sorted(glob.glob(os.path.join(d, "f_*.png")))
res = []
for f in frames:
    im = Image.open(f).convert("L")
    w, h = im.size
    px = im.load()
    # The menu bar: the lowest 18 rows (at 100 %) of the band.
    rows = range(max(0, h - 18), h - 2)
    vals = [px[x, y] for y in rows for x in range(0, w, 2)]
    med = statistics.median(vals)
    light = sum(v > 0x80 for v in vals) / len(vals)
    res.append({"frame": os.path.basename(f), "median": med, "light_share": round(light, 3)})
# The steady bar: the most common left-part image.
from collections import Counter
lefts = []
for f in frames:
    im = Image.open(f).convert("L")
    w, h = im.size
    box = im.crop((0, max(0, h - 18), min(360, w), h - 2))
    lefts.append(box.tobytes())
ref = Counter(lefts).most_common(1)[0][0] if lefts else b""
for r, lb in zip(res, lefts):
    r["left_diff"] = round(sum(abs(a - b) for a, b in zip(lb, ref)) / max(1, len(ref)), 2)
# A flash: the bar's median turns light, or a light patch covers a fifth of it; misdrawn: the
# labels' part differs from the steady bar.
flashes = [r for r in res if r["median"] > 0x80 or r["light_share"] > 0.2]
misdrawn = [r for r in res if r not in flashes and r["left_diff"] > 12]
summary = {"frames": len(res), "flash_frames": len(flashes), "first_flashes": [r["frame"] for r in flashes[:10]],
           "misdrawn_frames": len(misdrawn), "first_misdrawn": [r["frame"] for r in misdrawn[:10]],
           "max_left_diff": max((r["left_diff"] for r in res), default=0),
           "median_of_medians": statistics.median([r["median"] for r in res]) if res else None}
json.dump({"summary": summary, "frames": res}, open(os.path.join(d, "flicker.json"), "w"), indent=1)
print(json.dumps(summary))
