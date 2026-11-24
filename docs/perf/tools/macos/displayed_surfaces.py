"""Display-side frame pacing from an Instruments "Metal System Trace" recording.

usage: displayed_surfaces.py <trace> [--display external-1] [--hz 320] [--skip 0.5] [--json]

Exports the `displayed-surfaces-interval` table (one row per composited framebuffer
the window server put on a display, with how long it stayed on screen) and reports,
for one display, how many vblanks each frame was shown for. This is the macOS
analogue of the DXGI desktop-duplication method in PERF.md: it is outside the app,
but it sees the whole display, so any other on-screen change (cursor, other
windows) also produces frames and can mask stalls. Run with nothing else animating.

Display names are Instruments' (`external-0`, `external-1`, ...); list them with
--display ? .
"""
import collections
import json
import subprocess
import sys
import xml.etree.ElementTree as ET

args = sys.argv[1:]


def flag(name, default):
    return args[args.index(name) + 1] if name in args else default


trace = args[0]
display = flag("--display", "external-1")
hz = float(flag("--hz", "320"))
skip = float(flag("--skip", "0.5"))
P = 1000.0 / hz

xml = subprocess.run(
    ["xctrace", "export", "--input", trace, "--xpath",
     '/trace-toc/run[@number="1"]/data/table[@schema="displayed-surfaces-interval"]'],
    capture_output=True, text=True, check=True).stdout
root = ET.fromstring(xml)
ids = {}
cols = [c.findtext("mnemonic") for c in root.iter("col")]


def val(el):
    if "ref" in el.attrib:
        return ids[el.attrib["ref"]]
    v = el.text if el.text is not None else el.attrib.get("fmt", "")
    if "id" in el.attrib:
        ids[el.attrib["id"]] = v
    # register nested ids too
    for sub in el.iter():
        if sub is not el and "id" in sub.attrib:
            ids[sub.attrib["id"]] = sub.text or sub.attrib.get("fmt", "")
    return v


rows = []
for row in root.iter("row"):
    cells = list(row)
    rec = {}
    for name, el in zip(cols, cells):
        rec[name] = val(el) if el.tag != "sentinel" else None
    rows.append(rec)

names = collections.Counter(r["display-name"] for r in rows)
if display == "?":
    print(dict(names))
    sys.exit(0)
sel = sorted((int(r["start"]), int(r["duration"])) for r in rows if r["display-name"] == display)
if not sel:
    sys.exit(f"no rows for {display}; displays: {dict(names)}")
t0 = sel[0][0]
sel = [(s, d) for s, d in sel if (s - t0) / 1e9 >= skip]
durs = [d / 1e6 for _, d in sel[:-1]]  # the last frame's duration is cut by the end of the trace
span = (sel[-1][0] - sel[0][0]) / 1e9 if len(sel) > 1 else 0
cad = collections.Counter(max(1, round(d / P)) for d in durs)
out = {
    "display": display,
    "frames": len(sel),
    "secs": span,
    "updates_per_s": (len(sel) - 1) / span if span else 0,
    "max_ms": max(durs) if durs else 0,
    "over20": sum(1 for d in durs if d > 20),
    "cadence": dict(sorted(cad.items())),
}
if "--json" in args:
    print(json.dumps(out))
else:
    print(f"{display}: {out['frames']} frames over {span:.2f}s = {out['updates_per_s']:.1f}/s; "
          f"max on-screen {out['max_ms']:.2f} ms; >20ms: {out['over20']}")
    print(f"  vblanks per frame at {hz:g} Hz (count): {out['cadence']}")
