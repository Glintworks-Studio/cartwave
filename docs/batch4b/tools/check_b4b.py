"""Checks what at_b4b.ps1 saw and saved (B4B-xx, AT-17). PASS/FAIL per check; exit 1 on a failure."""
import json
import os
import sys

sys.stdout.reconfigure(encoding="utf-8")
work = sys.argv[1]
obs = json.load(open(os.path.join(work, "obs.json"), encoding="utf-8-sig"))
results = {}

# showcase.mid's parts (the plan: one track per part).
SHOWCASE_PARTS = 8


def lst(k):
    v = obs.get(k)
    if v is None:
        return []
    return v if isinstance(v, list) else [v]


def load(n):
    p = os.path.join(work, n + ".chipdaw")
    return json.load(open(p, encoding="utf-8")) if os.path.exists(p) else None


def check(sc, cond, msg):
    results.setdefault(sc, []).append((bool(cond), msg))


def clips_of(p, lane):
    return [c for c in p["clips"] if c["lane"] == lane]


def drop_bar(key):
    """The bar the app says the drop landed on ("... at bar N ...")."""
    import re
    m = [re.search(r" at bar (\d+)", l) for l in lst(key)]
    m = [x for x in m if x]
    return int(m[-1].group(1)) if m else None


def last_drag_bar(key):
    import re
    m = [re.search(r"bar (\d+)$", l.strip()) for l in lst(key)]
    m = [x for x in m if x]
    return int(m[-1].group(1)) if m else None


def ran(prefix):
    return any(k.startswith(prefix) for k in obs)


BAR = 3840
before = load("fx_board")

if ran("m1_"):
    a = load("m1_after")
    sc = "B4B-M1 (AT-17)"
    check(sc, a is not None, "the project was saved")
    if a:
        n0 = len(before["tracks"])
        check(sc, len(a["tracks"]) > n0, f"tracks added: {n0} -> {len(a['tracks'])}")
        old = {c["id"]: c for c in before["clips"]}
        same = all(any(c["id"] == i and c["lane"] == o["lane"] and c["start"] == o["start"] and c["notes"] == o["notes"] for c in a["clips"]) for i, o in old.items())
        check(sc, same, "every old clip is unchanged (id, track, start, notes)")
        check(sc, a["tracks"][:n0] == before["tracks"][:n0] or [t["id"] for t in a["tracks"][:n0]] == [t["id"] for t in before["tracks"]], "the old tracks keep their place")
        new = a["tracks"][n0:]
        check(sc, all("lock" not in t for t in new), "Import as studio tracks: the new tracks are studio tracks")
        check(sc, lst("m1_warn"), "the conflict question was asked (the board's profile has no room)")
        check(sc, lst("m1_import"), "dev: the import ran from the menu")

if ran("m2_"):
    a = load("m2_after")
    s0 = load("fx_studio")
    sc = "B4B-M2 (drop on empty space, no profile)"
    drag = lst("m2_drag")
    check(sc, len(drag) >= 3, f"the preview followed the cursor during the OS drag ({len(drag)} target changes logged)")
    check(sc, any("Onto(0)" in l for l in drag) and any("End" in l for l in drag), "over track 1, then over empty space")
    check(sc, not lst("m2_warn"), "no dialog: nothing conflicts without a profile")
    if a:
        n0 = len(s0["tracks"])
        check(sc, len(a["tracks"]) == n0 + SHOWCASE_PARTS, f"one new track per part: {len(a['tracks']) - n0}")
        starts = sorted({c["start"] // BAR for c in a["clips"] if c["lane"] >= n0})
        bar = drop_bar("m2_import")
        check(sc, bar is not None and bar == last_drag_bar("m2_drag"), f"the drop lands where the preview last was (bar {bar} vs {last_drag_bar('m2_drag')})")
        check(sc, starts and bar and starts[0] == bar - 1, f"the parts start at the drop bar {bar}: bars {[b + 1 for b in starts]}")
        check(sc, all("lock" not in t for t in a["tracks"]), "studio tracks")

if ran("m3_"):
    a = load("m3_after")
    sc = "B4B-M3 (drop onto track 2, Fit with arpeggios)"
    check(sc, lst("m3_warn"), "the conflict question was asked")
    if a:
        on2 = [c for c in clips_of(a, 1) if c["id"] not in {x["id"] for x in before["clips"]}]
        check(sc, len(on2) == 1, f"the first part's clip is on track 2 ({len(on2)})")
        if on2:
            bar = drop_bar("m3_import") or 1
            check(sc, on2[0]["start"] >= (bar - 1) * BAR, f"at or after the drop bar {bar} (starts bar {on2[0]['start'] / BAR + 1:.2f})")
        n0 = len(before["tracks"])
        check(sc, len(a["tracks"]) == n0 + SHOWCASE_PARTS - 1, f"the other parts on new tracks: {len(a['tracks']) - n0}")
        # Every clip on a locked track plays one note at a time.
        poly_ok = True
        for i, t in enumerate(a["tracks"]):
            if "lock" in t:
                for c in clips_of(a, i):
                    ns = sorted((n["start"], n["start"] + n["len"]) for n in c["notes"])
                    poly_ok &= all(ns[k][1] <= ns[k + 1][0] for k in range(len(ns) - 1))
        check(sc, poly_ok, "locked tracks have no chords or overlaps (Fit)")
        check(sc, lst("m3_import") and "Fit { arp: true }" in lst("m3_import")[-1], "dev: Fit with arpeggios")

if ran("m4_"):
    a = load("m4_after")
    sc = "B4B-M4 (a drag that leaves)"
    check(sc, lst("m4_drag"), "the preview showed during the drag")
    check(sc, not lst("m4_import"), "nothing imported")
    if a:
        check(sc, len(a["tracks"]) == len(before["tracks"]) and len(a["clips"]) == len(before["clips"]), "the song is unchanged")

def shot_px(name, x, y):
    """A client pixel of a window shot (the title bar and menu cropped by client_top)."""
    from PIL import Image
    sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "polish2", "tools"))
    from diff import client_top
    p = os.path.join(work, name + ".png")
    if not os.path.exists(p):
        return None
    im = Image.open(p).convert("RGB")
    t = client_top(im, 100)
    return im.getpixel((x + 1, y + t))


def lum(p):
    return max(p) if p else 0


def track(p, i):
    return p["tracks"][i]


if ran("x1_"):
    sc = "B4B-X1 (mixer = headers)"
    f, u, pm, ty = load("x1_fader"), load("x1_undo"), load("x1_pan_ms"), load("x1_typed")
    if f:
        db = track(f, 0).get("volume_db")
        check(sc, isinstance(db, (int, float)) and -13.5 <= db <= -10.5, f"the strip's fader set track 1's volume (about -12 dB): {db}")
    if u:
        check(sc, track(u, 0).get("volume_db") in (None, 0, 0.0), f"one Ctrl+Z restores 0 dB (one undo step per drag): {track(u, 0).get('volume_db')}")
    if pm:
        check(sc, (track(pm, 0).get("pan") or 0) > 20, f"the knob panned track 1 right: {track(pm, 0).get('pan')}")
        check(sc, track(pm, 0).get("mute") is True and track(pm, 1).get("solo") is True, "M and S on the strips are the tracks' mute and solo")
    if ty:
        check(sc, track(ty, 0).get("volume_db") == -3.0, f"a double-click on the readout types the value (-3): {track(ty, 0).get('volume_db')}")
    check(sc, any("mixer" in l for l in lst("x1_view")) and any("arrangement" in l for l in lst("x1_view")), "Tab shows the mixer and goes back")
    check(sc, lst("x1_help"), "the fader's hover help is the header's (\"Volume: drag to change …\")")

if ran("x2_"):
    sc = "B4B-X2 (locked strips)"
    a = load("x2_after")
    if a:
        t = track(a, 0)
        check(sc, t.get("lock") == "2a03_p1" and t.get("volume_db") in (None, 0, 0.0) and t.get("pan") in (None, 0), f"a locked strip's fader and pan don't move: {t.get('volume_db')}, {t.get('pan')}")
        ctl = lambda t: (t["id"], t.get("lock"), t.get("volume_db"), t.get("pan"), t.get("mute"), t.get("solo"), t["name"])
        check(sc, [ctl(t) for t in a["tracks"]] == [ctl(t) for t in before["tracks"]], "no track's controls changed (fader, pan, mute, solo, name)")
    check(sc, lst("x2_help"), "the reason shows on hover (\"Locked to the 2A03 Pulse 1 …\")")

if ran("u1_"):
    sc = "B4B-U1 (unison, detune)"
    a = load("u1_after")
    if a:
        t = track(a, 0)
        check(sc, t.get("unison") == 3, f"Unison 3 saved: {t.get('unison')}")
        check(sc, t.get("detune") == 15, f"Detune 15 cents saved: {t.get('detune')}")
        check(sc, "unison" not in track(a, 1), "the other track keeps one voice")
    check(sc, lst("u1_help"), "the voices row says what it does on hover")

if ran("c1_") or os.path.exists(os.path.join(work, "c1_lit.png")):
    sc = "B4B-C1 (clip mark)"
    lit, reset, mixer = shot_px("c1_lit", 589, 138), shot_px("c1_reset", 589, 138), shot_px("c1_mixer", 241 + 24 + 110, 76 + 60 + 150)
    check(sc, lum(lit) >= 0xE0, f"track 1's mark is lit white after it passed 0 dBFS, and held after stopping: {lit}")
    check(sc, reset is not None and lum(reset) < 0x40, f"a click on the mark resets it: {reset}")
    check(sc, lum(mixer) >= 0xE0, f"the mixer strip's mark lights too: {mixer}")
    quiet = shot_px("c1_lit", 589, 138 + 73)
    check(sc, quiet is not None and lum(quiet) < 0x40, f"the quiet track's mark stays dark: {quiet}")

if ran("t1_"):
    sc = "B4B-T1 (Tab leaves no button focused)"
    a, before = load("t1_after"), load("t1_before")
    if a and before:
        check(sc, a["clips"] == before["clips"] and a["instruments"] == before["instruments"], "Space after Tab played and stopped; the clips and instruments are unchanged (Make Unique wasn't pressed)")
    check(sc, len(lst("t1_views")) == 2, f"Tab showed the mixer and the arrangement: {lst('t1_views')}")

fail = 0
for sc, rs in results.items():
    for ok, msg in rs:
        print(f"{'PASS' if ok else 'FAIL'}  {sc}: {msg}")
        fail += 0 if ok else 1
print(f"{sum(len(r) for r in results.values()) - fail} passed, {fail} failed")
sys.exit(1 if fail else 0)
