#!/usr/bin/env python3
"""MAC_CATCHUP §4.7: the polish 2 lime audit on macOS scenes. The scenes of
docs/polish2/tools/lime_scenes_p2.ps1 (same clicks, the demo song), captured with real input,
converted to sRGB (the newCtx draw colour-matches from the display's profile) and padded to the
Windows harness's frame offset (1, 51) so docs/polish2/tools/lime_audit_p2.py runs unchanged
through the Pillow stand-in (docs/mac-catchup/tools/pilshim).

usage: lime_mac.py <out dir> [report.md]
"""
import os
import pathlib
import subprocess
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from mclib import *  # noqa: E402,F401,F403

out = pathlib.Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
TB = 32


def shot(a, name):
    raw = a.shot(f"lime_{name}")
    img("pad", raw, out / f"{name}.png", 1, 51 - TB)


a = App("lime")
a.seq(("click", 1300, 820), ("move", 1300, 905), ("sleep", .4))
shot(a, "stopped")
a.seq(("key", "space"), ("sleep", 1.5))
shot(a, "playing")
a.seq(("click", 293, 38), ("sleep", .4))
shot(a, "loop_off")
a.seq(("click", 293, 38), ("sleep", .4))
shot(a, "loop_on")
a.seq(("click", 241 + 300, 136 + 2 * 73 + 22), ("sleep", .7))
shot(a, "solo")
a.seq(("click", 241 + 268, 136 + 3 * 73 + 22), ("sleep", .7))
shot(a, "mute")
a.seq(("key", "space"), ("sleep", .3), ("click", 700, 172), ("sleep", .5))
shot(a, "inspector")
a.seq(("menu", "File", "Project Settings…"), ("sleep", .5))
shot(a, "project_settings")
a.quit()
a = App("lime2", settings={"headers_right": True, "strip_by_clips": True})
a.seq(("click", 700, 820), ("key", "space"), ("sleep", 1.5))
shot(a, "right_playing")
a.quit()
a = App("lime3", settings={"browser_right": True})
a.seq(("click", 900, 820), ("key", "space"), ("sleep", 1.5))
shot(a, "browser_right_playing")
a.quit()
env = dict(os.environ, PYTHONPATH=str(REPO / "docs/mac-catchup/tools/pilshim"))
r = subprocess.run([sys.executable, str(REPO / "docs/polish2/tools/lime_audit_p2.py"), str(out)] + sys.argv[2:3], env=env, capture_output=True, text=True)
print(r.stdout[-3000:], r.stderr[-2000:])
