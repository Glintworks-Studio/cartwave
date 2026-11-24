"""Frame pacing report: app-side submit intervals + display-side update gaps.

usage: pacing_analyze.py <framelog.csv> <ddagrab pts.txt> <secs> <cpu%> <label>
Display period P is 1000 / $CHIPDAW_REFRESH_HZ (default 59.951, the DWM rate of
the machine PERF.md was measured on). Use the exact rate of the display under test.
"""
import csv
import os
import statistics
import sys

log, pts_path, secs, cpu, label = sys.argv[1], sys.argv[2], float(sys.argv[3]), sys.argv[4], sys.argv[5]
P = 1000.0 / float(os.environ.get("CHIPDAW_REFRESH_HZ", "59.951"))

rows = list(csv.DictReader(open(log)))
iv = [float(r['interval_ms']) for r in rows if float(r['interval_ms']) > 0]
# keep the last `secs` seconds of the log (the capture window)
t_end = float(rows[-1]['t_ms'])
win = [float(r['interval_ms']) for r in rows if float(r['interval_ms']) > 0 and float(r['t_ms']) >= t_end - secs * 1000]
win.sort()
def pct(a, q):
    return a[min(len(a) - 1, int(q * len(a)))]

pts = [float(x) for x in open(pts_path, encoding='utf-8', errors='ignore').read().split() if x.strip()]
pts.sort()
gaps = [(b - a) * 1000 for a, b in zip(pts, pts[1:])]
# Ignore the first 0.5 s of capture start-up.
gaps = [g for g, t in zip(gaps, pts[1:]) if t - pts[0] > 0.5]
repeats = sum(1 for g in gaps if g > 1.5 * P)          # a vblank went by with no new frame
missing_vblanks = sum(max(0, round(g / P) - 1) for g in gaps if g > 1.5 * P)
dur = (pts[-1] - pts[0]) if len(pts) > 1 else 0
upd_rate = len(pts) / dur if dur else 0

phase = {k: statistics.mean(float(r[k]) for r in rows[-len(win):]) / 1000 for k in
         ['events_us', 'style_us', 'layout_us', 'make_current_us', 'draw_us', 'flush_us', 'present_us', 'custom_us']}
print(f"[{label}] cpu {cpu}%  frames {len(win)} in {secs:.0f}s")
print(f"  submit interval ms: mean {statistics.mean(win):.2f}  p50 {pct(win,.5):.2f}  p99 {pct(win,.99):.2f}  max {win[-1]:.2f}  >20ms: {sum(1 for x in win if x > 20)}")
print(f"  display updates: {len(pts)} over {dur:.1f}s = {upd_rate:.2f}/s; stalls (gap>1.5 vblank): {repeats} "
      f"({missing_vblanks} vblanks w/o new frame, one per {dur / max(1, repeats):.1f}s); max gap {max(gaps) if gaps else 0:.1f} ms")
print("  ui-thread ms/frame: " + "  ".join(f"{k[:-3]} {v:.2f}" for k, v in phase.items()))
