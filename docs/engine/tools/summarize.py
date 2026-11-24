"""Summarise matrix.ps1 logs into the ENGINE.md §7 table.

usage: python summarize.py <work dir>
Reads <work>/<test>_<buffer>.csv (and E.csv) and prints a Markdown table.
"""
import csv
import re
import sys
from pathlib import Path

NAMES = {
    "A": "A. Demo set playing",
    "B": "B. 200 clips + continuous scripted edits",
    "C": "C. UI stall 250 ms every 2 s",
    "D": "D. Resize / minimise / restore",
    "G": "G. A/V sync",
    "E": "E. Default output switched away and back",
}


def summary(path):
    lines = path.read_text(encoding="utf-8").splitlines()
    rows = [r for r in csv.DictReader(l for l in lines if not l.startswith("#"))]
    text = "\n".join(l for l in lines if l.startswith("#"))
    get = lambda pat, cast=float: cast(re.search(pat, text).group(1)) if re.search(pat, text) else None
    last = rows[-1] if rows else {}
    return {
        "secs": get(r"summary: (\d+) s", int),
        "callbacks": get(r"callbacks (\d+)", int),
        "frames": re.search(r"frames/cb (\d+\.\.\d+)", text).group(1) if "frames/cb" in text else "?",
        "mean": get(r"cost mean ([\d.]+)%"),
        "p99": get(r"p99 ([\d.]+)%"),
        "max": get(r"max ([\d.]+)% of period"),
        "starved": get(r"starved (\d+)", int),
        "late": get(r"late (\d+)", int),
        "overruns": get(r"overruns (\d+)", int),
        "gap": get(r"max callback gap ([\d.]+) ms"),
        "edits": get(r"edits applied (\d+)", int),
        "edit_tick": get(r"edit->frame tick max ([\d.]+) ms"),
        "send": get(r"send->engine max ([\d.]+) ms"),
        "dropped": get(r"dropped commands (\d+)", int),
        "alloc": get(r"alloc violations (\d+)", int),
        "stalls": get(r"ui stalls (\d+)", int),
        "rebuilds": get(r"device rebuilds (\d+)", int),
        "audio_k": get(r"audio ([\d.]+) k/callback"),
        "ui_k": get(r"ui ([\d.]+) k/presented"),
        "lb_blocks": get(r"loopback: (\d+) blocks", int),
        "lb_drop": get(r"(\d+) pilot dropouts", int),
        "av": re.search(r"avsync: (.*)", text).group(1) if "avsync:" in text else None,
        "ui_edits": int(last.get("ui_edits", 0) or 0),
    }


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    work = Path(sys.argv[1])
    print("| Test | Requested buffer | Run | Granted frames/cb | Underruns (loopback dropouts / starved / late) | Callback cost mean / p99 / max (% of period) | Audio thread k-cycles / callback | UI thread k-cycles / frame | Other |")
    print("|---|---|---|---|---|---|---|---|---|")
    for t in "ABCDGE":
        files = sorted(work.glob(f"{t}_*.csv")) + ([work / "E.csv"] if t == "E" and (work / "E.csv").exists() else [])
        for f in files:
            s = summary(f)
            buf = f.stem.split("_")[1] if "_" in f.stem else "default"
            other = []
            if t == "B":
                other.append(f"{s['ui_edits']} scripted edits, {s['edits']} snapshots applied, edit→tick max {s['edit_tick']} ms, send→engine max {s['send']} ms, dropped {s['dropped']}")
            if t == "C":
                other.append(f"{s['stalls']} UI stalls")
            if t == "E":
                other.append(f"{s['rebuilds']} rebuilds, no crash")
            if s["av"]:
                other.append(s["av"].replace("audio - visual (presented) offset ms:", "offset ms"))
            other.append(f"alloc violations {s['alloc']}")
            print(
                f"| {NAMES[t]} | {buf} | {s['secs']} s | {s['frames']} | **{s['lb_drop']}** / {s['starved']} / {s['late']} "
                f"| {s['mean']} / {s['p99']} / {s['max']} | {s['audio_k']} | {s['ui_k']} | {'; '.join(other)} |"
            )


if __name__ == "__main__":
    main()
