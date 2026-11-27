"""Batch 4b fixtures (BATCH4B_PROMPT.md): projects the gates and scenarios open.

  fx_board.chipdaw    The `main-track` / `fixups` boards' song: 2A03 + VRC6, four locked tracks
                      (Pulse 1, Triangle, Noise, VRC6 Pulse 1) with the boards' clips, Main at -3 dB.
  fx_board12.chipdaw  The same with eight more studio tracks first (track numbers up to 12: the
                      `fixups` board's two-digit column).
  fx_mixer.chipdaw    The `mixer` board's song: Famicom + VRC6, 9 tracks numbered 8-16 on the board
                      (here 1-9 after 7 empty studio tracks: 16 tracks), locked 2A03/VRC6 channels
                      and studio tracks with the board's pans and faders.
  fx_s64u.chipdaw     The 64-studio-track stress project with unison 2 / detune 15 on every
                      track (the audio gate, §9.5).

usage: python fixtures_b4b.py OUT_DIR"""
import json
import os
import sys

PPQ = 960
BAR = 4 * PPQ


def track(tid, name, voice, color, lock=None, **kw):
    t = {"id": tid, "name": name, "voice": voice, "color": color, "params": {}, "inst": 1}
    if lock:
        t["lock"] = lock
    t.update(kw)
    return t


def notes(n, p0, step=PPQ, length=PPQ // 2):
    return [{"id": i + 1, "start": i * step, "len": length, "pitch": p0 + (i % 5) * 2} for i in range(n)]


def clip(cid, lane, bar, bars, name, ns):
    return {"id": cid, "lane": lane, "start": bar * BAR, "len": bars * BAR, "name": name, "notes": ns}


def project(profile, tracks, clips, **kw):
    p = {"format": "chipdaw", "version": 8, "bpm": 150.0, "profile": profile, "filter": "auto", "loop_end": 8 * BAR,
         "instruments": [{"id": 1, "name": "Raw Channel"}], "loop_start": 0, "loop": True, "tracks": tracks, "clips": clips,
         "rate": 48000, "bits": 24}
    p.update(kw)
    return p


def board_tracks(first_id=1):
    return [
        track(first_id, "Pulse 1", "pulse", "#C9D24A", lock="2a03_p1", volume_db=-2.2),
        track(first_id + 1, "Triangle", "triangle", "#7F86E0", lock="2a03_tri", pan=-13, volume_db=-1.3),
        track(first_id + 2, "Noise", "noise", "#B5AC98", lock="2a03_noise", pan=15, volume_db=-3.9),
        track(first_id + 3, "VRC6 Pulse 1", "vrc6_pulse", "#D9579A", lock="vrc6_p1", pan=25, volume_db=-3.1),
    ]


def board_clips(lane0=0, id0=1):
    return [
        clip(id0, lane0, 0, 4, "Intro Lead", notes(16, 60)),
        clip(id0 + 1, lane0, 4, 4, "Hook A", notes(16, 64)),
        clip(id0 + 2, lane0 + 1, 0, 8, "Bassline", notes(32, 40)),
        clip(id0 + 3, lane0 + 2, 2, 6, "Groove", notes(24, 36)),
        clip(id0 + 4, lane0 + 3, 4, 4, "Strings", notes(16, 60)),
    ]


def board():
    # Locked tracks open with their stored fader/pan only through before_lock; the boards show
    # the locked rows with values, but the app pins locked tracks at 0 dB / C (VISUAL-SPEC §4b).
    t = board_tracks()
    for x in t:
        x.pop("volume_db", None)
        x.pop("pan", None)
    return project("famicom+vrc6", t, board_clips(), master_db=-3.0)


def board12():
    studio = [track(i + 1, f"Pulse {i + 3}", "pulse", "#4FA8E0") for i in range(8)]
    t = studio + board_tracks(9)
    for x in t[8:]:
        x.pop("volume_db", None)
        x.pop("pan", None)
    return project("famicom+vrc6", t, board_clips(8, 1), master_db=-3.0)


def mixer():
    # The board's strips 8..16: 2A03 P1, P2, TRI, NOI (locked), then studio tracks with pans
    # 13L, 15R, 25R, C, 25L and faders -6, -8.5, -10, -1.5, -12 dB; Main at -3 dB.
    filler = [track(i + 1, f"Track {i + 1}", "pulse", "#4FA8E0") for i in range(7)]
    t = filler + [
        track(8, "Pulse 1", "pulse", "#C9D24A", lock="2a03_p1"),
        track(9, "Pulse 2", "pulse", "#E39A42", lock="2a03_p2"),
        track(10, "Triangle", "triangle", "#7F86E0", lock="2a03_tri"),
        track(11, "Noise", "noise", "#B5AC98", lock="2a03_noise"),
        track(12, "Lead Stack", "pulse", "#3CC4A6", pan=-13, volume_db=-6.0, unison=3, detune=12),
        track(13, "Saw Pad", "saw", "#D9579A", pan=15, volume_db=-8.5),
        track(14, "Arp Bells", "pulse", "#7FD08A", pan=25, volume_db=-10.0),
        track(15, "Kick Sweep", "triangle", "#E0705A", volume_db=-1.5),
        track(16, "Echo Lead", "pulse", "#A77BE0", pan=-25, volume_db=-12.0),
    ]
    clips = [clip(i + 1, 7 + i, 0, 8, f"Part {i + 1}", notes(32, 55 + 2 * i, PPQ // 2, PPQ // 2)) for i in range(9)]
    return project("famicom+vrc6", t, clips, master_db=-3.0)


CHORDS = [[48, 52, 55, 59, 62, 65, 69, 72], [45, 48, 52, 55, 59, 62, 65, 69], [50, 53, 57, 60, 64, 67, 71, 74], [43, 47, 50, 53, 57, 60, 64, 67]]
KINDS = [("pulse", "Pulse"), ("vrc6_pulse", "VRC6 Pulse"), ("saw", "Saw"), ("triangle", "Triangle")]
COLOURS = ["#4FA8E0", "#D9579A", "#3CC4A6", "#7F86E0"]


def s64u():
    tracks, clips = [], []
    for i in range(64):
        voice, name = KINDS[i % 4]
        tracks.append(track(i + 1, f"{name} {i // 4 + 1}", voice, COLOURS[i % 4], unison=2, detune=15))
        ns = []
        nid = 1
        for beat in range(16 * 4):
            for p in CHORDS[(beat // 4) % 4]:
                ns.append({"id": nid, "start": beat * PPQ, "len": PPQ, "pitch": p - (12 if voice == "triangle" else 0)})
                nid += 1
        clips.append(clip(i + 1, i, 0, 16, f"Chords {i + 1}", ns))
    return project("none", tracks, clips, loop_end=16 * BAR)


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    for name, p in [("fx_board", board()), ("fx_board12", board12()), ("fx_mixer", mixer()), ("fx_s64u", s64u())]:
        with open(os.path.join(out, name + ".chipdaw"), "w", encoding="utf-8") as f:
            json.dump(p, f, indent=1)
    print("ok")


if __name__ == "__main__":
    main()
