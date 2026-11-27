# AB.md: the authenticity A/B pass (Batch 4c §4)

**What this is.** Our 2A03 + VRC6 emulation against two respected reference players, fed **the
same register writes**, to find where we differ before Mika's 100-preset bank. Measure and
report only: **no engine code changed** (every fix below is a proposal; each would change the
golden hash and how existing songs sound). Decisions: [RUNLOG](../RUNLOG.md) B4C-D8…D11.
Listening pack: [`docs/audio/ab/`](../audio/ab/README.md). Every number: [`results.tsv`](results.tsv)
(1,000+ rows: file, segment, metric, NSFPlay, gme).

## 1. Summary

**We match NSFPlay closely almost everywhere**: pitch 0.00 cents on all 57 notes measured
(2A03 pulses, triangle, VRC6 pulses and saw), duty and volume curves within 0.1–0.3 dB, the
2A03 pulse mixer's non-linearity exactly, DPCM direct-load steps within 0.1 dB, the noise LFSR's
loops (93 / 32767 steps) identical, envelopes and length counters within ±0.12 frame, the VRC6's
level and polarity. Where we differ from NSFPlay it is by ≈ 1 dB in the triangle/noise group's
balance against the pulses, the high-frequency response of the two players' output stages, and
the sweep unit's edges (where the two references also disagree with each other). gme
(Game_Music_Emu) differs from both of us in more places; most are its own EQ and older choices.

**Top 5 differences, ranked by how audible they are likely to be:**

| # | What | How big | Channel | Likely cause | Proposed fix |
|---|---|---|---|---|---|
| 1 | The triangle/noise/DMC group sits **≈ 1 dB lower against the pulses** in NSFPlay than in ours (triangle −0.4 dB, triangle + noise −1.0…−1.2 dB at every DMC level); gme lower still (−4.3 dB for the triangle, partly its bass EQ) | ≈ 1 dB balance shift between bass/drums and leads | tnd vs pulses | The tnd formula is the wiki's **approximation** (APU Mixer: "the approximate audio output level"; its table "within 4%") | **None yet**: a hardware recording of a pulse and the triangle at known volumes decides it; then a calibration constant in `mixer.rs` (§3.1) |
| 2 | **Sweep edges.** NSFPlay applies pulse 1's first sweep step later in one case (+384 cents for ~8 frames) and keeps a sweep sounding **1–6 frames longer** before the overflow mute; gme agrees with us (within 1 frame) | short tails and the first step of sweep effects (risers, drops, lasers) | pulse 1, 2 | Disagreement between the references; ours follows the wiki ("muting happens regardless of the state of the sweep unit"; reload flag rules, APU Pulse) | **None** (the wiki and gme agree with us); keep the pattern as a regression test (§3.2) |
| 3 | **High frequencies**: both references roll off above 8 kHz with their filters off (NSFPlay −1.1 dB at 8 kHz, −3.9 dB at 16 kHz; gme −0.4 / −3.1 dB); ours is flat to ≈ 19 kHz | audible only with our filter **Off** (NES mode's 14 kHz low-pass is below it) | all | The players' output stage (resampling), not the chip: the same curve on every channel and pattern | **None** (§3.3) |
| 4 | **Polarity**: gme inverts the **noise** and the **VRC6** relative to the 2A03 pulses; NSFPlay and ours don't differ | inaudible alone; changes how unisons of VRC6 and 2A03 pulses sum (p1 + VRC6 pulse at the same pitch: +0.9 dB in NSFPlay, −1.7 dB in gme, against ours) | noise, VRC6 | gme's older choice; ours follows the wiki (VRC6 audio: "roughly equivalent … (except inverted)") | **None** (§3.4) |
| 5 | **gme's non-linear levels**: quiet pulse volumes up to 1.3 dB quieter, two pulses together 1.2 dB louder, DPCM steps ±1.4 dB, the DMC's pull on triangle + noise a different curve; NSFPlay agrees with ours on all of these | ≈ 1 dB on quiet passages and dense chords, in gme only | pulse, DMC | gme's mixer approximation differs from the wiki's formulas, which ours and NSFPlay implement | **None** (§3.5) |

## 2. Method

**Material (all ours).** Ten targeted test patterns written as register writes per frame tick
(`ab patterns`): pulse 1 (4 duties, 9 pitches A2…A6, 16 volumes), pulse 2 (duties), envelopes
(4 decay rates, a looped decay, 4 length-counter values), sweeps (up/down on both pulses), the
triangle (11 pitches, the linear counter), noise long and short mode (all 16 periods each, a
decay), DPCM ($4011 direct-load steps up and down, the 4 built-in samples at 3 rates), the 2A03
mixer (pulse pairs at 5 volume pairs, pulse + triangle, triangle + noise under 3 DMC levels) and
the VRC6 (both pulses' 8 duties and digitized mode, 16 volumes, the saw at 7 rates and 5 pitches,
the 2A03 pulse against a VRC6 pulse and the saw). Plus **the demo and the showcase**, logged
from our driver (`ab log`): 3,198 and 1,769 frame ticks, 10,831 and 5,996 writes. Each song was
also split **channel by channel** (`ab solo`: one channel's writes plus $4015/$4017).
**The logs are complete:** replaying each song's log into a fresh emulator gives the app's own
render **bit for bit** (`ab check`: 2,553,600 and 1,412,570 samples, all identical).

**The container (ours).** `ab nsf` packs a log into an **NSF we author**: a small 6502 player (`build_nsf` in `examples/ab.rs`)
at $8000 replays (register, value) pairs once per PLAY call (NTSC rate 16,639 µs), reading the
stream through a 4 KiB window at $9000 switched with $5FF9; our 16 KiB DPCM ROM sits at
$C000–$FFFF where the DMC reads it; header bit 0 of $7B turns on the VRC6. No game ROM, rip or
copyrighted music anywhere. The CEO's private reference MIDI was not used.

**Renders** (`docs/ab/tools/render_ab.ps1`), all mono, 48 kHz:
- **ours**: `ab replay` with the filter **off** (only our 5 Hz DC blocker) and the writes an
  NSF player makes before INIT ($4000–$4013 = 0, $4015 = $0F, $4017 = $40, the NSF page);
- **NSFPlay 2.6**: its documented command line, filters off (`LPF=0`, `HPF=256`), no loop
  detection, no stop on silence, no fade, every other setting at its default;
- **gme**: Game_Music_Emu as linked in ffmpeg 8.1. Its EQ and its stop after silence can't be set
  through ffmpeg: the EQ is measured (the bands below), and its comparisons cover what it rendered
  (`coverage_%`: the patterns 88–100 %; three demo channels only their first 5 s, the rest
  "couldn't measure").

**Lining up** (`ab_compare`): coarse lag from the envelopes of the first difference (DC-free),
then the sample lag within ±2 ms by cross-correlation over both polarities. NSFPlay's lag is a
steady 116–129 ms (its INIT and first PLAY); gme's varies because it skips a track's leading
silence. Levels are matched on AC RMS; per-segment levels are reported **relative to an anchor
segment in the same file**, so no file-wide gain enters them; levels, timing and noise use the
same 20 Hz high-pass on all three signals (NSFPlay with its high-pass off keeps the 2A03's DC);
bands, DC and DMC steps use the raw signals.

## 3. The differences, one by one

### 3.1 The triangle/noise/DMC group against the pulses (rank 1)
- `mixer`: `mix tri alone` −0.4 dB (NSFPlay), −4.3 (gme); `mix p1 15 + tri` −0.3 / −1.7;
  `mix tri + noise under dmc 0 / 64 / 127` −1.2 / −1.0 / −1.2 (NSFPlay), −4.4 / −0.4 / +2.6 (gme).
  NSFPlay's offset is constant across the DMC levels, so its tnd **curve** is ours, shifted.
- **Cause.** Ours implements the wiki's APU Mixer formula (`tnd_out = 159.79 / (1 / (t/8227 +
  n/12241 + d/22638) + 100)`), which the page calls an approximation. NSFPlay's group level is
  ≈ 1 dB lower; gme's differs more and in shape. Neither reference is hardware, so neither
  settles it (the third-source rule): **a hardware recording** (a pulse at volume 15, then the
  triangle, then both, on a front-loader and a Famicom) does. ENGINE.md §6.5 gap 3 already names it.
- **If adopted:** a calibration factor on `tnd_out` in `crates/chip/src/mixer.rs`. It would change
  the golden render hash, every migration hash v2…v8, the stereo golden hash and every export of a
  song with triangle, noise or DPCM (both songs, every preset demo).

### 3.2 The sweep unit's edges (rank 2)
- `sweep`: pitch tracks agree everywhere except `p1 sweep up shift 2 period 4`, where NSFPlay is
  +384 cents (one sweep step, ×1.25) for the first windows; gme 0. Sounding frames before the
  overflow mute, ours / NSFPlay / gme: 21/22/22, 29/35/29, 22/26/22, 24/25/24, 29/32/30, 22/26/–.
- **Cause.** The references disagree. The wiki (APU Pulse, edited 1 Nov 2026): the target period
  is recalculated whenever the period changes; "if the target period is larger than $7FF, it
  silences the channel … regardless of the state of the sweep unit"; on a half-frame clock the
  period updates when the divider is 0, the sweep enabled, the shift non-zero and not muting; the
  divider reloads when it is 0 or the reload flag (set by a $4001 write) is set. Ours does this
  (ENGINE.md §2); gme agrees with us within a frame; NSFPlay sounds longer. The +384-cent case
  follows a segment that left the sweep disabled with its divider mid-count: a divider-state
  question the wiki answers the way we do.
- **Proposal:** no change. Keep `sweep` as a regression pattern; ask Mika to listen to
  `sweep_A/B/C` (the tails).

### 3.3 High-frequency response (rank 3)
- Bands, raw, every file: NSFPlay −1.0…−1.3 dB at 8 kHz and −3.2…−4.3 dB at 16 kHz; gme −0.1…−0.4
  and −3.0…−5.7. Up to 4 kHz both are within ±0.6 dB of ours (gme: its EQ, below).
- **Cause:** the players' output stages (the same curve on every channel), not the chip. Ours is
  band-limited synthesis flat to ≈ 19 kHz (ENGINE.md §3.1). The console's own 14 kHz low-pass
  (APU Mixer) is our NES filter mode, far below either.
- **Proposal:** none.

### 3.4 Polarity (rank 4)
- Global polarity against ours: NSFPlay **same** in every file; gme **inverted** in `noise_long`,
  `noise_short`, `mixer` and `vrc6`, same in the pulse files.
- **Cause:** ours follows the wiki: VRC6 audio (edited 2018): the VRC6 pulses are "roughly
  equivalent to the pulse channels of the 2A03 (except inverted)" (`mixer.rs`, `VRC6_SIGN = −1`).
  NSFPlay agrees. gme doesn't invert the VRC6 and inverts the noise.
- **Proposal:** none. A VRC6 hardware recording (ENGINE.md §6.5 gap 1) would confirm level and
  polarity together.

### 3.5 gme's mixer (rank 5)
- `pulse1` volume steps: NSFPlay +0.0…+0.1 dB at every volume; gme 0.0 at 15 falling to −1.3 dB
  at volume 1. `mixer` pulse pairs: NSFPlay 0.0 everywhere; gme +1.2 dB at 15 + 15, −0.7…−1.3 dB
  for quiet pulses alone. `dpcm` steps: NSFPlay ±0.1 dB, gme −1.4…+0.6 dB.
- **Cause:** ours and NSFPlay implement the wiki's non-linear pulse formula
  (`95.88 / (8128 / (p1 + p2) + 100)`); gme's levels follow a different approximation.
- **Proposal:** none.

## 4. What we match well

| Measurement (§4.3) | Result against NSFPlay / gme |
|---|---|
| **Pitch per note** | **0.00 cents** on all 57 notes (2A03 pulses A2…A6, triangle E1…A5, VRC6 pulses, saw A1…A5) / −0.38 cents everywhere (gme's clock) |
| **Envelope timing** | decay to −24 dB within **±0.12 frame** at all four rates / within ±0.2 frame except decay period 15 (+4.0 frames at −12 dB; −0.1 at −24 dB). The −6 dB point is unreliable for both (the start transient) |
| **Length counter, linear counter** | note ends within ±0.5 frame / ±0.7 frame |
| **Duty levels** | the four 2A03 duties and eight VRC6 duties within 0.1–0.2 dB / 0.0–0.1 dB |
| **Volume curves** | 2A03 pulse 15…1 within 0.1 dB / up to −1.3 dB (3.5); VRC6 15…1 within 0.3 dB / 0.1 dB |
| **Noise period and LFSR modes** | the loop correlates at **93 steps** (short) and **32767 steps** (long) in all three renders (0.88–1.00); the 31-step alternative stays low; spectral centroids 10–20 % lower in NSFPlay and 0–40 % in gme, from their high-frequency roll-off (3.3); levels across periods follow the same roll-off |
| **DPCM levels** | $4011 direct-load step heights within **0.1 dB** of ours in 9 steps / −1.4…+0.6 dB; the samples at 3 rates within 0.4 dB / −4.5…+4.4 dB (gme's EQ on low-rate samples) |
| **Mixer non-linearity** | pulse pairs at (15,15), (15,8), (8,8), (4,4), (15,2): **0.0 dB** / up to +1.2 dB; the DMC's pull on triangle + noise: the same curve as ours shifted −1.1 dB (3.1) / a different curve |
| **VRC6 vs 2A03** | equal level at volume 15 (pulse vs pulse, 0.0 dB) and the same polarity / the VRC6 1.4 dB quieter and inverted |
| **Saw** | rates 8…63 and 5 pitches within 0.3 dB / −0.3 dB (−2.1 dB at 55 Hz: its EQ) |
| **DC and filter response** | with filters off, ours keeps a 5 Hz DC blocker (DC ≈ 0); NSFPlay keeps the 2A03's DC (0.14–0.45 of full scale, unipolar DAC); gme removes it. Bands: NSFPlay ±0.6 dB from 31 Hz to 4 kHz; gme −5…−7 dB at 31 Hz, −1…−3 dB at 63 Hz, +0.6…+1.1 dB at 0.5–2 kHz (its built-in EQ) |
| **Spectral difference, the songs** | demo / showcase: NSFPlay within ±0.6 dB from 31 Hz to 4 kHz in both; gme as its EQ |
| **Waveform error** | median per segment after alignment: **−18…−27 dB** for the tonal patterns (pulse, triangle, VRC6, mixer) / −7…−16 dB. For noise, and for songs (whose drums are noise), ≈ 0 dB by construction: two LFSRs started in different states never correlate, and each note's phase follows the write's exact cycle, which an NSF player can't reproduce. For those the spectra and the measurements above are the comparison |

**Couldn't measure:** gme on the demo's DPCM and VRC6-pulse channels after 5 s (it stops on the
long rests; NSFPlay covers them); the VRC6 digitized mode's level (a constant output, i.e. DC,
which the comparison's high-pass removes: listen to `vrc6_*` instead).

## 5. The reference tools

| Tool | Version | Licence | Where from | Installed |
|---|---|---|---|---|
| **NSFPlay** (Brad Smith, after Mitsutaka Okazaki / Brezza) | 2.6 (2023-07-03) | No named licence file; its readme says the code "is distributed freely" and may be reused "without restriction" | https://github.com/bbbradsmith/nsfplay/releases/download/2.6/nsfplay26.zip (SHA-256 `19e27789…eb393`) | `C:\tools\reference\nsfplay26\` |
| **Game_Music_Emu** (libgme), via **ffmpeg** | as linked in ffmpeg 8.1 `full_build` (gyan.dev); ffmpeg was already installed (it is the screen grabber of the K1 tool) | libgme LGPL-2.1+; this ffmpeg build GPL-3.0 | Windows package manager (Gyan.FFmpeg) | `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Gyan.FFmpeg…` |

Both run only as separate programs. Nothing of theirs is linked, vendored or copied into the repo.

## 6. License hygiene

- **No reference tool's source code was read**, for implementing, for proposing fixes, or for
  setting them up. Everything about them came from their documentation and their output:
  NSFPlay's `nsfplay.txt` (command line, settings keys, changelog) and the GitHub release page;
  `ffmpeg -h demuxer=libgme`.
- Fixes are justified by the **NESdev wiki** (APU Pulse, APU Mixer, APU Noise, APU Triangle,
  VRC6 audio; read 2026-11-27, quoted briefly above) and by these measurements only.
- Also read in this batch, unrelated to the references: muda 0.22's dark menu-bar code (MIT /
  Apache-2.0, already a dependency) for K1.

## 7. Reproduce

```
cargo build --release -p ui --example ab --example ab_compare
target/release/examples/ab check demo ; ab check showcase          # the logs are complete
ab log demo W/demo.log ; ab log showcase W/showcase.log ; ab patterns W
ab solo W/demo.log W/demo-p1.log p1 …                               # per channel
ab nsf W/<name>.log W/<name>.nsf                                    # every log
pwsh docs/ab/tools/render_ab.ps1 -Work W                            # ours, NSFPlay, gme
pwsh docs/ab/tools/compare_ab.ps1 -Work W                           # results.tsv + the pack
```
