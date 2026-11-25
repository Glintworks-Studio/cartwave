//! ChipDAW's user interface. A library crate so the same UI can later be hosted
//! inside a plugin window (baseview) as well as the standalone app.

pub mod file;
pub mod dpcm;
pub mod export;
pub mod history;
pub mod midi;
pub mod instruments;
pub mod user_presets;
pub mod input;
pub mod mock;
pub mod notes;
pub mod perf;
pub mod render;
pub mod songs;
pub mod state;
pub mod testing;
pub mod theme;
pub mod views;

use vizia::prelude::*;

use engine::{Command, Engine, EngineConfig, FilterMode, ParamId, Params};

use crate::file::{Project, Recovery};
use crate::history::{Doc, History};
use crate::mock::Rng;
use crate::notes::{Note, RowKind};
use crate::state::*;
use crate::views::{
    arrangement::{track_headers, Arrangement as ArrangementView},
    browser::{browser, DragGhost, PresetDrag},
    chrome::{
        ContextMenu, Key, KeyAction, MenuBackdrop, PillKind, SpectrumBox, SplitHandle, StatPill,
        TimeCapsule,
    },
    dial::Dial,
    instrument::InstrumentView,
    piano_roll::PianoRoll,
    overlay::StatsOverlay,
    ruler::Ruler,
};

/// All reactive application state. Signals are cheap `Copy` handles.
#[derive(Clone, Copy)]
pub struct Signals {
    pub arrangement: Signal<state::Arrangement>,
    pub transport: Signal<Transport>,
    pub meters: Signal<Meters>,
    /// Values of the dial slots shown for `lane_sel`.
    pub dials: Signal<[f32; MAX_DIALS]>,
    /// Whether each dial slot is live, and its label (see `state::dial_states`).
    pub dial_state: Signal<[state::DialState; MAX_DIALS]>,
    /// Lane whose hardware controls the device panel shows.
    pub lane_sel: Signal<usize>,
    /// Bit per lane.
    pub mute: Signal<u8>,
    pub solo: Signal<u8>,
    pub audio: Signal<AudioStats>,
    /// Shown in the transport bar when the audio device has a problem.
    pub audio_msg: Signal<String>,
    pub chip: Signal<SoundChip>,
    pub stats: Signal<FrameStats>,
    /// Incremented once per animation tick; views that animate bind to it.
    pub frame: Signal<u64>,
    pub stress: Signal<bool>,
    /// The piano roll: open clip, view and note selection.
    pub piano: Signal<PianoState>,
    /// The bottom panel shows the piano roll (else the device panel).
    pub piano_open: Signal<bool>,
    /// Arrangement lane height.
    pub lane_h: Signal<f32>,
    /// Lane viewport height while the piano roll is open (split handle).
    pub piano_pane_h: Signal<f32>,
    /// Vertical lane scroll (logical px) and its current maximum (0 when all
    /// lanes fit).
    pub lane_scroll: Signal<f32>,
    pub lane_scroll_max: Signal<f32>,
    /// Arrangement time selection (drag across empty lane space).
    pub time_sel: Signal<Option<TimeSel>>,
    /// The open context menu, if any.
    pub menu: Signal<Option<ContextMenuSpec>>,
    /// The project file path field.
    pub file_path: Signal<String>,
    /// Short status line (saved, errors); clears itself.
    pub status: Signal<String>,
    /// The last session crashed and left a recovery file.
    pub recovery_offer: Signal<bool>,
    /// (audio callback load %, output sample rate Hz) for the CPU / kHz pills.
    /// Refreshed at most once a second so an idle app stays idle.
    pub audio_load: Signal<(u32, u32)>,
    /// The output filter's name, for the FILTER pill.
    pub filter_label: Signal<&'static str>,
    /// The song loops at its loop end (the capsule's loop light).
    pub loop_on: Signal<bool>,
    /// The clock shows bars.beats.sixteenths (else m:ss.cc); remembered in settings.
    pub clock_bars: Signal<bool>,
    /// The bottom inspector (device panel) is open: a clip is selected.
    pub inspector: Signal<bool>,
    /// The browser sidebar is shown (Ctrl+Alt+B).
    pub browser_open: Signal<bool>,
    pub browser_query: Signal<String>,
    /// The preset highlighted in the browser.
    pub browser_sel: Signal<Option<usize>>,
    /// A preset being dragged out of the browser.
    pub drag_preset: Signal<Option<PresetDrag>>,
    /// The user's saved presets (browser indices after the factory presets).
    pub user_presets: Signal<Vec<user_presets::UserPreset>>,
}

/// Loop end sent to the engine while looping is off: far past any song.
pub const NO_LOOP_TICKS: f64 = 1.0e12;

pub struct AppState {
    pub s: Signals,
    rng: Rng,
    /// When the model was last advanced.
    last_tick: Option<Instant>,
    /// `frame_count` of the presented frame the model was last advanced after.
    last_frame_advanced: u64,
    /// The animation pump and whether it currently runs at the fast rate.
    pump: Timer,
    pump_fast: bool,
    /// Last user input or presented frame; keeps the pump fast for a moment.
    last_activity: Instant,
    /// Recent presented-frame intervals (ms) for the overlay's worst-case figure.
    recent: std::collections::VecDeque<(Instant, f32)>,
    last_frame_seen: u64,
    cpu: perf::CpuMeter,
    frame_log: perf::FrameLog,
    cap: FrameCap,
    /// The (cap, display rate) last applied to the backend and the pump.
    applied: Option<(FrameCap, Option<f64>)>,
    /// The real-time audio engine (`None` if `CHIPDAW_AUDIO=0`).
    engine: Option<Engine>,
    /// UI copy of every lane's hardware parameters.
    params: Params,
    song_key: u64,
    song_version: u64,
    filter: FilterMode,
    /// The user picked the filter (F7 or `CHIPDAW_FILTER`); it no longer
    /// follows the chip.
    filter_manual: bool,
    note_ons: [u32; 8],
    /// When each lane's last note-on becomes audible (lights wait for it).
    light_due: [Option<Instant>; 8],
    last_audio_stats: Option<Instant>,
    /// Stream generation last seen, and a temporary "now playing on" notice.
    audio_generation: u32,
    device_notice: Option<(Instant, String)>,
    tests: testing::Tests,
    /// Playhead clock source counts for the F3 overlay: (audio, local) frames
    /// in the current second, and when that second started.
    clock_counts: (u32, u32, Instant),
    /// Global undo (arrangement + piano roll + tempo + lane parameters).
    history: History,
    /// Mouse gestures in progress (a drag is one undo step).
    gesture_depth: u32,
    /// Notes copied in the piano roll (starts relative to the first note).
    clipboard: Vec<Note>,
    recovery: Option<Recovery>,
    last_autosave: Instant,
    /// The document last written to the recovery file.
    autosaved: Option<Doc>,
    status_until: Option<Instant>,
    /// Loop end to restore when "loop this clip" is switched off.
    loop_end_saved: f64,
    /// Song loop start to restore when "loop this clip" is switched off.
    loop_start_saved: f64,
    /// The engine is looping over the open clip.
    clip_looping: bool,
    /// When the CPU / kHz pills were last refreshed.
    last_load: Option<Instant>,
    /// A browser audition note to send once the song carrying the audition
    /// instrument is on its way: (lane, pitch).
    pending_audition: Option<(usize, u8)>,
    /// The lane the current audition plays on.
    audition_lane: Option<usize>,
}

pub enum AppEvent {
    /// Animation pump timer.
    Tick,
    TogglePlay,
    Stop,
    ToggleRecord,
    SetBpm(f32),
    SetPlayhead(f64),
    SetDial(usize, f32),
    CycleChip,
    CycleSnap,
    /// The capsule's loop light: loop the song at its loop end, or play on.
    ToggleLoop,
    /// Click on the clock: m:ss.cc <-> bars.beats.sixteenths (remembered).
    ToggleClockFormat,
    /// The split handle above the piano roll: lane viewport height.
    SetPianoPaneHeight(f32),
    /// Scroll the lanes vertically by this many logical px.
    ScrollLanes(f32),
    /// The song loop (ticks). The ruler's loop brace sends this while dragged.
    SetLoop(f64, f64),
    /// Open / close a context menu, or run one of its commands (also bound
    /// to the usual DAW shortcuts).
    OpenMenu(ContextMenuSpec),
    CloseMenu,
    Menu(MenuAction),
    ToggleStats,
    ToggleStress,
    CycleFrameCap,
    RenameSelected(String),
    ToggleMute(usize),
    ToggleSolo(usize),
    CycleFilter,
    /// Open a clip in the piano roll / close it.
    OpenPiano(u32),
    ClosePiano,
    CyclePianoGrid,
    TogglePianoFollow,
    ToggleLoopClip,
    /// Audition while stopped (`None` releases).
    Preview { lane: usize, note: Option<(u8, Option<u8>)> },
    /// A mouse gesture started / ended (one undo step per gesture).
    GestureBegin,
    GestureEnd,
    /// A discrete edit finished: record an undo step.
    Commit,
    Undo,
    Redo,
    NewProject,
    /// Save to the current path (Ctrl+S); asks for one if there is none.
    SaveProject,
    /// Save under a new name: native save dialog (Ctrl+Shift+S).
    SaveProjectAs,
    /// Native open dialog (Ctrl+O).
    OpenProject,
    SetFilePath(String),
    CopyNotes { cut: bool },
    PasteNotes,
    RecoverAccept,
    RecoverDiscard,
    /// Draw on an instrument macro: lane, macro (0 volume, 1 arpeggio,
    /// 2 pitch, 3 duty), step, value. Undo groups a whole drag.
    EditMacro { inst: instruments::InstId, which: usize, step: usize, value: i8 },
    /// Set the lane instrument's vibrato: 0 depth (cents), 1 period
    /// (frames), 2 delay (frames). Undo groups a whole drag.
    EditVibrato { inst: instruments::InstId, which: usize, value: u8 },
    /// Move (or remove: `None`) a macro's loop point (`release: false`) or
    /// release point (`true`). Undo groups a whole drag.
    EditMacroPoint { inst: instruments::InstId, which: usize, release: bool, at: Option<u8> },
    /// Native open dialog for a .mid file; it becomes a new project (Ctrl+I).
    ImportMidi,
    /// Render the song to a WAV file (Ctrl+Shift+R).
    ExportWav,
    /// Native open dialog for a WAV file, converted into a DPCM sample.
    ImportAudio,
    RemoveSample(usize),
    /// Hear a user DPCM sample (`None` releases it).
    PreviewSample(Option<usize>),
    /// Show / hide the browser sidebar.
    ToggleBrowser,
    /// Highlight a preset in the browser.
    BrowserSelect(Option<usize>),
    /// Hear a preset (`None` releases it).
    Audition(Option<usize>),
    /// Load a preset onto a lane (`None`: the selected lane, or the first that fits).
    AssignPreset { lane: Option<usize>, preset: usize },
    /// A preset dropped on a clip: it becomes the clip's own instrument.
    AssignPresetToClip { clip: u32, preset: usize },
    /// The selected clips get their own copy of the instrument they play.
    MakeInstrumentUnique,
    /// The selected clips go back to their lane's default instrument.
    UseLaneInstrument,
    /// Save the instrument the inspector shows as a user preset.
    SaveUserPreset,
    /// Load a whole channel layout (chip + every lane's instrument).
    ApplyLayout(usize),
}

impl Model for AppState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| match e {
            AppEvent::Tick => self.tick(cx),
            AppEvent::TogglePlay => {
                let t = self.s.transport.get_untracked();
                if t.playing {
                    self.send(Command::Stop);
                } else {
                    // Start from where the UI shows the playhead.
                    self.send(Command::Seek(t.playhead_ticks));
                    self.send(Command::Play);
                }
                self.s.transport.update(|t| t.playing = !t.playing);
            }
            AppEvent::Stop => {
                let was_playing = self.s.transport.with_untracked(|t| t.playing);
                self.send(Command::Stop);
                if !was_playing {
                    self.send(Command::Seek(0.0));
                }
                self.s.transport.update(|t| {
                    if !t.playing {
                        t.playhead_ticks = 0.0;
                    }
                    t.playing = false;
                });
            }
            AppEvent::ToggleRecord => self
                .s
                .transport
                .update(|t| t.record_armed = !t.record_armed),
            AppEvent::SetBpm(b) => {
                let b = b.clamp(Transport::MIN_BPM, Transport::MAX_BPM);
                self.send(Command::Bpm(b as f64));
                self.s.transport.update(|t| t.bpm = b);
            }
            AppEvent::SetPlayhead(p) => {
                let p = p.max(0.0);
                self.send(Command::Seek(p));
                self.s.transport.update(|t| t.playhead_ticks = p);
            }
            AppEvent::SetDial(i, v) => {
                let lane = self.s.lane_sel.get_untracked();
                // A locked dial is inert (the view ignores input too).
                if self.s.dial_state.with_untracked(|d| d.get(*i).is_some_and(|d| d.locked.is_none())) {
                    self.set_param(lane, *i, *v);
                }
            }
            AppEvent::CycleChip => {
                let next = match self.s.chip.get_untracked() {
                    SoundChip::Base2A03 => SoundChip::WithVrc6,
                    SoundChip::WithVrc6 => SoundChip::Base2A03,
                };
                self.set_chip(next);
            }
            AppEvent::EditMacro { inst, which, step, value } => {
                self.s.arrangement.update(|a| {
                    if let Some(i) = a.instruments.get_mut(*inst) {
                        let q = match which {
                            0 => &mut i.inst.volume,
                            1 => &mut i.inst.arp,
                            2 => &mut i.inst.pitch,
                            _ => &mut i.inst.duty,
                        };
                        if *step < q.len as usize && q.data[*step] != *value {
                            q.data[*step] = *value;
                            i.edited = true;
                        }
                    }
                });
            }
            AppEvent::EditMacroPoint { inst, which, release, at } => {
                self.s.arrangement.update(|a| {
                    if let Some(i) = a.instruments.get_mut(*inst) {
                        let q = match which {
                            0 => &mut i.inst.volume,
                            1 => &mut i.inst.arp,
                            2 => &mut i.inst.pitch,
                            _ => &mut i.inst.duty,
                        };
                        let v = at.filter(|s| *s < q.len).unwrap_or(chip::instrument::NONE);
                        let slot = if *release { &mut q.release_at } else { &mut q.loop_at };
                        if *slot != v {
                            *slot = v;
                            i.edited = true;
                        }
                    }
                });
            }
            AppEvent::EditVibrato { inst, which, value } => {
                self.s.arrangement.update(|a| {
                    if let Some(i) = a.instruments.get_mut(*inst) {
                        let v = &mut i.inst.vibrato;
                        let f = match which {
                            0 => &mut v.depth,
                            1 => &mut v.period,
                            _ => &mut v.delay,
                        };
                        if *f != *value {
                            *f = *value;
                            i.edited = true;
                        }
                    }
                });
            }
            AppEvent::ToggleBrowser => self.s.browser_open.update(|o| *o = !*o),
            AppEvent::BrowserSelect(p) => self.s.browser_sel.set(*p),
            AppEvent::Audition(p) => self.audition(*p),
            AppEvent::AssignPreset { lane, preset } => self.assign_preset(*lane, *preset),
            AppEvent::AssignPresetToClip { clip, preset } => self.assign_preset_to_clip(*clip, *preset),
            AppEvent::MakeInstrumentUnique => self.make_instrument_unique(),
            AppEvent::UseLaneInstrument => self.use_lane_instrument(),
            AppEvent::SaveUserPreset => self.save_user_preset(),
            AppEvent::ApplyLayout(i) => self.apply_layout(*i),
            AppEvent::ToggleMute(i) => {
                self.s.mute.update(|m| *m ^= 1 << *i);
                let m = self.s.mute.get_untracked();
                self.send(Command::Mute(m));
            }
            AppEvent::ToggleSolo(i) => {
                self.s.solo.update(|m| *m ^= 1 << *i);
                let m = self.s.solo.get_untracked();
                self.send(Command::Solo(m));
            }
            AppEvent::CycleFilter => {
                let all = FilterMode::ALL;
                let i = all.iter().position(|m| *m == self.filter).unwrap_or(0);
                self.filter = all[(i + 1) % all.len()];
                self.filter_manual = true;
                self.send(Command::Filter(self.filter));
                self.s.filter_label.set(self.filter.label());
                self.last_audio_stats = None;
            }
            AppEvent::CycleSnap => self.s.arrangement.update(|a| a.snap = a.snap.next()),
            AppEvent::SetPianoPaneHeight(h) => {
                self.s.piano_pane_h.set_if_changed(*h);
                self.clamp_lane_scroll();
            }
            AppEvent::SetLoop(start, end) => self.set_song_loop(*start, *end, false),
            AppEvent::OpenMenu(m) => self.s.menu.set(Some(m.clone())),
            AppEvent::CloseMenu => self.s.menu.set(None),
            AppEvent::Menu(action) => {
                self.s.menu.set(None);
                self.run_menu_action(*action);
            }
            AppEvent::ScrollLanes(dy) => {
                let v = self.s.lane_scroll.get_untracked() + dy;
                self.s.lane_scroll.set_if_changed(v);
                self.clamp_lane_scroll();
            }
            AppEvent::ToggleClockFormat => {
                let bars = !self.s.clock_bars.get_untracked();
                self.s.clock_bars.set(bars);
                file::save_settings(&file::Settings { clock_bars: bars });
            }
            AppEvent::ToggleLoop => {
                self.s.loop_on.update(|l| *l = !*l);
                let end = self.s.transport.with_untracked(|t| t.loop_end_ticks);
                self.send(Command::LoopEnd(self.engine_loop_end(end)));
            }
            AppEvent::ToggleStats => self.s.stats.update(|s| s.visible = !s.visible),
            AppEvent::CycleFrameCap => {
                self.cap = self.cap.next();
                self.apply_frame_cap(cx);
            }
            AppEvent::ToggleStress => {
                let on = !self.s.stress.get();
                self.s.stress.set(on);
                self.s.arrangement.set(if on {
                    mock::stress_arrangement()
                } else {
                    mock::demo_arrangement()
                });
                // The stress set starts playing, in the engine too: setting
                // only the UI transport showed "playing" while the engine
                // stayed stopped (silent, meters idle, playhead on the local
                // clock; ENGINE-macos.md, bug 3).
                if on && !self.s.transport.with_untracked(|t| t.playing) {
                    let at = self.s.transport.with_untracked(|t| t.playhead_ticks);
                    self.send(Command::Seek(at));
                    self.send(Command::Play);
                    self.s.transport.update(|t| t.playing = true);
                }
            }
            AppEvent::RenameSelected(name) => {
                self.s.arrangement.update(|a| {
                    if let Some(id) = a.selection.first().copied() {
                        if let Some(c) = a.clip_mut(id) {
                            c.name = name.clone();
                        }
                    }
                });
                self.commit();
            }
            AppEvent::OpenPiano(id) => self.open_piano(*id),
            AppEvent::ClosePiano => self.close_piano(),
            AppEvent::CyclePianoGrid => self.s.piano.update(|p| p.grid = p.grid.next()),
            AppEvent::TogglePianoFollow => self.s.piano.update(|p| p.follow = !p.follow),
            AppEvent::ToggleLoopClip => {
                self.s.piano.update(|p| p.loop_clip = !p.loop_clip);
                self.sync_loop();
            }
            AppEvent::Preview { lane, note } => {
                // No audition during playback (the engine ignores it too).
                let playing = self.s.transport.with_untracked(|t| t.playing);
                if !playing || note.is_none() {
                    self.send(Command::Preview { lane: *lane as u8, note: *note });
                }
            }
            AppEvent::GestureBegin => self.gesture_depth += 1,
            AppEvent::GestureEnd => {
                self.gesture_depth = self.gesture_depth.saturating_sub(1);
                self.commit();
            }
            AppEvent::Commit => self.commit(),
            AppEvent::Undo => self.undo(false),
            AppEvent::Redo => self.undo(true),
            AppEvent::NewProject => {
                self.load_project(Project::new_empty());
                self.s.file_path.set(String::new());
                self.set_status("New project".into());
            }
            AppEvent::SaveProject => self.save_project(),
            AppEvent::ImportMidi => {
                if let Some(path) = rfd::FileDialog::new().add_filter("MIDI file", &["mid", "midi"]).pick_file() {
                    self.import_midi(&path);
                }
            }
            AppEvent::ImportAudio => {
                if let Some(path) = rfd::FileDialog::new().add_filter("WAV audio", &["wav"]).pick_file() {
                    self.import_sample(&path);
                }
            }
            AppEvent::RemoveSample(i) => {
                let name = self.s.arrangement.with_untracked(|a| a.samples.get(*i).cloned().flatten().map(|s| s.name));
                self.s.arrangement.update(|a| {
                    if let Some(slot) = a.samples.get_mut(*i) {
                        *slot = None;
                    }
                });
                self.commit();
                if let Some(n) = name {
                    self.set_status(format!("Removed sample {n}"));
                }
            }
            AppEvent::PreviewSample(slot) => {
                let lane = chip::driver::LANE_DMC as u8;
                let note = slot.map(|i| ((36 + chip::samples::BUILTIN_COUNT + i) as u8, None));
                if !self.s.transport.with_untracked(|t| t.playing) || note.is_none() {
                    self.send(Command::Preview { lane, note });
                }
            }
            AppEvent::ExportWav => self.export_wav(),
            AppEvent::SaveProjectAs => {
                if let Some(path) = self.file_dialog().save_file() {
                    self.s.file_path.set(path.display().to_string());
                    self.save_project();
                }
            }
            AppEvent::OpenProject => {
                let Some(path) = self.file_dialog().pick_file() else { return };
                match file::load(&path) {
                    Ok(p) => {
                        self.load_project(p);
                        self.s.file_path.set(path.display().to_string());
                        self.set_status(format!("Opened {}", path.display()));
                    }
                    Err(e) => self.set_status(e),
                }
            }
            // Only on a real change: the field is bound to this signal, and setting
            // it unchanged re-fires the field's edit event (an endless loop).
            AppEvent::SetFilePath(p) => self.s.file_path.set_if_changed(p.clone()),
            AppEvent::CopyNotes { cut } => self.copy_notes(*cut),
            AppEvent::PasteNotes => self.paste_notes(),
            AppEvent::RecoverAccept => {
                self.s.recovery_offer.set(false);
                if let Some(path) = self.recovery.as_ref().map(|r| r.file()) {
                    match file::load(&path) {
                        Ok(p) => {
                            self.load_project(p);
                            self.set_status("Recovered the last session (unsaved: use Save)".into());
                        }
                        Err(e) => self.set_status(e),
                    }
                }
            }
            AppEvent::RecoverDiscard => self.s.recovery_offer.set(false),
        });

        event.map(|we, meta| {
            if matches!(
                we,
                WindowEvent::MouseMove(..)
                    | WindowEvent::MouseDown(_)
                    | WindowEvent::MouseScroll(..)
                    | WindowEvent::KeyDown(..)
            ) {
                self.last_activity = Instant::now();
                self.set_pump(cx, true);
            }
            if let WindowEvent::Drop(DropData::File(path)) = we {
                let path = path.clone();
                self.open_dropped(&path);
            }
            if let WindowEvent::WindowClose = we {
                // Clean quit: autosave once more and clear the session marker.
                if let Some(r) = self.recovery.as_ref() {
                    if !self.s.recovery_offer.get_untracked() {
                        r.end_session(&self.project());
                    }
                }
            }
            if let WindowEvent::KeyDown(code, _) = we {
                let mods = *cx.modifiers();
                if input::command(&mods) {
                    let ev = match code {
                        Code::KeyZ if mods.shift() => Some(AppEvent::Redo),
                        Code::KeyZ => Some(AppEvent::Undo),
                        Code::KeyY => Some(AppEvent::Redo),
                        Code::KeyS if mods.shift() => Some(AppEvent::SaveProjectAs),
                        Code::KeyR if mods.shift() => Some(AppEvent::ExportWav),
                        Code::KeyI => Some(AppEvent::ImportMidi),
                        Code::KeyS => Some(AppEvent::SaveProject),
                        Code::KeyO => Some(AppEvent::OpenProject),
                        Code::KeyN => Some(AppEvent::NewProject),
                        // Loop selection, duplicate, insert MIDI clip.
                        Code::KeyL => Some(AppEvent::Menu(MenuAction::LoopSelection)),
                        Code::KeyD => Some(AppEvent::Menu(MenuAction::Duplicate)),
                        Code::KeyM if mods.shift() => Some(AppEvent::Menu(MenuAction::InsertClip)),
                        // Ctrl+Alt+B shows / hides the browser.
                        Code::KeyB if mods.alt() => Some(AppEvent::ToggleBrowser),
                        _ => None,
                    };
                    if let Some(ev) = ev {
                        cx.emit(ev);
                        meta.consume();
                        return;
                    }
                }
                match code {
                    Code::Space => {
                        cx.emit(AppEvent::TogglePlay);
                        meta.consume();
                    }
                    Code::Escape if self.s.menu.with_untracked(|m| m.is_some()) => {
                        self.s.menu.set(None);
                    }
                    Code::Escape if self.s.piano_open.get_untracked() => cx.emit(AppEvent::ClosePiano),
                    Code::F3 => cx.emit(AppEvent::ToggleStats),
                    Code::F4 => cx.emit(AppEvent::ToggleStress),
                    Code::F5 => cx.emit(WindowEvent::ReloadStyles),
                    Code::F6 => cx.emit(AppEvent::CycleFrameCap),
                    Code::F7 => cx.emit(AppEvent::CycleFilter),
                    _ => {}
                }
            }
        });
    }
}

impl AppState {
    /// Advances the model at most once per presented frame.
    ///
    /// The pump timer fires faster than the display refreshes; with vsync the
    /// swap blocks until vblank, so presented frames arrive at the display
    /// rate. Stepping once per presented frame, with dt taken from the
    /// vsync-spaced present interval, gives evenly spaced motion without the
    /// drift (and periodic dropped/doubled frame) of a free-running clock.
    fn tick(&mut self, cx: &mut EventContext) {
        self.tests.maybe_stall();
        self.idle_housekeeping();
        // Re-applies the cap if the window moved to a display with another rate.
        self.apply_frame_cap(cx);
        self.sync_engine();
        let ft = cx.frame_timings();
        self.adapt_pump(cx, &ft);
        // Without presents for this long (idle, minimized), fall back to wall time.
        const IDLE: Duration = Duration::from_millis(50);

        let now = Instant::now();
        let wall_dt = self.last_tick.map(|t| now - t);
        let new_frame = ft.frame_count != self.last_frame_advanced;
        let idle = wall_dt.is_none_or(|d| d > IDLE);
        if !new_frame && !idle {
            self.record_frame_stats(&ft, self.s.transport.with_untracked(|t| t.playhead_ticks));
            return;
        }
        let dt = match (new_frame && !idle, ft.interval) {
            // A present interval spanning an idle period is not a frame interval.
            (true, Some(interval)) if interval <= IDLE => interval,
            _ => wall_dt.unwrap_or(Duration::from_secs_f32(1.0 / 60.0)),
        };
        self.advance(dt, &ft);
    }

    /// Steps the model by `dt` and requests the frame that shows it.
    fn advance(&mut self, dt: Duration, ft: &FrameTimings) {
        // Clamp dt so a stall (window drag, breakpoint) doesn't teleport things.
        const MAX_DT: f32 = 0.1;
        let dt = dt.as_secs_f32().min(MAX_DT);
        self.last_tick = Some(Instant::now());
        self.last_frame_advanced = ft.frame_count;

        let mut tr = self.s.transport.get_untracked();
        if tr.playing {
            // The audio clock is the master: show what will be heard when this
            // frame reaches the screen (the next vblank after the last
            // present), not what is heard while it is being prepared.
            let now = Instant::now();
            let show_at = match (ft.presented_at, ft.interval) {
                (Some(p), Some(i)) if i > Duration::ZERO && i <= Duration::from_millis(50) => {
                    let mut t = p + i;
                    while t < now {
                        t += i;
                    }
                    t
                }
                _ => now,
            };
            let heard = self
                .engine
                .as_ref()
                .filter(|e| e.shared().stream_running.load(std::sync::atomic::Ordering::Relaxed))
                .and_then(|e| e.audible_position_at(show_at));
            match heard {
                Some((t, true)) => {
                    self.clock_counts.0 += 1;
                    tr.playhead_ticks = t;
                }
                _ => {
                    self.clock_counts.1 += 1;
                    self.tests.playhead_local_frames += 1;
                    // No audio (or a command in flight): integrate locally.
                    let ticks_per_sec = tr.bpm as f64 / 60.0 * PPQ as f64;
                    tr.playhead_ticks += ticks_per_sec * dt as f64;
                    tr.playhead_ticks = engine::chip::player::wrap_loop(
                        tr.playhead_ticks,
                        tr.loop_start_ticks,
                        self.engine_loop_end(tr.loop_end_ticks),
                    );
                }
            }
            self.s.transport.set(tr);
            self.follow_playhead(tr.playhead_ticks);
        }
        self.tests
            .av_frame(ft.frame_count, ft.presented_at, tr.playhead_ticks);
        self.tests.ui_frames = ft.frame_count;

        let meters = self.s.meters;
        let mut m = meters.get_untracked();
        if let Some(e) = self.engine.as_ref() {
            let sh = e.shared();
            let levels = sh.take_levels();
            let now = Instant::now();
            let latency = Duration::from_nanos(
                sh.position().map_or(0, |p| p.latency_ns),
            );
            let mut fire = [false; 8];
            for lane in 0..8 {
                let n = sh.note_ons[lane].load(std::sync::atomic::Ordering::Relaxed);
                if n != self.note_ons[lane] {
                    self.note_ons[lane] = n;
                    self.light_due[lane] = Some(now + latency);
                }
                if self.light_due[lane].is_some_and(|d| now >= d) {
                    self.light_due[lane] = None;
                    fire[lane] = true;
                }
            }
            mock::step_meters_engine(&mut m, levels, fire, dt);
        } else {
            let rng = &mut self.rng;
            self.s.arrangement.with_untracked(|a| {
                mock::step_meters(&mut m, a, tr.playhead_ticks, tr.playing, dt, rng);
            });
        }
        // Only notify (and so redraw) when something actually moved.
        meters.set_if_changed(m);

        self.record_frame_stats(ft, tr.playhead_ticks);

        self.s.frame.update(|f| *f += 1);
    }
}

impl AppState {
    /// The lane a preset goes to: `want` if given (and it fits), else the
    /// selected lane if it fits, else the first lane that fits (VRC6 lanes
    /// only with the VRC6 on).
    /// A factory or user preset by browser index.
    fn preset(&self, p: usize) -> Option<user_presets::PresetView> {
        self.s.user_presets.with_untracked(|u| user_presets::view(p, u))
    }

    fn lane_for(&self, preset: usize, want: Option<usize>) -> Option<usize> {
        let p = self.preset(preset)?;
        let vrc6 = self.s.chip.get_untracked() == SoundChip::WithVrc6;
        let ok = |l: usize| p.fits(l) && (vrc6 || l < chip::driver::LANE_V1);
        if let Some(l) = want {
            return p.fits(l).then_some(l);
        }
        let sel = self.s.lane_sel.get_untracked();
        if sel < engine::LANES && ok(sel) {
            return Some(sel);
        }
        (0..engine::LANES).find(|&l| ok(l))
    }

    fn audition(&mut self, preset: Option<usize>) {
        if let Some(lane) = self.audition_lane.take() {
            self.send(Command::Preview { lane: lane as u8, note: None });
        }
        let Some(p) = preset else { return };
        if self.s.transport.with_untracked(|t| t.playing) {
            return;
        }
        let Some(lane) = self.lane_for(p, None) else { return };
        let Some(pr) = self.preset(p) else { return };
        self.s.arrangement.update(|a| a.audition = Some(pr.inst));
        let pitch = match lane {
            chip::driver::LANE_TRI => 48,
            chip::driver::LANE_NOISE => 56,
            _ => 60,
        };
        self.pending_audition = Some((lane, pitch));
        self.audition_lane = Some(lane);
    }

    /// Loads preset `p` on a lane, with its channel controls; one undo step.
    fn assign_preset(&mut self, lane: Option<usize>, p: usize) {
        let Some(pr) = self.preset(p) else { return };
        let Some(lane) = self.lane_for(p, lane) else {
            self.set_status(format!("{} can't play on that channel.", pr.name));
            return;
        };
        self.load_preset_on(lane, pr.inst, pr.params);
        self.commit();
        let name = state::default_lanes()[lane].name;
        self.set_status(format!("{} loaded on {name}", pr.name));
    }

    /// A preset dropped on a clip becomes that clip's own instrument (a new
    /// table entry). The lane's channel controls stay as they are: they
    /// belong to the lane, not the clip. One undo step.
    fn assign_preset_to_clip(&mut self, clip: u32, p: usize) {
        let Some(pr) = self.preset(p) else { return };
        let Some(lane) = self.s.arrangement.with_untracked(|a| a.clip(clip).map(|c| c.lane)) else { return };
        if self.lane_for(p, Some(lane)) != Some(lane) {
            self.set_status(format!("{} can't play on that channel.", pr.name));
            return;
        }
        let mut ok = false;
        self.s.arrangement.update(|a| {
            if let Some(id) = a.instruments.add(pr.inst.clone()) {
                if let Some(c) = a.clip_mut(clip) {
                    c.instrument = Some(id);
                }
                let clips = a.clips.clone();
                a.instruments.prune(&clips);
                ok = true;
            }
        });
        if !ok {
            return self.set_status(format!("The project has {} instruments, the most it can hold.", instruments::MAX_INSTRUMENTS));
        }
        self.commit();
        let name = self.s.arrangement.with_untracked(|a| a.clip(clip).map(|c| c.name.clone())).unwrap_or_default();
        self.set_status(format!("{} is now the instrument of clip \"{name}\"", pr.name));
    }

    /// "Make unique": the selected clips get one new copy of the instrument
    /// each plays, so editing it no longer changes the others.
    fn make_instrument_unique(&mut self) {
        let mut made = 0;
        let mut full = false;
        self.s.arrangement.update(|a| {
            let ids: Vec<u32> = a.selection.clone();
            let mut copies: Vec<(instruments::InstId, instruments::InstId)> = Vec::new();
            for id in ids {
                let Some(c) = a.clip(id) else { continue };
                let from = a.instruments.id_for(c);
                let to = match copies.iter().find(|x| x.0 == from) {
                    Some(x) => x.1,
                    None => {
                        let Some(mut inst) = a.instruments.get(from).cloned() else { continue };
                        inst.name = format!("{} copy", inst.name.trim_end_matches('*'));
                        let Some(new) = a.instruments.add(inst) else {
                            full = true;
                            break;
                        };
                        copies.push((from, new));
                        made += 1;
                        new
                    }
                };
                if let Some(c) = a.clip_mut(id) {
                    c.instrument = Some(to);
                }
            }
            let clips = a.clips.clone();
            a.instruments.prune(&clips);
        });
        if made > 0 {
            self.commit();
            self.set_status("The selected clips have their own instrument now.".into());
        } else if full {
            self.set_status(format!("The project has {} instruments, the most it can hold.", instruments::MAX_INSTRUMENTS));
        }
    }

    /// The selected clips go back to their lane's default instrument.
    fn use_lane_instrument(&mut self) {
        let mut changed = false;
        self.s.arrangement.update(|a| {
            for id in a.selection.clone() {
                if a.clip(id).is_some_and(|c| c.instrument.is_some()) {
                    if let Some(c) = a.clip_mut(id) {
                        c.instrument = None;
                        changed = true;
                    }
                }
            }
            let clips = a.clips.clone();
            a.instruments.prune(&clips);
        });
        if changed {
            self.commit();
        }
    }

    /// Saves the inspector's instrument as a user preset (app-data folder).
    fn save_user_preset(&mut self) {
        let lane = self.s.lane_sel.get_untracked();
        let Some(inst) = self.s.arrangement.with_untracked(|a| a.instruments.get(a.current_instrument(lane)).cloned()) else {
            return;
        };
        let msg = match crate::user_presets::save(&inst, lane) {
            Ok(name) => {
                self.s.user_presets.set(crate::user_presets::load());
                format!("Saved \"{name}\" to your presets.")
            }
            Err(e) => e,
        };
        self.set_status(msg);
    }

    fn load_preset_on(&mut self, lane: usize, inst: instruments::LaneInstrument, params: &[(ParamId, i8)]) {
        self.s.arrangement.update(|a| {
            a.instruments.set_lane(lane, inst);
            let clips = a.clips.clone();
            a.instruments.prune(&clips);
        });
        for &(id, v) in params {
            self.params.set(lane, id, v as i32);
            let value = self.params.get(lane, id) as i8;
            self.send(Command::Param { lane: lane as u8, id, value });
        }
        if lane == self.s.lane_sel.get_untracked() {
            self.s.dials.set(self.dial_values(lane));
        }
    }

    fn apply_layout(&mut self, i: usize) {
        let Some(l) = chip::presets::LAYOUTS.get(i) else { return };
        let want = if l.vrc6 { SoundChip::WithVrc6 } else { SoundChip::Base2A03 };
        if self.s.chip.get_untracked() != want {
            self.set_chip(want);
        }
        for (lane, name) in l.lanes.iter().enumerate() {
            if let Some(pr) = chip::presets::find(name) {
                self.load_preset_on(lane, instruments::LaneInstrument::from_preset(pr), pr.params);
            }
        }
        self.commit();
        self.set_status(format!("Layout: {}", l.name));
    }

    fn set_chip(&mut self, chip: SoundChip) {
        self.s.chip.set(chip);
        let on = chip == SoundChip::WithVrc6;
        self.send(Command::Vrc6(on));
        if !self.filter_manual {
            self.filter = FilterMode::default_for(on);
            self.send(Command::Filter(self.filter));
            self.s.filter_label.set(self.filter.label());
            self.last_audio_stats = None;
        }
    }

    /// The loop end the engine should use: the song's, or none while the
    /// loop light is off (a clip loop in the piano roll always loops).
    fn engine_loop_end(&self, end: f64) -> f64 {
        if self.s.loop_on.get_untracked() || self.clip_looping {
            end
        } else {
            NO_LOOP_TICKS
        }
    }

    fn send(&mut self, c: Command) {
        if let Some(e) = self.engine.as_mut() {
            e.send(c);
        }
    }

    /// Dial slot `slot` of `lane` changed: store, show, send to the engine.
    fn set_param(&mut self, lane: usize, slot: usize, v: f32) {
        let dials = lane_dials(lane);
        let Some(&(pid, _)) = dials.get(slot) else { return };
        self.params.set(lane, pid, v.round() as i32);
        let stored = self.params.get(lane, pid);
        self.send(Command::Param {
            lane: lane as u8,
            id: pid,
            value: stored as i8,
        });
        if lane == self.s.lane_sel.get_untracked() {
            self.s.dials.update(|d| d[slot] = v);
        }
        self.sync_piano_lane_volume();
    }

    fn dial_values(&self, lane: usize) -> [f32; MAX_DIALS] {
        let mut d = [0f32; MAX_DIALS];
        for (slot, (pid, _)) in lane_dials(lane).into_iter().enumerate() {
            d[slot] = self.params.get(lane, pid) as f32;
        }
        d
    }

    /// Once per pump tick: keep the engine's song in step with the
    /// arrangement, free old snapshots, follow the selected lane, update the
    /// audio stats, and run the test hooks.
    fn sync_engine(&mut self) {
        // Selected lane follows the first selected clip.
        if let Some(l) = self
            .s
            .arrangement
            .with_untracked(|a| a.first_selected().map(|c| c.lane))
        {
            if l != self.s.lane_sel.get_untracked() {
                self.s.dials.set(self.dial_values(l));
                self.s.lane_sel.set(l);
            }
        }
        let any = self.s.arrangement.with_untracked(|a| !a.selection.is_empty());
        self.s.inspector.set_if_changed(any);
        // Which dials an instrument overrides (cheap; set only on change).
        let lane = self.s.lane_sel.get_untracked();
        let st = self.s.arrangement.with_untracked(|a| state::dial_states(lane, &self.params, a.instruments.get(a.current_instrument(lane))));
        self.s.dial_state.set_if_changed(st);
        // Test B: scripted edits through the same model operations.
        if self.tests.auto_edit {
            let mut dial = None;
            self.s
                .arrangement
                .update(|a| dial = self.tests.auto_edit(a));
            if let Some((lane, pid, v)) = dial {
                if let Some(slot) = lane_dials(lane).iter().position(|(p, _)| *p == pid) {
                    self.set_param(lane, slot, v as f32);
                }
            }
            if let Some((mute, lane)) = self.tests.auto_mute() {
                let sig = if mute { self.s.mute } else { self.s.solo };
                sig.update(|m| *m ^= 1 << lane);
                // Never leave a solo on for long: it would silence the test.
                if !mute && self.s.solo.get_untracked().count_ones() > 1 {
                    self.s.solo.set(0);
                }
                let (m, so) = (self.s.mute.get_untracked(), self.s.solo.get_untracked());
                self.send(Command::Mute(m));
                self.send(Command::Solo(so));
            }
        }
        let Some(e) = self.engine.as_mut() else { return };
        e.collect_garbage();
        let key = self.s.arrangement.with_untracked(|a| a.content_key());
        if key != self.song_key {
            self.song_key = key;
            self.song_version += 1;
            let v = self.song_version;
            let snap = self.s.arrangement.with_untracked(|a| a.song_snapshot(v));
            e.send_song(snap);
        }
        // After the song that carries the audition instrument.
        if let Some((lane, pitch)) = self.pending_audition.take() {
            e.send(Command::Preview { lane: lane as u8, note: Some((pitch, None)) });
        }
        // Status: a few times a second is plenty.
        let now = Instant::now();
        if self
            .last_audio_stats
            .is_none_or(|t| now - t > Duration::from_millis(250))
        {
            self.last_audio_stats = Some(now);
            use std::sync::atomic::Ordering::Relaxed;
            let sh = e.shared();
            let (_, mean, p99, max) = sh.callback_cost();
            // CPU pill: mean callback time, % of the buffer period.
            if self
                .last_load
                .is_none_or(|t| now - t >= Duration::from_secs(1))
            {
                self.last_load = Some(now);
                // At least 1% once audio runs, so the pill never reads "0%".
                let pct = (mean.round() as u32).clamp(1, 999);
                self.s.audio_load.set_if_changed((pct, e.sample_rate()));
            }
            let running = sh.stream_running.load(Relaxed);
            let stats = AudioStats {
                device: e.device_info(),
                running,
                filter: self.filter.label(),
                filter_auto: !self.filter_manual,
                frames_per_cb: sh.cb_frames_last.load(Relaxed),
                latency_ms: sh.position().map_or(0.0, |p| p.latency_ns as f32 / 1e6),
                cb_mean: mean as f32,
                cb_p99: p99 as f32,
                cb_max: max as f32,
                underruns: sh.underruns(),
                overruns: sh.xrun_overrun.load(Relaxed),
                edits: sh.edits_applied.load(Relaxed),
                edit_tick_ms: sh.edit_to_tick_max_ns.load(Relaxed) as f32 / 1e6,
            };
            // After a device change, say where the sound went, for a few
            // seconds (ENGINE-macos.md, test H).
            let generation = sh.generation.load(Relaxed);
            if running && generation > self.audio_generation {
                if self.audio_generation > 0 {
                    let dev = e.device_info();
                    let name = dev.split(" · ").next().unwrap_or("?").to_string();
                    self.device_notice = Some((now + Duration::from_secs(5), format!("AUDIO: now playing on {name}")));
                }
                self.audio_generation = generation;
            }
            let notice = self.device_notice.as_ref().filter(|(until, _)| now < *until);
            let msg = match sh.device_error.load(Relaxed) {
                _ if running && notice.is_some() => notice.map(|(_, m)| m.clone()).unwrap_or_default(),
                _ if running => String::new(),
                engine::device_error::NO_DEVICE => "AUDIO: no output device".into(),
                engine::device_error::REBUILDING => "AUDIO: device changed, reconnecting".into(),
                engine::device_error::BUILD_FAILED => "AUDIO: can't open output".into(),
                _ => "AUDIO: starting".into(),
            };
            // The stats change on every refresh and only the F3 overlay shows
            // them: publishing them while it is hidden would draw a frame every
            // 250 ms and keep the pump fast, so the app would never go idle.
            if self.s.stats.with_untracked(|s| s.visible) {
                self.s.audio.set_if_changed(stats);
            }
            self.s.audio_msg.set_if_changed(msg);
        }
        if let Some(e) = self.engine.as_ref() {
            self.tests.periodic(e);
        }
    }

    /// Passes the frame-rate cap to the backend. The pump stays at 4 ms for
    /// every cap: neither a shorter pump at 320 Hz nor a half-frame pump at
    /// 60 fps lowered CPU (PERF-macos.md §7).
    fn apply_frame_cap(&mut self, cx: &mut EventContext) {
        let display_hz = cx.display_refresh_rate();
        if self.applied == Some((self.cap, display_hz)) {
            return;
        }
        self.applied = Some((self.cap, display_hz));
        self.tests.display_hz = display_hz;
        cx.set_max_frame_rate(self.cap.max_fps());
        let cap = self.cap;
        self.s.stats.update(|s| {
            s.cap = cap;
            s.display_hz = display_hz;
        });
    }

    /// Runs the pump fast only while something animates; idle wakes it rarely.
    fn adapt_pump(&mut self, cx: &mut EventContext, ft: &FrameTimings) {
        const STAY_FAST: Duration = Duration::from_millis(250);
        let now = Instant::now();
        if ft.presented_at.is_some_and(|t| now - t < STAY_FAST) {
            self.last_activity = self.last_activity.max(ft.presented_at.unwrap_or(now));
        }
        let playing = self.s.transport.with_untracked(|t| t.playing);
        let fast = playing || self.tests.auto_edit || now - self.last_activity < STAY_FAST;
        self.set_pump(cx, fast);
    }

    fn set_pump(&mut self, cx: &mut EventContext, fast: bool) {
        if fast != self.pump_fast {
            self.pump_fast = fast;
            let interval = if fast { ANIMATION_PUMP } else { IDLE_PUMP };
            cx.modify_timer(self.pump, |t| {
                t.set_interval(interval);
            });
        }
    }

    /// Folds the backend's timings for newly presented frames into the F3
    /// stats and the optional frame log.
    fn record_frame_stats(&mut self, ft: &FrameTimings, playhead: f64) {
        self.cpu.update();
        if ft.frame_count == self.last_frame_seen {
            return;
        }
        self.last_frame_seen = ft.frame_count;
        let spans = perf::take_spans();
        let custom: Duration = spans.iter().sum();
        self.frame_log.log(ft, custom, playhead);

        let now = Instant::now();
        let interval_ms = ft.interval.map(|d| d.as_secs_f32() * 1000.0).unwrap_or(0.0);
        self.recent.push_back((now, interval_ms));
        while self
            .recent
            .front()
            .is_some_and(|(t, _)| now - *t > Duration::from_secs(1))
        {
            self.recent.pop_front();
        }
        let clock = if now - self.clock_counts.2 >= Duration::from_secs(1) {
            let c = (self.clock_counts.0, self.clock_counts.1);
            self.clock_counts = (0, 0, now);
            Some(c)
        } else {
            None
        };
        if !self.s.stats.with_untracked(|s| s.visible) {
            return;
        }
        let worst = self.recent.iter().map(|r| r.1).fold(0.0, f32::max);
        let phases = [
            ft.events,
            ft.style,
            ft.layout,
            ft.make_current,
            ft.draw,
            ft.flush,
            ft.present,
        ];
        let count = self.s.arrangement.with_untracked(|a| a.clips.len());
        let cpu = self.cpu.percent;
        let smooth = |old: f32, new: f32| {
            if old == 0.0 {
                new
            } else {
                old * 0.92 + new * 0.08
            }
        };
        self.s.stats.update(|s| {
            s.frame_ms = smooth(s.frame_ms, interval_ms);
            s.worst_ms = worst;
            for (o, d) in s.phases_ms.iter_mut().zip(phases) {
                *o = smooth(*o, d.as_secs_f32() * 1000.0);
            }
            for (o, d) in s.spans_ms.iter_mut().zip(spans) {
                *o = smooth(*o, d.as_secs_f32() * 1000.0);
            }
            s.cpu_percent = cpu;
            s.clip_count = count;
            s.frames = ft.frame_count;
            if let Some((audio, local)) = clock {
                s.clock_audio = audio;
                s.clock_local = local;
            }
        });
    }
}

/// Animation pump interval. Must be shorter than a frame; the model still
/// advances only once per presented frame (see `AppState::tick`).
const ANIMATION_PUMP: Duration = Duration::from_millis(4);
/// Pump interval while nothing animates (meter decay etc. still settles).
const IDLE_PUMP: Duration = Duration::from_millis(50);

/// Registers the bundled fonts with vizia's text system (for stylesheet text).
pub fn register_fonts(cx: &mut Context) {
    use views::gfx::*;
    for f in [
        INTER_REGULAR,
        INTER_MEDIUM,
        INTER_SEMIBOLD,
        INTER_BOLD,
        MONO_REGULAR,
        MONO_MEDIUM,
    ] {
        cx.add_font_mem(f);
    }
}

/// Adds the main stylesheet. Debug builds read it from disk so F5 hot-reloads it.
pub fn add_styles(cx: &mut Context) {
    #[cfg(debug_assertions)]
    {
        let path =
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/theme/style.css"));
        cx.add_stylesheet(path).expect("stylesheet");
    }
    #[cfg(not(debug_assertions))]
    cx.add_stylesheet(include_style!("src/theme/style.css"))
        .expect("stylesheet");
}

/// Builds the whole UI into the current context. Works for any window host.
pub fn build_ui(cx: &mut Context) {
    register_fonts(cx);
    add_styles(cx);

    // The default chip; the default output filter follows it.
    let chip = SoundChip::WithVrc6;
    let s = Signals {
        // Opens with nothing selected: the full-height arrangement of the prototype.
        arrangement: Signal::new(state::Arrangement {
            selection: Vec::new(),
            ..mock::demo_arrangement()
        }),
        transport: Signal::new(Transport {
            playing: false,
            record_armed: false,
            bpm: Transport::DEFAULT_BPM,
            playhead_ticks: 0.0,
            loop_end_ticks: (32 * TICKS_PER_BAR) as f64,
            loop_start_ticks: 0.0,
        }),
        meters: Signal::new(Meters::default()),
        dials: Signal::new([0.0; MAX_DIALS]),
        dial_state: Signal::new([state::DialState::default(); MAX_DIALS]),
        lane_sel: Signal::new(usize::MAX),
        mute: Signal::new(0),
        solo: Signal::new(0),
        audio: Signal::new(AudioStats::default()),
        audio_msg: Signal::new(String::new()),
        chip: Signal::new(chip),
        stats: Signal::new(FrameStats::default()),
        frame: Signal::new(0),
        stress: Signal::new(false),
        piano: Signal::new(PianoState::default()),
        piano_open: Signal::new(false),
        lane_h: Signal::new(LANE_H_NORMAL),
        piano_pane_h: Signal::new(PIANO_PANE_DEFAULT),
        lane_scroll: Signal::new(0.0),
        lane_scroll_max: Signal::new(0.0),
        time_sel: Signal::new(None),
        menu: Signal::new(None),
        file_path: Signal::new(String::new()),
        status: Signal::new(String::new()),
        recovery_offer: Signal::new(false),
        audio_load: Signal::new((0, 0)),
        filter_label: Signal::new(""),
        loop_on: Signal::new(true),
        clock_bars: Signal::new(file::load_settings().clock_bars),
        inspector: Signal::new(false),
        browser_open: Signal::new(true),
        browser_query: Signal::new(String::new()),
        browser_sel: Signal::new(None),
        drag_preset: Signal::new(None),
        user_presets: Signal::new(user_presets::load()),
    };

    // Animation pump; frames themselves are paced by vsync (see PERF.md).
    let timer = cx.add_timer(ANIMATION_PUMP, None, |cx, action| {
        if let TimerAction::Tick(_) = action {
            cx.emit(AppEvent::Tick);
        }
    });
    cx.start_timer(timer);

    let tests = testing::Tests::from_env();
    let mut state = AppState {
        s,
        rng: Rng::new(7),
        last_tick: None,
        last_frame_advanced: 0,
        pump: timer,
        pump_fast: true,
        last_activity: Instant::now(),
        recent: std::collections::VecDeque::with_capacity(128),
        last_frame_seen: 0,
        cpu: perf::CpuMeter::default(),
        frame_log: perf::FrameLog::from_env(),
        cap: FrameCap::from_env(),
        applied: None,
        engine: None,
        params: instruments::default_params(),
        song_key: 0,
        song_version: 0,
        filter: tests
            .filter
            .unwrap_or(FilterMode::default_for(chip == SoundChip::WithVrc6)),
        filter_manual: tests.filter.is_some(),
        note_ons: [0; 8],
        light_due: [None; 8],
        last_audio_stats: None,
        audio_generation: 0,
        device_notice: None,
        tests,
        clock_counts: (0, 0, Instant::now()),
        history: History::new(Doc::of(&s.arrangement.get_untracked(), 150.0, instruments::default_params())),
        gesture_depth: 0,
        clipboard: Vec::new(),
        recovery: Recovery::new(),
        last_autosave: Instant::now(),
        autosaved: None,
        status_until: None,
        loop_end_saved: (32 * TICKS_PER_BAR) as f64,
        loop_start_saved: 0.0,
        clip_looping: false,
        last_load: None,
        pending_audition: None,
        audition_lane: None,
    };
    if state.recovery.as_ref().is_some_and(|r| r.start_session()) {
        state.s.recovery_offer.set(true);
    }
    state.s.filter_label.set(state.filter.label());
    state.start_engine();
    state.apply_test_project();
    state.build(cx);

    VStack::new(cx, |cx| {
        top_bar(cx, s);
        HStack::new(cx, |cx| {
            browser(cx, s);
            Element::new(cx)
                .class("col-divider")
                .display(s.browser_open.map(|o| if *o { Display::Flex } else { Display::None }));
            track_headers(cx, s);
            Element::new(cx).class("col-divider");
            VStack::new(cx, |cx| {
                Ruler::new(cx, s).height(Pixels(theme::tokens::RULER_HEIGHT));
                ArrangementView::new(cx, s);
            })
            .width(Stretch(1.0))
            .overflow(Overflow::Hidden);
        })
        .class("main")
        .height(Memo::new(move |_| {
            if s.piano_open.get() {
                // Piano roll open: the arrangement gets the height the split
                // handle sets (its lanes scroll) and the piano roll the rest.
                Pixels(theme::tokens::RULER_HEIGHT + s.piano_pane_h.get())
            } else {
                Stretch(1.0)
            }
        }))
        .overflow(Overflow::Hidden);
        // Bottom panel: the piano roll while a clip is open, else the device
        // inspector while a clip is selected, else nothing (the prototype's
        // full-height arrangement).
        Binding::new(cx, s.piano_open, move |cx| {
            if s.piano_open.get() {
                SplitHandle::new(cx, s);
                piano_panel(cx, s);
            } else {
                Binding::new(cx, s.inspector, move |cx| {
                    if s.inspector.get() {
                        device_panel(cx, s);
                    }
                });
            }
        });
        notice(cx, s);
        context_menu(cx, s);
    })
    .class("shell");

    StatsOverlay::new(cx, s);
    DragGhost::new(cx, s);
}

impl AppState {
    /// Starts the audio engine and sends it the initial state.
    fn start_engine(&mut self) {
        if std::env::var("CHIPDAW_AUDIO").is_ok_and(|v| v == "0") {
            return;
        }
        let e = Engine::start(EngineConfig {
            buffer_frames: self.tests.buffer,
            ..Default::default()
        });
        self.engine = Some(e);
        if self.tests.stress {
            self.s.arrangement.set(mock::stress_arrangement());
            self.s.stress.set(true);
        } else if self.tests.heavy {
            self.s.arrangement.set(mock::heavy_arrangement());
        }
        let tr = self.s.transport.get_untracked();
        self.send(Command::Bpm(tr.bpm as f64));
        self.send(Command::LoopEnd(self.engine_loop_end(tr.loop_end_ticks)));
        self.send(Command::Filter(self.filter));
        if let Some(g) = self.tests.gain {
            self.send(Command::Gain(g));
        }
        let vrc6 = self.s.chip.get_untracked() == SoundChip::WithVrc6;
        self.send(Command::Vrc6(vrc6));
        self.send_all_params();
        if self.tests.pilot || self.tests.avsync {
            match engine::testkit::Loopback::start(
                self.tests.pilot.then_some(engine::testkit::pilot_hz()),
            ) {
                Ok(lb) => self.tests.loopback = Some(lb),
                Err(err) => eprintln!("loopback: {err}"),
            }
            if self.tests.pilot {
                self.send(Command::TestPilot(engine::testkit::pilot_hz()));
            }
        }
        if self.tests.avsync {
            // One-bar loop, music muted, a click at mid-bar.
            self.s
                .transport
                .update(|t| t.loop_end_ticks = testing::AV_LOOP_TICKS);
            self.send(Command::LoopEnd(testing::AV_LOOP_TICKS));
            self.s.mute.set(0xFF);
            self.send(Command::Mute(0xFF));
            self.send(Command::TestClickAt(testing::AV_MARKER_TICKS));
        }
        if self.tests.autoplay || self.tests.avsync || self.tests.stress {
            self.send(Command::Play);
            self.s.transport.update(|t| t.playing = true);
        }
    }
}

/// Smallest window (logical px) that shows everything at its designed size:
/// - width: the inspector's fixed sections (dials, channels) plus the
///   smallest CLIP section and the panel's padding and gaps. This is wider
///   than the top bar needs (capsule + two sides of 412: 1028);
/// - height: top bar + ruler + eight full lanes + the inspector panel.
pub const MIN_WINDOW_SIZE: (u32, u32) = (
    // The inspector (two paddings and two gaps), but never narrower than
    // the top bar's fixed controls need.
    max_u32(
        INSPECTOR_PAD * 4 + DIALS_SECTION_W as u32 + CLIP_SECTION_MIN_W + INSTRUMENT_SECTION_MIN_W as u32,
        TOPBAR_MIN_W,
    ),
    (theme::tokens::TOPBAR_HEIGHT + theme::tokens::RULER_HEIGHT + 8.0 * LANE_H_NORMAL) as u32
        + INSPECTOR_HEIGHT,
);
/// The device inspector's height, padding and gap; must match `.device` in style.css.
const INSPECTOR_HEIGHT: u32 = 232;
const INSPECTOR_PAD: u32 = 16;
/// Inspector section widths (logical px). Dials and channels are fixed so
/// they never squeeze; the CLIP section takes what's left.
const DIALS_SECTION_W: f32 = MAX_DIALS as f32 * DIAL_W + (MAX_DIALS - 1) as f32 * DIAL_GAP + 24.0;
const CLIP_SECTION_MIN_W: u32 = 240;
/// The INSTRUMENT section's smallest width (its four graphs stay readable).
const INSTRUMENT_SECTION_MIN_W: f32 = 320.0;
const CLIP_SECTION_MAX_W: f32 = 480.0;
/// The narrowest window the top bar's fixed controls fit (DESIGN.md §8).
const TOPBAR_MIN_W: u32 = 1140;

const fn max_u32(a: u32, b: u32) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}

/// The open context menu over a full-window backdrop that closes it.
fn context_menu(cx: &mut Context, s: Signals) {
    let open = Memo::new(move |_| s.menu.get().is_some());
    Binding::new(cx, open, move |cx| {
        if !open.get() {
            return;
        }
        MenuBackdrop::new(cx)
            .position_type(PositionType::Absolute)
            .width(Stretch(1.0))
            .height(Stretch(1.0));
        let Some(m) = s.menu.get_untracked() else { return };
        ContextMenu::new(cx, s, m);
    });
}

/// Width of the recovery prompt's keys (logical px).
const NOTICE_KEY_W: f32 = 64.0;

/// Messages of varying length (save status, audio device problems, the crash
/// recovery prompt) float in a capsule under the transport instead of living
/// in the top bar, where they would push the fixed-size controls around.
fn notice(cx: &mut Context, s: Signals) {
    let visible = Memo::new(move |_| {
        !s.status.get().is_empty() || !s.audio_msg.get().is_empty() || s.recovery_offer.get()
    });
    Binding::new(cx, visible, move |cx| {
        if !visible.get() {
            return;
        }
        HStack::new(cx, |cx| {
            Label::new(cx, s.audio_msg)
                .class("notice-text")
                .toggle_class("hidden", s.audio_msg.map(|m| m.is_empty()));
            Label::new(cx, s.status)
                .class("notice-text")
                .toggle_class("hidden", s.status.map(|m| m.is_empty()));
            Binding::new(cx, s.recovery_offer, move |cx| {
                if s.recovery_offer.get() {
                    Label::new(cx, "Last session didn't close cleanly").class("notice-text");
                    Key::new(cx, s, "RECOVER", KeyAction::Recover)
                        .width(Pixels(NOTICE_KEY_W))
                        .height(Pixels(theme::tokens::KEY_SIZE));
                    Key::new(cx, s, "DISCARD", KeyAction::Discard)
                        .width(Pixels(NOTICE_KEY_W))
                        .height(Pixels(theme::tokens::KEY_SIZE));
                }
            });
        })
        .class("notice")
        .position_type(PositionType::Absolute)
        .left(Stretch(1.0))
        .right(Stretch(1.0))
        .top(Pixels(theme::tokens::TOPBAR_HEIGHT + 8.0));
    });
}

/// Split-pill widths (logical px): label segment and whole pill, per column.
const PILL_TEMPO_LABEL_W: f32 = 44.0;
const PILL_TEMPO_W: f32 = 92.0;
const PILL_CHIP_LABEL_W: f32 = 52.0;
const PILL_CHIP_W: f32 = 132.0;
const PILL_STAT_LABEL_W: f32 = 32.0;
/// Word keys in the top bar (logical px).
const WORD_KEY_W: f32 = 44.0;
/// The import / export keys (longer labels).
const WIDE_KEY_W: f32 = 56.0;

/// The top bar: file keys and project pills on the left, the transport
/// capsule centred, the spectrum and CPU / kHz pills on the right.
fn top_bar(cx: &mut Context, s: Signals) {
    use theme::tokens::{BOX_H, BOX_W, KEY_SIZE, PILL_H, PILL_W};
    HStack::new(cx, |cx| {
        // Left
        HStack::new(cx, |cx| {
            for (label, action) in [("NEW", KeyAction::New), ("OPEN", KeyAction::Open), ("SAVE", KeyAction::Save)] {
                Key::new(cx, s, label, action)
                    .width(Pixels(WORD_KEY_W))
                    .height(Pixels(KEY_SIZE));
            }
            Element::new(cx).class("bar-gap");
            for (label, action) in [("MIDI IN", KeyAction::ImportMidi), ("EXPORT", KeyAction::ExportWav)] {
                Key::new(cx, s, label, action)
                    .width(Pixels(WIDE_KEY_W))
                    .height(Pixels(KEY_SIZE));
            }
            Element::new(cx).class("bar-gap");
            VStack::new(cx, |cx| {
                StatPill::new(cx, s, PillKind::Bpm, PILL_TEMPO_LABEL_W)
                    .width(Pixels(PILL_TEMPO_W))
                    .height(Pixels(PILL_H));
                StatPill::new(cx, s, PillKind::Snap, PILL_TEMPO_LABEL_W)
                    .width(Pixels(PILL_TEMPO_W))
                    .height(Pixels(PILL_H));
            })
            .class("pill-stack");
            VStack::new(cx, |cx| {
                StatPill::new(cx, s, PillKind::Chip, PILL_CHIP_LABEL_W)
                    .width(Pixels(PILL_CHIP_W))
                    .height(Pixels(PILL_H));
                StatPill::new(cx, s, PillKind::Filter, PILL_CHIP_LABEL_W)
                    .width(Pixels(PILL_CHIP_W))
                    .height(Pixels(PILL_H));
            })
            .class("pill-stack");
        })
        .class("bar-side")
        .class("left")
        // A narrow window clips the sides at the capsule instead of sliding
        // them under it.
        .overflow(Overflow::Hidden);

        TimeCapsule::new(cx, s)
            .width(Pixels(BOX_W))
            .height(Pixels(BOX_H));

        // Right
        HStack::new(cx, |cx| {
            SpectrumBox::new(cx, s)
                .width(Pixels(BOX_W))
                .height(Pixels(BOX_H));
            VStack::new(cx, |cx| {
                StatPill::new(cx, s, PillKind::Cpu, PILL_STAT_LABEL_W)
                    .width(Pixels(PILL_W))
                    .height(Pixels(PILL_H));
                StatPill::new(cx, s, PillKind::Khz, PILL_STAT_LABEL_W)
                    .width(Pixels(PILL_W))
                    .height(Pixels(PILL_H));
            })
            .class("pill-stack");
        })
        .class("bar-side")
        .class("right")
        .overflow(Overflow::Hidden);
    })
    .class("topbar");
}

/// Dial cell width and spacing (logical px); must match `.dial` in style.css.
const DIAL_W: f32 = 84.0;
const DIAL_GAP: f32 = 8.0;

fn device_panel(cx: &mut Context, s: Signals) {
    HStack::new(cx, |cx| {
        // Dials
        VStack::new(cx, |cx| {
            let title = s.lane_sel.map(move |&lane| {
                let name = state::default_lanes()
                    .get(lane)
                    .map_or("", |l| l.name)
                    .to_uppercase();
                format!("DEVICE  ·  {name}")
            });
            Label::new(cx, title).class("section-title");
            // Only the controls the selected channel's hardware has.
            Binding::new(cx, s.lane_sel, move |cx| {
                let lane = s.lane_sel.get();
                let dials = if lane < 8 { lane_dials(lane) } else { Vec::new() };
                HStack::new(cx, |cx| {
                    if lane == 2 {
                        Label::new(cx, "No controls: the triangle has no volume or duty.")
                            .class("section-title");
                    } else if lane >= 8 {
                        Label::new(cx, "Select a clip to edit its channel.").class("section-title");
                    }
                    for (slot, (_, spec)) in dials.into_iter().enumerate() {
                        Dial::new(cx, s, slot, spec).class("dial");
                    }
                })
                .gap(Pixels(DIAL_GAP))
                .height(Auto)
                .width(Auto);
            });
        })
        .class("device-section")
        .width(Pixels(DIALS_SECTION_W));

        // Text fields
        VStack::new(cx, |cx| {
            Label::new(cx, "CLIP").class("section-title");
            let name = s.arrangement.map(|a| {
                a.first_selected()
                    .map(|c| c.name.clone())
                    .unwrap_or_default()
            });
            HStack::new(cx, |cx| {
                Textbox::new(cx, name)
                    .placeholder("No clip selected")
                    .on_submit(|cx, text, _| cx.emit(AppEvent::RenameSelected(text)));
            })
            .class("field");
        })
        .class("device-section")
        .width(Stretch(1.0))
        .min_width(Pixels(CLIP_SECTION_MIN_W as f32))
        .max_width(Pixels(CLIP_SECTION_MAX_W));

        // The lane's instrument: its macros, editable.
        VStack::new(cx, |cx| {
            let title = s.arrangement.map(move |a| {
                let lane = s.lane_sel.get_untracked();
                let id = a.current_instrument(lane);
                let name = a.instruments.get(id).map(|i| i.label()).unwrap_or_default();
                let whose = if id == a.instruments.lane_id(lane) { "LANE DEFAULT" } else { "CLIP'S OWN" };
                let n = a.instruments.users(id, &a.clips);
                let shared = if n == 1 { "1 CLIP".to_string() } else { format!("{n} CLIPS") };
                format!("INSTRUMENT  ·  {}  ·  {whose}  ·  {shared}", name.to_uppercase())
            });
            Label::new(cx, title).class("section-title");
            InstrumentView::new(cx, s).width(Stretch(1.0)).height(Stretch(1.0));
        })
        .class("device-section")
        .width(Stretch(1.0))
        .min_width(Pixels(INSTRUMENT_SECTION_MIN_W));
        // No CHANNELS meters here: the top bar's spectrum shows the same
        // levels at all times (STABILIZE.md §A4.5); the room goes to the
        // instrument editor.
    })
    .class("panel")
    .class("device");
}

/// The bottom panel while a clip is open: a header row and the piano roll.
fn piano_panel(cx: &mut Context, s: Signals) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            let title = s.piano.map(move |p| {
                let Some(id) = p.clip else { return String::new() };
                s.arrangement.with_untracked(|a| {
                    a.clip(id)
                        .map(|c| format!("PIANO ROLL  ·  {}  ·  {}", c.name, a.lanes[c.lane].name.to_uppercase()))
                        .unwrap_or_default()
                })
            });
            Label::new(cx, title).class("section-title");
            Label::new(cx, "GRID").class("field-label");
            let grid = s.piano.map(|p| p.grid.label());
            Button::new(cx, |cx| Label::new(cx, grid))
                .class("btn")
                .width(Pixels(60.0))
                .on_press(|cx| cx.emit(AppEvent::CyclePianoGrid));
            let follow = s.piano.map(|p| p.follow);
            Button::new(cx, |cx| Label::new(cx, "Follow"))
                .class("btn")
                .toggle_class("checked", follow)
                .on_press(|cx| cx.emit(AppEvent::TogglePianoFollow));
            let looping = s.piano.map(|p| p.loop_clip);
            Button::new(cx, |cx| Label::new(cx, "Loop clip"))
                .class("btn")
                .toggle_class("checked", looping)
                .on_press(|cx| cx.emit(AppEvent::ToggleLoopClip));
            let hover = s.piano.map(|p| p.hover.clone());
            Label::new(cx, hover).class("field-label").class("mono-info");
            Element::new(cx).width(Stretch(1.0)).height(Pixels(1.0));
            Label::new(cx, "Shift-drag select · Ctrl-drag copy · Alt no snap · Q quantize · Esc close")
                .class("field-label");
            Button::new(cx, |cx| Label::new(cx, "Close"))
                .class("btn")
                .on_press(|cx| cx.emit(AppEvent::ClosePiano));
        })
        .class("piano-header");
        PianoRoll::new(cx, s);
    })
    .class("panel")
    .class("piano-panel");
}

impl AppState {
    /// The undoable document as it is now.
    fn doc(&self) -> Doc {
        let bpm = self.s.transport.with_untracked(|t| t.bpm);
        self.s.arrangement.with_untracked(|a| Doc::of(a, bpm, self.params))
    }

    /// Records an undo step if the document changed (dropped while a gesture
    /// is still in progress).
    fn commit(&mut self) {
        let busy = self.gesture_depth > 0
            || self.s.arrangement.with_untracked(|a| !a.editing.is_empty() || a.editing_notes.is_some());
        if !busy {
            // Instruments only deleted clips used are dropped with the step.
            if self.s.arrangement.with_untracked(|a| a.instruments.unused(&a.clips)) {
                self.s.arrangement.update(|a| {
                    let clips = a.clips.clone();
                    a.instruments.prune(&clips);
                });
            }
            let d = self.doc();
            self.history.commit(d);
        }
    }

    fn undo(&mut self, redo: bool) {
        if self.gesture_depth > 0 {
            return;
        }
        let now = self.doc();
        let d = if redo { self.history.redo(now) } else { self.history.undo(now) };
        match d {
            Some(d) => self.apply_doc(&d),
            None => self.set_status(if redo { "Nothing to redo" } else { "Nothing to undo" }.into()),
        }
    }

    /// Puts a document into the model and the engine (through the usual
    /// snapshot path: the content key changes, `sync_engine` sends it).
    fn apply_doc(&mut self, d: &Doc) {
        self.s.arrangement.update(|a| d.apply_to(a));
        if self.s.transport.with_untracked(|t| t.bpm) != d.bpm {
            self.send(Command::Bpm(d.bpm as f64));
            self.s.transport.update(|t| t.bpm = d.bpm);
        }
        if self.params != d.params {
            self.params = d.params;
            self.send_all_params();
            let lane = self.s.lane_sel.get_untracked();
            if lane < engine::LANES {
                self.s.dials.set(self.dial_values(lane));
            }
            self.sync_piano_lane_volume();
        }
        // The open clip may be gone; notes that no longer exist leave the selection.
        let alive = self.s.piano.with_untracked(|p| p.clip).and_then(|id| {
            self.s.arrangement.with_untracked(|a| a.clip(id).map(|c| c.notes.iter().map(|n| n.id).collect::<Vec<_>>()))
        });
        match alive {
            Some(ids) => self.s.piano.update(|p| p.sel.retain(|id| ids.contains(id))),
            None if self.s.piano_open.get_untracked() => self.close_piano(),
            None => {}
        }
    }

    fn send_all_params(&mut self) {
        for lane in 0..engine::LANES {
            for &id in ParamId::for_lane(lane) {
                let value = self.params.get(lane, id) as i8;
                self.send(Command::Param { lane: lane as u8, id, value });
            }
        }
    }

    fn project(&self) -> Project {
        let tr = self.s.transport.get_untracked();
        Project {
            arrangement: self.s.arrangement.get_untracked(),
            bpm: tr.bpm,
            chip: self.s.chip.get_untracked(),
            filter: self.filter,
            filter_auto: !self.filter_manual,
            loop_end_ticks: if self.clip_looping {
                self.loop_end_saved
            } else {
                tr.loop_end_ticks
            } as u32,
            loop_start_ticks: if self.clip_looping {
                self.loop_start_saved
            } else {
                tr.loop_start_ticks
            } as u32,
            loop_on: self.s.loop_on.get_untracked(),
            mute: self.s.mute.get_untracked(),
            solo: self.s.solo.get_untracked(),
            params: self.params,
            load_report: Vec::new(),
        }
    }

    /// Replaces the whole project (New, Open, Recover): stops playback,
    /// closes the piano roll and clears the undo history.
    fn load_project(&mut self, p: Project) {
        self.close_piano();
        self.send(Command::Stop);
        self.send(Command::Seek(0.0));
        let mut a = p.arrangement;
        a.view = self.s.arrangement.with_untracked(|cur| cur.view);
        a.snap = self.s.arrangement.with_untracked(|cur| cur.snap);
        self.s.arrangement.set(a);
        self.s.stress.set(false);
        self.s.transport.update(|t| {
            t.playing = false;
            t.playhead_ticks = 0.0;
            t.bpm = p.bpm;
            t.loop_end_ticks = p.loop_end_ticks as f64;
            t.loop_start_ticks = p.loop_start_ticks as f64;
        });
        self.loop_end_saved = p.loop_end_ticks as f64;
        self.loop_start_saved = p.loop_start_ticks as f64;
        self.clip_looping = false;
        self.s.loop_on.set(p.loop_on);
        if !p.load_report.is_empty() {
            self.set_status(p.load_report.join(" "));
        }
        self.send(Command::Bpm(p.bpm as f64));
        self.send(Command::LoopStart(p.loop_start_ticks as f64));
        self.send(Command::LoopEnd(self.engine_loop_end(p.loop_end_ticks as f64)));
        self.s.chip.set(p.chip);
        self.send(Command::Vrc6(p.chip == SoundChip::WithVrc6));
        self.filter = p.filter;
        self.filter_manual = !p.filter_auto;
        self.send(Command::Filter(self.filter));
        self.s.mute.set(p.mute);
        self.s.solo.set(p.solo);
        self.send(Command::Mute(p.mute));
        self.send(Command::Solo(p.solo));
        self.params = p.params;
        self.send_all_params();
        let lane = self.s.lane_sel.get_untracked();
        if lane < engine::LANES {
            self.s.dials.set(self.dial_values(lane));
        }
        let d = self.doc();
        self.history.reset(d);
        self.last_audio_stats = None;
    }

    /// Native file dialog (rfd), filtered to projects, starting in the
    /// current project's folder. Modal: it blocks the UI thread while open
    /// (the audio thread keeps running).
    /// Converts a WAV file into a DPCM sample in the first free user slot.
    fn import_sample(&mut self, path: &std::path::Path) {
        let (slot, free) = self.s.arrangement.with_untracked(|a| (dpcm::free_slot(&a.samples), dpcm::free_bytes(&a.samples)));
        let Some(slot) = slot else {
            return self.set_status("All four DPCM sample slots are used: remove one first.".into());
        };
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => return self.set_status(format!("Can't open {}: {e}", path.display())),
        };
        let name = path.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Sample".into());
        let result = dpcm::read_wav(&bytes).and_then(|(pcm, sr)| dpcm::convert(&name, &pcm, sr, free));
        match result {
            Ok((sample, truncated)) => {
                let msg = format!(
                    "{name} → DPCM slot {}: {:.2} s at {:.1} kHz{}. Play it on the DPCM row \"{name}\".",
                    slot + 1,
                    sample.seconds(),
                    sample.hz() / 1000.0,
                    if truncated { " (cut to fit the chip's memory)" } else { "" },
                );
                self.s.arrangement.update(|a| {
                    a.samples.resize(chip::samples::USER_SLOTS, None);
                    a.samples[slot] = Some(sample);
                });
                self.commit();
                self.set_status(msg);
            }
            Err(e) => self.set_status(e),
        }
    }

    /// A file dropped on the window: MIDI, audio or a project.
    fn open_dropped(&mut self, path: &std::path::Path) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "mid" | "midi" => self.import_midi(path),
            "wav" => self.import_sample(path),
            e if e == file::EXTENSION => match file::load(path) {
                Ok(p) => {
                    self.load_project(p);
                    self.s.file_path.set(path.display().to_string());
                    self.set_status(format!("Opened {}", path.display()));
                }
                Err(e) => self.set_status(e),
            },
            _ => self.set_status(format!("Drop a .mid, .wav or .{} file.", file::EXTENSION)),
        }
    }

    fn import_midi(&mut self, path: &std::path::Path) {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => return self.set_status(format!("Can't open {}: {e}", path.display())),
        };
        let song = match midi::parse(&bytes) {
            Ok(s) => s,
            Err(e) => return self.set_status(e),
        };
        let vrc6 = self.s.chip.get_untracked() == SoundChip::WithVrc6;
        let imp = midi::arrange(&song, vrc6);
        let mut p = self.project();
        let mut instruments = p.arrangement.instruments.clone();
        p.arrangement = imp.arrangement;
        // Keep the band; drop clip-only instruments the old clips used.
        instruments.prune(&p.arrangement.clips);
        p.arrangement.instruments = instruments;
        if let Some(bpm) = imp.bpm {
            p.bpm = bpm.clamp(Transport::MIN_BPM, Transport::MAX_BPM);
        }
        p.loop_end_ticks = imp.loop_end;
        p.mute = 0;
        p.solo = 0;
        self.load_project(p);
        self.s.file_path.set(String::new());
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        self.set_status(format!("Imported {name}: {}", imp.report));
    }

    fn export_wav(&mut self) {
        let cur = file::with_extension(&self.s.file_path.get_untracked());
        let stem = cur.file_stem().and_then(|n| n.to_str()).filter(|n| !n.is_empty()).unwrap_or("song");
        let mut d = rfd::FileDialog::new().add_filter("WAV audio", &["wav"]).set_file_name(format!("{stem}.wav"));
        if let Some(dir) = cur.parent().filter(|d| d.is_dir()) {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else { return };
        let t0 = Instant::now();
        let audio = export::render(&self.project());
        match export::write_wav(&audio, &path) {
            Ok(()) => self.set_status(format!(
                "Exported {:.1} s of audio to {} in {} ms",
                audio.len() as f32 / export::SAMPLE_RATE as f32,
                path.display(),
                t0.elapsed().as_millis()
            )),
            Err(e) => self.set_status(e),
        }
    }

    fn file_dialog(&self) -> rfd::FileDialog {
        let mut d = rfd::FileDialog::new().add_filter("ChipDAW project", &[file::EXTENSION]);
        let cur = file::with_extension(&self.s.file_path.get_untracked());
        if let Some(dir) = cur.parent().filter(|d| d.is_dir()) {
            d = d.set_directory(dir);
        }
        if let Some(name) = cur.file_name().and_then(|n| n.to_str()).filter(|n| !n.is_empty()) {
            d = d.set_file_name(name);
        }
        d
    }

    fn save_project(&mut self) {
        if self.s.file_path.get_untracked().trim().is_empty() {
            match self.file_dialog().save_file() {
                Some(path) => self.s.file_path.set(path.display().to_string()),
                None => return,
            }
        }
        let text = self.s.file_path.get_untracked();
        let path = file::with_extension(&text);
        match file::save(&self.project(), &path) {
            Ok(()) => {
                self.s.file_path.set(path.display().to_string());
                self.set_status(format!("Saved {}", path.display()));
            }
            Err(e) => self.set_status(e),
        }
    }

    fn set_status(&mut self, msg: String) {
        self.s.status.set(msg);
        self.status_until = Some(Instant::now() + Duration::from_secs(6));
    }

    fn open_piano(&mut self, id: u32) {
        let Some((lane, notes_mid)) = self.s.arrangement.with_untracked(|a| {
            a.clip(id).map(|c| {
                let kind = RowKind::for_lane(c.lane);
                let rows: Vec<u32> = c.audible_notes().map(|n| kind.row_of(n.pitch)).collect();
                let mid = if rows.is_empty() {
                    match kind {
                        RowKind::Piano { .. } => kind.row_of(60),
                        _ => kind.rows() / 2,
                    }
                } else {
                    (rows.iter().min().unwrap() + rows.iter().max().unwrap()) / 2
                };
                (c.lane, (kind, mid))
            })
        }) else {
            return;
        };
        let (kind, mid) = notes_mid;
        self.s.arrangement.update(|a| a.selection = vec![id]);
        let lane_volume = self.params.get(lane, ParamId::Volume) as u8;
        self.s.piano.update(|p| {
            if p.clip != Some(id) {
                p.sel.clear();
                p.scroll_ticks = 0.0;
            }
            p.clip = Some(id);
            p.lane_volume = lane_volume;
            // Centre the notes (or middle C) vertically; the view clamps.
            const ROWS_SHOWN_GUESS: f32 = 22.0;
            p.top = ((kind.rows() - 1 - mid) as f32 - ROWS_SHOWN_GUESS / 2.0).max(0.0);
        });
        self.s.piano_open.set(true);
        // Bring the clip's lane into view if it's scrolled out.
        if let Some(lane) = self.s.arrangement.with_untracked(|a| a.clip(id).map(|c| c.lane)) {
            let (top, bottom) = (lane as f32 * LANE_H_NORMAL, (lane + 1) as f32 * LANE_H_NORMAL);
            let pane = self.s.piano_pane_h.get_untracked();
            let v = self.s.lane_scroll.get_untracked();
            if top < v {
                self.s.lane_scroll.set(top);
            } else if bottom > v + pane {
                self.s.lane_scroll.set(bottom - pane);
            }
        }
        self.clamp_lane_scroll();
        self.sync_loop();
    }

    fn close_piano(&mut self) {
        if !self.s.piano_open.get_untracked() {
            return;
        }
        self.s.piano.update(|p| {
            p.clip = None;
            p.sel.clear();
            p.loop_clip = false;
            p.hover.clear();
        });
        self.sync_loop();
        self.s.piano_open.set(false);
        self.clamp_lane_scroll();
    }

    /// Keeps the lane scroll inside what the lane viewport allows: all lanes
    /// fit (no scroll) unless the piano roll's split makes the viewport short.
    fn clamp_lane_scroll(&mut self) {
        let content = 8.0 * LANE_H_NORMAL;
        let max = if self.s.piano_open.get_untracked() {
            (content - self.s.piano_pane_h.get_untracked()).max(0.0)
        } else {
            0.0
        };
        self.s.lane_scroll_max.set_if_changed(max);
        let v = self.s.lane_scroll.get_untracked().clamp(0.0, max);
        self.s.lane_scroll.set_if_changed(v);
    }

    /// Sets the song loop (the ruler's loop brace). `enable` also turns the
    /// loop light on (as a DAW's Loop Selection does). A piano-roll clip loop
    /// gives way to it.
    fn set_song_loop(&mut self, start: f64, end: f64, enable: bool) {
        let start = start.max(0.0);
        let end = end.max(start + PPQ as f64);
        self.loop_start_saved = start;
        self.loop_end_saved = end;
        if self.clip_looping {
            self.s.piano.update(|p| p.loop_clip = false);
            self.sync_loop();
        } else {
            self.s.transport.update(|t| {
                t.loop_start_ticks = start;
                t.loop_end_ticks = end;
            });
            self.send(Command::LoopStart(start));
            self.send(Command::LoopEnd(self.engine_loop_end(end)));
        }
        if enable && !self.s.loop_on.get_untracked() {
            self.s.loop_on.set(true);
            self.send(Command::LoopEnd(self.engine_loop_end(end)));
        }
    }

    /// Context-menu commands and their shortcuts.
    fn run_menu_action(&mut self, action: MenuAction) {
        let sel = self.s.time_sel.get_untracked();
        let span = self.s.arrangement.with_untracked(|a| a.selection_span());
        match action {
            // Ctrl+L: the time selection, else the selected clips.
            MenuAction::LoopSelection | MenuAction::LoopClips => {
                let range = match (action, sel) {
                    (MenuAction::LoopSelection, Some(ts)) => Some((ts.start, ts.end)),
                    _ => span,
                };
                if let Some((a, b)) = range {
                    self.set_song_loop(a as f64, b as f64, true);
                }
            }
            MenuAction::InsertClip => {
                let Some(ts) = sel else { return };
                self.s.arrangement.update(|a| {
                    let ids: Vec<u32> = (ts.lane0..=ts.lane1.min(a.lanes.len() - 1))
                        .map(|lane| a.add_clip(lane, ts.start, ts.end - ts.start))
                        .collect();
                    a.resolve_overlaps(&ids);
                    a.selection = ids;
                });
                self.s.time_sel.set(None);
                self.commit();
            }
            MenuAction::Duplicate => {
                if span.is_some() {
                    self.s.arrangement.update(|a| a.duplicate_selected());
                    self.commit();
                }
            }
            MenuAction::Delete => {
                self.s.arrangement.update(|a| a.delete_selected());
                self.commit();
            }
            MenuAction::MakeUnique => self.make_instrument_unique(),
            MenuAction::LaneInstrument => self.use_lane_instrument(),
            MenuAction::SavePreset => self.save_user_preset(),
        }
    }

    fn sync_piano_lane_volume(&mut self) {
        let Some(id) = self.s.piano.with_untracked(|p| p.clip) else { return };
        let Some(lane) = self.s.arrangement.with_untracked(|a| a.clip(id).map(|c| c.lane)) else { return };
        // A note without a volume plays at full (15), scaled by the channel
        // volume; in the hardware decay modes the control is its decay speed.
        let decay = ParamId::for_lane(lane).contains(&ParamId::Envelope) && self.params.get(lane, ParamId::Envelope) != 0;
        let v = if decay { self.params.get(lane, ParamId::Volume) as u8 } else { notes::MAX_VOLUME };
        if self.s.piano.with_untracked(|p| p.lane_volume) != v {
            self.s.piano.update(|p| p.lane_volume = v);
        }
    }

    /// "Loop clip": playback loops over the open clip (follows its edits);
    /// off restores the song loop.
    fn sync_loop(&mut self) {
        let p = self.s.piano.get_untracked();
        let want = if p.loop_clip {
            p.clip.and_then(|id| {
                self.s.arrangement.with_untracked(|a| a.clip(id).map(|c| (c.start_ticks as f64, c.end_ticks() as f64)))
            })
        } else {
            None
        };
        if want.is_none() && !self.clip_looping {
            return;
        }
        let tr = self.s.transport.get_untracked();
        let entering = want.is_some() && !self.clip_looping;
        if entering {
            // Remember the song loop to restore it afterwards.
            self.loop_end_saved = tr.loop_end_ticks;
            self.loop_start_saved = tr.loop_start_ticks;
        }
        self.clip_looping = want.is_some();
        let (start, end) = want.unwrap_or((self.loop_start_saved, self.loop_end_saved));
        if (tr.loop_start_ticks, tr.loop_end_ticks) != (start, end) {
            self.send(Command::LoopStart(start));
            self.send(Command::LoopEnd(self.engine_loop_end(end)));
            let outside = tr.playhead_ticks < start || tr.playhead_ticks >= end;
            self.s.transport.update(|t| {
                t.loop_start_ticks = start;
                t.loop_end_ticks = end;
            });
            if want.is_some() && outside {
                self.s.transport.update(|t| t.playhead_ticks = start);
                self.send(Command::Seek(start));
            }
        }
    }

    /// Pages the piano roll along with the playhead while playing.
    fn follow_playhead(&mut self, playhead: f64) {
        let p = self.s.piano.get_untracked();
        let (true, Some(id)) = (p.follow, p.clip) else { return };
        let Some((start, len)) = self.s.arrangement.with_untracked(|a| a.clip(id).map(|c| (c.start_ticks, c.len_ticks))) else {
            return;
        };
        let rel = playhead - start as f64;
        if rel < 0.0 || rel > len as f64 {
            return;
        }
        let visible = p.width_px as f64 / p.px_per_tick;
        if rel < p.scroll_ticks || rel > p.scroll_ticks + visible * 0.95 {
            let to = (rel - visible * 0.05).max(0.0);
            self.s.piano.update(|p| p.scroll_ticks = to);
        }
    }

    fn copy_notes(&mut self, cut: bool) {
        let p = self.s.piano.get_untracked();
        let Some(id) = p.clip else { return };
        let Some(clip) = self.s.arrangement.with_untracked(|a| a.clip(id).map(|c| notes::copy(&c.notes, &p.sel))) else {
            return;
        };
        if clip.is_empty() {
            return;
        }
        self.clipboard = clip;
        if cut {
            self.s.arrangement.update(|a| {
                if let Some(c) = a.clip_mut(id) {
                    notes::delete(&mut c.notes, &p.sel);
                }
            });
            self.s.piano.update(|p| p.sel.clear());
            self.commit();
        }
    }

    /// Ctrl+V: after the selection when there is one (repeated pastes chain),
    /// else at the playhead if it is inside the clip, else at the clip start.
    fn paste_notes(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        let p = self.s.piano.get_untracked();
        let Some(id) = p.clip else { return };
        let playhead = self.s.transport.with_untracked(|t| t.playhead_ticks);
        let clipboard = self.clipboard.clone();
        let mut new = Vec::new();
        self.s.arrangement.update(|a| {
            let Some(c) = a.clip_mut(id) else { return };
            let kind = RowKind::for_lane(c.lane);
            let rel = playhead - c.start_ticks as f64;
            let at = if let Some((_, end)) = notes::span(&c.notes, &p.sel) {
                end
            } else if rel >= 0.0 && rel < c.len_ticks as f64 {
                rel as u32
            } else {
                0
            };
            if at >= c.len_ticks {
                return;
            }
            let mut next = c.next_note_id;
            new = notes::paste(&mut c.notes, &mut next, &clipboard, at, kind);
            c.next_note_id = next;
        });
        if !new.is_empty() {
            self.s.piano.update(|p| p.sel = new);
            self.commit();
        }
    }

    /// Once per pump tick: commit finished edits, keep the clip loop in step,
    /// autosave, expire the status line.
    fn idle_housekeeping(&mut self) {
        self.commit();
        if self.s.piano.with_untracked(|p| p.loop_clip) {
            self.sync_loop();
        }
        let now = Instant::now();
        if self.status_until.is_some_and(|t| now >= t) {
            self.status_until = None;
            self.s.status.set(String::new());
        }
        if now - self.last_autosave >= file::AUTOSAVE_INTERVAL && !self.s.recovery_offer.get_untracked() {
            self.last_autosave = now;
            let d = self.doc();
            if self.autosaved.as_ref().is_none_or(|a| !a.same_as(&d)) {
                self.autosaved = Some(d);
                if let Some(path) = self.recovery.as_ref().map(|r| r.file()) {
                    // Serialising a big song takes milliseconds: off the UI thread.
                    let p = self.project();
                    std::thread::spawn(move || {
                        let _ = file::save(&p, &path);
                    });
                }
            }
        }
    }

    /// `CHIPDAW_PROJECT=path` opens a project at start (tests, benchmarks);
    /// `CHIPDAW_OPEN_CLIP=id` then opens that clip in the piano roll.
    fn apply_test_project(&mut self) {
        // Files on the command line (Open With, or a double-clicked file):
        // projects, MIDI and WAV files, in order.
        for arg in std::env::args_os().skip(1) {
            let path = std::path::PathBuf::from(arg);
            if path.is_file() {
                self.open_dropped(&path);
            }
        }
        if let Ok(path) = std::env::var("CHIPDAW_PROJECT") {
            match file::load(std::path::Path::new(&path)) {
                Ok(p) => {
                    self.load_project(p);
                    self.s.file_path.set(path);
                }
                Err(e) => eprintln!("CHIPDAW_PROJECT: {e}"),
            }
        }
        if let Some(id) = std::env::var("CHIPDAW_OPEN_CLIP").ok().and_then(|v| v.parse().ok()) {
            self.open_piano(id);
        }
        // `CHIPDAW_PIANO_ZOOM=px per bar` (the piano-roll benchmark shows
        // all 2,000 notes at 32).
        if let Some(ppb) = std::env::var("CHIPDAW_PIANO_ZOOM").ok().and_then(|v| v.parse::<f64>().ok()) {
            let ppb = ppb.clamp(PianoState::MIN_PX_PER_BAR, PianoState::MAX_PX_PER_BAR);
            self.s.piano.update(|p| p.px_per_tick = ppb / TICKS_PER_BAR as f64);
        }
    }
}
