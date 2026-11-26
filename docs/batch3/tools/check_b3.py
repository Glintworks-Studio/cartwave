"""Checks what at_b3.ps1 saw (obs.json), saved (*.chipdaw, settings) and drew (*.png) in its
work dir. Prints PASS/FAIL per check and a line per acceptance test; exit 1 on a failure.
A test whose files are missing (not run, see -Only) is SKIPPED."""
import os as _os, sys as _sys
_sys.path.insert(0, _os.path.join(_os.path.dirname(_os.path.abspath(__file__)), "..", "..", "batch4a", "tools"))
import v7compat  # noqa: E402  (Batch 4a: format 7 read as 6, RUNLOG B4A-D17)
v7compat.install()
import json, os, sys, re, struct, zlib

work = sys.argv[1]
obs = json.load(open(os.path.join(work, "obs.json"), encoding="utf-8-sig"))
fails, results = [], {}
B = 3840  # ticks per bar


def load(name):
    p = os.path.join(work, name + ".chipdaw")
    return json.load(open(p, encoding="utf-8")) if os.path.exists(p) else None


def check(at, cond, msg):
    print(("PASS " if cond else "FAIL ") + f"{at}: {msg}")
    results.setdefault(at, []).append(bool(cond))
    if not cond:
        fails.append(f"{at}: {msg}")


def o(key, default=None):
    v = obs.get(key, default)
    return default if v is None else v


def spans(f, lane):
    return sorted((c["start"], c["start"] + c["len"]) for c in f["clips"] if c["lane"] == lane)


def song_notes(f, lane):
    out = []
    for c in f["clips"]:
        if c["lane"] != lane:
            continue
        for n in c["notes"]:
            if n["start"] < c["len"]:
                out.append((c["start"] + n["start"], min(n["len"], c["len"] - n["start"]), n["pitch"]))
    return sorted(out)


def clip(f, cid):
    return next((c for c in f["clips"] if c["id"] == cid), None)


def rel(notes):
    return [(n["start"], n["len"], n["pitch"]) for n in notes]


def menu(lines, site):
    for l in lines:
        m = re.match(r"dev: menu (\S+): (.*)$", l)
        if m and m.group(1).startswith(site):
            return [x.lstrip("-") for x in m.group(2).split("|")], m.group(2)
    return None, ""


def png_luma(path, x0, y0, x1, y1):
    """Mean luma of a region of an RGB(A) PNG (8-bit, non-interlaced), no Pillow needed."""
    data = open(path, "rb").read()
    pos, chunks = 8, []
    w = h = ct = 0
    while pos < len(data):
        n = struct.unpack(">I", data[pos:pos + 4])[0]
        t = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + n]
        if t == b"IHDR":
            w, h, _, ct = struct.unpack(">IIBB", body[:10])
        elif t == b"IDAT":
            chunks.append(body)
        pos += 12 + n
    bpp = 4 if ct == 6 else 3
    raw = zlib.decompress(b"".join(chunks))
    stride = w * bpp
    rows, prev = [], bytearray(stride)
    i = 0
    for _ in range(h):
        f = raw[i]; line = bytearray(raw[i + 1:i + 1 + stride]); i += 1 + stride
        for k in range(stride):
            a = line[k - bpp] if k >= bpp else 0
            b = prev[k]
            c = prev[k - bpp] if k >= bpp else 0
            if f == 1: line[k] = (line[k] + a) & 255
            elif f == 2: line[k] = (line[k] + b) & 255
            elif f == 3: line[k] = (line[k] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c; pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[k] = (line[k] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(line); prev = line
    s = n = 0
    for y in range(y0, min(y1, h)):
        r = rows[y]
        for x in range(x0, min(x1, w)):
            o_ = x * bpp
            s += 0.299 * r[o_] + 0.587 * r[o_ + 1] + 0.114 * r[o_ + 2]; n += 1
    return s / max(n, 1)


fx = load("fx_basic")
two = load("fx_two")

# AT-07
f = load("at07_out")
if f:
    A = "AT-07"
    ha, hb = clip(fx, 2), clip(fx, 3)
    l1 = [c for c in f["clips"] if c["lane"] == 1]
    at9 = [c for c in l1 if c["start"] == 8 * B]
    at11 = [c for c in l1 if c["start"] == 10 * B]
    check(A, len(f["clips"]) == len(fx["clips"]) + 2, f"two more clips ({len(fx['clips'])} -> {len(f['clips'])})")
    check(A, len(at9) == 1 and rel(at9[0]["notes"]) == rel(ha["notes"]), "a copy of Harm A starts at bar 9 with identical notes")
    check(A, len(at11) == 1 and rel(at11[0]["notes"]) == rel(hb["notes"]), "a copy of Harm B follows at bar 11, identical notes")

# AT-08
f = load("at08_out")
if f:
    A = "AT-08"
    check(A, clip(f, 1)["name"] == "Verse", f"Ctrl+R renamed the clip: {clip(f, 1)['name']!r}")
    check(A, f["lanes"][2]["name"] == "Bass Line", f"Ctrl+R on a track header: {f['lanes'][2]['name']!r}")
    check(A, f["lanes"][0]["name"] == "Lead", f"double-click a track name: {f['lanes'][0]['name']!r}")
    check(A, f["lanes"][1]["name"] == "Harmony", f"Tab went on to the next track (R9): {f['lanes'][1]['name']!r}")
    u = load("at08_undo")
    check(A, u is not None and u["lanes"][1]["name"] == "Pulse 2" and u["lanes"][0]["name"] == "Lead", "one undo takes back one rename")

# AT-09
a = load("at09_a")
if a:
    A = "AT-09"
    check(A, o("t09_piano") is True, "the piano roll opened (dev line)")
    n = clip(a, 1)["notes"]
    check(A, len(n) == 1 and n[0]["pitch"] == 60 and n[0]["start"] == 0 and n[0]["len"] == 240, f"double-click on empty C4, bar 1: one 1/16 note: {n}")
    check(A, len(clip(load("at09_b"), 1)["notes"]) == 0, "double-click on the note: zero notes")
    check(A, len(clip(load("at09_click_off"), 1)["notes"]) == 0, "Draw Mode off: a single click draws nothing")
    check(A, len(clip(load("at09_c"), 1)["notes"]) == 1, "B, then a click: one note")

# AT-10
if "t10_menus" in obs:
    A = "AT-10"
    lines = o("t10_menus", [])
    want = {
        "Track": ["Rename", "Colour"],
        "Ruler": ["Set Loop to Selection", "Zoom to Fit"],
        "Piano(true)": ["Cut", "Copy", "Paste", "Duplicate", "Delete", "Quantize", "Select All", "Draw Mode"],
        "Piano(false)": ["Cut", "Copy", "Paste", "Duplicate", "Delete", "Quantize", "Select All", "Draw Mode"],
        "Clip": ["Cut", "Copy", "Paste", "Duplicate", "Delete", "Split", "Consolidate", "Rename", "Colour", "Loop Clip", "Make Instrument Unique", "Use Track Instrument", "Save Instrument as Preset"],
        "Empty": ["Paste", "Insert MIDI Clip", "Loop Selection", "Insert Silence", "Delete Time"],
    }
    for site, items in want.items():
        got, raw = menu(lines, site)
        ok = got is not None and all(i in got for i in items)
        if site == "Empty" and got:
            ok = ok and any(g.startswith("Grid") for g in got)
        check(A, ok, f"{site} menu: {raw or 'not opened'}")
    _, raw = menu(lines, "Piano(false)")
    check(A, "-Cut" in raw and "-Delete" in raw, "with no note selected, the note commands are disabled (they say why)")
    for s in ["at10_track", "at10_ruler", "at10_note", "at10_piano_empty", "at10_clip", "at10_empty", "at10_grid_sub"]:
        check(A, os.path.exists(os.path.join(work, s + ".png")), f"screenshot {s}")

# AT-11
s, j = load("at11_split"), load("at11_join")
if s:
    A = "AT-11"
    check(A, spans(s, 0) == [(0, 2 * B), (2 * B, 4 * B)], f"Ctrl+E at bar 3: clips 1-2 and 3-4: {spans(s, 0)}")
    check(A, song_notes(s, 0) == song_notes(fx, 0), "the notes are split between them, none lost")
    check(A, j is not None and spans(j, 0) == [(0, 4 * B)], f"Ctrl+J: one clip again: {spans(j, 0) if j else None}")
    check(A, j is not None and song_notes(j, 0) == song_notes(fx, 0), "with all the notes")

# AT-12
if "t12_zoom" in obs:
    A = "AT-12"
    lines = o("t12_zoom", [])
    z = next((l for l in lines if "ZoomToSelection" in l), "")
    m = re.search(r"range \((\d+), (\d+)\) px_per_bar ([\d.]+)", z)
    check(A, bool(m) and (int(m.group(1)), int(m.group(2))) == (4 * B, 6 * B), f"Z zooms to bars 5-6: {z}")
    if m:
        ppb = float(m.group(3)) * obs.get("scale", 100) / 100
        check(A, 320 <= ppb <= 420, f"bars 5-6 fill the view (~389 px per bar at 840 px): {float(m.group(3)):.1f}")
    back = next((l for l in lines if "ZoomBack" in l), "")
    check(A, "px_per_bar 92.00" in back, f"X restores the earlier zoom: {back}")
    fit = next((l for l in lines if "ZoomToFit" in l), "")
    m = re.search(r"range \((\d+), (\d+)\)", fit)
    check(A, bool(m) and int(m.group(1)) == 0 and int(m.group(2)) >= 8 * B, f"double-click on the ruler fits the song: {fit}")

# AT-14
g = os.path.join(work, "at14_guide.png")
if os.path.exists(g):
    A = "AT-14"
    reg = (860, 400, 1180, 490)
    lg, la = png_luma(g, *reg), png_luma(os.path.join(work, "at14_after_clip.png"), *reg)
    lh, lc = png_luma(os.path.join(work, "at14_help.png"), *reg), png_luma(os.path.join(work, "at14_closed.png"), *reg)
    check(A, lg > la + 2, f"a new project shows the guide; it goes with the first clip (luma {lg:.1f} -> {la:.1f})")
    check(A, lh > lc + 2, f"Help > Getting Started shows it again; Esc closes it (luma {lh:.1f} -> {lc:.1f})")

# AT-22
f = load("at22_out")
if f:
    A = "AT-22"
    for lane in (0, 1):
        check(A, spans(f, lane) == [(0, B), (3 * B, 4 * B)], f"track {lane + 1}: split at bars 2 and 4: {spans(f, lane)}")
        inside = [n for n in song_notes(f, lane) if B <= n[0] < 3 * B]
        check(A, not inside, f"track {lane + 1}: no notes left in bars 2-3")
        check(A, [n for n in song_notes(two, lane) if not (B <= n[0] < 3 * B)] == song_notes(f, lane), f"track {lane + 1}: the notes outside are untouched")

# AT-23
a, b = load("at23_a"), load("at23_b")
if a:
    A = "AT-23"
    check(A, clip(a, 3)["start"] == 2 * B + 480, f"-> -> nudges by 2 x 1/16: {clip(a, 3)['start'] - 2 * B}")
    check(A, b is not None and clip(b, 3)["start"] == 2 * B + 480 + 480, "Ctrl+2 widens the grid to 1/8: -> moves 480 (an eighth)")

# AT-24
a, b = load("at24_silence"), load("at24_deltime")
if a:
    A = "AT-24"
    check(A, spans(a, 0) == [(0, B), (2 * B, 5 * B)], f"Ctrl+I: a bar of silence at bar 2 on Pulse 1: {spans(a, 0)}")
    check(A, spans(a, 2) == [(0, B), (2 * B, 9 * B)] and spans(a, 1) == [(0, B), (2 * B, 3 * B), (3 * B, 5 * B)], "and on every other track")
    check(A, b is not None and all(song_notes(b, l) == song_notes(fx, l) for l in range(8)), "Ctrl+Shift+Del takes the time out again: every note back in place")

# AT-25
f = load("at25_out")
if f:
    A = "AT-25"
    texts = o("t25_texts", [])
    check(A, any("CHANNEL" in t and "PULSE 2" in t for t in texts), "a click on a track header selects it; the inspector shows it (I1)")
    check(A, [l["solo"] for l in f["lanes"][:3]] == [False, True, True], f"exclusive solo, Ctrl-click adds: {[l['solo'] for l in f['lanes'][:3]]}")
    check(A, o("t25_tip") is True and os.path.exists(os.path.join(work, "at25_tooltip_disabled.png")), "a disabled button's tooltip (screenshot)")

# AT-26
f = load("at26_out")
if f:
    A = "AT-26"
    check(A, f["lanes"][3].get("color") == "#4FA8E0", f"track colour from its menu is saved: {f['lanes'][3].get('color')}")
    check(A, clip(f, 1).get("color") == "#E0705A", f"clip colour from its menu is saved: {clip(f, 1).get('color')}")

# AT-27
a = load("at27_longer")
if a:
    A = "AT-27"
    n = clip(a, 1)["notes"]
    check(A, len(n) == 16 and all(x["len"] == 720 for x in n), "Shift+->: every note one 1/16 longer")
    n = clip(load("at27_chopped"), 1)["notes"]
    check(A, len(n) == 48 and all(x["len"] == 240 for x in n), f"Ctrl+E chops on the grid: {len(n)} notes")
    n = clip(load("at27_joined"), 1)["notes"]
    check(A, len(n) == 16 and all(x["len"] == 720 for x in n), f"Ctrl+J joins them again: {len(n)} notes")

# AT-28
if "t28_open" in obs:
    A = "AT-28"
    check(A, o("t28_open") is True and os.path.exists(os.path.join(work, "at28_dialog.png")), "Ctrl+Shift+R opens the dialog over a backdrop (screenshot)")
    check(A, o("t28_after_esc") is False, "Esc closes it")
    st = json.load(open(os.path.join(work, "at28_settings.json"), encoding="utf-8")) if os.path.exists(os.path.join(work, "at28_settings.json")) else {}
    # Design pass (DP-D8): the project's format is the export default, so a
    # reopened dialog starts from the project's 16-bit (the v4 fixture), not
    # the 24-bit picked before; the other choices are still remembered.
    ex = st.get("export", [])
    check(A, "bits:16" in ex and "range:loop" in ex and "stems:none" in ex, f"the choices are remembered, rate and depth from the project: {ex}")
    check(A, o("t28_tip") is True, "the disabled Selection choice says why (tooltip screenshot)")
    check(A, o("t28_wav") is True, "Enter exports")

# AT-29
a = load("at29_click")
if a:
    A = "AT-29"
    check(A, any("ab" in t for t in o("t29_typed", [])), f"typed into the search: {o('t29_typed')}")
    check(A, "field:" in o("t29_after_esc", []) and not any("ab" in t for t in o("t29_after_esc", [])), f"Esc clears it: {o('t29_after_esc')}")
    check(A, a["instruments"] == fx["instruments"] and [l["inst"] for l in a["lanes"]] == [l["inst"] for l in fx["lanes"]], "a click on a layout loads nothing")
    d = load("at29_dbl")
    check(A, "2A03" in (o("t29_dialog") or "") and o("t29_keep") is True, "a double-click asks about the profile (a template, decision 5)")
    check(A, d is not None and d["instruments"] != fx["instruments"], "and then loads the layout")

# AT-30
if "t30_sheet" in obs:
    A = "AT-30"
    for s in ["at30_menu_edit", "at30_menu_view", "at30_menu_help"]:
        check(A, os.path.exists(os.path.join(work, s + ".png")), f"screenshot {s}")
    t = o("t30_sheet", [])
    for item in ["Split", "Consolidate", "Rename", "Cut Time", "Insert Silence", "Zoom to Selection", "Draw Mode", "Narrow Grid"]:
        check(A, item in t, f"the shortcut sheet lists {item}")

# AT-31
if "t31_save" in obs:
    A = "AT-31"
    check(A, o("t31_save") is True and o("t31_presets") is True, "Save as Preset (a visible button, I3) wrote a user preset")
    got, raw = menu(o("t31_menu", []), "Preset")
    check(A, got is not None and got[:2] == ["Rename", "Delete"] and got[2].startswith("Show in"), f"a user preset's menu: {raw}")

print()
for at in sorted(results):
    ok = all(results[at])
    print(f"{at}: {'PASS' if ok else 'FAIL'} ({sum(results[at])}/{len(results[at])})")
sys.exit(1 if fails else 0)
