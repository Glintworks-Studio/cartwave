# VISION: ChipDAW (working title)

*Draft for CEO sign-off · 2026-11-25 · Owners: Product (Marcus) and Design (Sam)*

**Every prompt, spec and review starts from this page.** If a change conflicts with it,
the change is wrong, or this page needs updating first, with the CEO's sign-off.

---

## The one-liner
**Make chiptune music as easily as in Ableton, with the power of a modern retro-sound
instrument, and real-hardware authenticity whenever you want it.**

## Who it's for
Indie game developers and musicians who already know a modern DAW. **Ableton Live is the
only workflow reference.** Tracker conventions are not a design input.

## The three pillars
1. **As easy as Ableton.** If an Ableton user reaches for a behaviour (a double-click, a
   shortcut, a drag, a menu item), it should do what they expect.
2. **As powerful as a professional retro instrument library.** Rich, playable sounds:
   polyphony, layering, modulation, effects and a big preset library, across many classic
   sound chips over time.
3. **Authentic when you want it.** Load a hardware profile, lock a track, and it plays
   exactly like the real chip.

## Design principles
1. **Copy behaviour, never identity.** We match Ableton's *workflow conventions*, not its
   look, names, icons or text. "Ableton" never appears in the product.
2. **Every control has exactly one home.** No duplicates. If two places do the same thing,
   one of them goes.
3. **Nothing pretends to work.** A control that does nothing in the current state is hidden
   or clearly disabled, with the reason on hover.
4. **Authenticity is opt-in, per track.** Studio tracks are the default. The rules apply only
   where the user chose them.
5. **Warn, don't surprise.** Any action that would break an active hardware profile asks
   first, and offers a way forward, not just "are you sure?".
6. **Ask only when it matters.** No dialog when nothing conflicts.
7. **Game-ready output.** Seamless loops, stems and sensible loudness are core features.
8. **Fast and glitch-free on ordinary computers.** PERF.md and ENGINE.md budgets are
   requirements.
9. **Open source** (MIT OR Apache-2.0), with every asset provably ours (ASSETS.md).

## The core model

### Tracks are instruments
There are two kinds of track: **studio tracks** (the default: no hardware rules) and
**hardware-locked tracks** (🔒, below). "Studio" means *unrestricted*, not a pricing tier.

- **Studio tracks are unlimited, as in Ableton:** any number of tracks, unlimited polyphony
  and unlimited effects per track. The only limit is the computer's CPU, which a CPU meter
  shows. Each instrument has an optional **Voices** setting (default: unlimited) for anyone
  who wants a cap.
- Behind the scenes, the engine keeps a large pre-allocated voice pool that grows from the
  UI thread when needed. The audio thread never allocates (ENGINE.md §1). If a pool is ever
  exhausted, the quietest voice is stolen gracefully rather than glitching.
- Each track is an instrument: it plays chords, holds a preset, and has its own effects and
  modulation.
- Every track runs its own emulated chip voice(s). The emulator is cheap (~0.2% CPU per
  chip), so many instances are fine.
- MIDI clips on tracks, edited in the piano roll, as in Ableton.

### Hardware profiles and locked tracks
- A **hardware profile** is a rulebook for one real console setup: which chips, how many
  channels of each kind, and what each channel can do. The first profiles are **2A03** and
  **2A03 + VRC6**. More consoles arrive as more profiles.
- **Authentic presets** belong to a profile. Loading one puts the project under that profile
  (if it isn't already) and **locks the track**: 🔒 2A03 / 🔒 VRC6, shown on the track header
  and on the preset in the browser.
- **A locked track follows the real chip:** one note at a time, only the controls the
  hardware has, and no effects. Unavailable controls are hidden, not broken.
- **Click the lock to unlock:** the track becomes a studio track that sounds the same, with
  polyphony and effects.
- **Channel budget**, shown for the project: e.g. *Pulse 2/2 · Triangle 1/1 · Noise 1/1 ·
  DPCM 0/1 · VRC6 Pulse 1/2 · Saw 0/1*. It turns red when over.
  **All tracks locked + within budget = the song plays on real hardware.**
- **One expansion chip per project**, as on a real cartridge.
- **Under the hood:** locked tracks share one virtual console and its real (non-linear)
  mixer. Studio tracks each get their own chip instance, a normal mixer and effects.
- **Profile warnings:** any action that would break the active profile asks first. Examples:
  adding an effect to a locked track, adding a chord, a note outside the chip's range, a
  third pulse track, a preset from a different expansion chip. The options: **Unlock track** ·
  **Fit to hardware** (where one applies) · **Cancel**.

### Importing MIDI (drag and drop)
- Drag a `.mid` over the arrangement and a **live ghost preview** shows the tracks to be
  added: one per MIDI part, with its name and note count, at the drop position. Drop on
  empty space for new tracks, or onto an existing track to put the clip there.
- Parts that would break the active profile show a warning **while dragging**.
- On drop, a dialog appears **only if something conflicts**:
  - **Import as studio tracks** (recommended): plays everything as written.
  - **Fit to hardware**: assigns channels, folds notes into range, and reduces chords, with
    an option to turn **chords into fast arpeggios** (the classic chip trick).
  - **Cancel**.
  - ☐ *Don't ask again* (changeable in Settings).
- *To verify in the audit:* whether we can track the cursor during an operating-system file
  drag on Windows and macOS.

### Automation (as in Ableton)
- **Arrangement automation:** every track has automation lanes for any parameter
  (instrument, mixer, effects, macros). Breakpoint envelopes: click to add a point, drag to
  move it, drag a segment to curve it, draw freehand in draw mode, delete with a
  **single click** on the breakpoint (as in the reference) or Delete. Lanes show and hide per track.
- **Clip envelopes:** automation that lives inside a clip and moves with it.
- **Touching an automated control** overrides it until playback re-engages the automation
  (the standard re-enable behaviour). The control shows clearly that it's automated.
- **Hardware-locked tracks automate authentically:** values are quantised to the
  register's real steps (e.g. volume 0–15) and applied on the console's 60 Hz frame tick,
  which is exactly how game sound drivers did it. Studio tracks automate smoothly.
- Automation is undoable, saved in the project, and rendered identically in export.
- **Tempo automation** is in the first build, as in Ableton. It doesn't break any hardware
  profile: game drivers changed tempo freely, and notes on locked tracks are already
  quantised to the console's frame tick.

### Time signatures and tempo (exactly as in Ableton)
- **Real hardware has no idea what a time signature is.** The chips just play register
  writes on each frame tick, so any meter (3/4, 6/8, 5/4, 7/8, …) and any **mid-song
  change** plays back on the real console. Time signatures therefore **never** trigger a
  profile warning.
- Behaviour matches Ableton: a project time signature, plus **time-signature change
  markers** on the timeline. The ruler, grid, snap, bars.beats clock, loop brace, MIDI
  import and export all follow the signature in force.
- The only real hardware limit on timing is the **frame tick** (60 Hz): note starts on
  locked tracks land on frame boundaries, as they always have. That's tempo resolution, not
  meter. (A "frame-locked tempo" helper stays in IDEAS.md.)

### Scales (piano roll)
- Pick a **root and a scale** (C Major, A Minor, Dorian, Pentatonic, Blues, …) for the
  project, with an optional override per clip.
- Notes outside the scale are **disabled for input**: their rows are dimmed and drawing
  snaps to the scale. A **Fold** toggle shows only in-scale rows.
- **Existing notes are never deleted** when the scale changes. Out-of-scale notes are
  flagged so the user decides.
- Transposing with ↑/↓ moves **within the scale**. Shift still moves an octave.
- Scale has no effect on drum and sample rows (noise kits, DPCM).
- *Proposed:* suggest a scale when a MIDI file is imported.

### Sound systems (profile roadmap)
Each new system ships as **an emulator + a hardware profile + an idiomatic preset set**,
built clean-room from public documentation. A system is done only when its presets sound
the way that hardware was commonly used.

| Order | System | Chips | Character |
|---|---|---|---|
| 1 (done) | NES / Famicom | 2A03, + VRC6 | Pulses, triangle bass, noise drums, 1-bit samples |
| 2 | **Late-'80s arcade FM board** (the setup heard in Konami's 1989 Turtles *arcade* game; confirmed) | **YM2151** (8-voice, 4-operator FM) + **K007232** (PCM) + **µPD7759** (ADPCM speech) | FM brass, slap bass, e-pianos, bells, sampled drums, voice |
| 3 | **SNES** | S-DSP (8 sample voices, BRR samples, ADSR, echo with FIR filter) | Sampled orchestra, pads and choirs, the famous echo |

- **Presets per system** follow that hardware's common idioms, for example:
  - NES: thin-duty leads, triangle kick-bass, noise hats
  - arcade FM: FM brass, slap bass, bell and e-piano patches, sampled drums
  - SNES: strings, choir and pads through the echo

  Every preset is our own creation. **We never extract patches, samples or music from game
  ROMs.**
- Sample-based systems (the arcade PCM chips, the SNES) need **our own sample library**,
  recorded or synthesised, and listed in ASSETS.md. That's a content effort as well as an
  engineering one.
- **Naming:** profiles are named after the hardware ("YM2151 + PCM arcade"), never after a
  game or publisher. "Konami", "TMNT" and game titles don't appear in the product.

## The app shell (baseline for a real app)
- **Native menus.** On macOS the system menu bar: App, File, Edit, Create, View, Window, Help.
  On Windows the standard in-window menu bar.
  - **File:** New, Open, Open Recent, Save, Save As, Import (MIDI, audio), Export…,
    Project Settings, Quit
  - **Edit:** Undo, Redo, Cut, Copy, Paste, Duplicate, Delete, Select All, Rename
  - **Create:** Track, MIDI Clip
  - **View:** browser, piano roll, zoom
  - **Help:** keyboard shortcuts, About
- **Settings window** (Preferences on macOS): Audio (device, buffer size, sample rate),
  Display (frame-rate cap), Editing (default grid, follow playhead), Files (autosave,
  recovery), Warnings ("don't ask again" resets).
- **Project settings:** hardware profile, output filter. The chip and filter move **out of
  the top bar**. **Tempo lives in the top bar**, as in the reference, with the time signature
  next to it (not in Project Settings).
- **Ableton-standard editing**, including: double-click a note to delete it, double-click
  empty space to create one; Ctrl/Cmd+R renames clips and tracks; right-click context menus
  everywhere; a full shortcut map.
- **An empty-project start** that tells a new user what to do first.

## What carries over (nothing important is thrown away)
The emulator, band-limited synthesis, the real-time engine (zero-dropout architecture), the
Vizia rendering and performance work, the piano roll, undo, the file format with its
migrations, the asset provenance, and seamless loop and stem export. What changes is the
model above them: fixed hardware lanes become instrument tracks, plus profiles.

## Not in the first build (confirm)
Audio recording tracks, third-party plugin hosting, exporting to real-hardware music
formats. The arcade FM and SNES systems come after the app shell is solid. See IDEAS.md.

## Open questions for the CEO
1. **Business model.** The code is open source (MIT OR Apache-2.0), so limits in a "free
   version" can be removed by anyone who builds it from source. Options that fit open source:
   - (a) everything free; revenue from **paid preset and sample packs** (separately
     licensed content);
   - (b) **paid signed installers / store builds**, with free source;
   - (c) **sponsorships and donations**;
   - (d) **support and licensing for studios**;
   - (e) **change the licence** (e.g. open-core with closed paid features). That would
     reverse an earlier decision and affects contributors.
2. The first effects set for studio tracks (e.g. EQ, filter, delay, reverb, bitcrush,
   chorus, compressor)?
3. The product name.

*Decided 2026-11-25:* studio tracks are unlimited (tracks, voices, effects). Time
signatures and tempo work exactly as in Ableton, including mid-song changes and tempo
automation. The second system is the 1989 arcade FM board.

## How we work from here
1. **Every prompt starts with VISION.md.** Claude reads it first.
2. **New ideas go to IDEAS.md**, not straight into a build.
3. **Next step:** a code-free **UX audit** of the current app, measured against this page,
   which Sam turns into UX.md (must / should / later, marked by the CEO).
4. Then build in batches, **with an automated test for every specified behaviour**. Then
   dogfood with real songs.

## Decisions

*2026-11-25, CEO, recorded by the app-shell run (SHELL_PROMPT.md):*
- **Tempo lives in the top bar**, as in the reference, with the time signature next to it.
  It is not repeated in Project Settings (principle 2). Changed in "The app shell".
- **A single click on an automation breakpoint deletes it**, as in the reference (it was a
  double-click here). Changed in "Automation".
- **UX.md** (from the UX audit's draft, every proposed priority accepted) is the build spec
  from now on. The replaced `CLAUDE.md` points here first.
