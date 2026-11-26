"""Writes the Batch 4a test projects (format 7) into a directory (see at_b4a.ps1):
  fx4_studio.chipdaw   Famicom + VRC6: track 1 a studio Pulse at -6 dB and pan 20R with one
                       clip (bars 1-2), track 2 Pulse locked to 2A03 P2, track 3 Triangle locked,
                       track 4 a VRC6 Pulse locked to VRC6 P1 with a clip (bars 1-2).
  fx4_chord.chipdaw    NES: track 1 Pulse locked to 2A03 P1 with an empty 2-bar clip (the piano
                       roll's target), track 2 a studio Pulse.
Usage: python fixtures_b4a.py OUTDIR"""
import json, os, sys

PPQ = 960
BAR = 4 * PPQ
out = sys.argv[1]
os.makedirs(out, exist_ok=True)


def track(tid, name, voice, color, lock=None, **kw):
    t = {"id": tid, "name": name, "voice": voice, "color": color, "params": {}, "inst": 1}
    if lock:
        t["lock"] = lock
    t.update(kw)
    return t


def notes(n, p0, step=PPQ, length=PPQ // 2):
    return [{"id": i + 1, "start": i * step, "len": length, "pitch": p0 + (i % 5)} for i in range(n)]


def project(profile, tracks, clips):
    return {
        "format": "chipdaw", "version": 7, "bpm": 150.0, "profile": profile, "filter": "auto",
        "loop_end": 8 * BAR, "instruments": [{"id": 1, "name": "Raw Channel"}],
        "loop_start": 0, "loop": True, "tracks": tracks, "clips": clips, "rate": 48000, "bits": 24,
    }


FIX = {
    "fx4_studio": project("famicom+vrc6", [
        track(1, "Lead", "pulse", "#C9D24A", volume_db=-6.0, pan=20),
        track(2, "Pulse 2", "pulse", "#C9D24A", lock="2a03_p2"),
        track(3, "Bass", "triangle", "#7F86E0", lock="2a03_tri"),
        track(4, "Shine", "vrc6_pulse", "#D9579A", lock="vrc6_p1"),
    ], [
        {"id": 1, "lane": 0, "start": 0, "len": 2 * BAR, "name": "Lead A", "notes": notes(8, 64)},
        {"id": 2, "lane": 3, "start": 0, "len": 2 * BAR, "name": "Shine A", "notes": notes(8, 72)},
    ]),
    "fx4_chord": project("nes", [
        track(1, "Pulse 1", "pulse", "#C9D24A", lock="2a03_p1"),
        track(2, "Pulse 3", "pulse", "#4FA8E0"),
    ], [
        {"id": 1, "lane": 0, "start": 0, "len": 2 * BAR, "name": "Empty", "notes": []},
    ]),
}

for name, p in FIX.items():
    with open(os.path.join(out, name + ".chipdaw"), "w", encoding="utf-8") as f:
        json.dump(p, f, indent=2)
