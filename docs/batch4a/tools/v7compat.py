"""Batch 4a (RUNLOG B4A-D17): the earlier suites' checkers read saved projects in the version 6
shape ("lanes", "chip", colours only when not the default). Format 7 saves "tracks", "profile" and
every track's colour. `install()` wraps json.load so a version 7 project reads as the checkers
expect; nothing else changes. The checkers call it first thing."""
import json

_orig_load = json.load
_orig_loads = json.loads

# The legacy lanes' default colours (VISUAL-SPEC.md §9), by index.
DEFAULT = ["#C9D24A", "#C9D24A", "#7F86E0", "#B5AC98", "#E39A42", "#D9579A", "#D9579A", "#3CC4A6"]
PROFILE = {"none": "none", "nes": "2A03", "famicom+vrc6": "2A03+VRC6"}


def as_v6(v):
    if not (isinstance(v, dict) and v.get("format") == "chipdaw" and "tracks" in v):
        return v
    v = dict(v)
    lanes = []
    for i, t in enumerate(v["tracks"]):
        l = dict(t)
        if i < len(DEFAULT) and str(l.get("color", "")).upper() == DEFAULT[i]:
            l.pop("color", None)
        l.setdefault("mute", False)
        l.setdefault("solo", False)
        lanes.append(l)
    v["lanes"] = lanes
    v["chip"] = PROFILE.get(v.get("profile"), v.get("profile"))
    return v


def install():
    json.load = lambda *a, **k: as_v6(_orig_load(*a, **k))
    json.loads = lambda *a, **k: as_v6(_orig_loads(*a, **k))
