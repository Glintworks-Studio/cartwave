#!/usr/bin/env python3
"""The first preset bank (PRESETS_PROMPT.md §7, VISION.md "Presets").

Every preset here is designed from scratch with Cartwave's own macros: per-frame
volume, arpeggio (with its speed), pitch and duty sequences, delayed vibrato, the
channel controls, and on studio presets Unison / Detune / Drift. Nothing is
copied from games, ROMs, ripped patches or any third-party library, and no game,
console or brand name is used: names describe the sound. Author: Jeffrey Scott
Frederick.

Usage:  python presets/tools/design_bank.py [--levels presets/levels.json]
Writes  presets/bank.json  (read by crates/ui/src/bank.rs at build time).

`--levels` is the level-matching pass (§7.3): `crates/ui/examples/bank_pack.rs`
measures every preset playing its phrase (EBU R128 loudness) and writes, per
preset, the gain that brings it to its category's target; this script folds
that gain into the preset's volume (the channel Volume / Rate control, or the
volume macro's values), so the bank ships level-matched.

Sequence text is the project file's: values separated by spaces, "|" before
the loop start, "/" after the release step (Cartwave's file format).
"""
import argparse
import json
import math
import os

AUTHOR = "Jeffrey Scott Frederick"
DATE = "2026-11-27"
TOOL = "presets/tools/design_bank.py"

# ───────────────────────────── macro helpers ─────────────────────────────


def seq(vals, loop=None, rel=None):
    """Sequence text from values; `loop` / `rel` are step indices."""
    out = []
    for i, v in enumerate(vals):
        if loop is not None and i == loop:
            out.append("|")
        out.append(str(int(v)))
        if rel is not None and i == rel:
            out.append("/")
    assert len(vals) <= 64, vals
    return " ".join(out)


def env(attack=0, peak=15, decay=6, sustain=10, release=8, start=None, curve=1.6, hold=0):
    """An envelope as a volume macro: rise over `attack` frames to `peak`, hold
    `hold` frames, fall over `decay` frames to `sustain` (the release point:
    held there while the note is held), then fall to 0 over `release` frames.
    `sustain` 0 makes a one-shot (no release point)."""
    v = []
    if attack > 0:
        s0 = peak * 0.25 if start is None else start
        for i in range(attack):
            v.append(s0 + (peak - s0) * (i + 1) / attack)
    else:
        v.append(peak)
    v += [peak] * hold
    for i in range(decay):
        t = (i + 1) / decay
        v.append(peak + (sustain - peak) * (1 - (1 - t) ** curve))
    if sustain <= 0:
        v.append(0)
        return seq([round(x) for x in v])
    rel = len(v) - 1
    for i in range(release):
        t = (i + 1) / release
        v.append(sustain * (1 - t) ** 1.4)
    v.append(0)
    vals = [max(0, min(15, round(x))) for x in v]
    return seq(vals, None, rel)


def decay(peak=15, frames=20, curve=2.0, floor=0):
    """A one-shot decay from `peak` to `floor` over `frames` frames (then 0)."""
    v = [round(floor + (peak - floor) * (1 - i / frames) ** curve) for i in range(frames)]
    v.append(0)
    return seq([max(0, min(15, x)) for x in v])


def echo(peak=15, sustain=10, gap=6, echo_level=0.45, tail=3):
    """The classic echo: the note, then a quieter delayed copy of its attack
    (as drivers did with a duplicate a few frames late), then the sustain; the
    release repeats it quieter still."""
    att = [peak, peak - 1, peak - 2, sustain + 1, sustain]
    gap_v = [max(1, round(sustain * 0.35))] * 2 + [0] * max(0, gap - 2)
    e = round(peak * echo_level)
    rep = [e, max(1, e - 1), max(1, round(sustain * 0.8))]
    v = att + gap_v + rep
    rel = len(v) - 1
    for k in range(tail):
        lvl = round(sustain * 0.5 ** (k + 1))
        v += [lvl, max(0, lvl - 1), 0, 0, 0]
    v.append(0)
    return seq([max(0, min(15, x)) for x in v], None, rel)


def tremolo(level=12, depth=3, period=8, attack=4):
    """A held level with a slow volume wobble (a loop), released over 6 frames."""
    v = [round(level * (i + 1) / attack) for i in range(attack)] if attack else []
    lp = len(v)
    for i in range(period):
        v.append(round(level - depth * 0.5 * (1 - math.cos(2 * math.pi * i / period))))
    rel = len(v) - 1
    v += [round(level * 0.6), round(level * 0.35), round(level * 0.15), 0]
    return seq([max(0, min(15, x)) for x in v], lp, rel)


def gate(frames, level=15):
    """On for `frames` frames, then off (the triangle's volume is a gate)."""
    return seq([level] * frames + [0])


def arp(steps, loop=True):
    return seq(steps, 0 if loop else None, None)


def duty_loop(steps):
    return seq(steps, 0, None)


def ramp(a, b, n):
    return [round(a + (b - a) * i / max(1, n - 1)) for i in range(n)]


# ───────────────────────────── the bank ─────────────────────────────

PRESETS = []


def P(name, path, voice, blurb, tags, phrase, *, vol="", arpeggio="", pitch="", duty="", vib=(0, 0, 0), arp_speed=1,
      params=None, unison=None, profile=None, kit=None, octave=0):
    """One preset. `path` is its browser folder; `voice` its Chip Synth voice
    (pulse, triangle, noise, dpcm, vrc6_pulse, saw); `profile` (2A03 / VRC6)
    makes it an authentic preset (Hardware); `phrase` and `octave` pick what the
    listening pack plays."""
    inst = {"name": name, "volume": vol, "arpeggio": arpeggio, "pitch": pitch, "duty": duty, "vibrato": list(vib)}
    if arp_speed != 1:
        inst["arp_speed"] = arp_speed
    if kit:
        inst["kit"] = kit
    PRESETS.append({
        "name": name,
        "path": path,
        "voice": voice,
        "profile": profile,
        "tags": tags,
        "blurb": blurb,
        "phrase": phrase,
        "octave": octave,
        "instrument": inst,
        # A copy: the voices' shared defaults (SQ, VP, SAW, NZ) must not
        # change when one preset's level is matched.
        "params": dict(params or {}),
        "unison": list(unison) if unison else [1, 0, 0],
        "author": AUTHOR,
        "date": DATE,
        "tool": TOOL,
    })


def piece(name, top, pitch=None, **kw):
    """A kit piece (noise): notes up to `top` play it (at `pitch`, or the note's)."""
    inst = {"name": name, "volume": kw.get("vol", ""), "arpeggio": kw.get("arpeggio", ""), "pitch": kw.get("pitch", ""),
            "duty": kw.get("duty", ""), "vibrato": [0, 0, 0]}
    k = {"top": top, "inst": inst}
    if pitch is not None:
        k["pitch"] = pitch
    return k


SQ = {"envelope": 0, "sweep": 0, "volume": 15}           # pulse: constant volume, no sweep, full
NZ = {"envelope": 0, "noise_period": -1, "volume": 15}    # noise: the note picks the period
VP = {"volume": 15}                                      # VRC6 pulse
SAW = {"rate": 42}                                       # saw: the loudest rate without wrapping

L = ["Sounds", "Leads"]
B = ["Sounds", "Basses"]
A = ["Sounds", "Arps"]
PD = ["Sounds", "Pads"]
K = ["Sounds", "Bells & Keys"]
FX = ["Sounds", "FX"]
G = ["Sounds", "Game Sounds"]
DK = ["Drums", "Kits"]
DH = ["Drums", "Hits"]

# ── Leads (15)
P("Bright Square Lead", L + ["Square"], "pulse", "25% pulse, a quick bite, a gentle fall to sustain and delayed vibrato",
  ["lead", "pulse", "bright"], "lead", vol=env(0, 15, 8, 10, 10), duty=seq([4]), vib=(16, 6, 18), params=SQ)
P("Late Vibrato Lead", L + ["Square"], "pulse", "Round 50% square that holds still, then blooms into a wide vibrato",
  ["lead", "pulse", "vibrato", "warm"], "lead", vol=env(1, 14, 6, 11, 8), duty=seq([8]), vib=(30, 7, 34), params=SQ)
P("Duty Sweep Lead", L + ["Square"], "pulse", "The pulse width sweeps thin to full and back as the note holds",
  ["lead", "pulse", "duty sweep", "moving"], "lead", vol=env(0, 15, 6, 11, 8),
  duty=seq([2, 2, 3, 4, 5, 6, 7, 8, 8, 7, 6, 5, 4, 3], 2, None), vib=(12, 6, 24), params=SQ)
P("Hollow 12.5%", L + ["Thin & Nasal"], "pulse", "Thin 12.5% pulse with a soft swell: reedy and hollow",
  ["lead", "pulse", "thin", "nasal"], "lead", vol=env(3, 13, 4, 11, 8, start=6), duty=seq([2]), vib=(14, 6, 20), params=SQ)
P("Nasal Reed Lead", L + ["Thin & Nasal"], "vrc6_pulse", "VRC6 pulse at 3/16: a buzzy reed with a quick vibrato",
  ["lead", "vrc6", "nasal", "reed"], "lead", vol=env(2, 14, 5, 11, 8, start=8), duty=seq([3]), vib=(18, 5, 14), params=VP)
P("Glass Pulse Lead", L + ["Thin & Nasal"], "vrc6_pulse", "The VRC6's thinnest pulse (1/16): glassy and piercing",
  ["lead", "vrc6", "thin", "glassy"], "lead", vol=env(0, 15, 10, 9, 10), duty=seq([1]), vib=(10, 6, 16), params=VP)
P("Echo Square", L + ["Echo"], "pulse", "50% square with the classic echo: a quieter delayed repeat of every attack",
  ["lead", "pulse", "echo"], "lead", vol=echo(15, 10, 7, 0.45), duty=seq([8]), vib=(14, 6, 26), params=SQ)
P("Echo Saw Lead", L + ["Echo"], "saw", "Saw lead with a fading echo tail after each note",
  ["lead", "saw", "echo"], "lead", vol=echo(15, 11, 8, 0.5, 4), vib=(12, 6, 24), params=SAW)
P("Supersquare", L + ["Detuned"], "pulse", "Four detuned squares drifting apart: wide and thick (studio only)",
  ["lead", "pulse", "detuned", "wide", "studio"], "lead", vol=env(0, 15, 8, 11, 10), duty=seq([6]), vib=(8, 6, 30),
  params=SQ, unison=(4, 18, 3))
P("Wide Detuned Lead", L + ["Detuned"], "vrc6_pulse", "Three VRC6 pulses detuned for a chorus-like spread (studio only)",
  ["lead", "vrc6", "detuned", "wide", "studio"], "lead", vol=env(1, 15, 6, 11, 10), duty=seq([5]), vib=(10, 6, 24),
  params=VP, unison=(3, 12, 2))
P("Detuned Saw Lead", L + ["Detuned"], "saw", "Three saws spread 14 cents: a big, buzzing lead (studio only)",
  ["lead", "saw", "detuned", "big", "studio"], "lead", vol=env(0, 15, 6, 12, 10), vib=(10, 6, 28), params=SAW, unison=(3, 14, 2))
P("Saw Lead", L + ["Saw"], "saw", "Plain VRC6 saw with a soft decay and delayed vibrato",
  ["lead", "saw", "bright"], "lead", vol=env(0, 15, 8, 11, 10), vib=(16, 6, 22), params=SAW)
P("Scoop Lead", L + ["Expressive"], "pulse", "Each note scoops up from below into a 25% tone",
  ["lead", "pulse", "scoop", "expressive"], "lead", vol=env(1, 15, 6, 11, 8),
  pitch=seq([-40, 8, 8, 8, 8, 8]), duty=seq([4]), vib=(16, 6, 20), params=SQ)
P("Trill Lead", L + ["Expressive"], "pulse", "A whole-tone trill (the arpeggio at speed 3): ornamented and lively",
  ["lead", "pulse", "trill", "arpeggio"], "lead", vol=env(0, 14, 6, 11, 8), arpeggio=arp([0, 2]), arp_speed=3,
  duty=seq([4]), params=SQ)
P("Soft Triangle Lead", L + ["Soft"], "triangle", "Pure triangle with a gentle vibrato: a flute-like lead",
  ["lead", "triangle", "soft", "flute"], "lead", vib=(20, 7, 16))

# ── Basses (14)
P("Round Triangle Bass", B + ["Triangle"], "triangle", "Triangle bass with a tiny dip on the attack: round and deep",
  ["bass", "triangle", "deep"], "bass", pitch=seq([-6, 3, 3]), octave=0)
P("Kick-Drop Triangle Bass", B + ["Triangle"], "triangle", "The classic triangle kick-drop: a fast pitch dive into every bass note",
  ["bass", "triangle", "kick drop", "punchy"], "bass", arpeggio=seq([24, 12, 5, 0]))
P("Staccato Triangle Bass", B + ["Triangle"], "triangle", "Short gated triangle notes for bouncing basslines",
  ["bass", "triangle", "short"], "bass", vol=gate(7))
P("Triangle Slide Bass", B + ["Triangle"], "triangle", "Slides up a semitone into each note",
  ["bass", "triangle", "slide"], "bass", pitch=seq([-16, 4, 4, 4, 4]))
P("Thick Pulse Bass", B + ["Pulse"], "pulse", "50% pulse bass, a firm attack settling to a full sustain",
  ["bass", "pulse", "thick"], "bass", vol=env(0, 15, 6, 12, 6), duty=seq([8]), params=SQ)
P("Pluck Pulse Bass", B + ["Pulse"], "pulse", "25% pluck that dies quickly: tight and funky",
  ["bass", "pulse", "pluck", "short"], "bass", vol=decay(15, 16, 1.6), duty=seq([4]), params=SQ)
P("Growl Pulse Bass", B + ["Pulse"], "pulse", "The duty flicks between 12.5% and 25% every frame: a gritty growl",
  ["bass", "pulse", "growl", "gritty"], "bass", vol=env(0, 15, 4, 12, 6), duty=duty_loop([2, 4]), params=SQ)
P("Octave Bounce Bass", B + ["Pulse"], "pulse", "Jumps between the note and its octave four frames at a time",
  ["bass", "pulse", "octave", "arpeggio"], "bass", vol=env(0, 15, 4, 12, 6), arpeggio=arp([0, 12]), arp_speed=4,
  duty=seq([8]), params=SQ)
P("Hollow Bass", B + ["VRC6"], "vrc6_pulse", "VRC6 pulse at 4/16: hollow, woody low end",
  ["bass", "vrc6", "hollow"], "bass", vol=env(0, 15, 6, 11, 6), duty=seq([4]), params=VP)
P("Saw Sub Bass", B + ["Saw"], "saw", "A soft, low saw: weight without bite",
  ["bass", "saw", "sub", "soft"], "bass", vol=env(2, 14, 6, 12, 8, start=8), params=SAW)
P("Saw Pluck Bass", B + ["Saw"], "saw", "Saw bass with a bright pluck and a short tail",
  ["bass", "saw", "pluck"], "bass", vol=decay(15, 20, 1.4, 4), params=SAW)
P("Detuned Saw Bass", B + ["Detuned"], "saw", "Two saws 8 cents apart: a fat, moving bass (studio only)",
  ["bass", "saw", "detuned", "fat", "studio"], "bass", vol=env(0, 15, 6, 12, 6), params=SAW, unison=(2, 8, 1))
P("Wide Pulse Bass", B + ["Detuned"], "pulse", "Three detuned 50% pulses: a wide, chorused bass (studio only)",
  ["bass", "pulse", "detuned", "wide", "studio"], "bass", vol=env(0, 15, 6, 12, 6), duty=seq([8]), params=SQ, unison=(3, 10, 2))
P("Rubber Bass", B + ["Pulse"], "pulse", "Each note bends down a little after the attack: bouncy and elastic",
  ["bass", "pulse", "bend", "bouncy"], "bass", vol=env(0, 15, 8, 10, 6), pitch=seq([0, 0, -3, -3, -2, -2, -1, -1]),
  duty=seq([6]), params=SQ)

# ── Arps (14)
P("Fast Major Arp", A + ["Major & Minor"], "pulse", "Root, third, fifth every frame: a full major chord from one channel",
  ["arp", "pulse", "major", "fast"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 4, 7]), duty=seq([4]), params=SQ)
P("Fast Minor Arp", A + ["Major & Minor"], "pulse", "A minor chord arpeggiated every frame",
  ["arp", "pulse", "minor", "fast"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 3, 7]), duty=seq([4]), params=SQ)
P("Major Seventh Arp", A + ["Sevenths"], "vrc6_pulse", "Major seventh, shimmering on a 6/16 VRC6 pulse",
  ["arp", "vrc6", "major seventh", "lush"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 4, 7, 11]), duty=seq([6]), params=VP)
P("Minor Seventh Arp", A + ["Sevenths"], "pulse", "Minor seventh at 12.5%: moody and bright",
  ["arp", "pulse", "minor seventh", "moody"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 3, 7, 10]), duty=seq([2]), params=SQ)
P("Dominant Arp", A + ["Sevenths"], "saw", "Dominant seventh on the saw: tense, wants to resolve",
  ["arp", "saw", "dominant", "tense"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 4, 7, 10]), params=SAW)
P("Diminished Arp", A + ["Sevenths"], "pulse", "Stacked minor thirds: suspense and danger",
  ["arp", "pulse", "diminished", "dark"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 3, 6, 9]), duty=seq([8]), params=SQ)
P("Suspended Shimmer Arp", A + ["Major & Minor"], "vrc6_pulse", "Root, fourth, fifth, octave: open and unresolved",
  ["arp", "vrc6", "suspended", "open"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 5, 7, 12]), duty=seq([3]), params=VP)
P("Octave Pump Arp", A + ["Octave & Power"], "pulse", "Note and octave, two frames each: pumping",
  ["arp", "pulse", "octave"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 12]), arp_speed=2, duty=seq([8]), params=SQ)
P("Power Fifth Arp", A + ["Octave & Power"], "saw", "Root, fifth and octave on the saw: a power chord",
  ["arp", "saw", "power", "fifth"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 7, 12]), params=SAW)
P("Slow Major Arp", A + ["Slow"], "pulse", "A major arpeggio you can hear note by note (speed 4)",
  ["arp", "pulse", "major", "slow"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 4, 7, 12]), arp_speed=4, duty=seq([4]), params=SQ)
P("Slow Minor Arp", A + ["Slow"], "vrc6_pulse", "A minor arpeggio at speed 4 on a soft VRC6 pulse",
  ["arp", "vrc6", "minor", "slow"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 3, 7, 12]), arp_speed=4, duty=seq([7]), params=VP)
P("Rolling Arp", A + ["Slow"], "pulse", "Up and down the major chord, two frames a step",
  ["arp", "pulse", "rolling", "major"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 4, 7, 12, 7, 4]), arp_speed=2,
  duty=seq([4]), params=SQ)
P("Detuned Chord Arp", A + ["Detuned"], "saw", "A fast major arpeggio on two detuned saws (studio only)",
  ["arp", "saw", "detuned", "studio"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 4, 7]), params=SAW, unison=(2, 10, 1))
P("Triangle Octave Arp", A + ["Octave & Power"], "triangle", "The triangle flips octave and fifth: a bass arpeggio",
  ["arp", "triangle", "octave", "bass"], "bass", arpeggio=arp([0, 12, 7]), arp_speed=2)

# ── Pads (7)
P("Breathing Duty Pad", PD + ["Pulse"], "pulse", "A slow swell; the duty breathes from 12.5% to 50% and back",
  ["pad", "pulse", "duty sweep", "slow"], "pad", vol=env(14, 12, 4, 11, 16, start=2),
  duty=duty_loop([2, 2, 3, 3, 4, 5, 6, 7, 8, 8, 7, 6, 5, 4, 3, 3]), vib=(8, 9, 30), params=SQ)
P("Warm Square Pad", PD + ["Pulse"], "pulse", "Two slightly detuned squares fading in (studio only)",
  ["pad", "pulse", "warm", "detuned", "studio"], "pad", vol=env(16, 12, 2, 11, 16, start=1), duty=seq([8]), vib=(6, 9, 40),
  params=SQ, unison=(2, 6, 2))
P("Soft Choir Pad", PD + ["VRC6"], "vrc6_pulse", "VRC6 pulse at 7/16 with a slow attack and vibrato: airy",
  ["pad", "vrc6", "airy", "choir"], "pad", vol=env(18, 12, 2, 11, 16, start=1), duty=seq([7]), vib=(14, 8, 24), params=VP)
P("Glass Pad", PD + ["VRC6"], "vrc6_pulse", "Two thin VRC6 pulses drifting: a glassy shimmer (studio only)",
  ["pad", "vrc6", "glassy", "studio"], "pad", vol=env(14, 11, 2, 10, 16, start=1), duty=seq([2]), vib=(10, 10, 30),
  params=VP, unison=(2, 9, 4))
P("Wide Saw Pad", PD + ["Saw"], "saw", "Four drifting saws, a long fade-in: a big, wide wash (studio only)",
  ["pad", "saw", "wide", "studio"], "pad", vol=env(20, 13, 2, 12, 18, start=1), vib=(6, 10, 40), params=SAW, unison=(4, 20, 5))
P("Organ Pad", PD + ["Pulse"], "pulse", "A steady 50% organ with a gentle tremolo",
  ["pad", "pulse", "organ", "tremolo"], "pad", vol=tremolo(13, 3, 10, 3), duty=seq([8]), params=SQ)
P("Shimmer Pad", PD + ["Saw"], "saw", "A slow octave shimmer (arpeggio speed 8) with vibrato",
  ["pad", "saw", "shimmer", "octave"], "pad", vol=env(12, 12, 2, 11, 16, start=2), arpeggio=arp([0, 12]), arp_speed=8, vib=(8, 8, 20), params=SAW)

# ── Bells & Keys (7)
P("Clear Bell", K + ["Bells"], "pulse", "An octave strike, then a long ringing decay at 25%",
  ["bell", "pulse", "clear"], "keys", vol=decay(15, 48, 1.3), arpeggio=seq([12, 12, 0]), duty=seq([4]), params=SQ)
P("Glass Bell", K + ["Bells"], "vrc6_pulse", "Two octaves up for the strike, then a thin, glassy ring",
  ["bell", "vrc6", "glassy"], "keys", vol=decay(15, 44, 1.2), arpeggio=seq([24, 12, 0]), duty=seq([2]), params=VP)
P("Music Box Tine", K + ["Bells"], "pulse", "A tiny plucked tine at 12.5%",
  ["keys", "pulse", "music box", "small"], "keys", vol=decay(12, 24, 1.8), arpeggio=seq([12, 0]), duty=seq([2]), vib=(6, 4, 4), params=SQ)
P("Electric Piano", K + ["Keys"], "vrc6_pulse", "A soft 6/16 tine with a decay and a hint of detune (studio only)",
  ["keys", "vrc6", "piano", "soft", "studio"], "keys", vol=env(0, 15, 24, 6, 12), duty=seq([6, 6, 5]), vib=(4, 8, 20),
  params=VP, unison=(2, 5, 1))
P("Harpsichord Pluck", K + ["Keys"], "pulse", "A bright pluck that thins from 25% to 12.5% as it fades",
  ["keys", "pulse", "pluck", "bright"], "keys", vol=decay(15, 30, 1.5), duty=seq([4, 4, 4, 3, 3, 2]), params=SQ)
P("Vibraphone", K + ["Keys"], "pulse", "A mallet strike with a tremolo in the decay",
  ["keys", "pulse", "mallet", "tremolo"], "keys",
  vol=seq([15, 13, 12, 11, 10, 10, 9, 10, 9, 8, 9, 8, 7, 8, 7, 6, 7, 6, 5, 6, 5, 4, 5, 4, 3, 4, 3, 2, 3, 2, 1, 2, 1, 0]),
  duty=seq([8]), vib=(6, 8, 6), params=SQ)
P("Saw Celesta", K + ["Bells"], "saw", "A bright saw strike with an octave twinkle",
  ["bell", "saw", "celesta", "bright"], "keys", vol=decay(14, 34, 1.4), arpeggio=seq([12, 0, 12, 0]), params=SAW)

# ── FX (10)
P("Rising Sweep", FX + ["Sweeps"], "pulse", "The pitch climbs as long as the note is held",
  ["fx", "pulse", "rise", "sweep"], "fx", vol=env(0, 13, 2, 12, 8), pitch=seq([6], 0), duty=seq([4]), params=SQ)
P("Falling Bomb", FX + ["Sweeps"], "pulse", "A long whistling fall",
  ["fx", "pulse", "fall", "whistle"], "fx", vol=env(0, 12, 2, 12, 6), pitch=seq([-5], 0), duty=seq([8]), params=SQ, octave=1)
P("Warp Down", FX + ["Sweeps"], "saw", "The saw dives in pitch with a wobble",
  ["fx", "saw", "fall", "warp"], "fx", vol=env(0, 14, 4, 12, 8), pitch=seq([-12, -8], 0), vib=(40, 4, 0), params=SAW)
P("Noise Riser", FX + ["Noise"], "noise", "Noise that climbs through the periods: tension building",
  ["fx", "noise", "rise", "riser"], "fx", vol=env(10, 13, 2, 12, 8, start=4), arpeggio=seq([-8, -7, -6, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 6], 14),
  arp_speed=3, duty=seq([0]), params=NZ)
P("Ocean Wash", FX + ["Noise"], "noise", "Long-mode noise swelling and receding (hold the note)",
  ["fx", "noise", "ocean", "ambient"], "fx", vol=seq(ramp(2, 11, 16) + ramp(11, 2, 16), 0, None), arpeggio=seq([-3]), duty=seq([0]), params=NZ)
P("Static Crackle", FX + ["Noise"], "noise", "Short-mode noise flickering: radio static",
  ["fx", "noise", "static", "crackle"], "fx", vol=seq([12, 4, 9, 2, 11, 6, 3, 10, 5, 8], 0, None), arpeggio=seq([2, -1, 3, 0, 1], 0),
  duty=seq([1]), params=NZ)
P("Alarm", FX + ["Tones"], "pulse", "Two tones alternating six frames apart: a warning siren",
  ["fx", "pulse", "alarm", "siren"], "fx", vol=env(0, 13, 2, 12, 4), arpeggio=arp([0, 7]), arp_speed=6, duty=seq([4]), params=SQ)
P("Teleport", FX + ["Tones"], "vrc6_pulse", "A fast climbing arpeggio that rises in pitch: beaming out",
  ["fx", "vrc6", "teleport", "rise"], "fx", vol=decay(14, 40, 1.0), arpeggio=arp([0, 7, 12, 19]), pitch=seq([3], 0), duty=seq([2]), params=VP)
P("Ray Gun", FX + ["Tones"], "saw", "A wobbling saw that drops on every repeat",
  ["fx", "saw", "ray", "zap"], "fx", vol=seq([15, 14, 13, 12, 11, 10, 9, 8], 0, None), pitch=seq([-20, -20, -20, -20, -20, -20, 120], 0), params=SAW)
P("Heartbeat", FX + ["Tones"], "triangle", "Two soft thumps: a triangle double kick-drop",
  ["fx", "triangle", "heartbeat", "low"], "fx", vol=seq([15, 15, 15, 15, 0, 0, 0, 15, 15, 15, 0]), arpeggio=seq([12, 4, 0, -3, -3, -3, -3, 10, 2, -3, -3]))

# ── Game Sounds (12: coin, jump, hit, menu blip, power-up, 1-up, explosion, laser, select, error, door, pickup)
P("Coin Chime", G, "pulse", "Pick-up chime: a short note leaps a fourth and rings (try B5)",
  ["game", "pulse", "coin", "pickup"], "game", vol=seq([13] * 5 + ramp(13, 0, 18)), arpeggio=seq([0, 0, 0, 0, 5]), duty=seq([8]), params=SQ, octave=1)
P("Hop", G, "pulse", "Jump: a quick upward sweep",
  ["game", "pulse", "jump"], "game", vol=decay(13, 14, 1.2), pitch=seq([20, 18, 16, 14, 12, 10, 8, 6, 4]), duty=seq([4]), params=SQ)
P("Impact Crunch", G, "noise", "Hit: a crunchy short-mode impact",
  ["game", "noise", "hit", "impact"], "game", vol=decay(15, 9, 1.5), arpeggio=seq([3, -2, -6]), duty=seq([1, 1, 0]), params=NZ)
P("Cursor Blip", G, "pulse", "Menu blip: one tiny 12.5% tick",
  ["game", "pulse", "menu", "blip"], "game", vol=seq([12, 9, 4, 0]), duty=seq([2]), params=SQ, octave=1)
P("Rising Power", G, "pulse", "Power-up: a fast arpeggio ladder climbing two octaves",
  ["game", "pulse", "power-up", "rise"], "game", vol=seq([13] * 20 + ramp(12, 0, 8)),
  arpeggio=seq([0, 4, 7, 3, 7, 10, 5, 9, 12, 7, 12, 16, 10, 14, 17, 12, 16, 19, 24, 24]), duty=seq([4]), params=SQ)
P("Extra Life", G, "pulse", "1-up: a bright rising fanfare of six notes",
  ["game", "pulse", "1-up", "fanfare"], "game", vol=seq([13] * 24 + ramp(12, 0, 6)),
  arpeggio=seq([0] * 4 + [4] * 4 + [7] * 4 + [12] * 4 + [16] * 4 + [19] * 4), duty=seq([8]), params=SQ, octave=1)
P("Boom", G, "noise", "Explosion: a big long-mode blast that sinks and fades",
  ["game", "noise", "explosion", "big"], "game", vol=seq([15] * 3 + ramp(14, 0, 40)),
  arpeggio=seq([3, 2, 1] + ramp(0, -11, 30)), duty=seq([0]), params=NZ)
P("Zap Laser", G, "pulse", "Laser: a 12.5% shot diving fast",
  ["game", "pulse", "laser", "shot"], "game", vol=seq(ramp(15, 0, 14)), pitch=seq([-30], 0), duty=seq([2]), params=SQ, octave=1)
P("Confirm Select", G, "pulse", "Select: two quick rising notes, a fifth apart",
  ["game", "pulse", "select", "confirm"], "game", vol=seq([13, 12, 11, 0, 13, 12, 11, 9, 6, 3, 0]), arpeggio=seq([0, 0, 0, 0, 7]), duty=seq([4]), params=SQ, octave=1)
P("Error Buzz", G, "pulse", "Error: a low, harsh double buzz",
  ["game", "pulse", "error", "buzz"], "game", vol=seq([13] * 6 + [0, 0, 0] + [13] * 8 + [0]), arpeggio=seq([0, 1], 0), duty=seq([2]), params=SQ, octave=-2)
P("Door Slide", G, "noise", "Door: a grinding slide that ends with a thud",
  ["game", "noise", "door", "slide"], "game", vol=seq(ramp(8, 11, 20) + [15, 12, 8, 4, 0]),
  arpeggio=seq(ramp(-2, 2, 20) + [-8, -9, -10, -10, -10]), duty=seq([0]), params=NZ)
P("Item Pickup", G, "vrc6_pulse", "Pickup: a sparkling up-arpeggio on a thin VRC6 pulse",
  ["game", "vrc6", "pickup", "sparkle"], "game", vol=seq([13] * 9 + ramp(12, 0, 10)), arpeggio=seq([0, 0, 0, 7, 7, 7, 12, 12, 12, 19]), duty=seq([2]), params=VP, octave=1)

# ── Drums: 4 kits (noise) + single hits
KICK = dict(vol=decay(15, 8, 1.4), arpeggio=seq([0, -3, -5, -7, -8, -8]), duty=seq([0]))
SNARE = dict(vol=decay(15, 18, 1.6), arpeggio=seq([4, 0]), duty=seq([0]))
HAT = dict(vol=seq([11, 7, 4, 1, 0]), duty=seq([0]))
OHAT = dict(vol=decay(11, 22, 1.3), duty=seq([0]))
CRASH = dict(vol=decay(14, 50, 1.5), duty=seq([0]))
P("Punchy Noise Kit", DK, "noise", "Kick, snare, closed hat, open hat and crash on one noise channel (low rows: kick)",
  ["drums", "noise", "kit", "punchy"], "kit", params=NZ, kit=[
      piece("Punchy Kick", 47, None, **KICK), piece("Punchy Snare", 64, None, **SNARE), piece("Punchy Hat", 79, None, **HAT),
      piece("Punchy Open Hat", 87, None, **OHAT), piece("Punchy Crash", 127, None, **CRASH)])
P("Tight Noise Kit", DK, "noise", "Short and dry: a clipped kick, a crack snare, a tick hat",
  ["drums", "noise", "kit", "tight", "dry"], "kit", params=NZ, kit=[
      piece("Tight Kick", 47, None, vol=decay(15, 5, 1.6), arpeggio=seq([0, -4, -7]), duty=seq([0])),
      piece("Tight Snare", 64, None, vol=decay(15, 9, 1.8), arpeggio=seq([5, 2]), duty=seq([0])),
      piece("Tight Hat", 83, None, vol=seq([9, 4, 0]), duty=seq([0])),
      piece("Tight Rim", 127, None, vol=seq([13, 6, 2, 0]), duty=seq([1]))])
P("Big Room Noise Kit", DK, "noise", "Long tails: a booming kick, a wide snare and a washy crash",
  ["drums", "noise", "kit", "big", "long"], "kit", params=NZ, kit=[
      piece("Boom Kick", 47, None, vol=decay(15, 14, 1.3), arpeggio=seq([1, -2, -4, -6, -8, -9, -10]), duty=seq([0])),
      piece("Wide Snare", 64, None, vol=decay(15, 30, 1.4), arpeggio=seq([5, 1]), duty=seq([0])),
      piece("Soft Hat", 79, None, vol=seq([9, 7, 5, 3, 1, 0]), duty=seq([0])),
      piece("Long Open Hat", 87, None, vol=decay(10, 34, 1.2), duty=seq([0])),
      piece("Wash Crash", 127, None, vol=decay(14, 63, 1.3), duty=seq([0]))])
P("Metal Noise Kit", DK, "noise", "Short-mode noise for tuned, metallic hits: clanks and pings",
  ["drums", "noise", "kit", "metal", "industrial"], "kit", params=NZ, kit=[
      piece("Metal Kick", 47, None, vol=decay(15, 7, 1.4), arpeggio=seq([0, -3, -6, -8]), duty=seq([0])),
      piece("Clank Snare", 64, None, vol=decay(15, 14, 1.6), arpeggio=seq([3, 1]), duty=seq([1, 1, 0])),
      piece("Ping Hat", 79, None, vol=seq([11, 6, 3, 1, 0]), duty=seq([1])),
      piece("Anvil", 127, None, vol=decay(14, 26, 1.5), duty=seq([1]))])
P("Thump Kick", DH + ["Noise"], "noise", "A deep noise thump with a falling body",
  ["drums", "noise", "kick"], "hit", vol=decay(15, 10, 1.3), arpeggio=seq([0, -2, -4, -6, -8, -9]), duty=seq([0]), params=NZ, octave=-2)
P("Crack Snare", DH + ["Noise"], "noise", "A sharp crack with a short noisy tail",
  ["drums", "noise", "snare", "crack"], "hit", vol=decay(15, 14, 1.8), arpeggio=seq([6, 2]), duty=seq([0]), params=NZ)
P("Fat Snare", DH + ["Noise"], "noise", "A thick snare with a long body",
  ["drums", "noise", "snare", "fat"], "hit", vol=decay(15, 26, 1.3), arpeggio=seq([3, 0, -1]), duty=seq([0]), params=NZ)
P("Tick Hat", DH + ["Noise"], "noise", "A tiny closed hat",
  ["drums", "noise", "hat", "closed"], "hit", vol=seq([10, 5, 2, 0]), duty=seq([0]), params=NZ, octave=2)
P("Sizzle Hat", DH + ["Noise"], "noise", "An open hat that sizzles out",
  ["drums", "noise", "hat", "open"], "hit", vol=decay(11, 28, 1.2), duty=seq([0]), params=NZ, octave=2)
P("Ride Ping", DH + ["Noise"], "noise", "Short-mode noise: a pinging ride",
  ["drums", "noise", "ride", "metallic"], "hit", vol=decay(11, 30, 1.6), duty=seq([1]), params=NZ, octave=2)
P("Splash Cymbal", DH + ["Noise"], "noise", "A quick splash",
  ["drums", "noise", "cymbal", "splash"], "hit", vol=decay(14, 36, 1.7), arpeggio=seq([2, 1, 0]), duty=seq([0]), params=NZ, octave=2)
P("Hand Clap", DH + ["Noise"], "noise", "Three fast bursts and a tail: a clap",
  ["drums", "noise", "clap"], "hit", vol=seq([14, 3, 13, 3, 14, 11, 9, 7, 5, 3, 2, 1, 0]), arpeggio=seq([4]), duty=seq([0]), params=NZ)
P("Shaker", DH + ["Noise"], "noise", "A soft shaker swish",
  ["drums", "noise", "shaker", "soft"], "hit", vol=seq([3, 6, 8, 6, 4, 2, 1, 0]), duty=seq([0]), params=NZ, octave=2)
P("Drop Kick", DH + ["Triangle"], "triangle", "The triangle kick-drop: a fast dive from high to low (play around C4)",
  ["drums", "triangle", "kick", "kick drop"], "hit", vol=gate(7), arpeggio=seq([0, -7, -13, -18, -22, -24, -24]))
P("Low Tom", DH + ["Triangle"], "triangle", "A triangle tom falling a fifth",
  ["drums", "triangle", "tom", "low"], "hit", vol=gate(10), pitch=seq([-10], 0), octave=-1)
P("High Tom", DH + ["Triangle"], "triangle", "A short, high triangle tom",
  ["drums", "triangle", "tom", "high"], "hit", vol=gate(8), pitch=seq([-12], 0), octave=1)
P("Sampled Kick", DH + ["Samples"], "dpcm", "The built-in DPCM kick sample at full rate",
  ["drums", "dpcm", "kick", "sample"], "hit", params={"sample": 0, "rate": 15, "loop": 0})
P("Sampled Snare", DH + ["Samples"], "dpcm", "The built-in DPCM snare sample",
  ["drums", "dpcm", "snare", "sample"], "hit", params={"sample": 1, "rate": 15, "loop": 0})
P("Sampled Tom", DH + ["Samples"], "dpcm", "The built-in DPCM tom sample, a little slower",
  ["drums", "dpcm", "tom", "sample"], "hit", params={"sample": 2, "rate": 13, "loop": 0})
P("Sampled Crash", DH + ["Samples"], "dpcm", "The built-in DPCM crash sample",
  ["drums", "dpcm", "crash", "sample"], "hit", params={"sample": 3, "rate": 15, "loop": 0})

# ── Hardware: authentic presets (no unison, detune or drift; within each channel's real ranges)
N = ["Hardware", "NES (2A03)"]
V = ["Hardware", "Famicom + VRC6"]
HW = "2A03"
VR = "VRC6"
# 2A03 pulse (12)
P("Duty Lead", N + ["Pulse"], "pulse", "25% lead with a quick decay to sustain and delayed vibrato: the stock pulse lead",
  ["lead", "pulse", "authentic"], "lead", vol=env(0, 15, 6, 10, 8), duty=seq([4]), vib=(16, 6, 20), params=SQ, profile=HW)
P("Square Melody", N + ["Pulse"], "pulse", "50% square, a soft attack and sustain: a singing melody voice",
  ["lead", "pulse", "authentic", "soft"], "lead", vol=env(2, 13, 4, 11, 8, start=7), duty=seq([8]), vib=(18, 7, 24), params=SQ, profile=HW)
P("Thin Pulse Lead", N + ["Pulse"], "pulse", "12.5% lead: thin, nasal and cutting",
  ["lead", "pulse", "authentic", "thin"], "lead", vol=env(0, 14, 5, 10, 8), duty=seq([2]), vib=(14, 6, 16), params=SQ, profile=HW)
P("Duty Swell Lead", N + ["Pulse"], "pulse", "The duty steps 12.5% → 25% → 50% through each note",
  ["lead", "pulse", "authentic", "duty sweep"], "lead", vol=env(0, 15, 6, 11, 8), duty=seq([2, 2, 2, 4, 4, 4, 8]), vib=(12, 6, 24), params=SQ, profile=HW)
P("Echo Pulse Lead", N + ["Pulse"], "pulse", "The classic echo on one pulse: each note repeats quieter a few frames later",
  ["lead", "pulse", "authentic", "echo"], "lead", vol=echo(15, 10, 7, 0.45), duty=seq([4]), vib=(12, 6, 26), params=SQ, profile=HW)
P("Pulse Pluck", N + ["Pulse"], "pulse", "A short 25% pluck for fast runs",
  ["keys", "pulse", "authentic", "pluck"], "keys", vol=decay(15, 16, 1.6), duty=seq([4]), params=SQ, profile=HW)
P("Firm Pulse Bass", N + ["Pulse"], "pulse", "A firm 50% bass",
  ["bass", "pulse", "authentic"], "bass", vol=env(0, 15, 6, 12, 6), duty=seq([8]), params=SQ, profile=HW)
P("Major Chord Pulse", N + ["Pulse"], "pulse", "A fast major arpeggio: chords on a single pulse",
  ["arp", "pulse", "authentic", "major"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 4, 7]), duty=seq([4]), params=SQ, profile=HW)
P("Minor Chord Pulse", N + ["Pulse"], "pulse", "A fast minor arpeggio",
  ["arp", "pulse", "authentic", "minor"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 3, 7]), duty=seq([2]), params=SQ, profile=HW)
P("Hardware Sweep Zap", N + ["Pulse"], "pulse", "The pulse channel's own sweep unit bending the pitch down",
  ["fx", "pulse", "authentic", "sweep"], "fx", vol=decay(15, 28, 1.1), duty=seq([2]), params={"envelope": 0, "volume": 15, "sweep": 3, "sweep_rate": 1}, profile=HW, octave=1)
P("Decay Envelope Pluck", N + ["Pulse"], "pulse", "No macro: the chip's own hardware decay envelope",
  ["keys", "pulse", "authentic", "envelope"], "keys", duty=seq([4]), params={"envelope": 1, "volume": 4, "sweep": 0}, profile=HW)
P("Pulse Bell", N + ["Pulse"], "pulse", "An octave strike and a long ring",
  ["bell", "pulse", "authentic"], "keys", vol=decay(15, 44, 1.3), arpeggio=seq([12, 12, 0]), duty=seq([4]), params=SQ, profile=HW)
# 2A03 triangle (6)
P("Pure Triangle Bass", N + ["Triangle"], "triangle", "The plain triangle: pure, deep low end",
  ["bass", "triangle", "authentic"], "bass", profile=HW)
P("Triangle Kick Bass", N + ["Triangle"], "triangle", "A kick-drop into every bass note",
  ["bass", "triangle", "authentic", "kick drop"], "bass", arpeggio=seq([24, 12, 5, 0]), profile=HW)
P("Triangle Staccato", N + ["Triangle"], "triangle", "Short gated triangle notes",
  ["bass", "triangle", "authentic", "short"], "bass", vol=gate(8), profile=HW)
P("Triangle Flute", N + ["Triangle"], "triangle", "The triangle as a soft flute, with vibrato",
  ["lead", "triangle", "authentic", "flute"], "lead", vib=(20, 7, 16), profile=HW)
P("Triangle Kick", N + ["Triangle"], "triangle", "A triangle kick-drum: a fast dive (play around C4)",
  ["drums", "triangle", "authentic", "kick"], "hit", vol=gate(7), arpeggio=seq([0, -7, -13, -18, -22, -24, -24]), profile=HW)
P("Triangle Tom", N + ["Triangle"], "triangle", "A triangle tom falling in pitch",
  ["drums", "triangle", "authentic", "tom"], "hit", vol=gate(10), pitch=seq([-10], 0), profile=HW)
# 2A03 noise (8)
P("Noise Kit", N + ["Noise"], "noise", "Kick, snare, hats and crash on the noise channel",
  ["drums", "noise", "authentic", "kit"], "kit", params=NZ, profile=HW, kit=[
      piece("Noise Kit Kick", 47, None, **KICK), piece("Noise Kit Snare", 64, None, **SNARE), piece("Noise Kit Hat", 79, None, **HAT),
      piece("Noise Kit Open Hat", 87, None, **OHAT), piece("Noise Kit Crash", 127, None, **CRASH)])
P("Short-Mode Kit", N + ["Noise"], "noise", "A metallic kit in the noise channel's short mode",
  ["drums", "noise", "authentic", "kit", "metal"], "kit", params=NZ, profile=HW, kit=[
      piece("Short Kick", 47, None, vol=decay(15, 7, 1.4), arpeggio=seq([0, -3, -6, -8]), duty=seq([0])),
      piece("Short Snare", 64, None, vol=decay(15, 12, 1.6), arpeggio=seq([3, 1]), duty=seq([1, 0])),
      piece("Short Hat", 127, None, vol=seq([10, 5, 2, 0]), duty=seq([1]))])
P("Noise Snare", N + ["Noise"], "noise", "A snare: a crack and a noisy tail",
  ["drums", "noise", "authentic", "snare"], "hit", vol=decay(15, 16, 1.7), arpeggio=seq([5, 1]), duty=seq([0]), params=NZ, profile=HW)
P("Noise Hat", N + ["Noise"], "noise", "A closed hat",
  ["drums", "noise", "authentic", "hat"], "hit", vol=seq([11, 6, 3, 1, 0]), duty=seq([0]), params=NZ, profile=HW, octave=2)
P("Noise Crash", N + ["Noise"], "noise", "A long cymbal wash",
  ["drums", "noise", "authentic", "cymbal"], "hit", vol=decay(14, 54, 1.4), duty=seq([0]), params=NZ, profile=HW, octave=2)
P("Noise Explosion", N + ["Noise"], "noise", "A long-mode blast sinking away",
  ["fx", "noise", "authentic", "explosion"], "game", vol=seq([15] * 3 + ramp(14, 0, 38)), arpeggio=seq([2, 1, 0] + ramp(0, -10, 28)), duty=seq([0]), params=NZ, profile=HW)
P("Noise Wind", N + ["Noise"], "noise", "Gusting wind (hold the note)",
  ["fx", "noise", "authentic", "wind"], "fx", vol=seq(ramp(3, 8, 10) + ramp(8, 3, 10), 0, None), arpeggio=seq([3, 3, 4, 4, 3, 2, 2, 3], 0), duty=seq([0]), params=NZ, profile=HW)
P("Hardware Decay Hat", N + ["Noise"], "noise", "No macro: the chip's own decay envelope on the noise",
  ["drums", "noise", "authentic", "envelope"], "hit", params={"envelope": 1, "volume": 1, "noise_period": -1, "noise_mode": 0}, profile=HW, octave=2)
# 2A03 DPCM (4)
P("DPCM Kick", N + ["DPCM"], "dpcm", "The built-in kick sample",
  ["drums", "dpcm", "authentic", "kick"], "hit", params={"sample": 0, "rate": 15, "loop": 0}, profile=HW)
P("DPCM Snare", N + ["DPCM"], "dpcm", "The built-in snare sample",
  ["drums", "dpcm", "authentic", "snare"], "hit", params={"sample": 1, "rate": 15, "loop": 0}, profile=HW)
P("DPCM Low Tom", N + ["DPCM"], "dpcm", "The tom sample played at a lower rate",
  ["drums", "dpcm", "authentic", "tom"], "hit", params={"sample": 2, "rate": 12, "loop": 0}, profile=HW)
P("DPCM Drum Map", N + ["DPCM"], "dpcm", "The note picks the sample: C kick, C# snare, D tom, D# crash",
  ["drums", "dpcm", "authentic", "kit"], "dpcm_map", params={"sample": -1, "rate": 15, "loop": 0}, profile=HW)
# VRC6 pulse (7)
P("VRC6 Reed Lead", V + ["VRC6 Pulse"], "vrc6_pulse", "A 3/16 reed lead with vibrato",
  ["lead", "vrc6", "authentic", "reed"], "lead", vol=env(1, 14, 5, 11, 8, start=8), duty=seq([3]), vib=(16, 6, 16), params=VP, profile=VR)
P("VRC6 Hollow Lead", V + ["VRC6 Pulse"], "vrc6_pulse", "A hollow 8/16 square lead",
  ["lead", "vrc6", "authentic", "hollow"], "lead", vol=env(0, 15, 6, 11, 8), duty=seq([8]), vib=(14, 6, 22), params=VP, profile=VR)
P("VRC6 Glass Lead", V + ["VRC6 Pulse"], "vrc6_pulse", "The thinnest 1/16 pulse: piercing glass",
  ["lead", "vrc6", "authentic", "glassy"], "lead", vol=env(0, 15, 8, 9, 8), duty=seq([1]), vib=(10, 6, 16), params=VP, profile=VR)
P("VRC6 Width Sweep", V + ["VRC6 Pulse"], "vrc6_pulse", "The width sweeps through all eight VRC6 duties",
  ["lead", "vrc6", "authentic", "duty sweep"], "lead", vol=env(0, 15, 6, 11, 8), duty=seq([1, 2, 3, 4, 5, 6, 7, 8, 7, 6, 5, 4, 3, 2], 0), params=VP, profile=VR)
P("VRC6 Chord", V + ["VRC6 Pulse"], "vrc6_pulse", "A fast major-seventh arpeggio",
  ["arp", "vrc6", "authentic", "major seventh"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 4, 7, 11]), duty=seq([5]), params=VP, profile=VR)
P("VRC6 Bell", V + ["VRC6 Pulse"], "vrc6_pulse", "A two-octave strike and a ringing decay",
  ["bell", "vrc6", "authentic"], "keys", vol=decay(15, 44, 1.2), arpeggio=seq([24, 12, 0]), duty=seq([2]), params=VP, profile=VR)
P("VRC6 Pad", V + ["VRC6 Pulse"], "vrc6_pulse", "A slow swell at 7/16 with vibrato",
  ["pad", "vrc6", "authentic", "soft"], "pad", vol=env(16, 12, 2, 11, 14, start=1), duty=seq([7]), vib=(12, 8, 24), params=VP, profile=VR)
# VRC6 saw (5)
P("Saw Brass", V + ["VRC6 Saw"], "saw", "A swelling saw with a scoop: brassy",
  ["lead", "saw", "authentic", "brass"], "lead", vol=env(3, 15, 4, 12, 8, start=8), pitch=seq([-24, 6, 6, 6, 6]), vib=(14, 6, 24), params=SAW, profile=VR)
P("Fat Saw Bass", V + ["VRC6 Saw"], "saw", "The VRC6 saw as a fat bass",
  ["bass", "saw", "authentic", "fat"], "bass", vol=env(0, 15, 6, 12, 6), params=SAW, profile=VR)
P("Saw Pluck", V + ["VRC6 Saw"], "saw", "A bright saw pluck",
  ["keys", "saw", "authentic", "pluck"], "keys", vol=decay(15, 22, 1.4, 3), params=SAW, profile=VR)
P("Saw Power Arp", V + ["VRC6 Saw"], "saw", "Root, fifth, octave arpeggio on the saw",
  ["arp", "saw", "authentic", "power"], "arp", vol=env(0, 14, 4, 11, 6), arpeggio=arp([0, 7, 12]), params=SAW, profile=VR)
P("Saw Strings", V + ["VRC6 Saw"], "saw", "A slow bowed swell with vibrato",
  ["pad", "saw", "authentic", "strings"], "pad", vol=env(14, 13, 2, 12, 14, start=2), vib=(14, 7, 18), params=SAW, profile=VR)


# ───────────────────────────── level matching ─────────────────────────────

def scale_seq_text(text, g):
    """Scales a volume sequence's values by `g` (rounded, kept in 0..15, a
    non-zero step stays at least 1)."""
    if not text:
        return text
    out = []
    for tok in text.split():
        if tok in ("|", "/"):
            out.append(tok)
        else:
            v = int(tok)
            nv = 0 if v == 0 else max(1, min(15, round(v * g)))
            out.append(str(nv))
    return " ".join(out)


def apply_level(p, g):
    """Folds a gain into the preset: the channel's level control where it has
    one and has room (Volume 0–15 / the saw's Rate 0–42), else the volume macro."""
    if abs(20 * math.log10(g)) < 0.25:
        return
    params = p["params"]
    voice = p["voice"]
    if voice == "saw" and "rate" in params:
        params["rate"] = max(1, min(42, round(params["rate"] * g)))
        return
    if voice in ("pulse", "vrc6_pulse", "noise") and "volume" in params and params.get("envelope", 0) == 0:
        want = params["volume"] * g
        if want <= 15:
            params["volume"] = max(1, round(want))
            return
    inst = p["instrument"]
    seqs = [inst] + [k["inst"] for k in inst.get("kit", [])]
    if g > 1:
        # Louder: only as far as the loudest step allows, so the envelope
        # keeps its shape (never flattened against 15).
        peak = max([int(t) for q in seqs for t in q["volume"].split() if t not in ("|", "/")] or [15])
        g = min(g, 15 / max(1, peak))
    for q in seqs:
        if q["volume"]:
            q["volume"] = scale_seq_text(q["volume"], g)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--levels", help="presets/levels.json from bank_pack (its \"gains\": name -> linear gain)")
    ap.add_argument("--out", default=os.path.join(os.path.dirname(__file__), "..", "bank.json"))
    a = ap.parse_args()
    names = [p["name"] for p in PRESETS]
    assert len(names) == len(set(names)), "duplicate names"
    if a.levels:
        with open(a.levels, encoding="utf-8") as f:
            gains = json.load(f).get("gains", {})
        for p in PRESETS:
            if p["name"] in gains:
                apply_level(p, gains[p["name"]])
    for p in PRESETS:
        if p["profile"]:
            assert p["unison"] == [1, 0, 0], p["name"] + ": an authentic preset has no unison"
    bank = {
        "about": "Cartwave's first preset bank. Every preset designed from scratch by Jeffrey Scott Frederick with Cartwave's macros "
                 "(no game, ROM, ripped patch or third-party library). Generated by presets/tools/design_bank.py; do not edit by hand.",
        "presets": PRESETS,
    }
    with open(a.out, "w", encoding="utf-8", newline="\n") as f:
        json.dump(bank, f, indent=1, ensure_ascii=False)
        f.write("\n")
    by = {}
    for p in PRESETS:
        by[" / ".join(p["path"][:2])] = by.get(" / ".join(p["path"][:2]), 0) + 1
    print(f"{len(PRESETS)} presets -> {os.path.normpath(a.out)}")
    for k, v in by.items():
        print(f"  {k}: {v}")


if __name__ == "__main__":
    main()
