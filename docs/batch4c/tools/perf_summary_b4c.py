"""Batch 4c gate 5 (perf_b4c.ps1): three builds interleaved, judged on the means.

For each scenario: each build's mean over the rounds (CPU % of one core), the difference of
this branch's mean from main's and from pre-batch4b's (Δ), and each noise floor: the standard
error of the paired round-by-round differences. The gate, by its letter (BATCH4C_PROMPT.md §6.5):
vs main every scenario Δ ≤ +0.3; vs pre-batch4b the 64-track rows Δ ≤ +0.4, idle ≤ +0.1, row
resize ≤ +0.3 (other rows are reported, not judged against pre-batch4b). The noise floor is
reported beside every Δ, but it doesn't pass a row (unlike 4b's rule, B4B-D30).

usage: python perf_summary_b4c.py <work dir>   (writes <work dir>/perf_summary.md, prints it)"""
import math
import os
import re
import sys
from statistics import mean, stdev

sys.stdout.reconfigure(encoding="utf-8")
BUILDS = ("pre4b", "main", "b4c")


def parse(path, pat):
    runs, bad = {}, 0
    if not os.path.exists(path):
        return runs, bad
    for line in open(path, encoding="utf-8", errors="replace"):
        m = re.match(pat, line)
        if not m:
            continue
        if "INVALID" in line or "NOT REACHED" in line or "NOT" in line.split("scenario", 1)[-1]:
            bad += 1
            continue
        b, r = m.group(1), int(m.group(2))
        vals = {k: float(v) for k, v in re.findall(r"([a-z0-9_]+)\s+([\d.]+)%", line)}
        runs.setdefault(r, {}).setdefault(b, {}).update(vals)
    return runs, bad


def limit_pre(s):
    if s.startswith("s64_"):
        return 0.4
    return {"idle": 0.1, "row_resize": 0.3}.get(s)


def stats(runs, s, a, b):
    pairs = [(r[a][s], r[b][s]) for r in runs.values() if a in r and b in r and s in r[a] and s in r[b]]
    if len(pairs) < 2:
        return None
    d = [y - x for x, y in pairs]
    return len(pairs), mean(y for _, y in pairs) - mean(x for x, _ in pairs), stdev(d) / math.sqrt(len(d))


def main():
    work = sys.argv[1]
    runs, bad = parse(os.path.join(work, "bench_b4a_log.txt"), r"(pre4b|main|b4c)-(\d+)\s")
    for path, pat in ((os.path.join(work, "bench_log.txt"), r"pr-(pre4b|main|b4c)-(\d+)\s"), (os.path.join(work, "bench_mixer_log.txt"), r"mixer-(main|b4c)-(\d+)\s")):
        more, b2 = parse(path, pat)
        bad += b2
        for r, v in more.items():
            for b, vals in v.items():
                if "pr-" in pat:
                    vals = {("pr_" + k if not k.startswith("pr_") else k): x for k, x in vals.items()}
                runs.setdefault(r, {}).setdefault(b, {}).update(vals)
    scen = []
    for r in runs.values():
        for v in r.values():
            for k in v:
                if k not in scen:
                    scen.append(k)
    out = ["# Batch 4c perf: three builds interleaved, judged on the means", "",
           f"Rounds: {len(runs)}; runs left out (focus lost / scenario not reached): {bad}.",
           "Δ = this branch's mean − the other build's (CPU % of one core); SE = the noise floor "
           "(standard error of the paired differences). Limits (§6.5): vs main +0.3 everywhere; vs "
           "pre-batch4b the 64-track rows +0.4, idle +0.1, row resize +0.3.", "",
           "| scenario | n | pre-4b | main | 4c | Δ vs main (SE) | ok | Δ vs pre-4b (SE) | limit | ok |",
           "|---|---|---|---|---|---|---|---|---|---|"]
    all_main, all_pre = True, True
    for s in scen:
        m = {b: [r[b][s] for r in runs.values() if b in r and s in r[b]] for b in BUILDS}
        vm = stats(runs, s, "main", "b4c")
        vp = stats(runs, s, "pre4b", "b4c")
        lp = limit_pre(s)
        okm = vm is not None and vm[1] <= 0.3
        okp = None if (lp is None or vp is None) else vp[1] <= lp
        all_main &= okm or vm is None
        if okp is not None:
            all_pre &= okp
        f = lambda v: f"{mean(v):.1f}" if v else "–"
        out.append(f"| {s} | {vm[0] if vm else (vp[0] if vp else 0)} | {f(m['pre4b'])} | {f(m['main'])} | {f(m['b4c'])} | "
                   f"{'%+.2f (%.2f)' % (vm[1], vm[2]) if vm else '–'} | {'yes' if okm else ('–' if vm is None else '**no**')} | "
                   f"{'%+.2f (%.2f)' % (vp[1], vp[2]) if vp else '–'} | {lp if lp is not None else '–'} | "
                   f"{'–' if okp is None else ('yes' if okp else '**no**')} |")
    out += ["", f"**vs main: {'pass' if all_main else 'NOT MET'}; vs pre-batch4b: {'pass' if all_pre else 'NOT MET'}**"]
    text = "\n".join(out) + "\n"
    open(os.path.join(work, "perf_summary.md"), "w", encoding="utf-8").write(text)
    print(text)


if __name__ == "__main__":
    main()
