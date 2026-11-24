"""Frame pacing report for a chipbench run (macOS counterpart of pacing_analyze.py).

usage: pacing_mac.py <framelog.csv> <secs> [sck.txt] [--hz 320] [--json]

framelog.csv  CHIPDAW_FRAMELOG output; the last <secs> seconds are analyzed.
sck.txt       chipbench --sck output: display times (s) of the frames in which the
              app window's content changed, as seen by the window server.
--hz          refresh rate of the display under test (default $CHIPDAW_REFRESH_HZ or 320).

App side: presented-frame intervals (the backend's swap-to-swap time).
Display side (SCK): the interval between consecutive window updates, expressed in
vblanks. An interval of k vblanks means the window showed the same frame for k
refreshes. For a cap of N fps on an R Hz display the ideal is a constant R/N
vblanks; when R/N isn't an integer, the mix of floor/ceil is the cadence judder.
"""
import collections
import csv
import json
import os
import statistics
import sys

argv = [a for a in sys.argv[1:] if not a.startswith("--")]
flags = sys.argv[1:]
hz = float(os.environ.get("CHIPDAW_REFRESH_HZ", "320"))
if "--hz" in flags:
    hz = float(flags[flags.index("--hz") + 1])
    argv = [a for a in argv if a != flags[flags.index("--hz") + 1]]
P = 1000.0 / hz
log, secs = argv[0], float(argv[1])
sck = argv[2] if len(argv) > 2 else None


def pct(a, q):
    a = sorted(a)
    return a[min(len(a) - 1, int(q * len(a)))] if a else 0.0


out = {}
rows = list(csv.DictReader(open(log)))
if rows:
    t_end = float(rows[-1]["t_ms"])
    win = [r for r in rows if float(r["t_ms"]) >= t_end - secs * 1000 and float(r["interval_ms"]) > 0]
else:
    win = []
iv = [float(r["interval_ms"]) for r in win]
out["frames"] = len(iv)
out["fps"] = len(iv) / secs
if iv:
    out.update(
        iv_mean=statistics.mean(iv), iv_p50=pct(iv, 0.5), iv_p99=pct(iv, 0.99), iv_max=max(iv),
        over20=sum(1 for x in iv if x > 20),
    )
    for k in ["events_us", "style_us", "layout_us", "make_current_us", "draw_us", "flush_us", "present_us", "custom_us"]:
        out["ms_" + k[:-3]] = statistics.mean(float(r[k]) for r in win) / 1000

if sck:
    ts = sorted(float(x) for x in open(sck).read().split())
    gaps = [(b - a) * 1000 for a, b in zip(ts, ts[1:])]
    vb = [max(1, round(g / P)) for g in gaps]
    dur = ts[-1] - ts[0] if len(ts) > 1 else 0
    out["sck_updates_per_s"] = len(ts) / dur if dur else 0
    out["sck_gap_max_ms"] = max(gaps) if gaps else 0
    out["sck_over20"] = sum(1 for g in gaps if g > 20)
    out["sck_cadence"] = dict(sorted(collections.Counter(vb).items()))

if "--json" in flags:
    print(json.dumps(out))
else:
    print(f"app: {out['frames']} frames in {secs:.0f}s = {out['fps']:.1f} fps", end="")
    if iv:
        print(f"; interval mean {out['iv_mean']:.2f} p50 {out['iv_p50']:.2f} p99 {out['iv_p99']:.2f} max {out['iv_max']:.2f} ms; >20ms: {out['over20']}")
        print("  ui-thread ms/frame: " + "  ".join(f"{k[3:]} {v:.3f}" for k, v in out.items() if k.startswith("ms_")))
    else:
        print()
    if sck:
        print(f"display (SCK): {out['sck_updates_per_s']:.1f} updates/s, max gap {out['sck_gap_max_ms']:.1f} ms, >20ms: {out['sck_over20']}")
        print(f"  cadence (vblanks per update at {hz:g} Hz: count): {out['sck_cadence']}")
