#!/usr/bin/env python3
"""MAC_CATCHUP §6: the POLISH2.md performance scenarios on macOS with chipbench (process CPU,
P-core share, energy, instructions; PERF-macos.md's method), at the default 60 fps cap.

Builds are interleaved per round (CPU % on Apple Silicon is only comparable within a session;
instructions per presented frame is the stable measure, PERF-macos.md §5).

usage: perf_mac.py --label L --rounds N --display 320|60 [--only a,b] build=path ... > results.jsonl
  build: "new=target/release/app" or "base=../perf-macos-base/target/release/app:old" (":old" =
  the pre-design layout: 1440 x 900 window, the clip grab at 300,160, no Overview/E1 gestures).
"""
import argparse
import json
import os
import pathlib
import subprocess
import sys
import time

REPO = pathlib.Path(__file__).resolve().parents[3]
WORK = pathlib.Path(os.environ.get("MC_WORK", os.path.join(os.environ.get("TMPDIR", "/tmp"), "chipdaw-mac-catchup")))
BIN = WORK / "bin"
ap = argparse.ArgumentParser()
ap.add_argument("--label", default="perf")
ap.add_argument("--rounds", type=int, default=3)
ap.add_argument("--secs", type=int, default=12)
ap.add_argument("--display", default="320")
ap.add_argument("--only", default="")
ap.add_argument("builds", nargs="+")
a = ap.parse_args()
ONLY = set(filter(None, a.only.split(",")))

src = REPO / "docs/perf/tools/macos/chipbench.swift"
cb = BIN / "chipbench"
if not cb.exists() or src.stat().st_mtime > cb.stat().st_mtime:
    subprocess.run(["swiftc", "-O", str(src), "-o", str(cb)], check=True, capture_output=True)
piano = WORK / "songs" / "piano_bench.chipdaw"
if not piano.exists():
    (WORK / "songs").mkdir(parents=True, exist_ok=True)
    subprocess.run(["cargo", "run", "--release", "-q", "-p", "ui", "--example", "make_songs", "--", str(WORK / "songs")], cwd=REPO, check=True, capture_output=True)

POS = {"320": None, "60": "560,-1312"}[a.display]
PIANO_ENV = [f"CHIPDAW_PROJECT={piano}", "CHIPDAW_OPEN_CLIP=1", "CHIPDAW_PIANO_ZOOM=32"]
# name: (scenario, extra chipbench args, env, new-layout only)
NEW = {
    "idle": ("idle", [], [], False),
    "demo": ("demo", [], [], False),
    "200": ("200", [], [], False),
    "200drag": ("200-drag", ["--grab", "760,172", "--sway", "120,64"], [], False),
    "piano_play": ("demo", [], PIANO_ENV, False),
    "piano_drag": ("play-drag", ["--grab", "88,623", "--sway", "12,48"], PIANO_ENV, False),
    "row_resize": ("200-drag", ["--grab", "441,207", "--sway", "0,60"], [], True),
    "header_width": ("200-drag", ["--grab", "619,600", "--sway", "100,0"], [], True),
    "ruler_zoom": ("200-drag", ["--grab", "900,122", "--sway", "6,70"], [], True),
    "overview_drag": ("200-drag", ["--grab", "680,92", "--sway", "160,0"], [], True),
    "overview_zoom": ("200-drag", ["--grab", "680,92", "--sway", "4,50"], [], True),
    # Like-for-like for the Overview drag (POLISH2.md §4.6): the time scrollbar's thumb at the
    # lanes' bottom (above the status bar; at the window's bottom in the old layout).
    "scrollbar_drag": ("200-drag", ["--grab", "650,890", "--sway", "160,0"], [], False),
}


def run(build, path, old, name):
    sc, extra, env, new_only = NEW[name]
    if old and (new_only or name.startswith("piano")):
        return None   # the old build has neither the Polish 2 gestures nor today's file format
    if old and name == "200drag":
        extra = ["--grab", "300,160", "--sway", "120,64"]
    if old and name == "scrollbar_drag":
        extra = ["--grab", "650,894", "--sway", "160,0"]
    if old and name == "piano_drag":
        extra = ["--grab", "88,535", "--sway", "12,48"]
    data = WORK / f"data-perf-{build}"
    subprocess.run(["rm", "-rf", str(data)])
    data.mkdir(parents=True)
    (data / "settings.json").write_text(json.dumps({"audio_device": "Mac mini Speakers"}))
    fl = WORK / f"fl_{a.label}_{build}_{name}.csv"
    args = [str(cb), "--app", path, "--scenario", sc, "--secs", str(a.secs), "--content-height", "900" if old else "920",
            "--park", "1300,800" if not old else "700,40", "--framelog", str(fl),
            "--env", "CHIPDAW_DEV=1", "--env", f"CHIPDAW_DATA_DIR={data}"] + extra
    for e in env:
        args += ["--env", e]
    if POS:
        args += ["--pos", POS]
    subprocess.run(["pkill", "-x", "app"], capture_output=True)
    time.sleep(0.5)
    out = subprocess.run(args, capture_output=True, text=True, timeout=a.secs + 60).stdout.strip().splitlines()
    res = json.loads(out[-1]) if out else {}
    p = subprocess.run([sys.executable, str(REPO / "docs/perf/tools/macos/pacing_mac.py"), str(fl), str(a.secs), "--json"], capture_output=True, text=True)
    try:
        pj = json.loads(p.stdout)
        res["fps"] = round(pj.get("fps", 0), 1)
        res["frames_gt20"] = pj.get("over20"); res["iv_p99"] = round(pj.get("iv_p99", 0), 2); res["iv_max"] = round(pj.get("iv_max", 0), 2)
    except Exception:
        res["fps"] = 0
    if res.get("fps"):
        res["minstr_per_frame"] = round(res["ginstr_per_s"] * 1000 / res["fps"], 2)
    res.update(build=build, name=name, display=a.display, label=a.label)
    return res


builds = []
for b in a.builds:
    name, path = b.split("=", 1)
    old = path.endswith(":old")
    p = path[:-4] if old else path
    builds.append((name, str(pathlib.Path(p).resolve()), old))
therm = WORK / "bin" / "thermlog"
for r in range(a.rounds):
    for name in NEW:
        if ONLY and name not in ONLY:
            continue
        for b, path, old in builds:
            res = run(b, path, old, name)
            if res:
                res["round"] = r
                print(json.dumps(res), flush=True)
