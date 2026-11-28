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

**A1–A4.**

| Fix | Result |
|---|---|
| A1 the mixer and the bottom pane | **Met.** Hidden by default in the mixer, Shift+Tab shows / hides it there, per view for the session (AU-18). The mixer at 100 % and 150 % against `pre-presets`: only the strips' scrollbar thumb differs (the 420 px browser) ([report](docs/batch5/regression/a1-diff-100/report-100.md), [screen](docs/screens/batch5/mixer-pane-hidden.png)). |
| A2 three quiet presets | **Partly met: the targets aren't reachable.** Projects keep instrument copies, so the presets changed in place (299 of 299 old project files identical). Raised with the bank's own mechanism as far as the voice goes (Volume 15, the macro's loudest step 15, its shape kept): Pizzicato −33.4 → −31.6 LUFS (target −26.3), Music Box −34.7 → −30.3 (−26.3), Menu Blip −37.3 → −33.9 (−30.1). Remaining gaps 5.3, 4.0 and 3.8 LU (after A3 −31.6 / −30.3 / −33.9 are the pack's numbers): a thin pulse with a fast decay at the voice's maximum. Closing them needs a character change (the duty or the decay), Mika's call. WAVs, `levels.md`, PROVENANCE change lines done. |
| A3 studio baseline steps | **Met where it is DC; reported where it isn't.** Cause: unipolar voices + a 5 Hz bus blocker. Fix: bipolar voices. 50 ms swing ≤ 0.005: **27 → 141 of 202**; none worse; worst 0.18 → 0.041; the DC over each whole sound ≤ 0.0006 (was up to 0.0082); peak balance median 0.52 → 0.88; no clipping; loudness within ±0.3 LU for 196 (exceptions −0.56 … +0.35, listed). The 61 still over 0.005 are basses, noise drums / FX and DPCM: a 50 ms window catches their own low waveform (their locked twins measure the same way). Locked tracks bit-identical in all 308 project files. Re-baseline: [a3-rebaseline.md](docs/batch5/a3-rebaseline.md). Cost: none measurable (64-track render 2.45–2.57 s vs 2.50–2.53 s). |
| A4 private reference render | **Done, private.** §5. Nothing under `reference-private/` is tracked; `git status --ignored` shows it ignored before every push. |

DECISIONS

MERGE

## 2. A3: before and after (every preset; the full table is [`docs/batch5/a3-dc.md`](docs/batch5/a3-dc.md))

A3_TABLE

## 3. Automation: how it works

- **Where values come from.** An envelope's points hold values in the parameter's units (the
  registry's: register steps, macro steps, cents, dB with −inf as −70). The song snapshot carries
  them to the engine as immutable slices (`chip::auto::AutoEnv` / `AutoPoint`): no allocation on
  the audio thread (tested under the allocation checker in debug, where an allocation aborts).
- **Locked tracks**: the console's driver evaluates each lane's envelopes **once per frame
  tick** and rounds them to the register's steps (`LaneAuto`); channel controls replace the
  control's value for the frame, macro parameters replace the instrument's macro output (volume or
  the triangle's gate, arpeggio, duty, pitch, vibrato depth). The register writes per frame equal
  the quantised envelope (`au07_…` from a project, `locked_automation_writes_…` in the driver).
  No fader or pan lane: they are pinned.
- **Studio tracks**: the same per-frame evaluation for their channel controls, macros and
  Unison / Detune / Drift (the registry's frame-tick class: a studio voice runs the same driver
  code); the **fader and pan** ramp to each frame's value in 4 steps at fixed CPU cycles (about
  1 ms apart), so nothing zippers and a render is identical with any audio buffer size (tested at
  64, 333, 480, 1000, 4096). "Per block" in the prompt became "per frame tick": a device block
  would make renders depend on the buffer size (B5-D8).
- **Main**: its gain is evaluated at the frame tick and applied from the output sample that tick
  falls on, with a fixed 200-sample ramp.
- **Overrides** leave the envelope out of the snapshot: the control's own value plays.
- **Exports** render through the same `Mixdown`: audio exports include all automation, seamless
  loops stay seamless (intro + loop × 3 = a continuous render, with automation). The game export is
  audio (VISION.md puts hardware formats out of the first build), so locked automation is in it as
  the chip's per-frame register writes, and studio automation is rendered with a report note naming
  each studio track and lane that has no hardware equivalent (B5-D10).
- **Format 10**: `automation` and `auto_lanes`; nothing new written when empty; a v9 corpus written
  by `main` loads and renders identically (`version_9_files_render_bit_identically_in_version_10`).

## 4. Screens

`docs/screens/batch5/`: [automation-mode](docs/screens/batch5/automation-mode.png),
[lanes-chooser](docs/screens/batch5/lanes-chooser.png), [locked-stepped](docs/screens/batch5/locked-stepped.png),
[override-reenable](docs/screens/batch5/override-reenable.png), [draw-mode](docs/screens/batch5/draw-mode.png),
[mixer-pane-hidden](docs/screens/batch5/mixer-pane-hidden.png), [regression-diff](docs/screens/batch5/regression-diff.png).
For Sam: the colours are VISUAL-SPEC §14's starting proposal (no board yet).

## 5. A4: the private reference render

The files are in **`reference-private/`** (VISION.md's name; `private/` is ignored too, B5-D2), both
git-ignored: the fan MIDI, `SOURCE.txt` (its URL), `reference-famicom-vrc6.wav` and
`reference-studio.wav`, rendered by `crates/ui/examples/midi_render.rs` through the app's MIDI
import. Presets used:
- **Famicom + VRC6, Fit to hardware, authentic presets** (7 parts, every one locked): Pure
  Triangle Bass (2A03 triangle), Duty Lead and Square Melody (2A03 pulses), VRC6 Reed Lead and VRC6
  Hollow Lead (VRC6 pulses), Saw Brass (VRC6 saw), Noise Kit (2A03 noise).
- **Studio tracks, presets that fit**: Round Triangle Bass, Bright Square Lead, Late Vibrato Lead,
  Duty Sweep Lead, Hollow 12.5%, Echo Square, Punchy Noise Kit.

## 6. Limitations and open points

- **A2's targets** are out of reach without changing the three presets' character (above).
- **A3**: 61 presets keep a 50 ms swing over 0.005 that is their own low-frequency waveform, not DC.
- **Collapsing a track** (B2) has nothing to act on: the app has no track fold yet (rows resize
  only). The lanes follow the row height.
- **The lane's own height** is the track's row height (the prompt's "follow the headers' row
  heights"); a separate lane resize is not built.
- **Copy / Paste of a lane's time selection** works from the keyboard (Ctrl+C / X / V, Delete) in
  the arrangement; the Edit menu's items still act on clips and the arrangement's own time
  selection. Main's lane has no keyboard selection commands.
- **Clip duplication** (Ctrl-drag) doesn't copy automation; moves carry it (the prompt's B3.5).
- **Track On automation** and the M key are independent: the envelope switches the track, M mutes on
  top of it.
- **Tempo automation** (VISION.md "Automation") is not in this batch's list (the prompt's B1) and
  is not built.
- **A touch recording** keeps every point the control sent (about 30 per second); the reference
  thins them. A Simplify Envelope command (R115) would be the place.
- **Overriding** happens on any hand move of an automated control while not armed, stopped or
  playing (the prompt says during playback; the manual says any change).
- The first branch, `batch5/automation`, holds a 1.4 GB commit (B5-D5): deleting that remote branch
  is the owner's call.
