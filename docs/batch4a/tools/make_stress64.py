"""The 64-studio-track stress project (BATCH4A_PROMPT.md §9.5, §9.6): no hardware profile,
64 studio tracks (Pulse, VRC6 Pulse, Saw and Triangle in turn, unlimited voices), each with
16 bars of dense 8-note chords (a new chord every beat, each note a beat long: 8 voices per
track sounding all the time, 512 at once).

usage: python make_stress64.py OUT.chipdaw"""
import json
import sys

PPQ = 960
BAR = 4 * PPQ
KINDS = [("pulse", "Pulse"), ("vrc6_pulse", "VRC6 Pulse"), ("saw", "Saw"), ("triangle", "Triangle")]
COLOURS = ["#C9D24A", "#D9579A", "#3CC4A6", "#7F86E0"]
CHORDS = [[48, 52, 55, 59, 62, 65, 69, 72], [45, 48, 52, 55, 59, 62, 65, 69], [50, 53, 57, 60, 64, 67, 71, 74], [43, 47, 50, 53, 57, 60, 64, 67]]

tracks, clips = [], []
for i in range(64):
    voice, name = KINDS[i % 4]
    tracks.append({"id": i + 1, "name": f"{name} {i // 4 + 1}", "voice": voice, "color": COLOURS[i % 4], "params": {}, "inst": 1})
    notes = []
    nid = 1
    for beat in range(16 * 4):
        for p in CHORDS[(beat // 4) % 4]:
            notes.append({"id": nid, "start": beat * PPQ, "len": PPQ, "pitch": p - (12 if voice == "triangle" else 0)})
            nid += 1
    clips.append({"id": i + 1, "lane": i, "start": 0, "len": 16 * BAR, "name": f"Chords {i + 1}", "notes": notes})

p = {"format": "chipdaw", "version": 7, "bpm": 150.0, "profile": "none", "filter": "auto", "loop_end": 16 * BAR,
     "instruments": [{"id": 1, "name": "Raw Channel"}], "loop_start": 0, "loop": True, "tracks": tracks, "clips": clips,
     "rate": 48000, "bits": 24}
json.dump(p, open(sys.argv[1], "w", encoding="utf-8"))
