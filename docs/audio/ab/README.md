# The A/B listening pack (Batch 4c §4, for Mika)

Each test pattern and song excerpt three times: **A = Cartwave** (ours, filter off), **B =
NSFPlay 2.6** (its filters off), **C = Game_Music_Emu** (via ffmpeg; its own EQ, which can't be
turned off). The same register writes, lined up to the sample and level-matched (one gain for
all three, peaks at −1 dBFS), mono, 48 kHz, 16-bit. Measurements and the ranked differences:
[docs/ab/AB.md](../../ab/AB.md). Made by `docs/ab/tools/compare_ab.ps1` (`--pack`); all
material is ours (ASSETS.md: generated test material, MIT OR Apache-2.0).

What to listen for first: **the triangle and noise against the pulses in `mixer`** (B ≈ 1 dB
lower: AB.md 3.1), **the tails of `sweep`** (B rings longer: 3.2), and the top end of everything
(B and C are duller above 8 kHz: 3.3).

| Files (`_A_cartwave`, `_B_nsfplay`, `_C_gme`) | What |
|---|---|
| `pulse1_*` | 2A03 pulse 1: the four duties at A4, nine pitches A2…A6, then volume 15 down to 0 |
| `pulse2_*` | pulse 2: the four duties at A4 |
| `envelope_*` | pulse decays at four rates, a looped decay, four length-counter note lengths |
| `sweep_*` | sweeps down and up on pulse 1, then pulse 2 (pulse 1 negates in ones' complement) |
| `triangle_*` | the triangle E1…A5, then a note cut by its linear counter |
| `noise_long_*` | noise, long mode, periods 0…15 (bright to dark), then a decay |
| `noise_short_*` | noise, short (metallic) mode, periods 0…15, then a decay |
| `dpcm_*` | DMC direct-load steps (clicks up then down), then the four built-in samples at three rates each |
| `mixer_*` | pulse pairs at five volume pairs (each alone, then both), pulse + triangle, triangle + noise under three DMC levels |
| `vrc6_*` | VRC6 pulse 1 and 2: eight duties and digitized mode; volume steps; the saw at seven rates and five pitches; a 2A03 pulse against a VRC6 pulse and the saw |
| `demo_excerpt_*` | the demo song, 12 s from 0:06 |
| `showcase_excerpt_*` | the showcase song (with `hey.wav` as a DPCM sample), 12 s from 0:06 |
