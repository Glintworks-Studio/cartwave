#!/usr/bin/env python3
"""MAC_CATCHUP §4.6: Interface scale 100 / 125 / 150 % on both displays (backing 1.0 here):
the window's size and display, and a screenshot of each, composed into one image."""
import pathlib
import subprocess
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from mclib import *  # noqa: E402,F401,F403

winctl = BIN / "winctl"
out = []
for disp, pos in (("320", None), ("60", ("100", "-1400"))):
    for sc in (100, 125, 150):
        a = App("scale", REPO / "docs/polish2/songs/mockup.chipdaw", env={"CARTWAVE_AUDIO": "0"}, settings={"ui_scale": sc})
        if pos:
            subprocess.run([str(winctl), str(a.p.pid), "move", *pos], capture_output=True)
            time.sleep(2)
        info = a.drv("info").strip()
        shot = a.shot(f"scale_{disp}_{sc}")
        size = img("size", shot)
        print(f"{disp} Hz display, {sc}%: {info} | capture {size}")
        c = WORK / f"scale_c_{disp}_{sc}.png"
        img("crop", shot, c, 0, 0, int(int(size.split()[0]) * 0.55), int(int(size.split()[1]) * 0.45))
        l = WORK / f"scale_l_{disp}_{sc}.png"
        img("label", c, l, f"{disp} Hz display, interface scale {sc}%")
        out.append(l)
        a.quit()
img("stack", "v", WORK / "scale-check.png", *out)
