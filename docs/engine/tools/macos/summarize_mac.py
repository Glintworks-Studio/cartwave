#!/usr/bin/env python3
"""Summarise a matrix_mac.sh work directory into Markdown tables.

usage: summarize_mac.py <workdir> [--hist]

Per run (<test>_<buffer>.csv + .tap.csv + .crossings.csv):
- dropouts: tapmon pilot dropouts inside the app's playing window (from 2 s
  after start to 0.3 s before the end, so start-up and exit don't count);
- late / overloads / overruns from the app's own detectors;
- callback cost mean / p99 / max (% of the period) and the histogram;
- audio / UI thread instructions and energy, process mW and P-core share;
- A/V: each presented marker crossing paired with the nearest click onset
  (audio - picture, ms).
"""
import csv, glob, os, re, sys

W = sys.argv[1]
SHOW_HIST = "--hist" in sys.argv


def summary(path):
    out = {}
    for line in open(path):
        if not line.startswith("#"):
            continue
        m = re.search(r"frames/cb (\d+)\.\.(\d+), cost mean ([\d.]+)% p99 ([\d.]+)% max ([\d.]+)%", line)
        if m:
            out["frames"] = m.group(1) if m.group(1) == m.group(2) else f"{m.group(1)}..{m.group(2)}"
            out["cost"] = (float(m.group(3)), float(m.group(4)), float(m.group(5)))
        m = re.search(r"underruns: starved (\d+) late (\d+) \(overruns (\d+)\), max callback gap ([\d.]+) ms", line)
        if m:
            out["late"], out["overrun"], out["maxgap"] = int(m.group(2)), int(m.group(3)), float(m.group(4))
        m = re.search(r"edits applied (\d+), edit->frame tick max ([\d.]+) ms, send->engine max ([\d.]+) ms, dropped commands (\d+)", line)
        if m:
            out["edits"] = (int(m.group(1)), float(m.group(2)), float(m.group(3)), int(m.group(4)))
        m = re.search(r"alloc violations (\d+), ui stalls (\d+), device rebuilds (\d+)", line)
        if m:
            out["alloc"], out["stalls"], out["rebuilds"] = map(int, m.groups())
        m = re.search(r"audio thread: ([\d.]+) k instr/callback, ([\d.]+) uJ/callback, P-core share ([\d.?]+)%?; ui thread: ([\d.]+) k instr/frame, ([\d.]+) uJ/frame, P-core share ([\d.?]+)", line)
        if m:
            out["threads"] = m.groups()
        m = re.search(r"process: ([\d.]+) M instr/s, ([\d.]+) mW, P-core share ([\d.?]+)%?, ([\d.]+) % of a core; audio thread ([\d.]+) M instr/s, ([\d.]+) mW", line)
        if m:
            out["process"] = m.groups()
        m = re.search(r"overloads .*: (\d+); silent callbacks (\d+)", line)
        if m:
            out["overloads"], out["silent"] = int(m.group(1)), int(m.group(2))
        m = re.search(r"cb_hist_permille: (.*)", line)
        if m:
            out["hist"] = {int(a): int(b) for a, b in (x.split(":") for x in m.group(1).split())}
        m = re.search(r"window_host_ns: (\d+) (\d+)", line)
        if m:
            out["window"] = (int(m.group(1)), int(m.group(2)))
        m = re.search(r"summary: (\d+) s", line)
        if m:
            out["secs"] = int(m.group(1))
    return out


def tap(path, window):
    if not os.path.exists(path) or window is None:
        return None
    drops, clicks = [], []
    for row in csv.DictReader(open(path)):
        t = int(row["host_ns"])
        if row["kind"] == "dropout":
            drops.append(t)
        elif row["kind"] == "click":
            clicks.append(t)
    a, b = window[0] + 2_000_000_000, window[1] - 300_000_000
    return [t for t in drops if a <= t <= b], clicks, (a, b)


def avsync(cross_path, clicks):
    if not os.path.exists(cross_path) or not clicks:
        return None
    xs = [int(l) for l in open(cross_path).read().split()[1:]]
    offs = []
    for v in xs:
        best = min(clicks, key=lambda c: abs(c - v))
        o = (best - v) / 1e6
        if abs(o) < 400:
            offs.append(o)
    if not offs:
        return f"no pairs ({len(xs)} crossings, {len(clicks)} clicks)"
    n = len(offs)
    mean = sum(offs) / n
    sd = (sum((o - mean) ** 2 for o in offs) / n) ** 0.5
    return f"{n}/{len(xs)} pairs: {mean:+.2f} ms, sd {sd:.2f}, range {min(offs):+.2f} … {max(offs):+.2f}"


def hist_line(h):
    n = sum(h.values())
    edges = [1, 2, 5, 10, 20, 50, 100, 200, 201]
    out, prev = [], 0
    for e in edges:
        c = sum(v for k, v in h.items() if prev <= k < e)
        label = f"{prev / 10:g}–{e / 10:g}%" if e < 201 else "≥20%"
        out.append(f"{label}: {c}")
        prev = e
    return f"n={n}; " + ", ".join(out)


rows = []
for path in sorted(glob.glob(os.path.join(W, "*.csv"))):
    name = os.path.basename(path)[:-4]
    if "." in name:
        continue
    s = summary(path)
    if not s:
        continue
    t = tap(os.path.join(W, name + ".tap.csv"), s.get("window"))
    av = avsync(os.path.join(W, name + ".csv.crossings.csv"), t[1] if t else None)
    rows.append((name, s, t, av))

print("| Run | s | Frames/cb | **Tap dropouts** / late / overloads / overruns | Rebuilds | Cost % mean / p99 / max | Largest gap ms | Audio k instr/cb · µJ/cb · P share | UI M instr/frame · µJ/frame · P share | Process mW · P share · % core | Other |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for name, s, t, av in rows:
    drops = str(len(t[0])) if t else "n/a"
    c = s.get("cost", (0, 0, 0))
    th = s.get("threads")
    pr = s.get("process")
    other = []
    if s.get("edits") and s["edits"][0] > 1:
        e = s["edits"]
        other.append(f"{e[0]} snapshots; edit→tick ≤ {e[1]:.2f} ms; send→engine ≤ {e[2]:.2f} ms; {e[3]} dropped")
    if s.get("stalls"):
        other.append(f"{s['stalls']} UI stalls")
    if av:
        other.append("A/V " + av)
    if s.get("alloc"):
        other.append(f"ALLOC VIOLATIONS {s['alloc']}")
    if s.get("silent"):
        other.append(f"{s['silent']} silent callbacks")
    print(
        f"| {name} | {s.get('secs', '?')} | {s.get('frames', '?')} | **{drops}** / {s.get('late', '?')} / {s.get('overloads', '?')} / {s.get('overrun', '?')} "
        f"| {s.get('rebuilds', '?')} | {c[0]:.2f} / {c[1]:.1f} / {c[2]:.1f} | {s.get('maxgap', 0):.2f} "
        f"| {th[0] + ' · ' + th[1] + ' · ' + th[2] + '%' if th else '?'} "
        f"| {float(th[3]) / 1000:.1f} · {th[4]} · {th[5]}%" if th else "| ? "
        , end="")
    print(f" | {pr[1]} · {pr[2]}% · {pr[3]}" if pr else " | ?", end="")
    print(f" | {'; '.join(other)} |")

if SHOW_HIST:
    print("\nCallback cost histograms (% of the buffer period):\n")
    for name, s, _, _ in rows:
        if "hist" in s:
            print(f"- {name}: {hist_line(s['hist'])}")
