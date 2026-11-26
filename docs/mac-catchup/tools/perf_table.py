#!/usr/bin/env python3
"""Summarise perf_mac.py JSON lines: per (display, build, scenario) the median of the rounds.
usage: perf_table.py results.jsonl [--raw]"""
import json
import statistics
import sys
from collections import defaultdict

rows = []
for l in open(sys.argv[1]) if sys.argv[1] != "-" else sys.stdin:
    try:
        rows.append(json.loads(l))
    except Exception:
        pass
if "--raw" in sys.argv:
    for r in rows:
        print(f"{r.get('display', '?'):>3} {r.get('build', '?'):5} {r.get('name', '?'):13} cpu {r.get('cpu_pct', float('nan')):5.1f}% fps {r.get('fps', 0):6.1f} "
              f"Minstr/frame {r.get('minstr_per_frame')} P {r.get('pcore_share')} mW {r.get('power_mw')} p99 {r.get('iv_p99')} >20ms {r.get('frames_gt20')}")
    sys.exit()
g = defaultdict(list)
for r in rows:
    if "cpu_pct" in r:
        g[(r["display"], r["name"], r["build"])].append(r)
print("| Display | Scenario | Build | CPU % (median, range) | fps | M instr / frame | P-core share | mW | frames > 20 ms (app) | n |")
print("|---|---|---|---|---|---|---|---|---|---|")
for (d, n, b), v in sorted(g.items()):
    c = [x["cpu_pct"] for x in v]
    med = lambda k: statistics.median([x.get(k) or 0 for x in v])
    print(f"| {d} Hz | {n} | {b} | {statistics.median(c):.2f} ({min(c):.1f}–{max(c):.1f}) | {med('fps'):.1f} | {med('minstr_per_frame'):.2f} | {med('pcore_share'):.2f} | {med('power_mw'):.0f} | {max(x.get('frames_gt20') or 0 for x in v)} | {len(v)} |")
