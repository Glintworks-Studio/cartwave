"""Batch 5 A3: the before/after table from two `bank_pack --dc` runs.

usage: python a3_table.py <before.tsv> <after.tsv> > docs/batch5/a3-dc.md

Per preset (studio render): the 50 ms running mean's swing (the larger of
|min| and |max|), the peaks (negative / positive) and their balance
(the smaller over the larger magnitude: 1.0 = symmetric), loudness and its
change. For authentic presets the same sound on its locked channel is listed as
the reference (it is unchanged: the console doesn't use studio voices).
"""
import csv
import sys


def load(p):
    rows = {}
    with open(p, encoding="utf-8") as f:
        for r in csv.DictReader(f, delimiter="\t"):
            rows[(r["preset"], r["render"])] = r
    return rows


def swing(r):
    return max(abs(float(r["mean_lo"])), abs(float(r["mean_hi"])))


def balance(r):
    n, p = abs(float(r["peak_neg"])), abs(float(r["peak_pos"]))
    return min(n, p) / max(n, p, 1e-9)


def main():
    b, a = load(sys.argv[1]), load(sys.argv[2])
    names = [k[0] for k in a if k[1] == "studio"]
    out = []
    stats = {"n": 0, "under": 0, "under_b": 0, "lu_exc": [], "clip": [], "worse": [], "over_locked": [], "bal_b": [], "bal_a": []}
    for name in names:
        rb, ra = b.get((name, "studio")), a[(name, "studio")]
        lk = a.get((name, "locked"))
        stats["n"] += 1
        sa, sb = swing(ra), swing(rb)
        stats["under"] += sa <= 0.005
        stats["under_b"] += sb <= 0.005
        stats["bal_b"].append(balance(rb))
        stats["bal_a"].append(balance(ra))
        dlu = float(ra["lufs"]) - float(rb["lufs"])
        if abs(dlu) >= 0.3:
            stats["lu_exc"].append((name, dlu))
        if ra["clips"] == "true":
            stats["clip"].append(name)
        if sa > sb + 1e-4:
            stats["worse"].append((name, sb, sa))
        if sa > 0.005 and not (lk and sa <= swing(lk) * 1.25 + 0.0005):
            stats["over_locked"].append((name, sa, swing(lk) if lk else None))
        locked = f"{swing(lk):.4f} ({float(lk['peak_neg']):+.3f} / {float(lk['peak_pos']):+.3f})" if lk else "–"
        out.append(f"| {name} | {sb:.4f} | {sa:.4f} | {float(rb['peak_neg']):+.3f} / {float(rb['peak_pos']):+.3f} ({balance(rb):.2f}) | "
                   f"{float(ra['peak_neg']):+.3f} / {float(ra['peak_pos']):+.3f} ({balance(ra):.2f}) | {float(rb['lufs']):.2f} | {float(ra['lufs']):.2f} | {dlu:+.2f} | {locked} |")
    print("# Batch 5 A3: studio voices' baseline, before and after (every preset)\n")
    print("Written by `docs/batch5/tools/a3_table.py` from `bank_pack --dc` (before: A2's build; after: the bipolar voices). "
          "Swing: the larger of |min| and |max| of a 50 ms running mean while the preset sounds (full scale = 1). "
          "Peaks: negative / positive sample (balance: smaller / larger magnitude, 1.00 = symmetric). "
          "Locked: the same authentic preset on its locked channel (unchanged), for reference.\n")
    print(f"- Studio renders: **{stats['n']}**; swing ≤ 0.005: **{stats['under_b']} before → {stats['under']} after**.")
    print(f"- Swing larger after than before: {len(stats['worse'])}" + ("" if not stats["worse"] else ": " + ", ".join(f"{n} ({x:.4f} → {y:.4f})" for n, x, y in stats["worse"])) + ".")
    print(f"- Over 0.005 after and not within the locked render's own swing (×1.25 + 0.0005), or with no locked render: {len(stats['over_locked'])}" + ("" if not stats["over_locked"] else ": " + ", ".join(f"{n} {s:.4f}" + (f" (locked {l:.4f})" if l is not None else "") for n, s, l in stats["over_locked"])) + ".")
    print(f"- Loudness change of 0.3 LU or more: {len(stats['lu_exc'])}" + ("" if not stats["lu_exc"] else ": " + ", ".join(f"{n} {d:+.2f}" for n, d in stats["lu_exc"])) + ".")
    med = lambda v: sorted(v)[len(v) // 2]
    print(f"- Peak balance (smaller / larger peak magnitude): median **{med(stats['bal_b']):.2f} before → {med(stats['bal_a']):.2f} after**; "
          f"under 0.6: {sum(x < 0.6 for x in stats['bal_b'])} before → {sum(x < 0.6 for x in stats['bal_a'])} after "
          "(a thin pulse with its DC removed is lopsided by nature: 12.5 % swings +7/8 and -1/8 of its level).")
    print(f"- Clipping after: {len(stats['clip'])}" + ("" if not stats["clip"] else ": " + ", ".join(stats["clip"])) + ".\n")
    print("| Preset | Swing before | Swing after | Peaks before (balance) | Peaks after (balance) | LUFS before | LUFS after | Δ LU | Locked: swing (peaks) |")
    print("|---|---|---|---|---|---|---|---|---|")
    print("\n".join(out))


if __name__ == "__main__":
    main()
