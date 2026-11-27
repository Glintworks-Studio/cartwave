"""Batch 4c §2: per-phase frame cost from CARTWAVE_FRAMELOG files (threads_b4c.ps1 writes one
per run next to its audio log, `<tag><build>-<round>.frames` and `.spans`).

Prints, per build, the mean per presented frame (frames after the first 5 s) of every phase
(events, style, layout, draw, flush, present µs) and every custom draw span (µs), averaged over
the runs, and the difference of each build from the first one named.

usage: python phases_b4c.py <dir> <tag> <build> [<build> ...]
"""
import csv
import glob
import os
import statistics
import sys

d, tag, builds = sys.argv[1], sys.argv[2], sys.argv[3:]


def runs(b):
    for f in sorted(glob.glob(os.path.join(d, f"{tag}{b}-*.frames"))):
        rows = list(csv.DictReader(open(f, encoding="utf-8")))
        t0 = float(rows[0]["t_ms"]) if rows else 0
        keep = [i for i, r in enumerate(rows) if float(r["t_ms"]) - t0 > 5000]
        spans_f = f[: -len(".frames")] + ".spans"
        spans = list(csv.reader(open(spans_f, encoding="utf-8"))) if os.path.exists(spans_f) else []
        head, sp = (spans[0], spans[1:]) if spans else ([], [])
        out = {}
        for k in ["events_us", "style_us", "layout_us", "draw_us", "flush_us", "present_us", "custom_us"]:
            out[k] = statistics.mean(float(rows[i][k]) for i in keep)
        for j, name in enumerate(head):
            vals = [float(sp[i][j]) for i in keep if i < len(sp)]
            out["span:" + name] = statistics.mean(vals) if vals else 0.0
        out["frames"] = len(keep)
        yield out


res = {}
for b in builds:
    rs = list(runs(b))
    if not rs:
        continue
    keys = rs[0].keys()
    res[b] = {k: statistics.mean(r[k] for r in rs) for k in keys}
    res[b]["runs"] = len(rs)
base = builds[0]
keys = list(res[base].keys())
print("| µs per frame | " + " | ".join(builds) + " | " + " | ".join(f"Δ {b}" for b in builds[1:]) + " |")
print("|---" * (1 + 2 * len(builds) - 1) + "|")
for k in keys:
    vals = [res[b].get(k, 0.0) for b in builds]
    deltas = [v - vals[0] for v in vals[1:]]
    print(f"| {k} | " + " | ".join(f"{v:.1f}" for v in vals) + " | " + " | ".join(f"{x:+.1f}" for x in deltas) + " |")
