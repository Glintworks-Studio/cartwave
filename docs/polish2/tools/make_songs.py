"""Writes the polish 2 test projects (docs/polish2/songs) from the v5 showcase:
mockup.chipdaw (E1-header's four tracks: names, pan, dB, M on track 3, S on track 2),
mockup_rows.chipdaw (E1-row-heights: 72 / 144 / 240 px), rows.chipdaw (72 / 96 / 144 / 240).
usage: python make_songs.py"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))


def song(lanes):
    f = json.load(open(os.path.join(REPO, "docs", "songs", "v5", "showcase.chipdaw"), encoding="utf-8"))
    f["version"] = 6
    for i, kv in enumerate(lanes):
        f["lanes"][i].update({k: v for k, v in kv.items() if v is not None})
    return f


def write(name, f):
    p = os.path.join(REPO, "docs", "polish2", "songs", name + ".chipdaw")
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8", newline="\n") as o:
        json.dump(f, o, indent=2)
    print(p)


write("mockup", song([
    {"name": "Pulse 1", "volume_db": -2.2},
    {"name": "Triangle", "pan": -13, "volume_db": -1.3, "solo": True},
    {"name": "Noise", "pan": 15, "volume_db": -3.9, "mute": True},
    {"name": "VRC6 Pulse 1", "pan": 25, "volume_db": -3.1},
]))
write("mockup_rows", song([
    {"name": "Pulse 1", "volume_db": -2.2},
    {"name": "Triangle", "pan": -13, "volume_db": -1.3, "height": 144.0, "solo": True},
    {"name": "VRC6 Pulse 1", "pan": 25, "volume_db": -3.1, "height": 240.0},
]))
write("rows", song([
    {"volume_db": -2.2},
    {"pan": -13, "volume_db": -1.3, "height": 96.0},
    {"pan": 15, "volume_db": -3.9, "height": 144.0},
    {"pan": 25, "volume_db": -3.1, "height": 240.0},
]))
