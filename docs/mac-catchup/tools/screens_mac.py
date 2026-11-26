#!/usr/bin/env python3
"""MAC_CATCHUP §9 screenshots (docs/screens/mac/) and the Polish 2 layout checks on macOS:
the menu bar, the shortcut sheet, the four header layouts, header widths, the browser on the
right, row heights, the Overview on and off, the status bar (a message, the problem icon, hover
help, the problem list).

usage: screens_mac.py [only,names]
"""
import json
import pathlib
import subprocess
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from mclib import *  # noqa: E402,F401,F403

OUT = REPO / "docs/screens/mac"
OUT.mkdir(parents=True, exist_ok=True)
TB = 32
SONG = REPO / "docs/polish2/songs/mockup.chipdaw"
ROWS = REPO / "docs/polish2/songs/mockup_rows.chipdaw"
only = set(sys.argv[1].split(",")) if len(sys.argv) > 1 else set()
QUIET = {"CHIPDAW_FROZEN_METERS": "1"}


def do(name):
    return not only or name in only


def crop(src, dst, x, y, w, h, label=None):
    tmp = WORK / "crop_tmp.png"
    img("crop", src, tmp if label else dst, x, y + TB, w, h)
    if label:
        img("label", tmp, dst, label)
    return dst


if do("menubar"):
    a = App("menubar", SONG, env=QUIET)
    # The window under the menu bar, so the open menus lie over the app only.
    winctl = BIN / "winctl"
    if not winctl.exists():
        subprocess.run(["swiftc", "-O", str(REPO / "docs/engine/tools/macos/winctl.swift"), "-o", str(winctl)], check=True, capture_output=True)
    subprocess.run([str(winctl), str(a.p.pid), "move", "0", "40"], capture_output=True)
    time.sleep(1)
    parts = []
    for top, label in (("app", "App menu (About, Settings… ⌘, Quit ⌘Q)"), ("File", "File"), ("Edit", "Edit"), ("View", "View")):
        a.seq(("menu", top), ("sleep", .7))
        raw = WORK / f"mb_{top}.png"
        subprocess.run(["screencapture", "-x", "-R0,0,760,560", str(raw)])
        a.seq(("key", "escape"), ("sleep", .3))
        dst = WORK / f"mb_{top}_l.png"
        img("label", raw, dst, label)
        parts.append(dst)
    row1, row2 = WORK / "mb_r1.png", WORK / "mb_r2.png"
    img("stack", "h", row1, *parts[:2])
    img("stack", "h", row2, *parts[2:])
    img("stack", "v", OUT / "mac-menubar.png", row1, row2)
    a.quit()

if do("shortcuts"):
    a = App("sheet", SONG, env=QUIET)
    a.seq(("menu", "Help", "Keyboard Shortcuts"), ("sleep", .7))
    s1 = a.shot("sheet1")
    a.seq(("scroll", 700, 500, -12), ("sleep", .4))
    s2 = a.shot("sheet2")
    a.seq(("scroll", 700, 500, -40), ("sleep", .4))
    s3 = a.shot("sheet3")
    c = [crop(s, WORK / f"sheet_c{i}.png", 400, 100, 640, 545) for i, s in enumerate((s1, s2, s3))]
    img("stack", "h", OUT / "mac-shortcuts.png", *c)
    a.quit()

if do("layouts"):
    parts = []
    for right, byclips in ((False, False), (False, True), (True, True), (True, False)):
        a = App("layout", SONG, env=QUIET, settings={"headers_right": right, "strip_by_clips": byclips, "show_overview": False})
        a.seq(("key", "b", "cmd,alt"), ("sleep", .5))  # browser closed: header + lanes only
        s = a.shot(f"layout_{int(right)}{int(byclips)}")
        x0 = 0
        parts.append(crop(s, WORK / f"lay_{int(right)}{int(byclips)}.png", x0, 76, 1440, 2 * 73 + 28, f"Headers {'Right' if right else 'Left'} · strip {'by the clips' if byclips else 'outside'}"))
        a.quit()
    img("stack", "v", OUT / "mac-layouts-4.png", *parts)
    # Widths 380 / 520 / 600 (header_width), the browser on the right, row heights.
    parts = []
    for w in (380, 520, 600):
        a = App("width", SONG, env=QUIET, settings={"header_width": w, "show_overview": False})
        s = a.shot(f"width_{w}")
        parts.append(crop(s, WORK / f"w_{w}.png", 0, 76, 1440, 73 + 28, f"Header width {w}"))
        a.quit()
    a = App("bright", SONG, env=QUIET, settings={"browser_right": True, "show_overview": False})
    s = a.shot("browser_right")
    parts.append(crop(s, WORK / "w_br.png", 0, 0, 1440, 76 + 2 * 73 + 28, "Browser right"))
    a.quit()
    a = App("rows", ROWS, env=QUIET, settings={"show_overview": False})
    s = a.shot("rows")
    parts.append(crop(s, WORK / "w_rows.png", 0, 76, 1440, 28 + 72 + 144 + 240 + 3, "Row heights 72 / 144 / 240"))
    a.quit()
    img("stack", "v", WORK / "mac-layouts-more.png", *parts)

if do("overview"):
    parts = []
    for on in (True, False):
        a = App("ov", SONG, env=QUIET, settings={"show_overview": on})
        s = a.shot(f"ov_{on}")
        parts.append(crop(s, WORK / f"ov_{on}.png", 0, 0, 1440, 76 + 32 + 28 + 2 * 73, f"Overview {'on' if on else 'off (View ▸ Show Overview, ⌥⌘O)'}"))
        a.quit()
    img("stack", "v", OUT / "mac-overview.png", *parts)

if do("status"):
    parts = []
    a = App("status", SONG, env=dict(QUIET, CHIPDAW_FAKE_PROBLEM="Audio: can't open the output device (test)."))
    a.seq(("key", "s", "cmd"), ("sleep", .8), ("move", 1300, 905), ("sleep", .4))
    s = a.shot("st_msg")
    parts.append(crop(s, WORK / "st1.png", 0, 860, 1440, 60, "A message (after ⌘S), with the problem icon"))
    a.seq(("move", 241 + 268 + 12, 136 + 22), ("sleep", 1.0))
    s = a.shot("st_help")
    parts.append(crop(s, WORK / "st2.png", 0, 860, 1440, 60, "Hover help (the mouse on track 1's M key)"))
    a.seq(("click", 22, 908), ("sleep", .8))
    s = a.shot("st_list")
    parts.append(crop(s, WORK / "st3.png", 0, 640, 1440, 280, "A click on the problem icon lists the problems"))
    a.quit()
    img("stack", "v", OUT / "mac-status-bar.png", *parts)
print("done")
