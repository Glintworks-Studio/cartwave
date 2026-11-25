"""Checks the files scenario_v11.ps1 saved (s0..s6.chipdaw in the work dir)."""
import json, sys, os

work = sys.argv[1]
def load(n):
    with open(os.path.join(work, f"s{n}.chipdaw"), encoding="utf-8") as f:
        return json.load(f)
def table(f):
    return {i["id"]: i for i in f["instruments"]}
def lane_inst(f, lane):
    return table(f)[f["lanes"][lane]["inst"]]
def clip(f, name):
    return next(c for c in f["clips"] if c["name"] == name)

fails = []
def check(cond, msg):
    print(("PASS " if cond else "FAIL ") + msg)
    if not cond:
        fails.append(msg)

s = [load(n) for n in range(7)]
check(all(f["version"] == 3 for f in s), "every save is format v3 (the v2 file was migrated)")
p0, p1, p2, p3 = (lane_inst(f, 0) for f in s[:4])
check(p0["name"] == "Square Lead" and not p0.get("arpeggio") and not p0.get("pitch"), "start: Square Lead without arpeggio/pitch")
check(p1 == p0, "undo x4 restores the instrument exactly")
changed = [k for k in ("volume", "arpeggio", "pitch", "duty") if p2.get(k) != p0.get(k)]
check(changed == ["volume", "arpeggio", "pitch", "duty"], f"redo x4: all four macros drawn ({changed})")
check(len(p2.get("arpeggio", "").split()) == 16 and len(p2.get("pitch", "").split()) == 16, "unused macros started as 16 steps")
check(p2.get("edited") is True, "the instrument is marked edited")
rel = lambda v: v.split().index("/") if "/" in v.split() else None
check(rel(p3["volume"]) is not None and rel(p3["volume"]) > rel(p2["volume"]), f"release marker moved right ({rel(p2['volume'])} -> {rel(p3['volume'])})")
check(p3["vibrato"][0] == p2["vibrato"][0] + 3, f"vibrato depth +3 ({p2['vibrato'][0]} -> {p3['vibrato'][0]})")
f4 = s[4]
hook = clip(f4, "Hook A")
check(hook.get("instrument") is not None and table(f4)[hook["instrument"]]["name"] == "Cello", "Cello dropped on 'Hook A' is that clip's instrument")
check(clip(f4, "Intro Lead").get("instrument") is None, "other Pulse 1 clips keep the lane default")
check(lane_inst(f4, 3)["name"] == lane_inst(s[3], 3)["name"], "drop on the noise lane refused (lane unchanged)")
check(lane_inst(f4, 6)["name"] == "Cello", "drop on the VRC6 Pulse 2 header sets its default")
check(lane_inst(f4, 0)["name"] == "Square Lead", "Pulse 1 default unchanged by the clip drop")
check(lane_inst(s[5], 0)["name"] == "Cello", "double-click loads Cello on the selected lane (Pulse 1)")
layout = ["Violin", "Pizzicato", "Triangle Bass", "Tight Kit", "Sample Kit", "Cello", "Flute", "Horn Section"]
check([lane_inst(s[6], l)["name"] for l in range(8)] == layout, "Chamber Strings layout on all 8 lanes")
check(clip(s[6], "Hook A").get("instrument") is not None, "a layout keeps clip instruments")
print("RESULT:", "PASS" if not fails else f"FAIL ({len(fails)})")
sys.exit(1 if fails else 0)
