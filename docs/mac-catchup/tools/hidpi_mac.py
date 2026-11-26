#!/usr/bin/env python3
"""MAC_CATCHUP §4.7: the E1 boards' mockup match at 2× backing.

Neither display here is Retina (both run at backing 1.0), so the second display (XB271HU,
CG display 1) is switched for this session only to its HiDPI mode (1280 x 720 pt at
2560 x 1440 px, 60 Hz, ioModeID 7) and **always restored** to its own mode afterwards (the
`finally` below; `dispmode set 1 18` by hand if this script is killed). The app runs on that
display at backing 2.0, and mockup_match_mac.py compares each header element of the boards'
@2x PNGs within 2 device px (1 logical px).

usage: hidpi_mac.py <out dir>
"""
import os
import pathlib
import re
import subprocess
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from mclib import *  # noqa: E402,F401,F403

out = pathlib.Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
DISP, HIDPI = "1", "7"
dm = BIN / "dispmode"
if not dm.exists():
    subprocess.run(["swiftc", "-O", str(REPO / "docs/mac-catchup/tools/dispmode.swift"), "-o", str(dm)], check=True, capture_output=True)
winctl = BIN / "winctl"
if not winctl.exists():
    subprocess.run(["swiftc", "-O", str(REPO / "docs/engine/tools/macos/winctl.swift"), "-o", str(winctl)], check=True, capture_output=True)
orig = re.search(r"ioModeID (\d+)", subprocess.run([str(dm), "current", DISP], capture_output=True, text=True).stdout).group(1)
print("display", DISP, "mode before:", orig)
env_pil = dict(os.environ, PYTHONPATH=str(REPO / "docs/mac-catchup/tools/pilshim"))
QUIET = {"CHIPDAW_AUDIO": "0", "CHIPDAW_FROZEN_METERS": "1"}
results = []
try:
    print(subprocess.run([str(dm), "set", DISP, HIDPI], capture_output=True, text=True).stdout)
    time.sleep(4)
    # The HiDPI display's frame in global points (it sits above the main display).
    info = subprocess.run([str(BIN / "display_info")] if (BIN / "display_info").exists() else ["true"], capture_output=True, text=True).stdout
    print(info.strip()[:400])
    runs = [
        ("E1-header", REPO / "docs/polish2/songs/mockup.chipdaw", "104,177,250,323", "104,177,250,323", [], "0"),
        ("E1-row-heights", REPO / "docs/polish2/songs/mockup_rows.chipdaw", "28,101,246", "104,177,322", ["--heights", "72,144,240"], "0"),
        ("E1-overview", REPO / "docs/polish2/songs/mockup.chipdaw", "136,209,282,355", "136,209,282,355", [], "1"),
    ]
    for board, song, btops, atops, extra, overview in runs:
        a = App("hidpi", song, env=dict(QUIET, CHIPDAW_OVERVIEW=overview))
        # Onto the HiDPI display: global (40, -680) is inside its 1280 x 720 pt frame above the main display.
        subprocess.run([str(winctl), str(a.p.pid), "move", "40", "-690"], capture_output=True)
        time.sleep(2.5)
        i = a.drv("info")
        print(board, i.strip())
        a.drv("move", 1000, 600)
        shot = a.shot(f"hidpi_{board}")
        padded = out / f"{board}@2x-app.png"
        img("pad", shot, padded, 1, 0)
        r = subprocess.run([sys.executable, str(REPO / "docs/mac-catchup/tools/mockup_match_mac.py"), str(padded), board, btops, atops, "0", "241", str(out), "--scale", "2", "--tag", board + "@2x"] + extra,
                           env=env_pil, capture_output=True, text=True)
        last = [l for l in r.stdout.splitlines() if l.startswith("GATE 2")]
        bad = [l for l in r.stdout.splitlines() if "**NO**" in l]
        results.append((board, i.strip(), last[0] if last else r.stderr[-400:], bad))
        if board == "E1-header":
            img("crop", shot, out / "mac-header-e1@2x.png", 0, 64 + 76 * 2, 2 * 620, 2 * 4 * 73)
        a.quit()
finally:
    print(subprocess.run([str(dm), "set", DISP, orig], capture_output=True, text=True).stdout)
    now = subprocess.run([str(dm), "current", DISP], capture_output=True, text=True).stdout.strip()
    print("display", DISP, "restored:", now)
for board, info, gate, bad in results:
    print(board, "|", info, "|", gate)
    for b in bad:
        print("   ", b)
