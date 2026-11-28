# PRESETS.md: Chip Synth, the browser, the in-instrument picker, the first preset bank

Branch **`presets/browser`** (from `main` 1b8a288, Batch 4c merged), pushed. Prompt: `PRESETS_PROMPT.md`.
Decisions: [`docs/RUNLOG.md`](docs/RUNLOG.md) **PB-D1…D__**. Spec: [`VISION.md`](VISION.md) ("Instruments",
"Presets and the browser"), [`UX.md`](UX.md) §4g. Baseline: `main` exported to `../before-presets`.
Machine: Windows 11 Pro 26200, Ryzen 7 9800X3D, RTX 4070 SUPER, 2560 × 1440.

## 1. Summary (one screen)

**Merge: __MERGE__**

| # | Gate | Result |
|---|---|---|
| 1 | Bit-identical audio | **PASS**: golden render, every migration hash v2…v8 → 9 (a new v8 corpus written by `main`, 9 files), the stereo golden (`cargo test`); **299** project files in the repository render and export identically in `main` and this branch (render hash, export hash, channels); **101** game-export files byte-identical over 5 inputs ([identity](docs/presets/identity/)). Presets are content: no engine path changed (Arp Speed defaults to every frame) |
| 2 | Visual regression 100 % / 150 % vs `main` | **PASS** at both scales ([100](docs/presets/regression/report-100.md), [150](docs/presets/regression/report-150.md)): every changed pixel is in the browser, the bottom pane, or a named consequence of the two (the arrangement moved 180 px right with the 420 px browser: the lanes' and mixer's scrollbar thumbs are shorter, the right edge shows 180 px less of the song; the taller pane moves the Main row and the lanes' scrollbar up 124 px; panels and menus over the moved app; the shortcut sheet's new row). With the browser closed the window is identical to `main` (0 px at 100 %; at 150 % only the taskbar band below the work area). The gate found a real bug on the way (PB-D18) |
| 3 | Mockup match (`workflow-chip-synth`) | **PASS: 47 of 47** board parts within 2 px in the studio and locked layouts (header, ◀ ▶, the preset field's ends, the sections' corners and titles, the displays' corners, every knob the board draws, the meter), measured by normalised cross-correlation of board patches ([report](docs/presets/mockup-match/report-100.md), [image](docs/screens/presets/device-mockup-match.png)); the display colour `#E8DCC0` exact (841 px in the studio displays) |
| 4 | Tests | **PASS**: every earlier suite (shell 49, Batch 3 79, design 23, polish 14, polish 2 92, Batch 4a 53, Batch 4b 43; harness changes where controls moved, PB-D16); **PB 13/13** scenarios (`docs/presets/tools/at_pb.ps1`) and the offline PB-B1…B5 (`tests/presets.rs`); `cargo test` **299 passed, 0 failed, 2 ignored** in release and debug; clippy clean; release build 0 warnings |
| 5 | Audio (10 min, ◀ ▶ through 30 presets) | __G5__ |
| 6 | Performance (the §2 rule) | __G6__ |
| 7 | Lime audit, contrast | **PASS**: lime 0 violations in 18 scenes of the new UI (the DPCM Sampler's LOOP key while on is the only new lime: a toggle on-state) ([report](docs/presets/lime/report.md)); the new text ≥ **4.61 : 1** everywhere, the locked device's labels 4.77–5.34 ([contrast](docs/presets/lime/contrast.md)) |

**What changed.**
- **The device panel** (§3): the bottom pane is Live's Detail View: the **Clip view** (the piano
  roll) or the **Device view** (the inspector's CLIP section in the board's first 200 px, then
  the selected track's instrument). **Shift+Tab** switches; a click on a track header shows its
  Device view. The inspector's instrument controls (macro editor, vibrato, duty, the VOICES row,
  the channel dials) moved into the device and are gone from the inspector. No device-chain slots.
- **Chip Synth** (§4): header (colour tab, name, STUDIO / 🔒 LOCKED, the centred preset field
  between ◀ ▶); OSCILLATOR (the Voice selector, the voice's real waveform in `#E8DCC0`, Duty /
  Mode / Rate, Unison, Detune; Drift and the channel's other hardware controls in a second row);
  ENVELOPES (VOLUME / ARPEGGIO / PITCH / DUTY over one editor; Vib Depth, Vib Rate, **Arp Speed**,
  then Vib Delay); OUTPUT (the chip's Volume, large; the hardware Envelope; a meter). Controls a
  voice doesn't have are hidden. Locked: the channel's controls only, "4-BIT · PER FRAME",
  displays and knobs at 40 %, labels ≥ 4.5 : 1. **DPCM Sampler**: SAMPLE, PLAYBACK, OUTPUT.
- **Arp Speed** is real (PB-D3): `Instrument::arp_speed`, frames per arpeggio step; **format 9**
  (v8 files load bit-identically; a v8 corpus written by `main` and its hash test).
- **The Voice selector**: a studio track switches voice (one undo step; the pool rebuilt on the
  UI thread). **A preset remembers its voice.**
- **The browser** (§5), replacing the flat list: COLLECTIONS / LIBRARY / PLACES, folders with
  chevrons, search across everything with tags, select to preview, double-click or drag to load,
  drag onto empty space for a new track; Hardware loads lock the track and set the profile
  (asking where it matters); the channel layouts are Hardware ▸ … ▸ Bands. 420 px wide (the
  board's).
- **The picker** (§6): the same folders (a locked track's authentic presets first), Show in
  Browser, Save Preset As…; ◀ ▶ load at once, even while playing, one undo step per run; a
  studio-only preset on a locked track asks Unlock track · Cancel.
- **The bank** (§7): **141 new presets** designed from scratch by `presets/tools/design_bank.py` (Leads 15, Basses 14, Arps 14, Pads 7, Bells & Keys 7, FX 10, Game Sounds 12, Drums 4 kits + 16 hits, **Hardware 42 authentic**: NES 30, Famicom + VRC6 12), the 61 classic presets folded into Classic folders; level-matched (112 of 141 within ±1.5 LU, the rest at a hardware limit), none clips; one provenance entry each ([`presets/PROVENANCE.md`](presets/PROVENANCE.md), linked from ASSETS.md); a listening pack ([`docs/audio/presets/index.html`](docs/audio/presets/index.html), 202 WAVs)
- **The perf rule** (§2) in ENGINE.md §7b and CLAUDE.md; **K1 closed** (not reproducible) in UX.md §8.

## 2. The bank by folder

| Folder | New | Classic | What |
|---|---|---|---|
| Sounds / Leads | 15 | 16 | Square, Thin & Nasal, Echo, Detuned (studio), Saw, Expressive (scoop, trill), Soft |
| Sounds / Basses | 14 | 5 | Triangle (incl. the kick-drop), Pulse, VRC6, Saw, Detuned (studio) |
| Sounds / Arps | 14 | 5 | Major & Minor, Sevenths, Octave & Power, Slow (Arp Speed 2–4), Detuned |
| Sounds / Pads | 7 | 2 | Pulse (duty breathing, tremolo organ), VRC6, Saw (unison washes) |
| Sounds / Bells & Keys | 7 | 6 | Bells (octave strikes), Keys (tines, harpsichord, vibraphone) |
| Sounds / FX | 10 | 8 | Sweeps, Noise (riser, ocean, static), Tones (alarm, teleport, ray gun, heartbeat) |
| Sounds / Game Sounds | 12 | 6 | coin, jump, hit, menu blip, power-up, 1-up, explosion, laser, select, error, door, pickup |
| Drums / Kits | 4 | 2 | Punchy, Tight, Big Room, Metal (noise kits: kick, snare, hats, crash) |
| Drums / Hits | 16 | 11 | Noise (kicks, snares, hats, ride, splash, clap, shaker), Triangle (kick-drop, toms), Samples (DPCM) |
| Hardware / NES (2A03) | 30 | – | Pulse 12, Triangle 6, Noise 8 (2 kits), DPCM 4; Bands (the 2A03 layout) |
| Hardware / Famicom + VRC6 | 12 | – | VRC6 Pulse 7, VRC6 Saw 5; Bands (the four VRC6 layouts) |
| **Total** | **141** | **61** | 202 presets; 10 studio-only (Unison / Detune / Drift); 42 authentic |

Instruments ▸ Chip Synth ▸ (each voice) and DPCM Sampler list the same presets by voice; Clips
lists the bank's ten phrases; Samples the built-in DPCM samples and the project's imports.

## 3. Decisions taken alone

All in RUNLOG **PB-D1…D19**; the ones that shape what the owner sees:

1. **The board wins where it and the prompt differ** (PB-D2): the Voice selector in the
   OSCILLATOR title (the board), not the header; no dashed MIDI-effects / device-chain boxes
   (nothing pretends); locked labels at ≥ 4.5 : 1 (the board's are darker).
2. **Arp Speed is built** (PB-D3), with format 9, rather than drawing a knob that does nothing.
3. **Every old instrument control kept one home** (PB-D9): the board's knob rows exactly as drawn;
   Drift, Sweep, Sweep rate, the noise Period, Vibrato Delay and the hardware Envelope in a second
   row of small knobs. The DPCM Sampler shows no envelopes or unison (the DMC ignores them).
4. **The Device view** keeps the inspector's CLIP section at the device's left (PB-D10); Shift+Tab
   is on the Help sheet, not in the View menu.
5. **The browser is the board's 420 px** (PB-D11); the earlier suites run with a dev hook at the
   old 240 px (PB-D12), and the regression diff accounts for the move.
6. **What asks** (PB-D13, D14): a studio-only preset on a locked track asks Unlock · Cancel; ◀ ▶
   on a locked track skips what it can't play. A Hardware preset from no profile sets the
   profile silently; another chip setup or no free channel asks; 4a's lock questions stay.
7. **The bank's form** (PB-D4, D5): data generated by a committed script; the 61 classic presets
   folded into Classic folders unchanged (old projects, layouts and tests name them); the counts
   met by the new presets alone.
8. **Level matching** (PB-D6): fixed category targets at the 35th percentile; the classic presets
   reported, not changed.
9. **Collections, Clips, Samples, Places** (PB-D15): seven fixed coloured collections; Clips are the
   bank's phrases; Samples the built-ins and the project's imports.

## 4. Known limitations, for the owner

1. **Authentic presets are not bit-identical on studio voices** (PB-D7): 0 of 42. Pulse,
   triangle and VRC6 match in loudness within 0.4 LU (phase differs: the console's sequencers run
   freely, a studio voice starts per note); **noise and DPCM are 2.3–3.6 LU louder on a studio
   voice** than through the console's mixer. An engine property from 4a/4b, not preset data; this
   batch changed no engine code. Proposal: settle it with 4c's tnd calibration (a hardware
   recording), then match the studio voices' noise / DPCM level to the console's.
2. **29 presets sit outside ±1.5 LU of their category** at a hardware limit (no volume control on
   the triangle and DPCM; the shortest one-shots and the thinnest duties already at full volume).
   Listed in `docs/audio/presets/levels.md` for Mika's ear.
3. **The 356 px bottom pane covers the mixer's lower strip** (PEAK, VOICES, the readout) while a
   track is selected (it was 232 px and just cleared the readout); Esc hides it. Proposal: scale
   the strips to the space left, or hide the pane in the mixer until Shift+Tab.
4. **The browser isn't resizable**: at the board's 420 px the lanes are 180 px narrower; a draggable
   edge (as the headers') is a small follow-up (IDEAS.md).
5. **Collections have fixed names** (Favourites and six colours); renaming is a follow-up.
6. The prompt says 60 classic presets; the table has 61 (PB-D5).

## 5. Merge

__MERGE_DETAIL__
