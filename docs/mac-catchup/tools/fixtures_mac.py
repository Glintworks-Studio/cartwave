#!/usr/bin/env python3
"""Fixture for the MAC_CATCHUP scenario checks: the showcase's tracks and instruments with
short clips at known places (format v5, as the showcase; it opens in v6 unchanged).

  lane 0 (Pulse 1):  A bars 1-2, B bars 3-4 (8 notes each, on 1/8 steps)
  lane 1 (Pulse 2):  empty (double-click, time selection)
  lane 2 (Triangle): C bars 1-4 (16 notes)
  lane 3 (Noise):    D bars 5-6 (8 notes)
  lanes 4-7:         empty

usage: fixtures_mac.py <out.chipdaw>
"""
import json
import pathlib
import sys

BAR = 3840
here = pathlib.Path(__file__).resolve().parents[3]
d = json.loads((here / "docs/songs/v5/showcase.chipdaw").read_text())
nid = 1


def clip(cid, lane, bar, bars, name, n, pitch):
    global nid
    notes = []
    step = bars * BAR // n
    for i in range(n):
        notes.append({"id": nid, "start": i * step + (37 if i == 3 else 0), "len": step // 2, "pitch": pitch + (i % 5), "volume": 12})
        nid += 1
    return {"id": cid, "lane": lane, "start": (bar - 1) * BAR, "len": bars * BAR, "name": name, "notes": notes}


d["clips"] = [
    clip(1, 0, 1, 2, "A", 8, 72),
    clip(2, 0, 3, 2, "B", 8, 74),
    clip(3, 2, 1, 4, "C", 16, 45),
    clip(4, 3, 5, 2, "D", 8, 40),
]
d["loop"] = False
d["loop_start"] = 0
d["loop_end"] = 8 * BAR
pathlib.Path(sys.argv[1]).write_text(json.dumps(d, indent=1))
