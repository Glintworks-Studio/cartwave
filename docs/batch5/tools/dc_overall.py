"""Batch 5 A3: the true DC of each listening-pack WAV, before and after.

The 50 ms running mean (a3_table.py) also catches a low note's or a slow noise's
own waveform. This prints, per file, the mean over the whole time the preset
sounds (first to last sample above -60 dBFS): its DC. Then the median and the
largest |DC| of the studio files, before and after.

usage: python dc_overall.py <before pack> <after pack> [name filter ...]
"""
import os
import struct
import sys
import wave


def dc(path):
    with wave.open(path, "rb") as w:
        n, ch = w.getnframes(), w.getnchannels()
        x = [v / 32768.0 for v in struct.unpack("<%dh" % (n * ch), w.readframes(n))[::ch]]
    idx = [i for i, v in enumerate(x) if abs(v) > 10 ** (-3)]
    if not idx:
        return 0.0
    seg = x[idx[0]: idx[-1] + 1]
    return sum(seg) / len(seg)


def main():
    before, after = sys.argv[1], sys.argv[2]
    filt = [f.lower() for f in sys.argv[3:]]
    rows = []
    for d, _, fs in os.walk(after):
        for f in fs:
            if not f.endswith(".wav"):
                continue
            rel = os.path.relpath(os.path.join(d, f), after).replace("\\", "/")
            if rel.startswith("hardware/") or (filt and not any(k in rel for k in filt)):
                continue
            b = os.path.join(before, rel)
            rows.append((rel, dc(b) if os.path.exists(b) else float("nan"), dc(os.path.join(after, rel))))
    rows.sort()
    for r in rows:
        print(f"{r[0]}\t{r[1]:+.4f}\t{r[2]:+.4f}")
    ab = sorted(abs(r[1]) for r in rows)
    aa = sorted(abs(r[2]) for r in rows)
    print(f"studio files {len(rows)}: |DC| median before {ab[len(ab) // 2]:.4f}, after {aa[len(aa) // 2]:.4f}; largest before {ab[-1]:.4f}, after {aa[-1]:.4f}")


if __name__ == "__main__":
    main()
