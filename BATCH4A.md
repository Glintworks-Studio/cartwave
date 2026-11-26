# BATCH4A.md: tracks are instruments, hardware profiles and locks, the Main track

Branch **`batch4a/model`** (from `main` 0e669ac). Prompt: `BATCH4A_PROMPT.md` (v2). Decisions:
[`docs/RUNLOG.md`](docs/RUNLOG.md) **B4A-D1…D20**. Spec: [`VISION.md`](VISION.md),
[`UX.md`](UX.md) §1, §3, §4e, §5.2, §6; [`docs/design/VISUAL-SPEC.md`](docs/design/VISUAL-SPEC.md) §4b.

## 1. Summary (one screen)

GATES_TABLE

**Pre-flight.** `main` fast-forwarded to `origin/main` 0e669ac: **`mac/catchup` is merged** (its
last commit, MC-D24). Baseline = that `main`, built in `../before-batch4a`. **The CEO's
`B4a-main-track.*` reference folder was not handed over** (not in the repo, `polish2-ref/`, or the
owner's Documents, Downloads or Desktop): the Main row follows the prompt's §7 text on the E1
header grid, and the §9.3 check measures it against that (B4A-D1, B4A-D15).

**What changed.**
- **Fixups (§3):** keys never reach the song behind a dialog (MC-D9); colour previews never reach
  the undo history (the real cause: the per-tick commit); "▸" → ">" (Inter has neither ▸ nor 🔒:
  the lock is drawn); Ctrl/⌘-click toggles a clip and Shift-click selects a range (R28).
- **Tracks are instruments (§4):** a track has a voice type and is **studio** (its own pool of
  voices: unlimited tracks, unlimited polyphony, a Voices cap, quietest-voice stealing, the pool
  grown from the UI thread) or **locked** to one real channel (the full chip emulator, the last
  note wins, fader 0 dB / pan centre). Studio voices are the chip's own channel units driven by
  the driver's own lane code: no second emulator. Format **v7**; every older file opens with its
  tracks locked to their channels and **renders bit-identically**.
- **Profiles and locks (§5):** Project Settings ▸ None / NES (2A03) / Famicom + VRC6; the channel
  budget (Project Settings, compact in the TRACKS cell, red when over); the badge (🔒 + channel, or
  the type) opens the lock menu (taken channels disabled, naming the owner); "Reset and lock"; the
  locked header; profile warnings with Unlock / Fit / Cancel and "Don't ask again"; an old file
  over budget is a status-bar problem; layouts are the authentic presets.
- **Tracks:** Create ▸ Insert Track (Ctrl+Shift+T) and Insert Track ▸ a type, Duplicate, Delete,
  rename, header drag to reorder, Voices ▸; each one undo step.
- **The parameter registry (§6)** for Batch 5: [`docs/batch4a/parameters.md`](docs/batch4a/parameters.md).
- **The Main track (§7):** pinned at the bottom, black strip, OUT + device, PEAK, the master
  fader after the whole mix (saved, in the export JSON, bit-identical at 0 dB), a dark lane.

**Decisions taken alone** (§6 below): Create menu instead of a "Track" menu (VISION.md wins);
NES for an unused VRC6 with its filter kept; empty VRC6 lanes under NES become studio tracks; a
v6 fader ≠ 0 dB is locked like any other and reported; File > New is studio-only; studio macros
are frame-tick; Unlock is offered only where it fixes the conflict; layouts are the authentic
presets; the Main row from the prompt's text.

**Studio vs locked** (§5 below): one note sounds the same (+0.000 dB, measured); chords differ by
the console's non-linear mixer (two pulses at full volume: studio +1.26 dB).

**Merge:** MERGE_LINE

GATES_DETAIL
