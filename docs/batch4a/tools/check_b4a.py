"""Checks what at_b4a.ps1 saw and saved (B4A-xx, AT-13). PASS/FAIL per check; exit 1 on a failure."""
import json
import os
import re
import sys

work = sys.argv[1]
obs = json.load(open(os.path.join(work, "obs.json"), encoding="utf-8-sig"))
results = {}


def lst(k):
    v = obs.get(k)
    if v is None:
        return []
    return v if isinstance(v, list) else [v]


def load(n):
    p = os.path.join(work, n + ".chipdaw")
    return json.load(open(p, encoding="utf-8")) if os.path.exists(p) else None


def settings(n):
    p = os.path.join(work, n + ".settings.json")
    return json.load(open(p, encoding="utf-8-sig")) if os.path.exists(p) else None


def check(sc, cond, msg):
    print(("PASS " if cond else "FAIL ") + f"{sc}: {msg}")
    results.setdefault(sc, []).append(bool(cond))


def has(lines, pat):
    return any(re.search(pat, l) for l in lines)


def clip_ids(f):
    return sorted(c["id"] for c in f["clips"]) if f else None


if os.path.exists(os.path.join(work, "b401_after_export.chipdaw")):
    S = "B4A-01"
    fx, a, m = load("fx_basic"), load("b401_after_export"), load("b401_after_modal")
    check(S, a and clip_ids(a) == clip_ids(fx), f"Backspace/Delete behind Export: clips unchanged ({clip_ids(a)})")
    check(S, m and clip_ids(m) == clip_ids(fx), f"Return/Backspace/Delete behind Settings: clips unchanged ({clip_ids(m)})")
    check(S, not lst("b401_piano"), f"Return behind a dialog never opened the piano roll ({lst('b401_piano')})")


def track_colour(f, i):
    if f is None:
        return "missing file"
    if "tracks" in f:
        return f["tracks"][i].get("color")
    return f["lanes"][i].get("color")


if os.path.exists(os.path.join(work, "b402_esc.chipdaw")):
    S = "B4A-02"
    fx, e, pk, u = load("fx_basic"), load("b402_esc"), load("b402_picked"), load("b402_undone")
    orig = track_colour(fx, 2)
    check(S, track_colour(e, 2) == orig, f"hover a colour, Esc: the old colour is back ({track_colour(e, 2)} vs {orig})")
    check(S, obs.get("b402_title_esc") == obs.get("b402_title0"), f"... and nothing is unsaved ({obs.get('b402_title_esc')!r})")
    check(S, track_colour(pk, 2) not in (orig, None), f"pick B: the track takes it ({track_colour(pk, 2)})")
    check(S, track_colour(u, 2) == orig, f"hover A, pick B, undo: the original colour ({track_colour(u, 2)} vs {orig})")

print()
bad = [k for k, v in results.items() if not all(v)]
for k in sorted(results):
    print(("PASS " if all(results[k]) else "FAIL ") + k)
sys.exit(1 if bad else 0)
