#!/usr/bin/env python3
"""MAC_CATCHUP §4: every shortcut and mouse modifier with real (HID) key and mouse events on macOS.

Each check acts through macdrive (CGEvent input, AX menu reads) and proves its result from the
saved project (⌘S), settings.json, the dev-mode stderr lines, the menu bar's state (AX) or a
screenshot measurement. Prints one PASS/FAIL line per check.

usage: scen_mac.py [--app target/release/app] [--only name,name] [--out results.txt]
Needs Accessibility and Screen Recording permission for the terminal.
"""
import argparse
import json
import os
import pathlib
import re
import subprocess
import sys
import time

REPO = pathlib.Path(__file__).resolve().parents[3]
WORK = pathlib.Path(os.environ.get("MC_WORK", os.path.join(os.environ.get("TMPDIR", "/tmp"), "chipdaw-mac-catchup")))
BIN = WORK / "bin"
BAR = 3840
PX_BAR = 92.0   # default zoom
X0 = 622        # bar 1's line, content x
ROW0, PITCH = 136, 73

ap = argparse.ArgumentParser()
ap.add_argument("--app", default=str(REPO / "target/release/app"))
ap.add_argument("--only", default="")
ap.add_argument("--out", default="")
args = ap.parse_args()
ONLY = set(filter(None, args.only.split(",")))

for tool in ("macdrive", "img"):
    src = REPO / f"docs/mac-catchup/tools/{tool}.swift"
    out = BIN / tool
    if not out.exists() or src.stat().st_mtime > out.stat().st_mtime:
        BIN.mkdir(parents=True, exist_ok=True)
        subprocess.run(["swiftc", "-O", str(src), "-o", str(out)], check=True, capture_output=True)

results = []


def report(name, ok, detail=""):
    line = f"{'PASS' if ok else 'FAIL'}  {name}: {detail}"
    print(line, flush=True)
    results.append(line)


def x_of(bar, frac=0.0):
    return X0 + (bar - 1 + frac) * PX_BAR


def y_of(lane, part="body"):
    top = ROW0 + PITCH * lane
    return top + (10 if part == "header" else 44)


class App:
    def __init__(self, name, project=None, env=None, settings=None):
        subprocess.run(["pkill", "-x", "app"], capture_output=True)
        time.sleep(0.5)
        self.name = name
        self.data = WORK / f"data-{name}"
        subprocess.run(["rm", "-rf", str(self.data)])
        self.data.mkdir(parents=True)
        s = {"audio_device": os.environ.get("MC_DEVICE", "Mac mini Speakers")}
        s.update(settings or {})
        (self.data / "settings.json").write_text(json.dumps(s))
        e = dict(os.environ, CHIPDAW_DEV="1", CHIPDAW_DATA_DIR=str(self.data))
        self.proj = None
        if project:
            self.proj = WORK / f"{name}.chipdaw"
            self.proj.write_bytes(pathlib.Path(project).read_bytes())
            e["CHIPDAW_PROJECT"] = str(self.proj)
        e.update(env or {})
        self.log_path = WORK / f"{name}.log"
        self.p = subprocess.Popen([args.app], env=e, stdout=open(self.log_path, "w"), stderr=subprocess.STDOUT)
        for _ in range(60):
            if "window" in self.drv("info"):
                break
            time.sleep(0.2)
        time.sleep(1.5)
        self.drv("focus")
        self.drv("move", 1300, 860)

    def drv(self, *cmd):
        out = subprocess.run([str(BIN / "macdrive"), str(self.p.pid)] + [str(c) for c in cmd], capture_output=True, text=True)
        return out.stdout

    def seq(self, *cmds):
        """Several macdrive commands in one call: seq(("key","d","cmd"), ("sleep",0.2))."""
        flat = []
        for c in cmds:
            if flat:
                flat.append(";")
            flat += [str(x) for x in c]
        return self.drv(*flat)

    def log(self):
        return self.log_path.read_text(errors="replace")

    def mark(self):
        return len(self.log())

    def since(self, m):
        return self.log()[m:]

    def save(self):
        before = self.proj.stat().st_mtime_ns
        self.drv("key", "s", "cmd")
        for _ in range(40):
            time.sleep(0.1)
            if self.proj.stat().st_mtime_ns != before:
                break
        time.sleep(0.2)
        return json.loads(self.proj.read_text())

    def doc(self):
        return json.loads(self.proj.read_text())

    def settings(self):
        return json.loads((self.data / "settings.json").read_text())

    def menu(self, item):
        """(enabled, checked, key) of a menu item, from the menu bar."""
        for line in self.drv("menus").splitlines():
            t = line.strip()
            checked = t.startswith("✓ ")
            t = t[2:] if checked else t
            name = t.split("\t")[0].replace(" [disabled]", "")
            if name == item:
                return ("[disabled]" not in t, checked, t.split("\t")[1] if "\t" in t else "")
        return None

    def title(self):
        return self.drv("title").strip()

    def shot(self, name):
        path = WORK / f"{name}.png"
        self.drv("shot", path)
        return path

    def quit(self):
        self.p.kill()
        self.p.wait()


def img(*a):
    return subprocess.run([str(BIN / "img")] + [str(x) for x in a], capture_output=True, text=True).stdout.strip()


def mean(path, x, y, w, h):
    return float(img("mean", path, x, y + 32, w, h))  # screenshots include the 32 pt title bar


def clips(d, lane=None):
    return sorted([c for c in d["clips"] if lane is None or c["lane"] == lane], key=lambda c: (c["lane"], c["start"]))


def by_name(d, name):
    return [c for c in d["clips"] if c["name"] == name]


def want(name):
    return not ONLY or name in ONLY


FIX = WORK / "fixture.chipdaw"
subprocess.run([sys.executable, str(REPO / "docs/mac-catchup/tools/fixtures_mac.py"), str(FIX)], check=True)

# ---------------------------------------------------------------- File / app commands
if want("file"):
    a = App("file", FIX)
    t0 = a.title()
    a.seq(("click", x_of(5, .5), y_of(3, "header")), ("key", "d", "cmd"), ("sleep", .3))
    report("Cmd+D marks unsaved (title)", a.title().startswith("*"), f"{t0!r} -> {a.title()!r}")
    d = a.save()
    report("Cmd+S saves", len(d["clips"]) == 5 and not a.title().startswith("*"), f"{len(d['clips'])} clips, title {a.title()!r}")
    # Cmd+N on a clean document: no prompt, an empty Untitled project.
    a.seq(("key", "n", "cmd"), ("sleep", .8))
    w = a.drv("windows")
    report("Cmd+N (clean): no prompt, Untitled", "sheet" not in w and a.title().startswith("Untitled"), a.title())
    # Dirty + Cmd+N: the sheet, Cancel keeps the work.
    a.quit()
    a = App("file2", FIX)
    a.seq(("click", x_of(1, .5), y_of(0, "header")), ("key", "d", "cmd"), ("sleep", .3), ("key", "n", "cmd"), ("sleep", .8))
    w = a.drv("windows")
    report("Cmd+N (dirty): Save / Don't Save / Cancel sheet", all(b in w for b in ("'Save'", "'Don't Save'", "'Cancel'")), " ".join(re.findall(r"button '(.*)'", w)))
    a.seq(("press", "Cancel"), ("sleep", .5))
    report("… Cancel keeps the edit", a.title().startswith("*file2"), a.title())
    a.seq(("key", "n", "cmd"), ("sleep", .8), ("press", "Don't Save"), ("sleep", .8))
    report("… Don't Save: a new Untitled project", a.title().startswith("Untitled"), a.title())
    # Cmd+O: the open panel; Esc cancels.
    a.seq(("key", "o", "cmd"), ("sleep", 1.2))
    w = a.drv("windows")
    report("Cmd+O: the open panel", w.count("window role") >= 2 or "Open" in w, w.replace("\n", " | ")[:160])
    a.seq(("key", "escape"), ("sleep", .8))
    report("… Esc closes it", a.drv("windows").count("window role") == 1, "")
    # Cmd+, : Settings (in-app modal: the backdrop darkens the lanes).
    base = a.shot("file_base")
    a.seq(("key", ",", "cmd"), ("sleep", .6))
    s = a.shot("file_settings")
    lb, ls = mean(base, 700, 600, 300, 100), mean(s, 700, 600, 300, 100)
    report("Cmd+, opens Settings (a panel over the lanes)", abs(ls - lb) > 2, f"luma {lb:.1f} -> {ls:.1f}")
    a.seq(("key", "escape"), ("sleep", .4))
    # Shift+Cmd+R: the export dialog.
    a.seq(("key", "r", "shift,cmd"), ("sleep", .6))
    e = a.shot("file_export")
    a.seq(("key", "escape"), ("sleep", .4))
    after = a.shot("file_after_esc")
    le, la = mean(e, 700, 400, 200, 100), mean(after, 700, 400, 200, 100)
    report("Shift+Cmd+R opens Export, Esc closes it", abs(le - la) > 2 and abs(la - mean(base, 700, 400, 200, 100)) < 1, f"luma {la:.1f} -> {le:.1f} -> {la:.1f}")
    a.quit()

# ---------------------------------------------------------------- Edit commands, arrangement
if want("edit"):
    a = App("edit", FIX)
    A = (x_of(1, .5), y_of(0, "header"))
    B = (x_of(3, .5), y_of(0, "header"))
    a.seq(("click", *A), ("key", "c", "cmd"), ("click", x_of(7), y_of(1)), ("key", "v", "cmd"), ("sleep", .3))
    d = a.save()
    pasted = [c for c in d["clips"] if c["name"] == "A" and c["start"] != 0]
    report("Cmd+C, click bar 7 on lane 2, Cmd+V", len(d["clips"]) == 5 and len(pasted) == 1, f"pasted at lane {pasted[0]['lane'] if pasted else '-'} start {pasted[0]['start'] / BAR + 1 if pasted else '-'}")
    a.seq(("click", *B), ("key", "x", "cmd"), ("sleep", .3))
    d = a.save()
    report("Cmd+X cuts", len(by_name(d, "B")) == 0 and len(d["clips"]) == 4, f"{len(d['clips'])} clips")
    a.seq(("key", "z", "cmd"), ("sleep", .3))
    d = a.save()
    report("Cmd+Z restores", len(by_name(d, "B")) == 1, f"{len(d['clips'])} clips")
    a.seq(("key", "y", "cmd"), ("sleep", .3))
    d = a.save()
    report("Cmd+Y redoes", len(by_name(d, "B")) == 0, f"{len(d['clips'])} clips")
    a.seq(("key", "z", "cmd"), ("sleep", .2), ("click", *B), ("key", "delete"), ("sleep", .3))
    d = a.save()
    report("Delete (⌫) deletes the clip", len(by_name(d, "B")) == 0, f"{len(d['clips'])} clips")
    a.seq(("key", "z", "cmd"), ("sleep", .2), ("click", *B), ("key", "fwddelete"), ("sleep", .3))
    d = a.save()
    report("Forward delete (fn+⌫) deletes the clip", len(by_name(d, "B")) == 0, f"{len(d['clips'])} clips")
    a.seq(("key", "z", "cmd"), ("sleep", .2), ("click", 1300, 860), ("key", "a", "cmd"), ("sleep", .2))
    report("Cmd+A selects every clip (Duplicate enabled)", a.menu("Duplicate")[0], str(a.menu("Duplicate")))
    a.seq(("key", "escape"), ("sleep", .2))
    report("Esc clears the selection (Duplicate disabled)", not a.menu("Duplicate")[0], str(a.menu("Duplicate")))
    # Split at the insert marker (a click inside A at bar 2), then consolidate.
    a.seq(("click", x_of(2), y_of(0)), ("key", "e", "cmd"), ("sleep", .3))
    d = a.save()
    lane0 = clips(d, 0)
    report("Cmd+E splits A at bar 2", len(lane0) == 3 and lane0[0]["len"] == BAR, " ".join(f"{c['name']}@{c['start'] // BAR + 1}+{c['len'] / BAR:g}" for c in lane0))
    a.seq(("click", x_of(1, .5), y_of(0, "header")), ("click", x_of(2, .5), y_of(0, "header"), "shift"), ("key", "j", "cmd"), ("sleep", .3))
    d = a.save()
    lane0 = clips(d, 0)
    report("Shift-click adds, Cmd+J consolidates", len(lane0) == 2 and lane0[0]["len"] == 2 * BAR and len(lane0[0]["notes"]) == 8, " ".join(f"{c['name']}@{c['start'] // BAR + 1}+{c['len'] / BAR:g} n{len(c['notes'])}" for c in lane0))
    # Shift-click toggles a clip in the selection (every platform; UX A4's Ctrl/⌘-click
    # toggle isn't built on Windows either: shared, not a Mac item).
    a.seq(("click", *A), ("click", *B, "shift"), ("sleep", .2))
    two = not a.menu("Rename")[0] and a.menu("Duplicate")[0]
    a.seq(("click", *B, "shift"), ("sleep", .2))
    report("Shift-click toggles a clip (two selected, then one)", two and a.menu("Rename")[0], "")
    # Cmd+R renames in place.
    a.seq(("click", *A), ("key", "r", "cmd"), ("sleep", .3), ("key", "a", "cmd"), ("type", "Verse"), ("key", "return"), ("sleep", .3))
    d = a.save()
    report("Cmd+R renames the clip", any(c["name"] == "Verse" for c in d["clips"]), str(sorted(c["name"] for c in d["clips"])))
    # Arrows: nudge by the grid; Cmd+2 widens the grid.
    s0 = [c for c in a.doc()["clips"] if c["name"] == "Verse"][0]["start"]
    a.seq(("click", *A), ("key", "right"), ("key", "right"), ("sleep", .2))
    s1 = [c for c in a.save()["clips"] if c["name"] == "Verse"][0]["start"]
    a.seq(("key", "2", "cmd"), ("key", "right"), ("sleep", .2))
    s2 = [c for c in a.save()["clips"] if c["name"] == "Verse"][0]["start"]
    report("→ → nudges 2 × 1/16, Cmd+2 then → one 1/8", (s1 - s0, s2 - s1) == (480, 480), f"{s1 - s0} then {s2 - s1} ticks")
    a.seq(("key", "1", "cmd"), ("key", "left"), ("key", "left"), ("key", "left"), ("sleep", .2))
    a.quit()

# ---------------------------------------------------------------- Time commands
if want("time"):
    a = App("time", FIX)
    # A time selection over bars 7-8 on the empty lane 2, extended over lanes 1-4 with Shift-click.
    a.seq(("drag", x_of(7), y_of(1), x_of(8), y_of(1), "", 20, 10), ("sleep", .2), ("key", "m", "shift,cmd"), ("sleep", .3))
    d = a.save()
    new = [c for c in d["clips"] if c["lane"] == 1]
    report("Shift+Cmd+M inserts a MIDI clip in the time selection", len(new) == 1 and new[0]["start"] == 6 * BAR, f"{[(c['start'] // BAR + 1, c['len'] / BAR) for c in new]}")
    a.seq(("key", "z", "cmd"), ("drag", x_of(2), y_of(1), x_of(3), y_of(1), "", 20, 10), ("click", x_of(2, .5), y_of(3), "shift"), ("key", "i", "cmd"), ("sleep", .3))
    d = a.save()
    C = by_name(d, "C")[0]
    Dc = by_name(d, "D")[0]
    report("Cmd+I inserts a bar of silence on every track", Dc["start"] == 5 * BAR, f"D now at bar {Dc['start'] / BAR + 1:g}")
    a.seq(("key", "delete", "shift,cmd"), ("sleep", .3))
    d = a.save()
    report("Shift+Cmd+⌫ deletes the time again", by_name(d, "D")[0]["start"] == 4 * BAR, f"D at bar {by_name(d, 'D')[0]['start'] / BAR + 1:g}")
    a.seq(("drag", x_of(5), y_of(3), x_of(7), y_of(3), "", 20, 10), ("key", "d", "shift,cmd"), ("sleep", .3))
    d = a.save()
    report("Shift+Cmd+D duplicates the time", len(d["clips"]) >= 5, f"{len(d['clips'])} clips; D starts {[c['start'] // BAR + 1 for c in d['clips'] if c['lane'] == 3]}")
    a.seq(("key", "z", "cmd"), ("drag", x_of(5), y_of(3), x_of(7), y_of(3), "", 20, 10), ("key", "c", "shift,cmd"), ("click", x_of(7), y_of(3)), ("key", "v", "shift,cmd"), ("sleep", .3))
    d = a.save()
    report("Shift+Cmd+C, Shift+Cmd+V pastes time", len([c for c in d["clips"] if c["lane"] == 3]) == 2, f"lane 4 starts {[c['start'] // BAR + 1 for c in d['clips'] if c['lane'] == 3]}")
    a.seq(("key", "z", "cmd"), ("drag", x_of(5), y_of(3), x_of(7), y_of(3), "", 20, 10), ("key", "x", "shift,cmd"), ("sleep", .3))
    d = a.save()
    report("Shift+Cmd+X cuts time", not [c for c in d["clips"] if c["lane"] == 3], f"lane 4: {[c['start'] for c in d['clips'] if c['lane'] == 3]}")
    a.seq(("key", "z", "cmd"), ("drag", x_of(1), y_of(1), x_of(3), y_of(1), "", 20, 10), ("key", "l", "cmd"), ("sleep", .3))
    d = a.save()
    report("Cmd+L loops the selection", d["loop"] and d["loop_start"] == 0 and d["loop_end"] == 2 * BAR, f"loop {d['loop']} {d['loop_start']}..{d['loop_end']}")
    a.quit()

# ---------------------------------------------------------------- Transport and view keys
if want("view"):
    a = App("view", FIX)
    def disp(p, tag):
        out = WORK / f"disp_{tag}.png"
        img("crop", p, out, 500, 40 + 32, 380, 56)  # TIME and POSITION cells
        return out
    s0 = a.shot("v0")
    a.seq(("key", "space"), ("sleep", 1.0))
    s1 = a.shot("v1")
    a.seq(("key", "space"), ("sleep", .3))
    report("Space plays (the display's TIME moves)", not img("diff", disp(s0, 0), disp(s1, 1), 20).startswith("0 "), img("diff", disp(s0, 0), disp(s1, 1), 20))
    m = a.mark()
    a.seq(("key", "home"), ("sleep", .4))
    s2 = a.shot("v2")
    report("Home rewinds (TIME and POSITION back to 0:00.00, 1.1.1)", img("diff", disp(s0, 0), disp(s2, 2), 20).startswith("0 "), img("diff", disp(s0, 0), disp(s2, 2), 20))
    m = a.mark()
    # One macdrive call so the taps are 0.5 s apart (a key press takes 0.14 s).
    a.seq(("key", "t"), ("sleep", .36), ("key", "t"), ("sleep", .36), ("key", "t"), ("sleep", .36), ("key", "t"), ("sleep", .3))
    taps = re.findall(r"dev: tap bpm ([0-9.]+)", a.since(m))
    report("T taps the tempo (4 taps 0.5 s apart: ~120)", bool(taps) and 110 < float(taps[-1]) < 130, f"bpm {taps}")
    a.seq(("key", "z", "cmd"), ("sleep", .2))
    for item, keys in (("Browser", ("b", "cmd,alt")), ("Follow", ("f", "alt,shift")), ("Show Overview", ("o", "cmd,alt")), ("Draw Mode", ("b", ""))):
        before = a.menu(item)
        a.seq(("click", 1300, 860), ("key", *keys), ("sleep", .4))
        after = a.menu(item)
        a.seq(("key", *keys), ("sleep", .4))
        back = a.menu(item)
        report(f"{'+'.join(k for k in keys[::-1] if k)} toggles View ▸ {item} (key text {before[2]!r})", before[1] != after[1] and back[1] == before[1], f"{before[1]} -> {after[1]} -> {back[1]}")
    m = a.mark()
    a.seq(("click", 1300, 860), ("key", "w"), ("sleep", .3), ("key", "w"), ("sleep", .3))
    out = a.since(m)
    report("W fits the song, W again goes back", "zoom FitSong" in out and "ZoomBack" in out, " / ".join(re.findall(r"dev: zoom (\w+)", out)))
    r0 = a.shot("z0")
    a.seq(("key", "="), ("sleep", .3))
    r1 = a.shot("z1")
    a.seq(("key", "-"), ("sleep", .3))
    r2 = a.shot("z2")
    report("= zooms in, - zooms out (the ruler changes, then is back)", not img("diff", r0, r1, 8).startswith("0 ") and img("diff", r0, r2, 8).startswith("0 "), f"{img('diff', r0, r1, 8)} / {img('diff', r0, r2, 8)}")
    m = a.mark()
    a.seq(("drag", x_of(3), y_of(1), x_of(5), y_of(1), "", 20, 10), ("key", "z"), ("sleep", .3), ("key", "x"), ("sleep", .3))
    out = a.since(m)
    report("Z zooms to the time selection, X back", "ZoomToSelection" in out and "ZoomBack" in out, " / ".join(re.findall(r"dev: zoom (\w+)", out)))
    m = a.mark()
    a.seq(("click", x_of(1, .5), y_of(0, "header")), ("key", "return"), ("sleep", .6))
    report("Return opens the clip in the piano roll", "piano roll bounds" in a.since(m), a.since(m).strip()[:120])
    a.quit()

# ---------------------------------------------------------------- Piano roll keys
if want("piano"):
    a = App("piano", FIX, env={"CHIPDAW_OPEN_CLIP": "1"})
    m = a.log()
    bounds = re.search(r"piano roll bounds ([0-9.]+) ([0-9.]+) ([0-9.]+) ([0-9.]+)", m)
    report("CHIPDAW_OPEN_CLIP opens clip A", bool(bounds), bounds.group(0) if bounds else m[-200:])
    px, py, pw, ph = (float(v) for v in bounds.groups()) if bounds else (400, 500, 900, 300)
    a.seq(("click", px + pw * .6, py + ph * .5), ("key", "a", "cmd"), ("sleep", .2))
    notes0 = sorted((n["start"], n["pitch"], n["len"]) for n in a.save()["clips"][0]["notes"])

    def notes():
        d = a.save()
        return sorted((n["start"], n["pitch"], n["len"]) for n in [c for c in d["clips"] if c["id"] == 1][0]["notes"])

    a.seq(("key", "up"), ("sleep", .2))
    n1 = notes()
    report("↑ transposes a semitone", [p for _, p, _ in n1] == [p + 1 for _, p, _ in notes0], f"{notes0[0][1]} -> {n1[0][1]}")
    a.seq(("key", "up", "shift"), ("sleep", .2))
    n2 = notes()
    report("Shift+↑ transposes an octave", [p for _, p, _ in n2] == [p + 12 for _, p, _ in n1], f"{n1[0][1]} -> {n2[0][1]}")
    a.seq(("key", "down", "shift"), ("key", "down"), ("key", "right"), ("sleep", .2))
    n3 = notes()
    report("→ nudges by the grid (1/16)", [s for s, _, _ in n3] == [s + 240 for s, _, _ in notes0], f"{notes0[0][0]} -> {n3[0][0]}")
    a.seq(("key", "left"), ("key", "right", "shift"), ("sleep", .2))
    n4 = notes()
    report("Shift+→ lengthens by the grid", [l for _, _, l in n4] == [l + 240 for _, _, l in notes0], f"{notes0[0][2]} -> {n4[0][2]}")
    a.seq(("key", "left", "shift"), ("key", "u", "cmd"), ("sleep", .2))
    n5 = notes()
    report("Cmd+U quantizes (the 37-tick note snaps)", all(s % 240 == 0 for s, _, _ in n5) and any(s % 240 for s, _, _ in notes0), f"off-grid {[s for s, _, _ in notes0 if s % 240]} -> {[s for s, _, _ in n5 if s % 240]}")
    a.seq(("key", "e", "cmd"), ("sleep", .2))
    n6 = notes()
    report("Cmd+E chops the notes on the grid", len(n6) > len(n5), f"{len(n5)} -> {len(n6)} notes")
    a.seq(("key", "a", "cmd"), ("key", "j", "cmd"), ("sleep", .2))
    n7 = notes()
    report("Cmd+J joins them", len(n7) < len(n6), f"{len(n6)} -> {len(n7)} notes")
    a.seq(("key", "z", "cmd"), ("key", "z", "cmd"), ("key", "a", "cmd"), ("key", "d", "cmd"), ("sleep", .2))
    n8 = notes()
    report("Cmd+D duplicates the notes", len(n8) == 2 * len(n5), f"{len(n5)} -> {len(n8)}")
    a.seq(("key", "delete"), ("sleep", .2))
    n9 = notes()
    report("⌫ deletes the selected notes", len(n9) == len(n5), f"{len(n8)} -> {len(n9)}")
    a.seq(("key", "escape"), ("sleep", .3), ("key", "a", "cmd"), ("key", "c", "cmd"), ("key", "escape"), ("sleep", .2))
    a.quit()

# ---------------------------------------------------------------- Mouse modifiers
if want("mouse"):
    a = App("mouse", FIX)
    A = (x_of(1, .5), y_of(0, "header"))
    a.seq(("drag", *A, x_of(5, .5), y_of(1, "header"), "alt", 25, 12), ("sleep", .3))
    d = a.save()
    copies = by_name(d, "A")
    report("⌥-drag copies a clip (original stays)", len(copies) == 2 and any(c["start"] == 0 and c["lane"] == 0 for c in copies), str([(c["lane"], c["start"] / BAR + 1) for c in copies]))
    a.seq(("key", "z", "cmd"), ("drag", *A, x_of(5, .37), y_of(1, "header"), "", 25, 12), ("sleep", .3))
    snapped = by_name(a.save(), "A")[0]["start"]
    a.seq(("key", "z", "cmd"), ("drag", *A, x_of(5, .37), y_of(1, "header"), "cmd", 25, 12), ("sleep", .3))
    free = by_name(a.save(), "A")[0]["start"]
    report("A drag snaps; ⌘-drag doesn't", snapped % 240 == 0 and free % 240 != 0, f"snapped {snapped}, ⌘ {free}")
    a.seq(("key", "z", "cmd"), ("sleep", .2))
    # ⌥+wheel over a track header: its height (12 px a notch); without ⌥ nothing changes.
    a.seq(("scroll", 400, y_of(1), 1, "alt"), ("sleep", .3))
    d = a.save()
    h = [l.get("height") for l in d["lanes"]]
    report("⌥+wheel over track 2 grows it 12 px", h[1] == 84, f"heights {h}")
    # ⌥ while dragging a row's bottom edge: every track.
    a.seq(("key", "z", "cmd"), ("drag", 400, ROW0 + PITCH - 2, 400, ROW0 + PITCH + 70, "alt", 25, 12), ("sleep", .3))
    d = a.save()
    h = [l.get("height") for l in d["lanes"]]
    report("⌥-drag of a row's bottom edge sets every track", len(set(h)) == 1 and h[0] and h[0] > 72, f"heights {h}")
    a.seq(("key", "z", "cmd"), ("sleep", .3))
    # Solo: exclusive; ⌘-click adds.
    def solo_at(lane):
        return (540, ROW0 + PITCH * lane + 22)
    a.seq(("click", *solo_at(0)), ("click", *solo_at(1)), ("click", *solo_at(2), "cmd"), ("sleep", .3))
    d = a.save()
    report("S on 1 then 2 = only 2; ⌘-click S on 3 adds", [l["solo"] for l in d["lanes"][:3]] == [False, True, True], str([l["solo"] for l in d["lanes"][:4]]))
    a.seq(("click", *solo_at(1)), ("click", *solo_at(2), "cmd"), ("sleep", .2))
    # ⌘+wheel zooms the arrangement.
    z0 = a.shot("mz0")
    a.seq(("scroll", x_of(4), y_of(5), 2, "cmd"), ("sleep", .3))
    z1 = a.shot("mz1")
    report("⌘+wheel zooms the lanes", not img("diff", z0, z1, 8).startswith("0 "), img("diff", z0, z1, 8))
    a.seq(("scroll", x_of(4), y_of(5), -2, "cmd"), ("sleep", .3))
    # Wheel scrolls in time; Shift+wheel too (AppKit turns it into a horizontal delta).
    m = a.mark()
    a.seq(("scroll", x_of(4), y_of(5), -3), ("sleep", .3))
    s1 = a.shot("ms1")
    # Shift+wheel: macOS delivers it as a horizontal delta (wheel up = left, clamped at bar 1),
    # so scroll one way and check the other comes back.
    a.seq(("scroll", x_of(4), y_of(5), 3), ("sleep", .3), ("scroll", x_of(4), y_of(5), 3, "shift"), ("sleep", .3))
    s2 = a.shot("ms2")
    a.seq(("scroll", x_of(4), y_of(5), -3, "shift"), ("sleep", .3))
    s3 = a.shot("ms3")
    report("Shift+wheel back the other way returns to bar 1", img("diff", z0, s3, 8).startswith("0 ") or int(img("diff", z0, s3, 8).split()[0]) < 1000, img("diff", z0, s3, 8))
    report("Wheel and Shift+wheel scroll in time", not img("diff", z0, s1, 8).startswith("0 ") and not img("diff", z0, s2, 8).startswith("0 "), f"wheel {img('diff', z0, s1, 8)}; shift {img('diff', z0, s2, 8)}")
    a.quit()

# ---------------------------------------------------------------- Quit and close
if want("quit"):
    a = App("quit", FIX)
    a.seq(("click", x_of(1, .5), y_of(0, "header")), ("key", "d", "cmd"), ("sleep", .3), ("key", "q", "cmd"), ("sleep", .8))
    w = a.drv("windows")
    report("Cmd+Q with changes: the save sheet", "'Don't Save'" in w, " ".join(re.findall(r"button '(.*)'", w)))
    a.seq(("press", "Cancel"), ("sleep", .5))
    report("… Cancel: still running", a.p.poll() is None, "")
    # The window's close button (the last window): the same sheet.
    info = a.drv("info")
    fx, fy = (float(v) for v in re.search(r"frame \(([0-9.]+), ([0-9.]+)", info).groups())
    subprocess.run([str(BIN / "macdrive"), str(a.p.pid), "click", "20", "-14"], capture_output=True)
    time.sleep(.8)
    w = a.drv("windows")
    report("The close button with changes: the same sheet", "'Don't Save'" in w, " ".join(re.findall(r"button '(.*)'", w)))
    a.seq(("press", "Don't Save"), ("sleep", 1.5))
    report("… Don't Save: the app quits (closing the last window quits, MC-D6)", a.p.poll() is not None, f"exit {a.p.poll()}")
    rec = list(a.data.glob("recovery*"))
    report("… and keeps the recovery file", bool(rec), str([r.name for r in rec]))
    a = App("quit2", FIX)
    a.seq(("key", "q", "cmd"), ("sleep", 1.2))
    report("Cmd+Q, clean: quits at once", a.p.poll() is not None, f"exit {a.p.poll()}")

if args.out:
    pathlib.Path(args.out).write_text("\n".join(results) + "\n")
print(f"{sum(r.startswith('PASS') for r in results)} pass, {sum(r.startswith('FAIL') for r in results)} fail")
