"""Checks what at_p2.ps1 saw and saved (P2-01…P2-11). PASS/FAIL per check; exit 1 on a failure."""
import os as _os, sys as _sys
_sys.path.insert(0, _os.path.join(_os.path.dirname(_os.path.abspath(__file__)), "..", "..", "batch4a", "tools"))
import v7compat  # noqa: E402  (Batch 4a: format 7 read as 6, RUNLOG B4A-D17)
v7compat.install()
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


def lane(f, i, k, default=None):
    if f is None:
        return "missing file"
    return f["lanes"][i].get(k, default)


def check(sc, cond, msg):
    print(("PASS " if cond else "FAIL ") + f"{sc}: {msg}")
    results.setdefault(sc, []).append(bool(cond))


def has(lines, pat):
    return any(re.search(pat, l) for l in lines)


if os.path.exists(os.path.join(work, "p201_typed.chipdaw")):
    S = "P2-01"
    t, u, e, inf, away = (load(n) for n in ("p201_typed", "p201_undone", "p201_esc", "p201_inf", "p201_away"))
    check(S, t and t["version"] in (6, 7, 8, 9), f"saved by this build (format {t and t['version']}: 6 before Batch 4a, 7 since, 8 since Batch 4b, 9 since the Presets batch)")
    check(S, lane(t, 0, "volume_db") == -3.0, f"'-3' + Enter: track 1 at -3 dB ({lane(t, 0, 'volume_db')})")
    check(S, lane(u, 0, "volume_db") is None, f"one Ctrl+Z restores 0 dB ({lane(u, 0, 'volume_db')})")
    check(S, lane(e, 0, "volume_db") is None, f"Esc cancels ({lane(e, 0, 'volume_db')})")
    check(S, lane(inf, 0, "volume_db") == "-inf", f"'-inf' is silence ({lane(inf, 0, 'volume_db')})")
    check(S, lane(away, 1, "volume_db") == 2.5, f"'+2.5' and a click elsewhere commits on track 2 ({lane(away, 1, 'volume_db')})")
    check(S, obs.get("p201_title") != obs.get("p201_title0"), f"marked unsaved: title {obs.get('p201_title0')!r} -> {obs.get('p201_title')!r}")

if os.path.exists(os.path.join(work, "p202_left.chipdaw")):
    S = "P2-02"
    l, r, u, m = (load(n) for n in ("p202_left", "p202_right", "p202_undone", "p202_minus"))
    check(S, lane(l, 1, "pan") == -25, f"'25L': pan -25 ({lane(l, 1, 'pan')})")
    check(S, lane(r, 1, "pan") == 30, f"'30R': pan 30 ({lane(r, 1, 'pan')})")
    check(S, lane(u, 1, "pan") == -25, f"one Ctrl+Z goes back to 25L ({lane(u, 1, 'pan')})")
    check(S, lane(m, 2, "pan") == -12, f"'-12' is 12L ({lane(m, 2, 'pan')})")

if os.path.exists(os.path.join(work, "p203_fader.chipdaw")):
    S = "P2-03"
    f, u, p = (load(n) for n in ("p203_fader", "p203_undone", "p203_pan"))
    v = lane(f, 0, "volume_db")
    check(S, isinstance(v, (int, float)) and v < -3.0, f"dragging the fader down lowers the volume ({v})")
    check(S, lane(u, 0, "volume_db") is None, f"one Ctrl+Z undoes the whole drag ({lane(u, 0, 'volume_db')})")
    pv = lane(p, 0, "pan")
    check(S, isinstance(pv, int) and pv > 15, f"dragging the pan thumb right pans right ({pv})")

if os.path.exists(os.path.join(work, "p204_edge.chipdaw")):
    S = "P2-04"
    e, w, u = (load(n) for n in ("p204_edge", "p204_wheel", "p204_undone"))
    check(S, lane(e, 0, "height") == 144.0, f"dragging the bottom edge down 72 px: 144 ({lane(e, 0, 'height')})")
    check(S, lane(w, 1, "height") == 84.0, f"Alt+wheel up one notch over track 2: 84 ({lane(w, 1, 'height')})")
    check(S, lane(u, 1, "height") is None, f"Ctrl+Z restores 72 ({lane(u, 1, 'height')})")
    check(S, len(lst("p204_dev")) >= 2, f"dev lines: {lst('p204_dev')[:3]}")

if settings("p205_520") is not None:
    S = "P2-05"
    check(S, settings("p205_max")["header_width"] == 600.0, f"the width stops at 600 ({settings('p205_max').get('header_width')})")
    check(S, settings("p205_min")["header_width"] == 380.0, f"and at 380 ({settings('p205_min').get('header_width')})")
    check(S, settings("p205_520")["header_width"] == 520.0, f"dragging the boundary 140 px right: 520 ({settings('p205_520').get('header_width')})")
    check(S, os.path.exists(os.path.join(work, "p205_restart.png")), "kept after a restart (screenshot p205_restart.png)")

if obs.get("p206_dev") is not None:
    S = "P2-06"
    a, b, c = settings("p206_right"), settings("p206_byclips"), settings("p206_browser")
    check(S, a and a["headers_right"] is True and not a["strip_by_clips"] and not a["browser_right"], "Track headers: Right (saved, the others untouched)")
    check(S, b and b["headers_right"] and b["strip_by_clips"] is True, "Colour strip: By the clips")
    check(S, c and c["browser_right"] is True, "Browser: Right")
    check(S, len(lst("p206_dev")) >= 3, f"applied at once: {lst('p206_dev')[-1:] }")

if obs.get("p207_dev") is not None:
    S = "P2-07"
    d = lst("p207_dev")
    check(S, has(d, r"^dev: ruler click"), "a click on the ruler places the playhead (a click, not a drag)")
    drags = [l for l in d if l.startswith("dev: ruler drag")]
    ppb = [float(re.search(r"px_per_bar ([\d.]+)", l).group(1)) for l in drags]
    check(S, len(ppb) >= 2 and ppb[0] > 92.0, f"dragging down zooms in: {ppb}")
    sc = [float(re.search(r"scroll ([\d.-]+)", l).group(1)) for l in drags]
    check(S, len(sc) >= 2 and sc[1] != sc[0], f"dragging sideways scrolls: {sc}")
    check(S, has(d, r"^dev: zoom ZoomToFit"), "a double-click zooms to fit")
    check(S, has(d, r"^dev: help Ruler"), "the ruler's hover help")

if obs.get("p208_dev") is not None:
    S = "P2-08"
    d = lst("p208_dev")
    check(S, has(d, r"^dev: zoom FitSong"), "W fits the whole song")
    check(S, has(d, r"^dev: zoom ZoomBack"), "W again goes back")

if obs.get("p209_dev") is not None:
    S = "P2-09"
    d = lst("p209_dev")
    check(S, has(d, r"^dev: overview click"), "a click outside the outline jumps")
    check(S, sum(1 for l in d if l.startswith("dev: overview drag")) >= 2, "drags scroll and zoom")
    check(S, has(d, r"^dev: zoom ZoomToFit"), "a double-click zooms to fit")
    check(S, settings("p209_off") and settings("p209_off")["show_overview"] is False, "Ctrl+Alt+O hides it (saved)")
    check(S, settings("p209_on") and settings("p209_on")["show_overview"] is True, "and shows it again")
    check(S, has(d, r"^dev: help Overview"), "the Overview's hover help")

if obs.get("p210_dev") is not None:
    S = "P2-10"
    d = lst("p210_dev")
    for name, pat in [("fader", r"^dev: help Volume"), ("M key", r"^dev: help Mute"), ("pan", r"^dev: help Pan"),
                      ("view keys", r"^dev: help Arrangement view"), ("transport", r"^dev: help (Play|Stop|Record|Loop)"),
                      ("display", r"^dev: help Display"), ("TAP", r"^dev: help Tap"), ("spectrum", r"^dev: help Spectrum"),
                      ("CPU/MEM", r"^dev: help CPU"), ("ruler", r"^dev: help Ruler"), ("Overview", r"^dev: help Overview")]:
        check(S, has(d, pat), f"hover help for the {name}")
    check(S, has(d, r"^dev: menu Problems"), "a click on the problem icon lists the problems")
    a, b = (os.path.join(work, n) for n in ("p210_status.png", "p210_problems.png"))
    if os.path.exists(a) and os.path.exists(b):
        from PIL import Image, ImageChops
        ia, ib = Image.open(a).convert("RGB"), Image.open(b).convert("RGB")
        box = (0, ia.height - 260, 400, ia.height - 30)
        diff = ImageChops.difference(ia.crop(box), ib.crop(box)).getbbox()
        check(S, diff is not None, f"and the list stays on screen (the area above the icon changed: {diff})")

for tag in ("lo", "lc", "rc", "ro"):
    f = load(f"p211_{tag}_after")
    if f is None:
        continue
    S = "P2-11"
    m = [l.get("mute", False) for l in f["lanes"]]
    so = [l.get("solo", False) for l in f["lanes"]]
    check(S, m[0] and not any(m[1:]), f"{tag}: M on track 1 mutes it ({m})")
    check(S, so[1] and not so[0] and not any(so[2:]), f"{tag}: S on track 2 solos it ({so})")
    check(S, lane(f, 2, "volume_db") == -6.0, f"{tag}: the dB value of track 3 ({lane(f, 2, 'volume_db')})")
    check(S, lane(f, 3, "pan") == 10, f"{tag}: the pan value of track 4 ({lane(f, 3, 'pan')})")
    v = lane(f, 4, "volume_db")
    check(S, isinstance(v, (int, float)) and v < -10.0, f"{tag}: the fader of track 5 ({v})")
    check(S, lane(f, 5, "height") == 96.0, f"{tag}: the bottom edge of track 6 ({lane(f, 5, 'height')})")
    check(S, lane(f, 5, "name") == "Hit", f"{tag}: the name of track 6 ({lane(f, 5, 'name')}; track 7 is under the Main row since Batch 4a)")
    st = settings(f"p211_{tag}")
    check(S, st is not None and st.get("header_width") == 420.0, f"{tag}: the width boundary ({st and st.get('header_width')})")
    d = lst(f"p211_{tag}_dev")
    check(S, has(d, r"^dev: ruler click"), f"{tag}: the ruler")
    check(S, has(d, r"^dev: overview click"), f"{tag}: the Overview")

bad = [k for k, v in results.items() if not all(v)]
print("\n" + ("ALL PASS: " if not bad else "FAILED: ") + (", ".join(sorted(results)) if not bad else ", ".join(sorted(bad))))
sys.exit(1 if bad else 0)
