# BATCH5.md: automation (Live's behaviour), plus four fixes from the Presets review

Branch **`batch5/automation-2`** (from `main` 4529f1e, the Presets batch merged), pushed; the first
branch `batch5/automation` holds a mistaken 1.4 GB commit and nothing else of value (B5-D5).
Prompt: `BATCH5_PROMPT.md`. Decisions: [`docs/RUNLOG.md`](docs/RUNLOG.md) **B5-D1…** Spec:
[`VISION.md`](VISION.md) ("Automation", "Hardware profiles and locked tracks"), [`UX.md`](UX.md) §5.6, the
Live 12 manual ch. 26 ([`docs/ux-audit/manual-refs.md`](docs/ux-audit/manual-refs.md) R104–R115).
Baseline: `main` exported to `../before-batch5`. Machine: Windows 11 Pro 26200 (not macOS).

## 1. Summary (one screen)

GATES_TABLE

**What changed.**
- **A1** The mixer view hides the bottom pane (the Detail View) by default; **Shift+Tab** shows or
  hides it there; each view keeps its own state for the session (the arrangement's unchanged).
- **A2** Music Box, Pizzicato and Menu Blip raised to their voice's maximum (Volume 15, the volume
  macro to a loudest step of 15, shape kept): +4.4, +1.8 and +3.4 LU; still 2.5–3.8 LU short
  of their targets (the voice's ceiling, reported). Projects keep instrument copies: old projects
  unchanged (299 of 299).
- **A3** Studio voices are **bipolar**: each subtracts its waveform's own mean, so the baseline
  no longer jumps with each note (the cause: unipolar voices and only a 5 Hz blocker on the
  studio bus). Locked tracks bit-identical everywhere; studio goldens re-baselined with old → new
  hashes ([`docs/batch5/a3-rebaseline.md`](docs/batch5/a3-rebaseline.md)).
- **A4** A private reference render through the app's own MIDI import, git-ignored (§5); a real
  import bug found on the way: Fit to hardware left extra pulse parts off the VRC6 (fixed).
- **Automation** (Part B): Automation Mode (**A**) with lanes under each track (chooser,
  +, −, Show Automated Parameters Only), breakpoint editing as the reference (click the line to
  add, click a point to delete, drag with Shift fine and grid snap, Alt-drag to curve, a lane's
  time selection to move / copy / paste / delete, Draw Mode **B** steps, double-click away from the
  line adds), clips carry their automation (**Edit ▸ Lock Envelopes** off), **Automation Arm** and
  touch recording, **override** (grey) and **Re-Enable Automation** (orange) in the transport, the
  white dot on automated controls, the Main gain's lane over the Main row. Locked tracks automate
  their instrument parameters **per frame, quantised to the chip's steps** (their lanes draw the
  steps), with no fader or pan lanes; studio tracks automate their fader, pan, on/off, channel
  controls, macros and Unison / Detune / Drift; Main its gain. **Format 10**; every export
  renders automation; the game export says which studio lanes have no hardware equivalent.

A_RESULTS

DECISIONS

MERGE

## 2. A3: before and after (every preset; the full table is [`docs/batch5/a3-dc.md`](docs/batch5/a3-dc.md))

A3_TABLE

## 3. Automation: how it works

ENGINE_TEXT

## 4. Screens

`docs/screens/batch5/`: [automation-mode](docs/screens/batch5/automation-mode.png),
[lanes-chooser](docs/screens/batch5/lanes-chooser.png), [locked-stepped](docs/screens/batch5/locked-stepped.png),
[override-reenable](docs/screens/batch5/override-reenable.png), [draw-mode](docs/screens/batch5/draw-mode.png),
[mixer-pane-hidden](docs/screens/batch5/mixer-pane-hidden.png), [regression-diff](docs/screens/batch5/regression-diff.png).
For Sam: the colours are VISUAL-SPEC §14's starting proposal (no board yet).

## 5. A4: the private reference render

PRIVATE

## 6. Limitations and open points

LIMITS
