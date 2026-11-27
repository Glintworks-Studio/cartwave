# Batch 4b listening pack: gain staging (§5.2)

A 2-bar sustained pulse chord at 150 BPM, 48 kHz 16-bit mono (every track centred). Studio tracks use the scheme of RUNLOG B4B-D9: one voice at the console's level, unison voices trimmed by 1/√n, and the studio bus's safety limiter at −1 dBFS. Console files play the same notes on locked tracks (the 2A03 + VRC6 mixer): the console has five pulse-like channels, so 8 and 16 voices have no console version, and unison doesn't exist there.

Measured on the sustained part (`game_export::measure_ch`: EBU R128 integrated loudness, true peak, sample peak).

| File | What | Sample peak (dBFS) | True peak (dBTP) | Loudness (LUFS) |
|---|---|---|---|---|
| `studio_01_voices.wav` | 1 voice on a studio track | -17.7 | -17.2 | -27.4 |
| `studio_03_voices.wav` | 3 voices on a studio track | -8.1 | -7.7 | -22.4 |
| `studio_08_voices.wav` | 8 voices on a studio track | -1.0 | +0.1 | -17.7 |
| `studio_16_voices.wav` | 16 voices on a studio track | -1.0 | +0.1 | -14.4 |
| `studio_unison4_detune15.wav` | 1 note, unison 4, detune 15 cents (studio) | -11.7 | -11.2 | -27.1 |
| `studio_03_voices_unison4_detune15.wav` | 3 voices, unison 4, detune 15 (12 voices, studio) | -2.1 | -1.7 | -22.4 |
| `console_01_voices.wav` | 1 note on locked tracks (P1, P2, VRC6 P1, P2, Saw) | -17.7 | -17.3 | -27.6 |
| `console_03_voices.wav` | 3 notes on locked tracks (P1, P2, VRC6 P1, P2, Saw) | -12.8 | -12.3 | -23.3 |
| `console_05_voices.wav` | 5 notes on locked tracks (P1, P2, VRC6 P1, P2, Saw) | -8.7 | -8.6 | -18.9 |

Written by `cargo run --release -p ui --example listening_b4b -- docs/audio/batch4b`.
