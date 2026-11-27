# VISION: Cartwave

*Draft for CEO sign-off · 2026-11-25, updated 2026-11-26 (the meeting) · Owners: Product (Marcus)
and Design (Sam) · Sound design: Mika Torres*

**Every prompt, spec and review starts from this page.** If a change conflicts with it,
the change is wrong, or this page needs updating first, with the CEO's sign-off.

> **Cartwave** is the app, made by **Glintworks** (below). **The rename is done** (2026-11-27,
> RENAME.md): the app, its files (`.cartwave`; `.chipdaw` opens forever), settings folder,
> environment variables and identifiers say Cartwave, and the repository is
> `glintworks-studio/cartwave`.

*Decided 2026-11-26:* the working title "ChipDAW" is replaced by **Cartwave**, and Mika Torres
(Sound Design) joins the team.

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
   or clearly disabled, with the reason on hover (in the status bar: there are no tooltips).
4. **Authenticity is opt-in, per track.** Studio tracks are the default. The rules apply only
   where the user chose them.
5. **Warn, don't surprise.** Any action that would break an active hardware profile asks
   first, and offers a way forward, not just "are you sure?".
6. **Ask only when it matters.** No dialog when nothing conflicts.
7. **Game-ready output.** Seamless loops, stems and sensible loudness are core features.
8. **Fast and glitch-free on ordinary computers.** PERF.md and ENGINE.md budgets are
   requirements.
9. **Open core.** The app and the NES/Famicom engine are open source (MIT OR Apache-2.0),
   free forever, with every asset provably ours (ASSETS.md). Paid extensions live outside this
   repository (see "Company, product and business model").

*Decided 2026-11-26:* principle 9 is open core (it was "open source"); hover help goes to the
status bar (principle 3).

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

### Instruments
- **Chip Synth** is the main instrument. Its **Voice** selector picks the sound source:
  **Pulse · Triangle · Noise · VRC6 Pulse · VRC6 Saw**. A preset remembers its voice.
- **DPCM Sampler** is a separate instrument. **FM Synth** arrives with the arcade board.
- **Studio-only controls:** Unison, Detune and pulse-width drift. A locked track never shows
  them.
- **On a locked track** the voice is fixed to the real channel, only that channel's controls
  are shown, and the device is drawn **dimmed** (labels stay readable, ≥ 4.5:1).
- Reference: `docs/design/meeting-2026-11-26/workflow-chip-synth.*` (locked for layout).

*Built in 4a:* a track is typed by its voice (Pulse, Triangle, Noise, DPCM, VRC6 Pulse, Saw:
Create ▸ Insert Track ▸ a type, the badge shows the type) and the inspector has no Chip Synth
device, voice selector or studio-only controls. **To change in the Presets + Browser + device
panel batch** (Unison and Detune come first, in 4b).

*Decided 2026-11-26:* this section is new.

### The device chain (as in Ableton)
Every track runs **MIDI effects → instrument → audio effects**. Devices are dragged from the
browser onto any track, including Main.
- **MIDI effects:** **Chip Arp** (chords into frame-locked arpeggios), **Chord** and **Scale**.
  On a locked track they are allowed only if the result stays one note at a time; otherwise
  they ask first.
- **Audio effects:** **Compressor, Limiter, Delay and Reverb.** **None on locked tracks** (the
  real chip has none): adding one asks first (**Unlock track** · **Cancel**). **Always allowed
  on Main**, which comes after the chip mix.

*Built in 4a:* no MIDI or audio effects exist yet, and Main has no device slot (its OUT badge
and output device only). **To change in the device-chain batch** (MIDI effects, then audio
effects).

*Decided 2026-11-26:* this section is new; it answers the "first effects set" question.

### Hardware profiles and locked tracks
- A **hardware profile** is a rulebook for one real console setup: which chips, how many
  channels of each kind, and what each channel can do. The first profiles are **2A03** and
  **2A03 + VRC6**. More consoles arrive as more profiles.
- **Authentic presets** belong to a profile and live in the browser's **Hardware** folder.
  Loading one puts the project under that profile (if it isn't already) and **locks the
  track**: 🔒 2A03 / 🔒 VRC6, shown on the track header and on the preset in the browser.
- **A locked track follows the real chip:** one note at a time, only the controls the
  hardware has, no audio effects, and MIDI effects only while the result stays one note at a
  time. Unavailable controls are hidden, not broken.
- **Locked tracks in the mixer:** the fader is pinned at 0 dB and the pan centred. Locking a
  track whose fader or pan isn't neutral asks to reset them; unlocking restores them.
- **Click the lock to unlock:** the track becomes a studio track that sounds the same, with
  polyphony and effects.
- **Channel budget**, shown for the project: e.g. *Pulse 2/2 · Triangle 1/1 · Noise 1/1 ·
  DPCM 0/1 · VRC6 Pulse 1/2 · Saw 0/1*. It turns red when over.
  **All tracks locked + within budget = the song plays on real hardware.**
- **One expansion chip per project**, as on a real cartridge.
- **The Main track** is pinned at the bottom: a black strip (black is reserved for it), PEAK,
  and a master fader that **always works, even when every track is locked**. Main's devices
  come after the chip mix (above). Reference: `docs/design/meeting-2026-11-26/main-track.*`.
- **Under the hood:** locked tracks share one virtual console and its real (non-linear)
  mixer. Studio tracks each get their own chip instance, a normal mixer and effects.
- **Profile warnings:** any action that would break the active profile asks first. Examples:
  adding an effect to a locked track, adding a chord, a note outside the chip's range, a
  third pulse track, a preset from a different expansion chip. The options: **Unlock track** ·
  **Fit to hardware** (where one applies) · **Cancel**.

*Built in 4a:* the locked fader and pan, "Reset and lock", restore on unlock, and the Main
track (black strip, PEAK, a master fader after the whole mix) are done (UX.md §4e). The Main
row was built from the prompt's text because its board was missing (RUNLOG B4A-D1, D15); the
board is now in the pack. **To check against `main-track.*` in 4b's visual fixups.** The
authentic presets are today the channel layouts (B4A-D14); **the Hardware folder comes in the
Presets + Browser batch.**

*Decided 2026-11-26:* locked tracks in the mixer, the Main track, effects on locked tracks
and the Hardware folder.

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

### Importing tracker projects
- In this order: **FamiStudio** projects (MIT), then **FamiTracker text export**, then
  **`.ftm`**. The format names may appear in the Import menu (CEO decision).
- **Clean-room, from format documentation, never from GPL source.**
- Channels become tracks, instrument sequences become macros, and effects become frame-tick
  automation.
- **NSF import is not allowed:** an NSF is a ROM rip (the ROM rule, "Sound systems").

*Decided 2026-11-26:* this section is new.

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
the way that hardware was commonly used. The NES/Famicom system is part of the free open
core; the systems after it are paid **Emulations** (see "Company, product and business
model").

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

**The authenticity method:**
- **Never translate or paraphrase GPL code.**
- Accurate open-source cores serve as **external reference oracles**, kept outside the repo:
  feed both the same register writes, diff the audio, then Mika reviews by ear.
- Where a permissively licensed core exists, use it: **ymfm (BSD-3) for the arcade board's
  YM2151.**

**Reference listening:** a fan MIDI of copyrighted game music may be used **privately, by ear,
only**. It lives in `reference-private/`, which is git-ignored, and never appears in the repo, screenshots, demos, fixtures or release
notes. Presets are never tuned to copy a specific game's sound.

*Decided 2026-11-26:* the authenticity method, reference listening, ymfm for the YM2151, and
paid Emulations after the NES.

## Presets and the browser

### Presets
- Every preset is **credited to Jeffrey Scott Frederick** as its author, and every preset has
  its own provenance entry in ASSETS.md. Nothing is derived from any third-party library, game
  ROM or ripped patch.
- The NES/Famicom starter bank is free and genuinely good; Mika builds the 100+ preset bank.

*Built in 4a:* the 60 presets and 5 channel layouts share one ASSETS.md row. **Per-preset
entries to add in the Presets + Browser batch**, with Mika's bank.

### The browser (Live's structure)
- **Sidebar:** **Collections / Library / Places**.
- **Library:** **Sounds** (by role: Leads, Basses, Arps, Pads, Bells & Keys, FX, Game
  Sounds), **Drums**, **Instruments**, **Hardware** (authentic presets by profile; loading one
  locks the track), **Audio Effects**, **MIDI Effects**, **Clips**, **Samples**.
- **Places:** User Library, Current Project, Imported Projects.
- **Folders with expand and collapse are the main way to browse;** tags only power search.
- Select to preview; double-click or drag to load.
- **The old flat browser is replaced, not kept alongside.**

### The preset picker inside the instrument
A centred preset field with **◀ ▶** to step through its folder (with preview), a dropdown with
the same folders as the browser, *Show in Browser* and *Save Preset As…*.

*Built in 4a:* the browser is one flat, searchable list (presets by category, then "Channel
Layouts") with a Preview key (UX.md §5.4), and the inspector has no preset field. **To change
in the Presets + Browser + device panel batch.** Reference:
`docs/design/meeting-2026-11-26/workflow-chip-synth.*`.

*Decided 2026-11-26:* this section is new.

## The mixer view
- **Machined knobs, plastic fader caps,** and 24 px M/S keys in the mixer's darker key style.
- **Warm meters:** warm white → amber → orange → red above 0 dB; **PEAK** per channel;
  **VOICES n / ∞** on studio tracks, the hardware channel on locked tracks.
- **The channel-budget bar** across the top, **two-digit number slots**, and wider strips that
  scroll sideways.
- Reference: `docs/design/meeting-2026-11-26/mixer.*` and `knobs.*` (option A);
  `meter-colours.*` option 2.

*Built in 4a:* the Mixer view key is disabled with its reason. *Built in 4b:* the mixer view
(Tab), as above (BATCH4B.md).

*Decided 2026-11-26:* this section is new.

## Export
- Seamless loops, stems and the game export, as today.
- **Loudness targets** are an export setting (e.g. "Game: −16 LUFS / −1 dBTP").
- **Authentic export** bypasses Main's devices, so the file is exactly the chip mix.

*Built in 4a:* the export applies Main's master gain after the mix and writes it to the JSON;
there are no loudness targets and no devices on Main yet. **To change in the export polish**
(authentic export once Main has devices).

*Decided 2026-11-26:* this section is new.

## The app shell (baseline for a real app)
- **Native menus.** On macOS the system menu bar: App, File, Edit, Create, View, Window, Help.
  On Windows the standard in-window menu bar.
  - **File:** New, Open, Open Recent, Save, Save As, Import (MIDI, audio, tracker projects),
    Export…, Project Settings, Quit
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

## Company, product and business model
- **Company: Glintworks.** Website **glintworks.studio**: one domain that hosts every product
  page (e.g. `glintworks.studio/cartwave`), in the style of getartcraft.com.
- **App: Cartwave.**
- **App icon:** concept A, "Chip, refined": a graphite board, a brushed-metal lid with an
  etched square wave, and one lime LED. A simplified variant is used at 32 px and below. It's
  drawn as vector. Reference: `docs/design/meeting-2026-11-26/cartwave-icon.*` (column A).
- **Company logo:** an **eight-point glint** (four long curved points plus four short
  diagonal triangles) in **one plain colour**: warm white `#F2E9D6` on dark, near-black
  `#1A1A1A` on light. No gradient, no glow.
  - Beside it, **Glintworks** in bold, with **STUDIO** letter-spaced underneath and tight to
    the wordmark.
  - The glint alone is the avatar and favicon.
  - Final-file tweaks (Sam): the star 3 px lower (centred on the visible letters), a 10 px gap
    to the wordmark, STUDIO's trailing letter-space trimmed, and slightly customised
    letterforms.
  - Reference: `docs/design/meeting-2026-11-26/glintworks-logo.*` (column A).
- **Business model: open core.**
  - **Free and open source forever** (MIT OR Apache-2.0): the app, the NES/Famicom engine and
    profile, the core effects, and a genuinely good starter preset bank. **A free feature is
    never moved behind a paywall.**
  - **Paid extensions**, from private repos under a commercial licence:
    - **Emulations** (the arcade FM board, the SNES, future systems);
    - **premium effects;**
    - **Cartridges:** premium preset banks, kits, templates and genre packs, under a content
      licence.
  - One store on glintworks.studio. No nag screens, no crippled free version, and
    offline-friendly licence keys.
  - **Outside contributions** go into the open core only. GPL stays excluded everywhere. BSD-3
    (e.g. ymfm) is fine inside paid extensions, with attribution.
- ~~Still to do (CEO): official trademark checks for Glintworks and Cartwave~~: **cleared**
  (the rename prompt, 2026-11-27); the code rename is done (RENAME.md).

*Decided 2026-11-26:* this section is new. It answers open questions 1 (business model) and 3
(product name).

## Open questions for the CEO
1. ~~Business model~~: **decided 2026-11-26**, open core ("Company, product and business
   model").
2. ~~The first effects set~~: **decided 2026-11-26** for audio effects (Compressor, Limiter,
   Delay, Reverb) and MIDI effects (Chip Arp, Chord, Scale): "The device chain". Premium
   effects are paid extensions.
3. ~~The product name~~: **decided 2026-11-26**, Cartwave by Glintworks.

*Decided 2026-11-25:* studio tracks are unlimited (tracks, voices, effects). Time
signatures and tempo work exactly as in Ableton, including mid-song changes and tempo
automation. The second system is the 1989 arcade FM board.

## Build roadmap (decided 2026-11-26; replaces the order from Batch 4a)
1. **4a** (done, merged: tracks as instruments, hardware profiles and locks, the Main track);
2. **the authenticity A/B pass;**
3. **4b** (built on `batch4b/mixer`, BATCH4B.md: visual fixups, the mixer view, audible stereo
   pan, MIDI import, unison and detune);
4. **Presets + Browser + device panel** (Chip Synth, DPCM Sampler, the in-instrument picker,
   Mika's 100+ bank);
5. **Batch 5: automation** (the parameter registry is ready: docs/batch4a/parameters.md);
6. **the device chain** (MIDI effects, then audio effects);
7. **tracker import;**
8. **recording and MIDI input;**
9. **time signatures and scales;**
10. **the export polish;**
11. **the Metal backend;**
12. **the arcade board** (ymfm);
13. **the SNES.**

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

*2026-11-26, CEO, the meeting, recorded by the docs update (DOCS_UPDATE_PROMPT.md,
[DOCS-2026-11-26.md](DOCS-2026-11-26.md), RUNLOG DOC-D*):*
- Cartwave by Glintworks, open core, the icon and the logo; Mika Torres joins (Sound Design).
- Chip Synth with a Voice selector, DPCM Sampler, FM Synth later; studio-only Unison, Detune
  and pulse-width drift; locked devices dimmed.
- Locked tracks in the mixer; the Main track; the device chain and which effects are allowed
  where; export loudness targets and authentic export.
- The browser in Live's structure (replacing the flat one) and the in-instrument preset picker;
  presets credited to Jeffrey Scott Frederick.
- The mixer view; tracker import (never NSF); the authenticity method; reference listening.
- The roadmap above. Reference boards: `docs/design/meeting-2026-11-26/` (its README gives
  each board's status).
