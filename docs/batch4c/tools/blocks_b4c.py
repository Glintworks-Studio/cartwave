"""K1 (Batch 4c): frames of a menu_flicker_b4c.ps1 capture with a solid light block (6 x 6 px all
above #909090) anywhere in the band: the title bar, the menu bar and (with -Below) the top of the
client area. Text never fills such a block; a flash of the system's light bar, or an unpainted
light area, does. Blocks present in most frames are the steady picture (the app icon, light
controls in the app's top bar) and don't count: a frame is flagged for a block the steady picture
doesn't have.

usage: python blocks_b4c.py <dir>   (prints the count and the first hits; writes blocks.json)"""
import glob
import json
import os
import sys

from PIL import Image

d = sys.argv[1]
frames = sorted(glob.glob(os.path.join(d, "f_*.png")))
per = []
for f in frames:
    im = Image.open(f).convert("L")
    w, h = im.size
    px = im.load()
    found = set()
    for x in range(0, w - 6, 3):
        for y in range(0, h - 6, 3):
            if all(px[x + i, y + j] > 0x90 for i in range(6) for j in range(6)):
                found.add((x, y))
    per.append((os.path.basename(f), found))
from collections import Counter
count = Counter(b for _, fs in per for b in fs)
steady = {b for b, n in count.items() if n > len(per) / 2}
hits = [{"frame": f, "at": sorted(fs - steady)[:3]} for f, fs in per if fs - steady]
json.dump({"frames": len(frames), "light_block_frames": len(hits), "hits": hits}, open(os.path.join(d, "blocks.json"), "w"), indent=1)
print(json.dumps({"frames": len(frames), "light_block_frames": len(hits), "first": hits[:5]}))
