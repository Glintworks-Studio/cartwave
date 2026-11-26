"""Checks what at_dp.ps1 saw (obs.json), saved (*.chipdaw, settings) and drew (*.png).
Prints PASS/FAIL per check and a line per scenario; exit 1 on a failure."""
import json
import os
import re
import sys

from PIL import Image, ImageChops

work = sys.argv[1]
obs = json.load(open(os.path.join(work, "obs.json"), encoding="utf-8-sig"))
results = {}


def o(k, d=None):
    v = obs.get(k, d)
    return d if v is None else v


def load(name):
    p = os.path.join(work, name + ".chipdaw")
    return json.load(open(p, encoding="utf-8")) if os.path.exists(p) else None


def check(sc, cond, msg):
    print(("PASS " if cond else "FAIL ") + f"{sc}: {msg}")
    results.setdefault(sc, []).append(bool(cond))


def as_list(v):
    return v if isinstance(v, list) else ([v] if v else [])


# DP-01: TAP
if "dp01_taps" in obs:
    S = "DP-01"
    taps = as_list(o("dp01_taps"))
    bpm = [float(re.search(r"bpm ([\d.]+)", t).group(1)) for t in taps]
    steps = [re.search(r"steps \((\d+), (\d+)\)", t).groups() for t in taps]
    check(S, len(bpm) >= 3 and abs(bpm[2] - 120.0) <= 1.5, f"four taps 500 ms apart: {bpm[:3]} BPM (the unit test proves exactly 120)")
    first = steps[:3]
    check(S, len(set(first)) == 1 and first[0][0] != "0", f"the first sequence is one undo step: {first}")
    a, u, r = load("dp01_tapped"), load("dp01_undone"), load("dp01_redone")
    check(S, a is not None and abs(a["bpm"] - 120.0) <= 1.5, f"saved tempo {a and a['bpm']}")
    check(S, u is not None and u["bpm"] == 150.0, f"one Ctrl+Z restores 150: {u and u['bpm']}")
    check(S, r is not None and r["bpm"] == a["bpm"], f"Ctrl+Y redoes it: {r and r['bpm']}")
    check(S, len(bpm) >= 5, f"the TAP key taps too (a second sequence): {bpm[3:]}")

# DP-02: spectrum style
if "dp02_first" in obs:
    S = "DP-02"
    menu = " ".join(as_list(o("dp02_menu")))
    check(S, "Bars|LED Segments|Line|Peak Dots" in menu, f"the right-click menu lists the styles: {menu}")
    check(S, any("spectrum style leds" in l and "settings" not in l for l in as_list(o("dp02_first"))), f"choosing LED Segments: {o('dp02_first')}")
    st = json.load(open(os.path.join(work, "dp02_settings.json"), encoding="utf-8-sig")) if os.path.exists(os.path.join(work, "dp02_settings.json")) else {}
    check(S, st.get("spectrum_style") == "leds", f"settings.json: {st.get('spectrum_style')}")
    check(S, any("spectrum style leds (settings)" in l for l in as_list(o("dp02_second"))), f"after a restart: {o('dp02_second')}")

# DP-03: preview on select
if "dp03_on" in obs:
    S = "DP-03"
    on = as_list(o("dp03_on"))
    lanes = [l for l in on if re.search(r"preset \d+ lane \d+", l)]
    check(S, len(lanes) >= 2, f"a click and an arrow key each audition a preset: {lanes}")
    if len(lanes) >= 2:
        p1, p2 = (int(re.search(r"preset (\d+)", l).group(1)) for l in lanes[:2])
        check(S, p2 == p1 + 1, f"Down moves to the next preset: {p1} -> {p2}")
    off = as_list(o("dp03_off"))
    check(S, any("off (Preview key)" in l for l in off), f"with the Preview key off, a click is silent: {off[-1:] }")
    check(S, len(as_list(o("dp03_playing"))) > 0, f"during playback it says why: {o('dp03_playing')}")
    st = json.load(open(os.path.join(work, "dp03_settings.json"), encoding="utf-8-sig")) if os.path.exists(os.path.join(work, "dp03_settings.json")) else {}
    check(S, st.get("preview_on_select") is True, "the Preview key's state is remembered (back on)")

# DP-04: disabled reasons
if "dp04_mixer" in obs:
    S = "DP-04"
    for k, what in (("dp04_mixer", "Mixer key"), ("dp04_record", "Record key"), ("dp04_pan", "pan slider (editable; heard with stereo mixing)"), ("dp04_tri", "triangle's fader (works since polish 2)")):
        check(S, len(as_list(o(k))) > 0, f"the {what} says why on the status bar: {o(k)}")

# DP-05: bit depth
if "dp05_dev" in obs:
    S = "DP-05"
    dev = as_list(o("dp05_dev"))
    check(S, any(l.endswith(" 16") for l in dev[:1]), f"a version 4 file opens as 16-bit: {dev[:1]}")
    check(S, any(l.endswith(" 24") for l in dev), f"Project Settings sets 24-bit: {dev}")
    f = load("dp05_saved")
    check(S, f is not None and f.get("bits") == 24 and f.get("version") >= 5, f"saved: version {f and f.get('version')}, bits {f and f.get('bits')}")
    b, a = (os.path.join(work, n) for n in ("dp05_before.png", "dp05_after.png"))
    if os.path.exists(b) and os.path.exists(a):
        # The indicators' format text (client x 416..488, y 40..56; window y + 51, x + 1).
        box = (417, 91, 490, 108)
        diff = ImageChops.difference(Image.open(b).convert("RGB").crop(box), Image.open(a).convert("RGB").crop(box)).getbbox()
        check(S, diff is not None, f"the display's indicator changes: {diff}")

bad = 0
for sc, r in results.items():
    ok = all(r)
    bad += not ok
    print(f"{sc}: {'PASS' if ok else 'FAIL'} ({sum(r)}/{len(r)})")
sys.exit(1 if bad else 0)
