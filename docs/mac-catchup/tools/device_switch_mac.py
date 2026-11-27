#!/usr/bin/env python3
"""MAC_CATCHUP §5: a device switch while playing, built-in <-> USB, through Settings ▸ Audio (the
app's own path, Engine::reconfigure): the system's default output is never changed. The app's
per-second audio log gives the gap (largest callback gap in the second of each switch), the
device name per second and any overloads; real mouse clicks on the Settings buttons.

usage: device_switch_mac.py <out.csv> [switches=4]
"""
import csv
import pathlib
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from mclib import *  # noqa: E402,F401,F403

out = pathlib.Path(sys.argv[1])
n = int(sys.argv[2]) if len(sys.argv) > 2 else 4
secs = 15 + n * 12
os.environ["MC_DEVICE"] = "Mac mini Speakers"
a = App("devsw", env={"CARTWAVE_AUDIOLOG": str(out), "CARTWAVE_QUIT_AFTER": str(secs), "CARTWAVE_AUTOPLAY": "1"})
# Settings ▸ Audio: find the device buttons by screenshot positions (content coordinates).
BUTTONS = {"M2": (611, 313), "Mac mini Speakers": (664, 341)}
t0 = time.time()
log = []
time.sleep(8)
for i in range(n):
    to = "M2" if i % 2 == 0 else "Mac mini Speakers"
    a.seq(("key", ",", "cmd"), ("sleep", .6), ("click", *BUTTONS[to]), ("sleep", .4), ("key", "escape"))
    log.append((round(time.time() - t0, 1), to))
    time.sleep(11)
while a.p.poll() is None and time.time() - t0 < secs + 30:
    time.sleep(0.5)
if a.p.poll() is None:
    a.quit()
print("switches (s after launch, to):", log)
text = out.read_text(errors="replace")
print("\n".join(l for l in text.splitlines() if l.startswith("#") and any(k in l for k in ("summary", "underruns", "overloads", "rebuilds", "alloc"))))
rows = [r for r in csv.DictReader(l for l in text.splitlines() if not l.startswith("#"))]
prev = None
for r in rows:
    dev = r.get("device_error", ""), r.get("generation", "")
    gap = float(r.get("max_gap_ms") or 0)
    if prev is not None and (r.get("generation") != prev or gap > 30):
        print(f"t {r['t_s']} s: generation {prev} -> {r.get('generation')}, frames/cb {r.get('frames_per_cb')}, max callback gap {gap:.1f} ms, overloads {r.get('overloads')}")
    prev = r.get("generation")
