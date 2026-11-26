"""Checks what at_shell.ps1 saw (obs.json) and saved (*.chipdaw, settings.json) in its
work dir. Prints PASS/FAIL per check and a line per acceptance test; exit 1 on a failure.
A test whose files are missing (not run, see -Only) is reported as SKIPPED."""
import os as _os, sys as _sys
_sys.path.insert(0, _os.path.join(_os.path.dirname(_os.path.abspath(__file__)), "..", "..", "batch4a", "tools"))
import v7compat  # noqa: E402  (Batch 4a: format 7 read as 6, RUNLOG B4A-D17)
v7compat.install()
import json, os, sys

work = sys.argv[1]
obs = json.load(open(os.path.join(work, "obs.json"), encoding="utf-8-sig"))
fails, results = [], {}


def load(name):
    p = os.path.join(work, name)
    return json.load(open(p, encoding="utf-8")) if os.path.exists(p) else None


def check(at, cond, msg):
    print(("PASS " if cond else "FAIL ") + f"{at}: {msg}")
    results.setdefault(at, []).append(bool(cond))
    if not cond:
        fails.append(f"{at}: {msg}")


def o(key, default=""):
    v = obs.get(key, default)
    return default if v is None else v


def names(f):
    return [c["name"] for c in f["clips"]]


orig = json.load(open(os.path.join(os.path.dirname(__file__), "..", "..", "songs", "v2", "demo_band.chipdaw"), encoding="utf-8"))
N = len(orig["clips"])

# AT-01
if "t01_start" in obs:
    A = "AT-01"
    check(A, not o("t01_start").startswith("*") and "at01" in o("t01_start"), f"opens clean, named after the file: {o('t01_start')!r}")
    check(A, o("t01_dirty").startswith("*"), f"an edit marks the title: {o('t01_dirty')!r}")
    check(A, "before starting a new project" in o("t01_new_prompt"), "Ctrl+N with changes asks to save")
    check(A, o("t01_new_cancel") is True and o("t01_after_cancel").startswith("*"), "Cancel keeps the edited song")
    saved = load("at01_saved.chipdaw")
    check(A, saved is not None and len(saved["clips"]) == N - 1 and "Intro Lead" not in names(saved), "Save wrote the edit (one clip fewer)")
    check(A, not o("t01_after_save").startswith("*"), f"saving clears the mark: {o('t01_after_save')!r}")
    check(A, o("t01_after_undo").startswith("*"), "undo after the save: unsaved again")
    check(A, not o("t01_after_redo").startswith("*"), "redo back to the saved state: clean again")
    after = load("at01_after_dontsave.chipdaw")
    check(A, o("t01_dontsave") is True and o("t01_after_dontsave").startswith("Untitled"), f"Don't Save gives a new empty project: {o('t01_after_dontsave')!r}")
    check(A, after == saved, "Don't Save left the file as last saved")
    check(A, o("t01_clean_new_prompt") is False, "Ctrl+N with no changes: no prompt")
    check(A, o("t01_untitled_dirty").startswith("*Untitled"), "a new clip in the empty project marks it")
    check(A, "before opening another project" in o("t01_open_prompt") and o("t01_open_cancel") is True, "Ctrl+O asks; Cancel")
    check(A, o("t01_open_dontsave") is True and o("t01_open_filedlg_cancel") is True, "Ctrl+O, Don't Save: the Open dialog appears (cancelled)")
    check(A, "before replacing the song with a MIDI file" in o("t01_midi_prompt") and o("t01_midi_cancel") is True, "File > New from MIDI File asks")
    check(A, "before opening the dropped file" in o("t01_drop_prompt") and o("t01_drop_cancel") is True, "a dropped .chipdaw asks")
    check(A, o("t01_after_drop_cancel").startswith("*Untitled"), "Cancel on the drop keeps the song")
    check(A, "before closing" in o("t01_close_prompt") and o("t01_alive_after_cancel") is True, "closing the window asks; Cancel keeps it open")
    check(A, o("t01_close_dontsave") is True and o("t01_alive_after_dontsave") is False, "Don't Save closes the app")
    check(A, o("t01_marker") == "unsaved-quit" and o("t01_recovery_file") is True, "the unsaved work is kept for recovery (setting on by default)")
    check(A, "Unsaved changes from your last session" in o("t01_restart_texts"), "the next start offers it")

# AT-02
b, a = load("at02_before.chipdaw"), load("at02_after.chipdaw")
if b is not None:
    A = "AT-02"
    check(A, a == b, "F3-F7 without CHIPDAW_DEV: the saved project is identical")
    check(A, o("t02_diff_pixels", -1) == 0, f"F3-F7 without CHIPDAW_DEV: no overlay, the window is pixel-identical ({o('t02_diff_pixels')} px differ)")
    check(A, o("t02_dev_f3_diff", 0) > 1000, f"control: in dev mode F3 does show the overlay ({o('t02_dev_f3_diff')} px differ)")
    dev = load("at02_dev_after.chipdaw")
    check(A, dev is not None and len(dev["clips"]) != len(b["clips"]), f"control: in dev mode F4 does swap in the stress set ({len(dev['clips']) if dev else '?'} clips)")

# AT-03
s3 = load("at03_saved.chipdaw")
if s3 is not None:
    A = "AT-03"
    texts = o("t03_search_texts")
    check(A, "field:a b" in texts.split(" | ") or "field:a " in texts, f"the search field holds the typed text ({[t for t in texts.split(' | ') if t.startswith('field:')]})")
    check(A, not o("t03_title_after_typing").startswith("*"), "typing Space, Ctrl+Z, Delete in the field changed nothing in the song")
    check(A, len(s3["clips"]) == N, "no clip was deleted by Delete in the field")
    # Synthetic key presses without Shift type lower case.
    check(A, "verse" in names(s3) and "Intro Lead" not in names(s3), f"the rename survived Ctrl+Z in the field: {[n for n in names(s3) if n.lower() in ('verse', 'intro lead')]}")

# AT-04
s4 = load("at04_saved.chipdaw")
if s4 is not None:
    A = "AT-04"
    check(A, "VRC6 lanes have" in o("t04_prompt") and o("t04_switch") is True, "switching to 2A03 with VRC6 clips asks first")
    check(A, s4["chip"] == "2A03", f"the profile is saved: {s4['chip']}")
    check(A, len(s4["clips"]) == N, "a click + Delete on a dimmed VRC6 clip removed nothing")
    l = o("t04_luma", [1, 1, 1, 1])
    r0, r1 = l[0] / max(l[1], 1), l[2] / max(l[3], 1)
    check(A, r1 < r0 * 0.75, f"the VRC6 headers are drawn dimmed (brightness against the 2A03 headers {r0:.2f} -> {r1:.2f})")

# AT-06
s6 = load("at06_saved.chipdaw")
if s6 is not None:
    A = "AT-06"
    check(A, o("t06_dirty").startswith("*") and not o("t06_after_menu_save").startswith("*"), "File > Save from the menu saves (mark cleared)")
    check(A, o("t06_written") is True and len(s6["clips"]) == N - 1, "the file was written with the edit")
    check(A, all(os.path.exists(os.path.join(work, f"at06_menu_{m}.png")) for m in ("file", "edit", "create", "view", "help", "recent")), "a screenshot of every menu")

# AT-15
st = load("at15_settings.json")
if st is not None:
    A = "AT-15"
    check(A, "SETTINGS" in o("t15_open_texts"), "Ctrl+, opens Settings")
    check(A, st.get("buffer_frames") == 256, f"the buffer size is saved: {st.get('buffer_frames')}")
    check(A, st.get("frame_cap") == 120, f"the frame-rate cap is saved: {st.get('frame_cap')}")
    check(A, "SETTINGS" not in o("t15_modal_closed_texts"), "Esc closes it")

# AT-16
s16 = load("at16_saved.chipdaw")
if s16 is not None:
    A = "AT-16"
    check(A, s16["chip"] == "2A03" and s16["filter"].lower() in ("fc", "famicom"), f"profile and filter saved in the project: {s16['chip']}, {s16['filter']}")

# AT-18
s18 = load("at18_settings.json")
if s18 is not None:
    A = "AT-18"
    rec = [os.path.basename(p) for p in s18.get("recent", [])]
    check(A, rec[:2] == ["second.chipdaw", "first.chipdaw"], f"Open Recent: newest first: {rec}")
    check(A, "missing project" in o("t18_start_texts"), "a missing file is removed, with a notice")
    after = load("at18_settings_after.chipdaw.json") or {}
    check(A, not any(p.endswith("gone.chipdaw") for p in after.get("recent", [])), "and it is gone from settings.json")
    check(A, o("t18_opened").startswith("second"), f"Open Recent opens it: {o('t18_opened')!r}")
    check(A, "ABOUT CHIPDAW" in o("t18_about_texts") and "MIT OR Apache-2.0" in o("t18_about_texts"), "Help > About: version, licence, credits")
    sheet = o("t18_sheet_texts")
    check(A, "KEYBOARD SHORTCUTS" in sheet and "Ctrl+Shift+S" in sheet and "Ctrl+U" in sheet, "Help > Keyboard Shortcuts lists the table")

# AT-19
if "t19_saved" in obs:
    A = "AT-19"
    check(A, o("t19_saved") is True and not o("t19_title").startswith("*"), "Ctrl+S still saves")
    check(A, os.path.exists(os.path.join(work, "at19_topbar.png")), "top-bar screenshot taken (checked by eye: no file keys, no CHIP/FILTER/SNAP)")

print()
for at in sorted(results):
    print(f"{at}: {'PASS' if all(results[at]) else 'FAIL'} ({sum(results[at])}/{len(results[at])})")
sys.exit(1 if fails else 0)
