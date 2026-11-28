# PRESETS.md: Chip Synth, the browser, the in-instrument picker, the first preset bank

Branch **`presets/browser`** (from `main` 1b8a288, Batch 4c merged), pushed. Prompt: `PRESETS_PROMPT.md`.
Decisions: [`docs/RUNLOG.md`](docs/RUNLOG.md) **PB-D1…D__**. Spec: [`VISION.md`](VISION.md) ("Instruments",
"Presets and the browser"), [`UX.md`](UX.md) §4g. Baseline: `main` exported to `../before-presets`.
Machine: Windows 11 Pro 26200, Ryzen 7 9800X3D, RTX 4070 SUPER, 2560 × 1440.

## 1. Summary (one screen)

**Merge: __MERGE__**

| # | Gate | Result |
|---|---|---|
| 1 | Bit-identical audio | __G1__ |
| 2 | Visual regression 100 % / 150 % vs `main` | __G2__ |
| 3 | Mockup match (`workflow-chip-synth`) | __G3__ |
| 4 | Tests | __G4__ |
| 5 | Audio (10 min, ◀ ▶ through 30 presets) | __G5__ |
| 6 | Performance (the §2 rule) | __G6__ |
| 7 | Lime audit, contrast | __G7__ |

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
- **The bank** (§7): __BANK__
- **The perf rule** (§2) in ENGINE.md §7b and CLAUDE.md; **K1 closed** (not reproducible) in UX.md §8.

## 2. The bank by folder

__FOLDERS__

## 3. Decisions taken alone

__DECISIONS__

## 4. Known limitations, for the owner

__LIMITS__

## 5. Merge

__MERGE_DETAIL__
