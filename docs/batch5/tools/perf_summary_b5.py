"""Batch 5's performance summary (BATCH5_PROMPT.md §8.5, ENGINE.md §7b): perf_summary_pb.py with
this branch's runs labelled b5, and the automation scenarios (bench_b5.ps1: labels auto-main-N for
the plain songs on this build, auto-b5-N for the automated ones) paired the same way.

From the Presets + Browser batch's summary (PRESETS_PROMPT.md §2, ENGINE.md §7b).

Primary measure: per-thread cycle counters, from each run's audio log (one row per second):
UI kcycles per presented frame and audio kcycles per callback, averaged over each scenario's
window (bench_pb.ps1's windows.tsv; the piano roll's and the mixer's windows from their scripts'
fixed timing). Process CPU % of a core (the bench logs) is reported beside them for context.

Rounds pair main and this branch; per scenario: the means, the difference, and the noise floor
(the standard error of the paired round differences). The rule: a scenario **fails only if its
difference is over its limit and more than 2x its noise floor**. The limit is Batch 4c's
+0.3 points of a core, in cycles at this machine's clock (`--ghz`, 4.7 for the Ryzen 7 9800X3D's
TSC): the UI at 60 presented frames per second, the audio at the device's callbacks per second
(`--cbps`, 100: the HDMI endpoint's 480-frame periods at 48 kHz).

usage: python perf_summary_pb.py <work dir> [--ghz 4.7] [--cbps 100]
Writes <work>/perf_summary.md and prints it.
"""
import csv
import math
import os
import re
import statistics
import sys
sys.stdout.reconfigure(encoding="utf-8")
from collections import defaultdict

work = sys.argv[1]
args = sys.argv[2:]
GHZ = float(args[args.index("--ghz") + 1]) if "--ghz" in args else 4.7
CBPS = float(args[args.index("--cbps") + 1]) if "--cbps" in args else 100.0
LIMIT_PTS = 0.3
UI_LIMIT = LIMIT_PTS / 100 * GHZ * 1e9 / 60 / 1e3          # kcycles per frame
AU_LIMIT = LIMIT_PTS / 100 * GHZ * 1e9 / CBPS / 1e3        # kcycles per callback

# Fixed windows (s since start) of the scripts without windows.tsv.
FIXED = {
    "pr": [("pr_playing", 6.0, 17.0), ("pr_dragging", 18.5, 29.0)],
    "mixer": [("s64_arrangement", 5.5, 16.0), ("s64_mixer", 19.0, 30.0), ("s64_mixer_scroll", 31.0, 42.0)],
}


def rows(path):
    try:
        with open(path, newline="", encoding="utf-8", errors="replace") as f:
            out = []
            for r in csv.DictReader(f):
                t = r.get("t_s", "")
                if not t or t.startswith("#"):
                    continue
                try:
                    out.append((float(t), float(r.get("ui_kcycles_per_frame") or "nan"), float(r.get("audio_kcycles_per_cb") or "nan")))
                except ValueError:
                    pass
            return out
    except OSError:
        return []


def window(rs, t0, t1):
    ui = [u for t, u, a in rs if t0 + 1 <= t <= t1 - 0.5 and not math.isnan(u) and u > 0]
    au = [a for t, u, a in rs if t0 + 1 <= t <= t1 - 0.5 and not math.isnan(a) and a > 0]
    return (statistics.mean(ui) if ui else None, statistics.mean(au) if au else None)


# cycles[(scenario)][build][round] = (ui, audio)
cyc = defaultdict(lambda: defaultdict(dict))
wfile = os.path.join(work, "windows.tsv")
if os.path.exists(wfile):
    for r in csv.DictReader(open(wfile, encoding="utf-8"), delimiter="\t"):
        lab = r["label"]
        m = re.match(r"(?:new-|auto-)?(main|b5)-(\d+)$", lab)
        if not m:
            continue
        rs = rows(os.path.join(work, f"{lab}-{r['log']}.csv"))
        cyc[r["scenario"]][m.group(1)][int(m.group(2))] = window(rs, float(r["t0"]), float(r["t1"]))
for kind, wins in FIXED.items():
    for fn in os.listdir(work):
        m = re.match(rf"{kind}-(main|b5)-(\d+)\.csv$", fn)
        if not m:
            continue
        rs = rows(os.path.join(work, fn))
        for name, t0, t1 in wins:
            cyc[name][m.group(1)][int(m.group(2))] = window(rs, t0, t1)

# Proof per run (bench_pb.ps1 keeps each run's stderr as err-<label>-main.txt): the Overview
# drags must have hovered the Overview (its help line). A run without it aimed elsewhere (one
# branch run dragged the top bar's Display: the tempo) and is left out of those two scenarios.
unproven = []
for fn in sorted(os.listdir(work)):
    m = re.match(r"err-(main|b5)-(\d+)-main\.txt$", fn)
    if not m:
        continue
    if "help Overview" not in open(os.path.join(work, fn), encoding="utf-8", errors="replace").read():
        unproven.append(f"{m.group(1)}-{m.group(2)} (overview_drag, overview_zoom: no Overview hover in its log)")
        for name in ("overview_drag", "overview_zoom"):
            cyc[name][m.group(1)].pop(int(m.group(2)), None)

# CPU % from the bench logs.
cpu = defaultdict(lambda: defaultdict(dict))
invalid = []
for log in ("bench_b4a_log.txt", "bench_mixer_log.txt"):
    p = os.path.join(work, log)
    if not os.path.exists(p):
        continue
    for line in open(p, encoding="utf-8", errors="replace"):
        m = re.match(r"(?:new-|mixer-|auto-)?(main|b5)-(\d+)\s", line)
        if not m:
            continue
        if "INVALID" in line or "NOT REACHED" in line:
            invalid.append(line.split()[0])
            continue
        for name, v in re.findall(r"(\w+)\s+([\d.]+)%", line):
            cpu[name][m.group(1)][int(m.group(2))] = float(v)
p = os.path.join(work, "bench_log.txt")
if os.path.exists(p):
    for line in open(p, encoding="utf-8", errors="replace"):
        m = re.match(r"pr-(main|b5)-(\d+)\s.*playing\s+([\d.]+)%\s+dragging\s+([\d.]+)%.*scenario reached", line)
        if m:
            cpu["pr_playing"][m.group(1)][int(m.group(2))] = float(m.group(3))
            cpu["pr_dragging"][m.group(1)][int(m.group(2))] = float(m.group(4))


for u in unproven:
    b, r = u.split(" ")[0].split("-")
    for name in ("overview_drag", "overview_zoom"):
        cpu[name][b].pop(int(r), None)


# Runs whose proof failed (B5-D14): their cycles leave the verdict too, scenario by scenario
# (the first gate run dropped them only from the CPU % columns). A lost focus or INVALID drops
# every scenario of that run; a missing scenario proof drops that scenario's.
S64 = ("s64_playing", "s64_scroll", "s64_insert_delete", "s64_header_drag")
PROOFS = [("64-track scenario NOT REACHED", S64), ("a32 Automation Mode NOT REACHED", ("auto_scroll", "auto_timesel")),
          ("time selection NOT REACHED", ("auto_timesel",)), ("a2000 drawing NOT REACHED", ("auto_draw_2000",))]
dropped = []
for log in ("bench_b4a_log.txt", "bench_mixer_log.txt", "bench_b5_log.txt", "bench_log.txt"):
    p = os.path.join(work, log)
    if not os.path.exists(p):
        continue
    for line in open(p, encoding="utf-8", errors="replace"):
        m = re.match(r"(new-|mixer-|auto-|pr-)?(main|b5)-(\d+)\s", line)
        if not m:
            continue
        kind, b, r = m.group(1) or "", m.group(2), int(m.group(3))
        if kind == "pr-":
            gone = ("pr_playing", "pr_dragging") if "scenario reached" not in line else ()
        elif "INVALID" in line or "FOCUS LOST" in line:
            gone = [n for n, d in cyc.items() if r in d.get(b, {}) and (kind in ("auto-",) and n.startswith("auto_") or kind == "mixer-" and n.startswith("s64_m") or kind == "mixer-" and n == "s64_arrangement" or kind in ("", "new-") and not n.startswith(("auto_", "pr_", "s64_m", "s64_arr")))]
        else:
            gone = [n for text, names in PROOFS if text in line for n in names]
        for n in gone:
            if cyc[n][b].pop(r, None) is not None:
                dropped.append(f"{n} {b}-{r}")


def paired(d):
    rounds = sorted(set(d.get("main", {})) & set(d.get("b5", {})))
    return rounds


def stats(main, pb, rounds):
    diffs = [pb[r] - main[r] for r in rounds]
    if not diffs:
        return None
    se = statistics.stdev(diffs) / math.sqrt(len(diffs)) if len(diffs) > 1 else float("inf")
    return statistics.mean(main[r] for r in rounds), statistics.mean(pb[r] for r in rounds), statistics.mean(diffs), se


order = ["idle", "demo", "200", "200drag", "zoom_wheel", "scrollbar_drag", "window_resize", "row_resize", "header_width", "ruler_zoom", "overview_drag", "overview_zoom",
         "s64_playing", "s64_scroll", "s64_insert_delete", "s64_header_drag", "pr_playing", "pr_dragging", "s64_arrangement", "s64_mixer", "s64_mixer_scroll",
         "device_playing", "browser_search", "browser_scroll", "step"]
names = [n for n in order if n in cyc or n in cpu] + sorted((set(cyc) | set(cpu)) - set(order))
lines = ["| scenario | n | UI k/frame main | this | Δ (SE) | Audio k/cb main | this | Δ (SE) | verdict | CPU % main | this | Δ (SE) |", "|---|---|---|---|---|---|---|---|---|---|---|---|"]
fails = []
for n in names:
    c = cyc.get(n, {})
    ui_m = {r: v[0] for r, v in c.get("main", {}).items() if v[0] is not None}
    ui_p = {r: v[0] for r, v in c.get("b5", {}).items() if v[0] is not None}
    au_m = {r: v[1] for r, v in c.get("main", {}).items() if v[1] is not None}
    au_p = {r: v[1] for r, v in c.get("b5", {}).items() if v[1] is not None}
    su = stats(ui_m, ui_p, sorted(set(ui_m) & set(ui_p)))
    sa = stats(au_m, au_p, sorted(set(au_m) & set(au_p)))
    sc = stats(cpu[n].get("main", {}), cpu[n].get("b5", {}), paired(cpu[n])) if n in cpu else None
    rn = len(sorted(set(ui_m) & set(ui_p))) or len(paired(cpu[n]) if n in cpu else [])
    verdict = "–"
    bad = []
    if su and su[2] > UI_LIMIT and su[2] > 2 * su[3]:
        bad.append("UI")
    if sa and sa[2] > AU_LIMIT and sa[2] > 2 * sa[3]:
        bad.append("audio")
    if su or sa:
        verdict = "**fail (" + ", ".join(bad) + ")**" if bad else "pass"
    if bad:
        fails.append(n)
    only = ""
    if not su and not sa and n in cyc:
        b = "b5" if c.get("b5") else "main"
        uvals = [v[0] for v in c.get(b, {}).values() if v[0] is not None]
        avals = [v[1] for v in c.get(b, {}).values() if v[1] is not None]
        only = f" (this branch only: UI {statistics.mean(uvals):.0f} k/frame, audio {statistics.mean(avals):.0f} k/cb)" if uvals and avals else ""
        verdict = "this branch only" + only

    def f(s, k):
        return "–" if not s else f"{s[k]:.1f}"

    def d(s):
        return "–" if not s else f"{s[2]:+.1f} ({s[3]:.1f})"

    lines.append(f"| {n} | {rn} | {f(su, 0)} | {f(su, 1)} | {d(su)} | {f(sa, 0)} | {f(sa, 1)} | {d(sa)} | {verdict} | {f(sc, 0)} | {f(sc, 1)} | {d(sc)} |")

head = (
    "# Performance: this branch against main (the §2 rule)\n\n"
    f"Rounds paired by number, the order alternating. Primary: the per-thread cycle counters (UI kcycles per presented frame, audio kcycles per callback), "
    f"the means over each scenario's window; process CPU % of a core beside them for context. Limits: +{LIMIT_PTS} points of a core at {GHZ} GHz: "
    f"UI **+{UI_LIMIT:.0f} k/frame** (60 frames/s), audio **+{AU_LIMIT:.0f} k/callback** ({CBPS:.0f} callbacks/s). A scenario fails only if its difference "
    "is over the limit and more than 2x its noise floor (SE).\n\n"
)
verdict = f"**{'PASS' if not fails else 'FAIL: ' + ', '.join(fails)}**" + (f"; runs left out (focus lost or scenario not reached): {', '.join(invalid)}" if invalid else "; 0 runs left out")
verdict += (f"; left out of single scenarios for want of proof: {'; '.join(unproven)}" if unproven else "")
verdict += (f"; cycle samples left out for want of proof ({len(dropped)}): {', '.join(dropped)}" if dropped else "")
text = head + verdict + "\n\n" + "\n".join(lines) + "\n"
open(os.path.join(work, "perf_summary.md"), "w", encoding="utf-8").write(text)
print(text)
