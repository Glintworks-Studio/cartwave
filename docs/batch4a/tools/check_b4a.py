"""Checks what at_b4a.ps1 saw and saved (B4A-xx, AT-13). PASS/FAIL per check; exit 1 on a failure."""
import json
import os
import re
import sys

sys.stdout.reconfigure(encoding="utf-8")
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


DEFAULT = ["#C9D24A", "#C9D24A", "#7F86E0", "#B5AC98", "#E39A42", "#D9579A", "#D9579A", "#3CC4A6"]


def track_colour(f, i):
    """A track's colour; a pre-v7 file leaves the default out (v7 writes it)."""
    if f is None:
        return "missing file"
    if "tracks" in f:
        return f["tracks"][i].get("color")
    return f["lanes"][i].get("color") or DEFAULT[i]


if os.path.exists(os.path.join(work, "b402_esc.chipdaw")):
    S = "B4A-02"
    fx, e, pk, u = load("fx_basic"), load("b402_esc"), load("b402_picked"), load("b402_undone")
    orig = track_colour(fx, 2)
    check(S, track_colour(e, 2) == orig, f"hover a colour, Esc: the old colour is back ({track_colour(e, 2)} vs {orig})")
    check(S, obs.get("b402_title_esc") == obs.get("b402_title0"), f"... and nothing is unsaved ({obs.get('b402_title_esc')!r})")
    check(S, track_colour(pk, 2) not in (orig, None), f"pick B: the track takes it ({track_colour(pk, 2)})")
    check(S, track_colour(u, 2) == orig, f"hover A, pick B, undo: the original colour ({track_colour(u, 2)} vs {orig})")


if os.path.exists(os.path.join(work, "b403_ctrl_click.chipdaw")):
    S = "B4A-03"
    cc, two, tog, rng = (load(n) for n in ("b403_ctrl_click", "b403_del_two", "b403_del_toggled", "b403_del_range"))
    check(S, clip_ids(cc) == [1, 2, 3, 4], f"a Ctrl-click makes no copies ({clip_ids(cc)})")
    check(S, clip_ids(two) == [2, 4], f"click Lead A, Ctrl-click Harm B, Delete: both gone ({clip_ids(two)})")
    check(S, clip_ids(tog) == [1, 2, 4], f"... Ctrl-click Lead A again toggles it off: only Harm B gone ({clip_ids(tog)})")
    check(S, clip_ids(rng) == [1], f"click Harm A, Shift-click Bass: the range (Harm A, Harm B, Bass) ({clip_ids(rng)})")
    cd = load("b403_ctrl_drag")
    starts = sorted(c["start"] for c in cd["clips"] if c["lane"] == 0) if cd else None
    check(S, cd and len(cd["clips"]) == 5 and starts == [0, 4 * 3840], f"Ctrl-drag still copies: Lead A stays, a copy at bar 5 ({starts})")


def tracks(f):
    return f["tracks"] if f and "tracks" in f else None


if os.path.exists(os.path.join(work, "b404_inserted.chipdaw")):
    S = "B4A-04"
    ins, mov, dele, col = (load(n) for n in ("b404_inserted", "b404_moved", "b404_deleted", "b404_colour"))
    t = tracks(ins)
    check(S, t and len(t) == 9 and t[1]["voice"] == "pulse" and "lock" not in t[1] and t[1]["name"] == "Pulse 3", f"Ctrl+Shift+T: a studio Pulse 3 below track 1 ({t and [(x['name'], x.get('lock')) for x in t[:3]]})")
    check(S, t and t[1]["color"] not in (t[0]["color"], t[2]["color"]), f"its colour is a new automatic one ({t and t[1]['color']})")
    m = tracks(mov)
    check(S, m and m[0]["name"] == "Pulse 3" and m[1]["name"] == "Pulse 1", f"dragged above track 1 ({m and [x['name'] for x in m[:3]]})")
    check(S, mov and all(c["lane"] != 0 for c in mov["clips"]) and any(c["lane"] == 1 and c["name"] == "Lead A" for c in mov["clips"]), "the clips moved with their tracks")
    dl = tracks(dele)
    check(S, dl and len(dl) == 8 and not any(x["name"] == "Pulse 3" for x in dl), f"Delete removes it ({dl and len(dl)})")
    c = tracks(col)
    check(S, c and dl and c[2]["color"] != dl[2]["color"], f"the colour from the context menu is saved ({c and c[2]['color']})")
    check(S, has(lst("b404_dev"), r"insert track 1 Pulse") and has(lst("b404_dev"), r"move track 1 -> 0"), f"dev lines {lst('b404_dev')}")

if os.path.exists(os.path.join(work, "b405_locked.chipdaw")):
    S = "B4A-05"
    lk, un, re_, ul = (tracks(load(n)) for n in ("b405_locked", "b405_undone", "b405_redone", "b405_unlocked"))
    check(S, has(lst("b405_menu"), r"Studio \(unlocked\)\|2A03 Pulse 1\|-2A03 Pulse 2  \(Pulse 2\)"), f"the badge's menu: {lst('b405_menu')}")
    check(S, "fader is at −6.0 dB" in (obs.get("b405_dialog") or "") and "0 dB and centre" in (obs.get("b405_dialog") or ""), f"the reset dialog: {obs.get('b405_dialog')!r}")
    check(S, lk and lk[0].get("lock") == "2a03_p1" and "volume_db" not in lk[0] and "pan" not in lk[0] and lk[0].get("before_lock") == [-6.0, 20], f"Reset and lock: locked at 0 dB / C, old values kept ({lk and lk[0]})")
    check(S, un and "lock" not in un[0] and un[0].get("volume_db") == -6.0 and un[0].get("pan") == 20, f"one undo restores lock state and values ({un and un[0]})")
    check(S, re_ and re_[0].get("lock") == "2a03_p1", "redo locks again")
    check(S, ul and "lock" not in ul[0] and ul[0].get("volume_db") == -6.0 and ul[0].get("pan") == 20, f"unlocking restores the fader and pan ({ul and ul[0]})")

for ans, want in (("unlock", "unlock"), ("fit", "keep"), ("cancel", "cancel")):
    n = "b406_" + ans
    if os.path.exists(os.path.join(work, n + ".chipdaw")):
        S = "B4A-06"
        f = load(n)
        t = tracks(f)
        warned = has(lst("b406_warn_" + ans), r'Switch the hardware profile to NES \(2A03\).*"Shine" \(VRC6 Pulse 1\)')
        check(S, warned, f"{ans}: the warning lists the VRC6 track ({lst('b406_warn_' + ans)})")
        if want == "unlock":
            check(S, f["profile"] == "nes" and "lock" not in t[3], f"Unlock tracks: NES, Shine is a studio track ({f['profile']}, {t[3].get('lock')})")
        elif want == "keep":
            check(S, f["profile"] == "nes" and t[3].get("lock") == "vrc6_p1", f"Switch anyway: NES, Shine stays locked (silent) ({f['profile']}, {t[3].get('lock')})")
        else:
            check(S, f["profile"] == "famicom+vrc6" and t[3].get("lock") == "vrc6_p1", f"Cancel: nothing changed ({f['profile']})")

for ans in ("fit", "unlock", "cancel"):
    n = "b407_" + ans
    if os.path.exists(os.path.join(work, n + ".chipdaw")):
        S = "B4A-07"
        f = load(n)
        t = tracks(f)
        ns = f["clips"][0]["notes"]
        warned = has(lst("b407_warn_" + ans), r"A locked track plays one note at a time")
        check(S, warned, f"{ans}: the chord asked first ({lst('b407_warn_' + ans)})")
        over = any(a["start"] < b["start"] + b["len"] and b["start"] < a["start"] + a["len"] for i, a in enumerate(ns) for b in ns[i + 1:])
        if ans == "fit":
            check(S, len(ns) >= 1 and not over and t[0].get("lock") == "2a03_p1", f"Fit: no overlaps, still locked ({ns})")
        elif ans == "unlock":
            check(S, len(ns) == 2 and over and "lock" not in t[0], f"Unlock: the chord stays on a studio track ({ns}, {t[0].get('lock')})")
        else:
            check(S, len(ns) == 1 and t[0].get("lock") == "2a03_p1", f"Cancel: the second note is gone, still locked ({ns})")

if os.path.exists(os.path.join(work, "b408_minus3.chipdaw")):
    S = "B4A-08"
    f = load("b408_minus3")
    check(S, f.get("master_db") == -3.0, f"three wheel notches down: Main at -3 dB, saved ({f.get('master_db')})")
    check(S, has(lst("b408_help"), "PEAK"), f"PEAK's hover help ({lst('b408_help')})")

if os.path.exists(os.path.join(work, "b409_noise.chipdaw")):
    S = "B4A-09"
    t = tracks(load("b409_noise"))
    check(S, has(lst("b409_menu"), r"Rename\|Colour\|Insert Track\|Duplicate Track\|Delete Track\|Lock\|Voices"), f"the header's menu: {lst('b409_menu')}")
    check(S, t and len(t) == 5 and t[1]["voice"] == "noise" and t[1]["name"] == "Noise 1" and "lock" not in t[1], f"Insert Track > Noise: a studio Noise 1 below ({t and [(x['name'], x['voice']) for x in t]})")
    v = tracks(load("b409_voices"))
    check(S, v and v[0].get("voices") == 4, f"Voices > 4 on the studio track ({v and v[0].get('voices')})")
    dp = tracks(load("b409_dup"))
    check(S, dp and len(dp) == 6 and dp[1]["name"] == "Lead 2" and dp[1].get("voices") == 4, f"Ctrl+D on a selected track duplicates it ({dp and [x['name'] for x in dp]})")
    f = load("b409_dup")
    check(S, f and sum(1 for c in f["clips"] if c["lane"] == 1) == 1, "with a copy of its clip")

print()
bad = [k for k, v in results.items() if not all(v)]
for k in sorted(results):
    print(("PASS " if all(results[k]) else "FAIL ") + k)
sys.exit(1 if bad else 0)
