"""Writes the Batch 3 test projects (format v4) into a directory:
  fx_basic.chipdaw  Lead A on Pulse 1 (bars 1-4, 16 quarter notes), Harm A / Harm B
                    on Pulse 2 (bars 1-2, 3-4), Bass on the triangle (bars 1-8)
  fx_two.chipdaw    Lead A on Pulse 1 and Harm on Pulse 2, both bars 1-4; the rest empty
  fx_empty_clip.chipdaw  one empty clip on Pulse 1, bars 1-2 (for the piano roll)
The header (instruments, lane controls) is the v3 corpus demo's, so the files are
what the app writes. Usage: python fixtures.py OUTDIR"""
import json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
BASE = json.load(open(os.path.join(HERE, "..", "..", "songs", "v3", "demo_band.chipdaw"), encoding="utf-8"))
PPQ = 960
BAR = 4 * PPQ


def notes(n, first_pitch, step=PPQ, length=PPQ // 2):
    return [{"id": i + 1, "start": i * step, "len": length, "pitch": first_pitch + (i % 12)} for i in range(n)]


def clip(cid, lane, bar0, bars, name, ns):
    return {"id": cid, "lane": lane, "start": bar0 * BAR, "len": bars * BAR, "name": name, "notes": ns}


def project(clips):
    p = dict(BASE)
    p["version"] = 4
    p["clips"] = clips
    return p


FIXTURES = {
    "fx_basic": [
        clip(1, 0, 0, 4, "Lead A", notes(16, 60)),
        clip(2, 1, 0, 2, "Harm A", notes(8, 64)),
        clip(3, 1, 2, 2, "Harm B", notes(8, 67)),
        clip(4, 2, 0, 8, "Bass", notes(16, 40, step=2 * PPQ, length=PPQ)),
    ],
    "fx_two": [
        clip(1, 0, 0, 4, "Lead A", notes(16, 60)),
        clip(2, 1, 0, 4, "Harm", notes(16, 64)),
    ],
    "fx_empty_clip": [clip(1, 0, 0, 2, "Empty", [])],
}

if __name__ == "__main__":
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    for name, clips in FIXTURES.items():
        with open(os.path.join(out, name + ".chipdaw"), "w", encoding="utf-8") as f:
            json.dump(project(clips), f, indent=2)
    print("fixtures written to", out)
