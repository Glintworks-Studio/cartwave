#!/usr/bin/env python3
"""MAC_CATCHUP §5: the per-track dB fader (Polish 2) under stress, automation-free. The demo
plays at a 64-frame buffer while every track's fader is dragged in turn (real mouse events) for
the whole run; the app's audio log (CARTWAVE_AUDIOLOG) gives CoreAudio overloads, late callbacks,
the largest callback gap and the audio thread's allocation violations (NoAllocGuard).

usage: fader_stress.py <device name> <secs> <out.csv>
"""
import pathlib
import re
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from mclib import *  # noqa: E402,F401,F403

device, secs, out = sys.argv[1], int(sys.argv[2]), pathlib.Path(sys.argv[3])
env = {"CARTWAVE_AUDIOLOG": str(out), "CARTWAVE_QUIT_AFTER": str(secs + 4), "CARTWAVE_BUFFER": "64", "CARTWAVE_AUTOPLAY": "1"}
os.environ["MC_DEVICE"] = device
a = App("fader", env=env)
FADER_X = 241 + 380 - 132 + 8 + 56 + 16 + 20   # the column B well's centre (header x 241, 380 wide)
t0 = time.time()
moves = 0
while time.time() - t0 < secs - 2:
    for lane in range(8):
        top = ROW0 + PITCH * lane
        thumb = top + 8 + round((1 - (0 + 48) / 54) * 40)   # 0 dB on a 72 px row
        # A drag down and back up, ~0.6 s per track (press sets the value).
        a.seq(("drag", FADER_X, thumb, FADER_X, thumb + 18, "", 12, 12), ("drag", FADER_X, thumb + 18, FADER_X, thumb - 4, "", 12, 12))
        moves += 2
        if time.time() - t0 >= secs - 2:
            break
deadline = time.time() + 30
while a.p.poll() is None and time.time() < deadline:
    time.sleep(0.5)
if a.p.poll() is None:
    a.quit()
text = out.read_text(errors="replace") if out.exists() else ""
summary = [l for l in text.splitlines() if l.startswith("#")]
print(f"{moves} fader drags over {secs} s on {device}")
print("\n".join(l for l in summary if re.search(r"summary|callbacks|underruns|alloc|overloads|process", l)))
