# UX audit, part 3: technical feasibility

> *Historical report, kept as written under the working title ChipDAW. Since the rename (2026-11-27, [RENAME.md](../../RENAME.md)) the app is Cartwave by Glintworks: projects save as `.cartwave`, the executable is `cartwave`, and environment variables are `CARTWAVE_*` (the `CHIPDAW_*` names still work).*

*2026-11-25 · build `stabilize/v1.1` @ `c0037d3` · Windows 11 · vizia fork `159bbe55`, winit
0.30.13. Probes live in [`probes/`](probes/) and are built separately: each is its own Cargo
project with an empty `[workspace]`, never linked into the app. Raw logs and step-by-step evidence:
[`probes/RESULTS.md`](probes/RESULTS.md).*

| Question | Answer in one line |
|---|---|
| 1. Native menus | **Yes**: `muda` 0.22 (MIT/Apache) on our fork, no fork change; commands reach the model in < 1 ms; no accelerator hook on Windows (it double-fires) |
| 2. Cursor position during an OS file drag | **Not today**: winit sends no positions during a hover (Windows or macOS). A small fork change (forward hover events + poll the cursor per frame) worked in a probe |
| 3. A separate Settings window | **Possible but not recommended**: works on winit with two fork bugs (title, focus) and is impossible under baseview. Use an in-app modal panel |
| 4. 8 fixed lanes → unlimited instrument tracks + hardware profiles | **Feasible without a rewrite.** The emulator, macros, snapshot/garbage pattern, undo and file migration all carry over. The work is a track-model refactor (behaviour-neutral, provable with the existing hash tests), then a studio voice engine, then profiles. About 5 batches; §4.6 |

## 1. Native menus

**Answer: yes, with `muda` 0.22 (MIT OR Apache-2.0; its whole dependency tree is MIT/Apache/Zlib/BSD), and no fork change.** Probe: [`probes/menu_vizia/`](probes/menu_vizia/), a vizia window on our fork with File and Edit menus.

- **Integration.** The fork's public `ModifyWindow::window()` (`vizia_winit/src/lib.rs:11-35`) gives the winit window, and `HasWindowHandle` gives the HWND. `init_for_hwnd` adds the menu bar; the client area shrinks, winit sends `Resized`, and vizia lays out to the new client rect (screenshot `probes/logs/menu_open.png`).
- **Commands → our event system.** muda's `set_event_handler` runs synchronously on the UI thread. A stored `ContextProxy` `emit`s an `AppEvent`, and the model received it within 0.3 ms. No polling and no fork hook are needed.
- **Accelerators (Windows).** vizia builds winit's event loop itself (`application.rs:160`), so muda's message hook can't be installed and muda never translates accelerators. With the stock setup, Ctrl+S and Ctrl+Z reach only vizia's `KeyDown`, so our shortcuts fire **once**, and the menu shows the key text. A `WH_GETMESSAGE` hook calling `TranslateAcceleratorW` **double-fired** (logged); swallowing the key fixed that but stole Ctrl+Z from text fields. **Recommendation: no hook on Windows.** The menu displays the accelerators and our one shortcut table handles the keys (FINDINGS F-30).
- **Event loop.** While a menu is open, Windows runs a modal loop: **rendering and vizia timers stop** (a 1.9 s stall was logged), so the playhead and meters freeze; **audio keeps playing** (separate thread). This is acceptable (other DAWs on Windows behave the same) but worth knowing for the demo video. Use `MenuTheme::Dark` (Auto drew a light bar).
- **macOS (from source; test in the Mac pass).** winit installs a default menu at launch; call `init_for_nsapp` from the first vizia event (after `resumed`) to replace it. The first submenu must be the App menu (muda's predefined About/Settings/Hide/Quit items). A matching ⌘S fires the menu item and vizia doesn't see the key, so no double fire, but on macOS the menu becomes the only path for those shortcuts. The shortcut table must therefore register them as accelerators there.
- **Plugin builds.** A baseview plugin has no menu bar. Native menus are standalone-only, and every command also needs an in-app home (context menus, the shortcut table). That matches VISION.md.

**Recommendation:** muda, Windows without an accelerator hook, macOS with menu accelerators, both driven from one shortcut table. Effort M.

## 2. Cursor position during an OS file drag

**Answer: not with the stock stack. Two small fixes both worked in probes.** Probes: [`probes/dnd_winit/`](probes/dnd_winit/), [`probes/dnd_vizia/`](probes/dnd_vizia/); a real OLE drag driven by [`tools/oledrag.ps1`](tools/oledrag.ps1).

- **Events from winit 0.30 (Windows), in order:** `HoveredFile` once on enter, then `DroppedFile` or `HoveredFileCancelled`. **No `CursorMoved` arrives during the hover**; the first one comes after the drop (logs `probes/logs/dnd_winit_*.txt`). winit's `DragOver` ignores the point, as do `DragEnter` and `Drop` (`platform_impl/windows/drop_handler.rs:79-155`).
- **macOS (source):** winit implements no `draggingUpdated:` (`window_delegate.rs:367-430`), so no positions there either.
- **Our fork** discards `HoveredFile` / `HoveredFileCancelled` (`vizia_winit/src/application.rs:636-637`). At the drop, `cx.mouse()` reads (−1, −1). A view's drop handler still reaches the pane under the cursor through the mouse move that follows the drop. So **dropping onto a lane works today; hover feedback (ghost tracks, lane highlight) doesn't.**
- **Fix A (polling):** between enter and drop/cancel, `GetCursorPos` every 16 ms tracked every move; the UI thread isn't blocked during a drag from another process.
- **Fix B (own `IDropTarget`):** about 120 lines of windows-sys COM revoke winit's drop target and register ours. `DragOver` gives screen coordinates, converted to client px and emitted through `ContextProxy`. Live positions, leave and drop all worked.
- **Side finding:** an Esc that cancels the drag is also delivered to the window as a key press afterwards, and would reach our Esc handler (close piano roll / clear selection).

**Recommendation:** a small fork change. Forward hover enter/leave as vizia events, and while hovering, poll the cursor once per frame: Windows `GetCursorPos`; macOS `mouseLocationOutsideOfEventStream` (verify on the Mac). Use the own drop target (B) only if we want a "no-drop" cursor for unsupported file types, because winit accepts every file. Risk: low for A (read-only polling, no COM); medium for B (replaces winit's COM object; must be re-done if winit changes). Effort S–M. This unblocks VISION.md's live MIDI preview (F-47).

## 3. A separate Settings window

**Answer: the fork can open one, but with two bugs, and baseview can't. Recommendation: an in-app modal panel.** Probe: [`probes/settings_window/`](probes/settings_window/).

- **Works:** `Window::new` from a button opened a second OS window; both rendered at 60 fps; a slider and checkbox drove the shared root model and the main window's labels updated; `on_close` closed it cleanly.
- **Bug 1:** the second window's `.title()` renamed the main window too: `SetTitle` isn't consumed (`vizia_winit/src/window.rs:488-496`) and bubbles to the root window. A one-line `meta.consume()` would fix it (`probes/logs/settings_title_bug.png`).
- **Bug 2:** keyboard focus is global. Right after the window opened, keys typed into Settings went to the main window's focused control, so Space toggled playback until a settings control took focus.
- **Minor:** one first click into the new window was lost.
- **baseview:** no `Window` view or multi-window path; the fork's multiwindow example says it isn't supported there.

**Recommendation:** Settings and Project Settings as **in-app modal panels** (backdrop, Esc closes, focus trapped). They work identically in the app and a future plugin; the cost is that they can't move to another monitor. Revisit a real window only if the fork's bugs are fixed upstream and the plugin case has a fallback. Effort M (panel framework shared with the export dialog, F-26).

---

## 4. The model change: 8 fixed hardware lanes → unlimited instrument tracks + hardware profiles

Read-only analysis. No code was written. VISION.md, "The core model", is the target.

### 4.1 Where "8 fixed lanes" lives today

`chip::driver::LANES = 8` and the `LANE_P1 … LANE_SAW` constants (`driver.rs:15-23`) define a lane
as **one hardware channel**. Counting uses of `LANES`, fixed `[_; 8]` arrays, lane constants and
`lanes.len()` across the non-test code: `chip/driver.rs` 57, `ui/lib.rs` 16, `ui/instruments.rs`
13, `ui/file.rs` 13, `ui/views/arrangement.rs` 8, `engine/shared.rs` 8, `engine/song.rs` 7,
`chip/presets.rs` 6, `ui/state.rs` 5, and 1–3 in each of about ten more files.

| Module | What assumes 8 hardware lanes | What changes | What is reused unchanged |
|---|---|---|---|
| **chip::driver** (`driver.rs`) | `Song.lanes: [&[Note]; 8]`, `lane_inst: [u8; 8]`; lane index = channel; one `MacroState` and one sounding note per lane; writes registers into one `Synth` | Split into a **console driver** (locked tracks → assigned channels of one console; today's code, with a track→channel map instead of a fixed index) and a **voice driver** (studio tracks: N voices per track) | Register-write logic per oscillator (`Osc`), "write only on change" rules, pitch tables, `sample_for_pitch`, frame-tick timing, `Note` (gains a track id) |
| **chip::instrument** | none (per-voice state already) | none | **All of it**: macros, loop/release, vibrato, kits. One `MacroState` per voice instead of per lane |
| **chip::synth / mixer / oscillators** | one 2A03 + one VRC6 with the real non-linear mixer; `take_peaks() -> [f32; 8]` | Console path unchanged. Studio path: oscillators (`pulse.rs`, `triangle.rs`, `noise.rs`, `dmc.rs`, `vrc6.rs` are already separate modules) driven per voice, each writing band-limited steps into its **track's** BLEP buffer, mixed linearly | Oscillators, `blep.rs`, `filter.rs` (per track or on the master), `frame_counter.rs` |
| **engine** (`song.rs`, `shared.rs`, `command.rs`, `core.rs`) | `SongSnapshot.lanes: [Vec<Note>; 8]`; `Shared.levels`/`note_ons: [AtomicU32; 8]`; `Command::Mute/Solo(u8 bitmask)` | `SongSnapshot.tracks: Vec<TrackSnapshot>` (kind, channel or voice budget, notes, instrument, params, mute/solo). Levels: a pre-sized array (e.g. 256) replaced from the UI thread when exceeded. Mute/solo move into the snapshot | The immutable-snapshot swap and the **garbage queue** (`core.rs:35, 116-146`: old snapshots go back to the UI thread to be freed), the rtrb command queue, the no-allocation guard, the stress harness |
| **ui::state** | `Arrangement.lanes` from `default_lanes()`; `Clip.lane: usize`; `mute/solo: u8`; `lane_sel` | `tracks: Vec<Track>` with stable ids; `Clip.track: TrackId`; per-track mute/solo/colour/name/kind; track selection becomes real (it doesn't exist today) | Clip and note model, `Rev` stamps, selection, snap, view/zoom, `resolve_overlaps` |
| **ui::notes** | `resolve_mono` runs on every lane | Mono only on locked tracks; studio tracks allow overlaps (chords) | Everything else (transpose, quantize, duplicate, nudge) |
| **ui::instruments** | lane defaults per index; `MAX_INSTRUMENTS = 32` because the engine bank index is `u8` (`instruments.rs:101`) | Default instrument per track; raise the cap (u16 index) | Instrument table, sharing by reference, Make Unique / Use Lane (→ Track) Instrument |
| **ui views** | `track_headers` builds rows `0..8` once (`arrangement.rs:1271`); `lane_dials(lane)`, `RowKind::for_lane`, browser `lane_kind(lane)` keyed by hardware index | Rows from a reactive list; dials and piano-roll rows keyed by the track's **channel kind**, not its index | Clip drawing, piano roll, dials, macro editor, browser drag (target = track) |
| **file** (`file.rs`, v3) | `lanes[]` (8, positional), `chip`, `mute`/`solo` masks | v4: `profile`, `tracks[]` (id, name, colour, kind, channel kind, lock, instrument, params, mute, solo, later effects and automation), clips by track id | The versioned loader, the migration framework and its tests (`tests/migration.rs`), atomic save, recovery |
| **history** (`history.rs`) | `Doc` = clips, bpm, params, instruments, samples | Add tracks (and later automation) to `Doc` | The snapshot-based undo as a whole (1000 steps / 64 MB) |
| **export / game_export** | "8 channel" stems; groups per lane; loop latching clones the one emulator | Stems per track; groups per track; the latch clones the console **and** the studio voice pool | The latch idea, `smpl`/JSON writing, loudness, the identity tests |
| **midi** (`midi.rs::arrange`) | maps parts onto the 8 channels, reduces chords to the top note | Default: **one studio track per part, as written**. The existing arranger becomes **Fit to hardware** | Parser, drum mapping, octave folding, the report |
| **presets** (`presets.rs`) | `kinds` bitmask = which hardware channels a preset fits | Presets gain "authentic for profile X" (locks the track) vs studio; layouts become templates | All 60 presets |

### 4.2 Two engine paths

**Locked tracks share one virtual console** (VISION.md, "Under the hood"). This is today's engine,
renamed: the console driver plays each locked track on its assigned channel, through the real
non-linear mixer, at the frame tick. It stays allocation-free as now, and the authenticity
promise is kept by construction, because it is the code that already passed the hardware tests
(ENGINE.md).

**Studio tracks get their own voices.** Each studio track owns a voice list; each voice is one
oscillator of the instrument's channel kind plus its `MacroState`. Voices write steps into a
per-track BLEP buffer; tracks then mix linearly (with their effects later) into the master, and
the console's output joins the master as one more source. Per-track chip instances cost about
0.2% CPU each (VISION.md); per-voice oscillators are cheaper still.

### 4.3 The voice pool (allocation-free audio thread)

- The engine owns a **pre-allocated pool** (start at e.g. 256 voices; each voice is a fixed-size
  struct, so the pool is one `Box<[Voice]>`).
- **Growing:** the UI thread watches the high-water mark (published through `Shared`, like the
  levels). Above 75% it builds a pool twice the size and sends it over the command queue. The audio
  thread copies the live voices into it at the next tick (a bounded memcpy) and pushes the old pool
  into the existing **garbage queue**, where the UI thread frees it. This is the same pattern as
  song snapshots today.
- **Exhaustion** before a grow arrives: steal the quietest released voice, else the quietest
  voice, with a 1–2 ms fade (VISION.md: "stolen gracefully").
- An instrument's optional **Voices** cap is enforced at note-on by the same allocator.
- The no-allocation guard (`engine::alloc_check`) and the stress matrix (ENGINE.md §7) prove it.
  Add a stress case: 64 studio tracks × 8-note chords.

### 4.4 Migration of v3 files

1. Each of the 8 v3 lanes becomes a **locked track** on the same channel, with the same name,
   colour, instrument, params, mute and solo. The profile is **2A03 + VRC6** if `chip` was
   `WithVrc6`, else **2A03**.
2. VRC6 lanes in a v3 file saved with the chip **off** were silent. They become locked VRC6 tracks,
   **muted**, and a load message says so (keeps the render exact and loses nothing).
3. All tracks locked and within budget means the song runs through the console path, so the
   **existing migration hashes** (`tests/migration.rs`, the v1 golden song, the v2 corpus) must
   still match bit for bit. That is the acceptance gate for batches M1 and M2.
4. v4 files are refused by v3 builds with the existing "saved by a newer ChipDAW" message.

### 4.5 Risks

| Risk | Mitigation |
|---|---|
| Hash drift while refactoring the driver | M1/M2 are behaviour-neutral, gated on the existing hash tests before any new feature |
| Audio-thread allocation with dynamic tracks | Pool + garbage queue (§4.3); the no-alloc guard aborts debug builds |
| vizia list performance with many track rows | Rows are custom-drawn (`TrackRow`), so a keyed list binding plus the existing damage tracking should hold; measure with 100 tracks against PERF.md's budgets |
| Undo memory (more state per `Doc`) | `Arc` clips already; keep tracks `Arc`'d too; the 64 MB cap stays |
| Stems on the console don't sum (non-linear) | Unchanged and documented (EXPORT.md §5); studio stems will sum exactly |
| Live loop vs export latch with many voices | The latch clones the pool; add the identity test with studio tracks |
| Scope creep (effects, automation) | Separate batches after the model lands; effects wait for the CEO's list (VISION.md, open question 2) |

### 4.6 Proposed order of work

| Batch | Content | Size | Gate |
|---|---|---|---|
| **M0** | Write the v4 schema, the track/profile types and the migration spec; get CEO sign-off on the migration rules (§4.4) | S | Spec reviewed |
| **M1** | UI model refactor: `lanes` → `tracks` with ids, still exactly 8 locked tracks; dials/rows/browser keyed by channel kind; file v4 + v3 migration | M | All 153 tests; migration hashes identical; scenario scripts pass |
| **M2** | Engine snapshot → `Vec<TrackSnapshot>`; console driver with a track→channel map | M | Hashes identical; stress matrix 0 dropouts |
| **M3** | Studio tracks: voice driver + pool, polyphonic piano roll, add/delete/reorder/colour tracks, MIDI import as studio tracks | L | New stress case; PERF.md budgets with 32 tracks; no-alloc guard |
| **M4** | Hardware profiles: lock/unlock, channel budget display, profile warnings (Unlock · Fit to hardware · Cancel), authentic presets, Fit to hardware for MIDI | M | Acceptance tests in UX-DRAFT.md |
| **M5** | Automation lanes, clip envelopes, tempo automation, then effects | L | Own spec |

Total for M0–M4: roughly the size of the product pass plus the stabilize pass. M3 carries most of
the risk.
