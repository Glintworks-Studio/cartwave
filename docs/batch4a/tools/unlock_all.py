"""Unlocks every track of a format 7 project in place (RUNLOG B4A-D27: the polish 2 fader and
pan scenarios need editable tracks). Refuses anything that isn't format 7.

usage: python unlock_all.py <project.chipdaw>"""
import json
import sys

p = sys.argv[1]
v = json.load(open(p, encoding="utf-8"))
assert v.get("version") in (7, 8) and "tracks" in v, f"not a format 7 or 8 project: version {v.get('version')}"
for t in v["tracks"]:
    t["lock"] = None
json.dump(v, open(p, "w", encoding="utf-8"), indent=1)
print(f"unlocked {len(v['tracks'])} tracks")
