"""Batch 5 A3: the baseline of every listening-pack WAV, before and after.

For each preset WAV (16-bit PCM, mono or stereo; the first channel), it reports:
- the **baseline swing**: the minimum and maximum of a 50 ms running mean, taken
  only while the preset sounds (from its first sample above -60 dBFS to its last),
  in full-scale units;
- the **peaks**: the most negative and most positive sample (full scale), and
  their asymmetry (|max + min| / max(|max|, |min|));
- whether the file is a locked (authentic, `hardware/`) or a studio render.

usage: python dc_measure.py <pack dir> [<other pack dir>]
With two dirs it prints a before/after table for the files both have, plus the
loudness from `levels.md` in each dir. Plain Python (no numpy).
"""
import os
import re
import struct
import sys
import wave

WIN_S = 0.050


def read(path):
    with wave.open(path, "rb") as w:
        n, ch, sw, sr = w.getnframes(), w.getnchannels(), w.getsampwidth(), w.getframerate()
        raw = w.readframes(n)
    assert sw == 2, path
    vals = struct.unpack("<%dh" % (n * ch), raw)
    return [v / 32768.0 for v in vals[::ch]], sr


def measure(path):
    x, sr = read(path)
    if not x:
        return None
    floor = 10 ** (-60 / 20)
    idx = [i for i, v in enumerate(x) if abs(v) > floor]
    if not idx:
        return None
    a, b = idx[0], idx[-1] + 1
    win = int(WIN_S * sr)
    s = sum(x[a:a + win])
    lo = hi = s / win
    for i in range(a + win, b):
        s += x[i] - x[i - win]
        m = s / win
        lo, hi = min(lo, m), max(hi, m)
    mn, mx = min(x), max(x)
    asym = abs(mx + mn) / max(abs(mx), abs(mn), 1e-9)
    return {"lo": lo, "hi": hi, "min": mn, "max": mx, "asym": asym}


def walk(root):
    out = {}
    for d, _, fs in os.walk(root):
        for f in fs:
            if f.endswith(".wav"):
                p = os.path.join(d, f)
                out[os.path.relpath(p, root).replace("\\", "/")] = p
    return out


def loudness(root):
    """Preset name -> LUFS from levels.md (its table)."""
    res = {}
    p = os.path.join(root, "levels.md")
    if not os.path.exists(p):
        return res
    for line in open(p, encoding="utf-8"):
        m = re.match(r"\| ([^|]+) \| ([^|]+) \| (new|classic) \| ([-\d.]+|-inf) \|", line)
        if m:
            res[m.group(1).strip()] = float(m.group(4)) if m.group(4) != "-inf" else float("-inf")
    return res


def main():
    a = walk(sys.argv[1])
    b = walk(sys.argv[2]) if len(sys.argv) > 2 else None
    print("| File | Kind | Before: 50 ms mean min / max | Before: peaks - / + | After: 50 ms mean min / max | After: peaks - / + |")
    print("|---|---|---|---|---|---|")
    worst = {"before": 0.0, "after": 0.0}
    for k in sorted(a):
        ma = measure(a[k])
        mb = measure(b[k]) if b and k in b else None
        kind = "locked" if k.startswith("hardware/") else "studio"
        if ma:
            worst["before"] = max(worst["before"], abs(ma["lo"]), abs(ma["hi"])) if kind == "studio" else worst["before"]
        if mb:
            worst["after"] = max(worst["after"], abs(mb["lo"]), abs(mb["hi"])) if kind == "studio" else worst["after"]
        f = lambda m: (f"{m['lo']:+.4f} / {m['hi']:+.4f}", f"{m['min']:+.3f} / {m['max']:+.3f}") if m else ("-", "-")
        print(f"| {k} | {kind} | {f(ma)[0]} | {f(ma)[1]} | {f(mb)[0]} | {f(mb)[1]} |")
    print()
    print(f"studio worst |50 ms mean|: before {worst['before']:.4f}, after {worst['after']:.4f}")


if __name__ == "__main__":
    main()
