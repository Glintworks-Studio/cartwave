"""Batch 4b §3.13 / gate 6: the interleaved A/B judged on its means, with the noise floor.

For each scenario: the baseline's and this branch's mean over the rounds, the difference of
the means (Δ, CPU % of one core), and the noise floor: the standard error of the paired
round-by-round differences (how far Δ moves by chance; 2 × it is the ~95 % band). A scenario
passes when Δ ≤ +0.3, or when Δ is inside 2 × the noise floor (the method can't tell it from
zero). Runs that lost focus or didn't reach their scenario are left out and counted.

usage: python perf_summary_b4b.py <work dir>   (writes <work dir>/perf_summary.md, prints it)"""
import math
import os
import re
import sys
from statistics import mean, stdev

sys.stdout.reconfigure(encoding="utf-8")
LIMIT = 0.3


def parse(path, pat_label):
    runs = {}
    bad = 0
    if not os.path.exists(path):
        return runs, bad
    for line in open(path, encoding="utf-8", errors="replace"):
        m = re.match(pat_label, line)
        if not m:
            continue
        if "INVALID" in line or "NOT REACHED" in line:
            bad += 1
            continue
        build, rnd = m.group(1), int(m.group(2))
        vals = {k: float(v) for k, v in re.findall(r"([a-z0-9_]+)\s+([\d.]+)%", line)}
        runs.setdefault(rnd, {})[build] = vals
    return runs, bad


def parse_pr(path):
    runs = {}
    bad = 0
    if not os.path.exists(path):
        return runs, bad
    for line in open(path, encoding="utf-8", errors="replace"):
        m = re.match(r"pr-(base|b4b)-(\d+)\s", line)
        if not m:
            continue
        if "NOT" in line.split("scenario", 1)[-1]:
            bad += 1
            continue
        pl = re.search(r"playing\s+([\d.]+)%", line)
        dr = re.search(r"dragging\s+([\d.]+)%", line)
        runs.setdefault(int(m.group(2)), {})[m.group(1)] = {"pr_playing": float(pl.group(1)), "pr_dragging": float(dr.group(1))}
    return runs, bad


def table(runs):
    rows = []
    scen = []
    for r in runs.values():
        for b in r.values():
            for k in b:
                if k not in scen:
                    scen.append(k)
    for s in scen:
        pairs = [(r["base"][s], r["b4b"][s]) for r in runs.values() if "base" in r and "b4b" in r and s in r["base"] and s in r["b4b"]]
        if len(pairs) < 2:
            continue
        base = [p[0] for p in pairs]
        new = [p[1] for p in pairs]
        d = [b - a for a, b in pairs]
        delta = mean(new) - mean(base)
        se = stdev(d) / math.sqrt(len(d))
        ok = delta <= LIMIT or delta <= 2 * se
        rows.append((s, len(pairs), mean(base), stdev(base), mean(new), stdev(new), delta, se, ok))
    return rows


def main():
    work = sys.argv[1]
    runs, bad = parse(os.path.join(work, "bench_b4a_log.txt"), r"(base|b4b)-(\d+)\s")
    pr, bad_pr = parse_pr(os.path.join(work, "bench_log.txt"))
    for k, v in pr.items():
        for b, vals in v.items():
            runs.setdefault(k, {}).setdefault(b, {}).update(vals)
    rows = table(runs)
    out = ["# Batch 4b perf: interleaved A/B, judged on the means", "",
           f"Rounds paired: {len(runs)}; runs left out (focus lost / scenario not reached): {bad + bad_pr}.",
           f"Pass: Δ ≤ +{LIMIT} or Δ within 2 × the noise floor (the standard error of the paired differences).", "",
           "| scenario | n | baseline mean (sd) | 4b mean (sd) | Δ | noise floor (SE) | pass |", "|---|---|---|---|---|---|---|"]
    allok = True
    for s, n, mb, sb, mn, sn, dl, se, ok in rows:
        allok &= ok
        out.append(f"| {s} | {n} | {mb:.1f} ({sb:.1f}) | {mn:.1f} ({sn:.1f}) | {dl:+.2f} | {se:.2f} | {'yes' if ok else '**no**'} |")
    # The mixer: this branch only, against the same run's arrangement.
    mix = []
    p = os.path.join(work, "bench_mixer_log.txt")
    if os.path.exists(p):
        for line in open(p, encoding="utf-8", errors="replace"):
            if line.startswith("mixer-") and "INVALID" not in line and "NOT REACHED" not in line:
                mix.append({k: float(v) for k, v in re.findall(r"([a-z0-9_]+)\s+([\d.]+)%", line)})
    if mix:
        out += ["", "## The mixer (this branch only; 64 studio tracks playing)", "",
                "| scenario | n | mean (sd) |", "|---|---|---|"]
        for k in mix[0]:
            v = [m[k] for m in mix if k in m]
            out.append(f"| {k} | {len(v)} | {mean(v):.1f} ({stdev(v) if len(v) > 1 else 0:.1f}) |")
    out += ["", f"**Gate 6: {'pass' if allok and rows else 'FAIL'}**"]
    text = "\n".join(out) + "\n"
    open(os.path.join(work, "perf_summary.md"), "w", encoding="utf-8").write(text)
    print(text)


if __name__ == "__main__":
    main()
