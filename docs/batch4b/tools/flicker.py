"""K1: counts the frames of menu_flicker.ps1's capture where the menu bar flashes.

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
# A flash: the bar's median turns light, or a light patch covers a fifth of it.
flashes = [r for r in res if r["median"] > 0x80 or r["light_share"] > 0.2]
summary = {"frames": len(res), "flash_frames": len(flashes), "first_flashes": [r["frame"] for r in flashes[:10]],
           "median_of_medians": statistics.median([r["median"] for r in res]) if res else None}
json.dump({"summary": summary, "frames": res}, open(os.path.join(d, "flicker.json"), "w"), indent=1)
print(json.dumps(summary))
