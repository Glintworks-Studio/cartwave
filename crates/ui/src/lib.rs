//! ChipDAW's user interface. A library crate so the same UI can later be hosted
//! inside a plugin window (baseview) as well as the standalone app.

pub mod dev;
pub mod edit;
pub mod file;
pub mod dpcm;
pub mod export;
pub mod game_export;
pub mod history;
pub mod menus;
pub mod midi;
pub mod instruments;
pub mod user_presets;
pub mod input;
pub mod mock;
pub mod notes;
pub mod perf;
pub mod render;
pub mod shortcuts;
pub mod songs;
pub mod state;
pub mod tap;
pub mod testing;
pub mod theme;
pub mod views;

use vizia::prelude::*;

use engine::{Command, Engine, EngineConfig, FilterMode, ParamId, Params};

use crate::file::{Project, Recovery};
use crate::shortcuts::{Action, Ctx};
use crate::views::modal::{FilterPick, Modal, Set, SettingsTab};
use crate::history::{Doc, History};
use crate::mock::Rng;
use crate::notes::{Note, RowKind};
use crate::state::*;
use crate::views::{
    arrangement::{track_headers, Arrangement as ArrangementView},
    browser::{browser, DragGhost, PresetDrag},
    chrome::{ContextMenu, Key, KeyAction, MenuBackdrop, SpectrumBox, SplitHandle},
    topbar::{CpuMem, DisplayWell, TopKey, TopKeyView},
    dial::Dial,
    instrument::InstrumentView,
    piano_roll::PianoRoll,
    overlay::StatsOverlay,
    ruler::Ruler,
};

/// A running export: its progress / cancel flags and its worker thread.
type ExportJob = (std::sync::Arc<game_export::Control>, std::thread::JoinHandle<Result<game_export::Report, String>>);

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
    /// The bottom inspector (device panel) is open: a clip is selected.
    pub inspector: Signal<bool>,
    /// The browser sidebar is shown (Ctrl+Alt+B).
    pub browser_open: Signal<bool>,
    pub browser_query: Signal<String>,
    /// The preset highlighted in the browser.
    pub browser_sel: Signal<Option<usize>>,
    /// A preset being dragged out of the browser.
    pub drag_preset: Signal<Option<PresetDrag>>,
    /// The export dialog, while it is open.
    pub export_dlg: Signal<Option<views::export_dialog::ExportDialog>>,
    /// The user's saved presets (browser indices after the factory presets).
    pub user_presets: Signal<Vec<user_presets::UserPreset>>,
    /// Unsaved changes since the last New, Open or Save (F-01).
    pub dirty: Signal<bool>,
    /// The open modal panel: Settings, Project Settings, About, shortcuts.
    pub modal: Signal<Option<Modal>>,
    /// App settings (settings.json) as the Settings panel shows them.
    pub settings: Signal<file::Settings>,
    /// Output device names, listed when Settings > Audio opens.
    pub audio_devices: Signal<Vec<String>>,
    /// The running device and the last switch's gap (Settings > Audio).
    pub audio_info: Signal<String>,
    /// The output filter and whether it follows the profile (Project Settings).
    pub filter_state: Signal<(FilterMode, bool)>,
    /// Why recovery is offered (crash, or an unsaved quit).
    pub recovery_msg: Signal<&'static str>,
    /// The in-place rename field, while open (A13, T2, B5).
    pub rename: Signal<Option<edit::RenameSpec>>,
    /// The getting-started guide shown on demand (Help ▸ Getting Started)
    /// even when the project has clips (A22, F-14).
    pub guide: Signal<bool>,
    /// The track headers' width, side and strip position (Settings ▸ Look
    /// & Feel; the width also follows the boundary drag).
    pub header: Signal<views::header_layout::HeaderLayout>,
    /// The status bar's hover help: one line for the control under the mouse.
    pub hover_help: Signal<String>,
    /// Persistent problems (no audio device, dropouts): the status bar's
    /// warning icon stays while this isn't empty (polish 2 §6.4).
    pub problems: Signal<Vec<String>>,
    /// The project's format: (sample rate Hz, bit depth). The display's
    /// indicators show it; it's the export's default (Project Settings).
    pub project_format: Signal<(u32, u16)>,
    /// The process's private memory in MB (refreshed with the CPU read-out).
    pub mem_mb: Signal<Option<u32>>,
}

/// Project fields outside undo that a save writes (F-01: changing them
/// leaves unsaved changes too).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Extras {
    chip: SoundChip,
    filter: FilterMode,
    filter_auto: bool,
    loop_start: u32,
    loop_end: u32,
    loop_on: bool,
    mute: u8,
    solo: u8,
    /// The project's format (sample rate, bit depth).
    format: (u32, u16),
}

/// What the user chose in a "Save changes?" prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Proceed {
    /// Nothing unsaved, or saved: go ahead.
    Clean,
    /// Don't Save: go ahead and drop the changes.
    Discard,
    /// Cancel (or the save failed / its dialog was cancelled): stop.
    Stop,
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
    /// A running export: its progress / cancel flags and worker thread.
    export_job: Option<ExportJob>,
    /// The text field being edited (F-03: shortcuts don't fire while typing).
    text_edit: Option<Entity>,
    /// The editor that last had the mouse: Edit-menu commands act on it.
    editor: Ctx,
    /// The document and extras as last saved / opened / created (F-01).
    saved: (Doc, Extras),
    /// The window is closing (the user already answered the save prompt).
    quitting: bool,
    /// The window title last set.
    title: String,
    /// settings.json (the `settings` signal mirrors it).
    settings: file::Settings,
    #[cfg(feature = "menus")]
    menus: Option<menus::NativeMenus>,
    menus_tried: bool,
    /// When the audio settings last changed (its gap is read a moment later).
    audio_switch: Option<Instant>,
    /// Clips copied in the arrangement (A9; notes have `clipboard`).
    clip_board: Option<state::ClipBoard>,
    /// Earlier arrangement zooms, for X (R31).
    zoom_back: Vec<Viewport>,
    /// Where playback last stopped (Shift+Space continues there, R137).
    stop_point: f64,
    /// The rename field's text as typed.
    rename_text: String,
    /// Where a preset's context menu opened (its rename field goes there).
    preset_rename_at: (f32, f32),
    /// The window's scale factor and logical size, as of the last tick.
    scale: f32,
    window_logical: (f32, f32),
    /// The colours a colour-grid hover preview replaced.
    colour_backup: Option<edit::ColourBackup>,
    /// A brief audition (arrow keys in the browser) ends then.
    audition_until: Option<Instant>,
    /// When MEM was last read.
    last_mem: Option<Instant>,
    /// TAP tempo: the current tap sequence, and the undo step count right
    /// after its first commit (later taps amend that step).
    tap: tap::TapTempo,
    tap_step: Option<(usize, usize)>,
    /// Where the BPM field opens (window logical x, y, width).
    bpm_rename_at: (f32, f32, f32),
    /// The track gains the engine has (it starts at unity).
    sent_gains: [f32; 8],
    /// Buffer underruns seen so far and when the count last rose (the
    /// status bar's dropout problem).
    underruns_seen: (u64, Option<Instant>),
}

/// A dropout problem clears after this long without another underrun.
const DROPOUT_HOLD: Duration = Duration::from_secs(10);

pub enum AppEvent {
    /// Animation pump timer.
    Tick,
    TogglePlay,
    Stop,
    ToggleRecord,
    SetBpm(f32),
    SetPlayhead(f64),
    SetDial(usize, f32),
    /// TAP tempo (the TAP key, or its shortcut).
    Tap,
    /// Double-click on the display's BPM: type a tempo in a field at
    /// (x, y, width), window logical px.
    EditBpm(f32, f32, f32),
    /// A track's volume fader: lane, 0..1 of the fader's travel.
    SetLaneVolume(usize, f32),
    /// A track's pan, −50 … 50 (polish 2 §3: saved, heard with stereo mixing).
    SetLanePan(usize, i8),
    /// The status bar's hover-help line (empty: none).
    HoverHelp(String),
    /// The status bar's warning icon was clicked (window logical x, y).
    ShowProblems(f32, f32),
    /// A track's row height, logical px (polish 2 §4); clamped to its steps.
    SetLaneHeight(usize, f32),
    /// The spectrum's style (its right-click menu; remembered in settings.json).
    SetSpectrumStyle(views::chrome::SpectrumStyle),
    /// The browser's Preview key: audition presets on select (remembered).
    TogglePreviewOnSelect,
    /// Audition a preset for a moment (selected with the arrow keys).
    AuditionBrief(usize),
    /// Hovering a colour grid cell previews it (`None`: put the colours back).
    ColourPreview(Option<MenuAction>),
    /// Project Settings: the project's bit depth (16 / 24).
    SetBitDepth(u16),
    /// Project Settings: the project's sample rate (44 100 / 48 000).
    SetProjectRate(u32),
    /// The capsule's loop light: loop the song at its loop end, or play on.
    ToggleLoop,
    /// The split handle above the piano roll: lane viewport height.
    SetPianoPaneHeight(f32),
    /// Scroll the lanes vertically by this many logical px.
    ScrollLanes(f32),
    /// The song loop (ticks). The ruler's loop brace sends this while dragged.
    SetLoop(f64, f64),
    /// Open / close a context menu, or run one of its commands (also bound
    /// to the usual DAW shortcuts).
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
    /// Export dialog: a choice, a lane's stem group, start, cancel, close.
    ExportPick(views::export_dialog::Pick),
    ExportGroup(usize),
    ExportGo,
    ExportCancel,
    ExportClose,
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
    /// A native menu item.
    MenuCmd(menus::Cmd),
    /// A keyboard action (keys and menu items run the same code).
    Run(Action),
    OpenModal(Modal),
    CloseModal,
    SetSetting(Set),
    /// Project Settings: the hardware profile (asks first if VRC6 clips would go silent).
    SetProfile(SoundChip),
    SetFilter(FilterPick),
    /// An editor got the mouse (Edit-menu commands act on it).
    EditorFocus(Ctx),
    /// Clear Open Recent / open entry n.
    OpenRecent(usize),
    /// A right-click (Batch 3): where, at (x, y) window logical px.
    ContextMenu(edit::MenuSite, f32, f32),
    /// Open a context submenu next to its row.
    OpenSub(SubMenu, f32, f32),
    /// The rename field: typed text, Enter / Tab (true: next track), Esc.
    RenameText(String),
    RenameCommit(bool),
    RenameSubmit(edit::RenameTarget, String),
    RenameCancel,
    OpenRename(edit::RenameTarget),
    /// A track header was clicked (T1; `true`: Shift / Ctrl adds).
    SelectTrack(usize, bool),
    /// The insert marker (A1): tick, track.
    SetInsert(u32, usize),
    CloseGuide,
    /// A short message in the notice capsule.
    Status(String),
}

/// An edit command for the piano roll view, broadcast from the root (the
/// Edit menu while the piano roll is the active editor).
pub struct PianoCmd(pub Action);

impl Model for AppState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| match e {
            AppEvent::Tick => self.tick(cx),
            AppEvent::TogglePlay => {
                let t = self.s.transport.get_untracked();
                if t.playing {
                    // Stop: remember the stop point (Shift+Space continues
                    // there, R137); the playhead goes back to the insert
                    // marker, where Space starts next time.
                    self.send(Command::Stop);
                    self.stop_point = t.playhead_ticks;
                    let at = self.s.arrangement.with_untracked(|a| a.insert.0) as f64;
                    self.send(Command::Seek(at));
                    self.s.transport.update(|t| {
                        t.playing = false;
                        t.playhead_ticks = at;
                    });
                } else {
                    // Start from the insert marker (where the playhead sits
                    // while stopped).
                    self.send(Command::Seek(t.playhead_ticks));
                    self.send(Command::Play);
                    self.s.transport.update(|t| t.playing = true);
                }
            }
            AppEvent::Stop => {
                let was_playing = self.s.transport.with_untracked(|t| t.playing);
                if was_playing {
                    self.stop_point = self.s.transport.with_untracked(|t| t.playhead_ticks);
                } else {
                    self.s.arrangement.update(|a| a.insert.0 = 0);
                }
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
                // While stopped the playhead is the insert marker.
                if !self.s.transport.with_untracked(|t| t.playing) {
                    self.s.arrangement.update(|a| a.insert.0 = p as u32);
                }
                self.s.transport.update(|t| t.playhead_ticks = p);
            }
            AppEvent::Tap => self.tap(),
            AppEvent::EditBpm(x, y, w) => {
                self.bpm_rename_at = (*x, *y, *w);
                self.open_rename(edit::RenameTarget::Bpm);
            }
            AppEvent::SetLaneVolume(lane, db) => {
                // Polish 2 §3: the track volume in dB (a post-chip gain; the
                // engine hears it from `sync_gains`). Undo groups a drag.
                let db = if *db == f32::NEG_INFINITY { *db } else { db.clamp(state::VOL_DB_MIN_TYPED, state::VOL_DB_MAX) };
                if self.s.arrangement.with_untracked(|a| a.lanes.get(*lane).is_some_and(|l| l.vol_db != db)) {
                    self.s.arrangement.update(|a| a.lanes[*lane].vol_db = db);
                    self.sync_gains();
                }
            }
            AppEvent::SetLanePan(lane, pan) => {
                let pan = (*pan).clamp(-state::PAN_MAX, state::PAN_MAX);
                if self.s.arrangement.with_untracked(|a| a.lanes.get(*lane).is_some_and(|l| l.pan != pan)) {
                    self.s.arrangement.update(|a| a.lanes[*lane].pan = pan);
                }
            }
            AppEvent::ShowProblems(x, y) => self.open_context_menu(edit::MenuSite::Problems, *x, *y),
            AppEvent::HoverHelp(t) => {
                if self.s.hover_help.with_untracked(|h| h != t) {
                    if dev::enabled() && !t.is_empty() {
                        eprintln!("dev: help {t}");
                    }
                    self.s.hover_help.set(t.clone());
                }
            }
            AppEvent::SetLaneHeight(lane, h) => {
                let h = state::clamp_row_h(*h);
                if self.s.arrangement.with_untracked(|a| a.lanes.get(*lane).is_some_and(|l| l.height != h)) {
                    self.s.arrangement.update(|a| a.lanes[*lane].height = h);
                    if dev::enabled() {
                        eprintln!("dev: row height {lane} {h}");
                    }
                }
            }
            AppEvent::SetSpectrumStyle(st) => {
                self.settings.spectrum_style = st.key().into();
                self.save_settings();
                if dev::enabled() {
                    eprintln!("dev: spectrum style {}", st.key());
                }
            }
            AppEvent::TogglePreviewOnSelect => {
                let on = !self.settings.preview_on_select;
                self.settings.preview_on_select = on;
                self.save_settings();
            }
            AppEvent::SetBitDepth(b) => {
                self.s.project_format.update(|f| f.1 = *b);
                self.update_dirty();
                self.log_format();
            }
            AppEvent::SetProjectRate(r) => {
                self.s.project_format.update(|f| f.0 = *r);
                self.update_dirty();
                self.log_format();
            }
            AppEvent::SetDial(i, v) => {
                let lane = self.s.lane_sel.get_untracked();
                // A locked dial is inert (the view ignores input too).
                if self.s.dial_state.with_untracked(|d| d.get(*i).is_some_and(|d| d.locked.is_none())) {
                    self.set_param(lane, *i, *v);
                }
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
                        // Drawing on an unused macro starts one: 16 neutral steps.
                        if q.len == 0 {
                            let neutral = views::instrument::NEUTRAL[*which.min(&3)];
                            *q = chip::instrument::Seq::new(&[neutral; views::instrument::NEW_SEQ_LEN], chip::instrument::NONE, chip::instrument::NONE);
                            i.edited = true;
                        }
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
            AppEvent::ToggleBrowser => {
                self.s.browser_open.update(|o| *o = !*o);
                self.settings.browser_open = self.s.browser_open.get_untracked();
                self.save_settings();
            }
            AppEvent::BrowserSelect(p) => self.s.browser_sel.set(*p),
            AppEvent::Audition(p) => self.audition(*p),
            AppEvent::AuditionBrief(p) => {
                self.audition(Some(*p));
                self.audition_until = self.audition_lane.map(|_| Instant::now() + AUDITION_BRIEF);
            }
            AppEvent::AssignPreset { lane, preset } => self.assign_preset(*lane, *preset),
            AppEvent::AssignPresetToClip { clip, preset } => self.assign_preset_to_clip(*clip, *preset),
            AppEvent::MakeInstrumentUnique => self.make_instrument_unique(),
            AppEvent::UseLaneInstrument => self.use_lane_instrument(),
            AppEvent::SaveUserPreset => self.save_user_preset(),
            AppEvent::ApplyLayout(i) => self.apply_layout(cx, *i),
            AppEvent::MenuCmd(c) => self.run_menu_cmd(cx, *c),
            AppEvent::Run(a) => {
                self.run_action(cx, *a, true);
            }
            AppEvent::OpenModal(m) => self.open_modal(*m),
            AppEvent::CloseModal => self.s.modal.set(None),
            AppEvent::SetSetting(v) => self.set_setting(cx, v.clone()),
            AppEvent::SetProfile(chip) => self.set_profile(cx, *chip),
            AppEvent::SetFilter(p) => self.set_filter(p.0),
            AppEvent::EditorFocus(c) => self.editor = *c,
            AppEvent::OpenRecent(i) => self.open_recent(cx, *i),
            AppEvent::ToggleMute(i) => {
                self.s.mute.update(|m| *m ^= 1 << *i);
                let m = self.s.mute.get_untracked();
                self.send(Command::Mute(m));
            }
            AppEvent::ToggleSolo(i) => {
                // Exclusive solo; Ctrl / Cmd-click adds or removes (R143, T5).
                let add = crate::input::command(cx.modifiers());
                let bit = 1u8 << *i;
                self.s.solo.update(|m| *m = if add || *m & bit != 0 { *m ^ bit } else { bit });
                let m = self.s.solo.get_untracked();
                self.send(Command::Solo(m));
            }
            AppEvent::CycleFilter => {
                let all = FilterMode::ALL;
                let i = all.iter().position(|m| *m == self.filter).unwrap_or(0);
                self.set_filter(Some(all[(i + 1) % all.len()]));
            }
            AppEvent::SetPianoPaneHeight(h) => {
                self.s.piano_pane_h.set_if_changed(*h);
                self.clamp_lane_scroll();
            }
            AppEvent::SetLoop(start, end) => self.set_song_loop(*start, *end, false),
            AppEvent::CloseMenu => {
                self.restore_colours();
                self.s.menu.set(None);
            }
            AppEvent::ColourPreview(a) => self.preview_colour(*a),
            AppEvent::Menu(action) => {
                if !matches!(action, MenuAction::ClipColor(_) | MenuAction::TrackColor(..)) {
                    self.restore_colours();
                }
                if !matches!(action, MenuAction::Sub(_) | MenuAction::Separator) {
                    self.s.menu.set(None);
                    self.run_menu_action(cx, *action);
                }
            }
            AppEvent::ContextMenu(site, x, y) => self.open_context_menu(*site, *x, *y),
            AppEvent::OpenSub(sub, x, y) => self.open_sub(*sub, *x, *y),
            AppEvent::RenameText(t) => self.rename_text = t.clone(),
            AppEvent::RenameCommit(next) => self.rename_commit(*next),
            AppEvent::RenameSubmit(target, text) => {
                if self.s.rename.with_untracked(|r| r.as_ref().is_some_and(|r| r.target == *target)) {
                    self.rename_text = text.clone();
                    self.rename_commit(false);
                }
            }
            AppEvent::RenameCancel => self.s.rename.set(None),
            AppEvent::OpenRename(t) => self.open_rename(*t),
            AppEvent::SelectTrack(l, add) => self.select_track(*l, *add),
            AppEvent::SetInsert(t, l) => {
                self.set_insert(*t, *l);
                self.s.arrangement.update(|a| a.track_sel.clear());
            }
            AppEvent::CloseGuide => self.s.guide.set(false),
            AppEvent::Status(m) => self.set_status(m.clone()),
            AppEvent::ScrollLanes(dy) => {
                let v = self.s.lane_scroll.get_untracked() + dy;
                self.s.lane_scroll.set_if_changed(v);
                self.clamp_lane_scroll();
                if dev::enabled() {
                    eprintln!("dev: lane scroll {:.0} max {:.0}", self.s.lane_scroll.get_untracked(), self.s.lane_scroll_max.get_untracked());
                }
            }
            AppEvent::ToggleLoop => {
                self.s.loop_on.update(|l| *l = !*l);
                let end = self.s.transport.with_untracked(|t| t.loop_end_ticks);
                self.send(Command::LoopEnd(self.engine_loop_end(end)));
                self.commit();
            }
            AppEvent::ToggleStats => {
                self.s.stats.update(|s| s.visible = !s.visible);
                eprintln!("dev: F3 overlay {}", if self.s.stats.with_untracked(|s| s.visible) { "on" } else { "off" });
            }
            AppEvent::CycleFrameCap => {
                self.cap = self.cap.next();
                self.apply_frame_cap(cx);
            }
            AppEvent::ToggleStress => {
                let on = !self.s.stress.get();
                self.s.stress.set(on);
                eprintln!("dev: F4 stress set {}", if on { "on (200 clips)" } else { "off (demo song)" });
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
                if self.ask_save(cx, "starting a new project") == Proceed::Stop {
                    return;
                }
                self.load_project(Project::new_empty());
                self.s.file_path.set(String::new());
                self.mark_saved();
                self.set_status("New project".into());
            }
            AppEvent::SaveProject => {
                self.save_project();
            }
            // File > New from MIDI File: replaces the song (until the model
            // change adds tracks instead), so it asks to save first.
            AppEvent::ImportMidi => {
                if self.ask_save(cx, "replacing the song with a MIDI file") == Proceed::Stop {
                    return;
                }
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
            AppEvent::ExportWav => {
                if self.s.export_dlg.with_untracked(|d| d.is_none()) {
                    let ts = self.s.time_sel.get_untracked().map(|t| (t.start, t.end));
                    let mut d = views::export_dialog::ExportDialog::new(ts.is_some());
                    // E4: the last choices.
                    d.restore(&self.settings.export, ts);
                    // The project's format is the export default (design pass §3.2).
                    let (rate, bits) = self.s.project_format.get_untracked();
                    d.opts.rate = rate;
                    d.opts.bits = bits;
                    self.s.export_dlg.set(Some(d));
                }
            }
            AppEvent::ExportPick(p) => {
                let sel = self.s.time_sel.get_untracked().map(|t| (t.start, t.end));
                self.s.export_dlg.update(|d| {
                    if let Some(d) = d {
                        d.pick(*p, sel);
                    }
                });
            }
            AppEvent::ExportGroup(lane) => self.s.export_dlg.update(|d| {
                if let Some(d) = d {
                    d.group_of[*lane] = (d.group_of[*lane] + 1) % 4;
                }
            }),
            AppEvent::ExportGo => self.export_go(),
            AppEvent::ExportCancel => {
                if let Some((c, _)) = &self.export_job {
                    c.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            AppEvent::ExportClose => {
                if self.export_job.is_none() {
                    if let Some(d) = self.s.export_dlg.get_untracked() {
                        self.settings.export = d.remembered();
                        self.save_settings();
                    }
                    self.s.export_dlg.set(None);
                }
            }
            AppEvent::SaveProjectAs => {
                self.save_project_as();
            }
            AppEvent::OpenProject => {
                if self.ask_save(cx, "opening another project") == Proceed::Stop {
                    return;
                }
                let Some(path) = self.file_dialog().pick_file() else { return };
                self.open_project_file(&path);
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
                            // Recovered work isn't saved anywhere yet.
                            self.mark_unsaved();
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
                // A project or MIDI file replaces the song: ask first.
                let replaces = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| ["mid", "midi", file::EXTENSION].contains(&e.to_ascii_lowercase().as_str()));
                if replaces && self.ask_save(cx, "opening the dropped file") == Proceed::Stop {
                    return;
                }
                self.open_dropped(&path);
            }
            if let WindowEvent::WindowClose = we {
                if !self.quitting {
                    let choice = self.ask_save(cx, "closing");
                    if choice == Proceed::Stop {
                        // Cancel: the window stays (models see the event
                        // before the window view that would close it).
                        meta.consume();
                        return;
                    }
                    self.quitting = true;
                    self.end_session(choice == Proceed::Discard);
                }
            }
            if let WindowEvent::KeyDown(code, _) = we {
                let mods = *cx.modifiers();
                // F-03 / dialogs: while a dialog is open, or a text field is
                // being edited, no shortcut reaches the document.
                let ctx = if self.s.modal.with_untracked(|m| m.is_some()) || self.s.export_dlg.with_untracked(|d| d.is_some()) {
                    Ctx::Modal
                } else if self.text_edit.is_some_and(|e| e == cx.focused()) {
                    Ctx::Text
                } else if self.piano_active() {
                    Ctx::PianoRoll
                } else {
                    Ctx::Arrangement
                };
                if let Some(a) = shortcuts::lookup(*code, &mods, ctx) {
                    if self.run_action(cx, a, false) {
                        meta.consume();
                    }
                }
                if ctx == Ctx::Modal {
                    meta.consume();
                }
            }
        });

        // F-03: which text field is being edited (vizia's Textbox announces
        // it; the events bubble up to here).
        event.map(|te: &TextEvent, meta| match te {
            TextEvent::StartEdit => self.text_edit = Some(meta.origin),
            TextEvent::EndEdit if self.text_edit == Some(meta.origin) => self.text_edit = None,
            _ => {}
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
        self.scale = cx.scale_factor();
        let win = cx.cache.get_bounds(Entity::root());
        self.window_logical = (win.w / self.scale, win.h / self.scale);
        // The lane viewport follows the window and the inspector.
        self.clamp_lane_scroll();
        self.tests.maybe_stall();
        self.idle_housekeeping();
        self.install_menus(cx);
        self.sync_shell(cx);
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
            // `CHIPDAW_PLAYHEAD=<ticks>` (dev): the playhead stays put while
            // playing (the polish pass's regression screenshots).
            if let Some(p) = dev::var("CHIPDAW_PLAYHEAD").and_then(|v| v.parse::<f64>().ok()) {
                tr.playhead_ticks = p;
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
        // `CHIPDAW_FROZEN_METERS=1` (dev): levels, lights and the spectrum stay
        // at rest (regression screenshots, performance attribution).
        if !dev::var("CHIPDAW_FROZEN_METERS").is_some_and(|v| v == "1") {
            meters.set_if_changed(m);
        }

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
        self.audition_until = None;
        if let Some(lane) = self.audition_lane.take() {
            self.send(Command::Preview { lane: lane as u8, note: None });
        }
        let Some(p) = preset else { return };
        // The browser's Preview key (on by default).
        if !self.settings.preview_on_select {
            if dev::enabled() {
                eprintln!("dev: audition preset {p} off (Preview key)");
            }
            return;
        }
        if self.s.transport.with_untracked(|t| t.playing) {
            // B2: no preview voice yet; say why nothing plays.
            return self.set_status("Presets play on click while the song is stopped: the channels are busy playing it. Stop (Space) to hear this one.".into());
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
        if dev::enabled() {
            eprintln!("dev: audition preset {p} lane {lane}");
        }
    }

    /// Loads preset `p` on a lane, with its channel controls; one undo step.
    fn assign_preset(&mut self, lane: Option<usize>, p: usize) {
        let Some(pr) = self.preset(p) else { return };
        if lane.is_some_and(|l| state::lane_off(self.s.chip.get_untracked(), l)) {
            return self.set_status(state::VRC6_OFF_HINT.into());
        }
        let Some(lane) = self.lane_for(p, lane) else {
            self.set_status(format!("{} can't play on that channel.", pr.name));
            return;
        };
        self.load_preset_on(lane, pr.inst, pr.params);
        self.commit();
        let name = self.s.arrangement.with_untracked(|a| a.lanes[lane].name.clone());
        self.set_status(format!("{} loaded on {name}", pr.name));
    }

    /// A preset dropped on a clip becomes that clip's own instrument (a new
    /// table entry). The lane's channel controls stay as they are: they
    /// belong to the lane, not the clip. One undo step.
    fn assign_preset_to_clip(&mut self, clip: u32, p: usize) {
        let Some(pr) = self.preset(p) else { return };
        let Some(lane) = self.s.arrangement.with_untracked(|a| a.clip(clip).map(|c| c.lane)) else { return };
        if state::lane_off(self.s.chip.get_untracked(), lane) {
            return self.set_status(state::VRC6_OFF_HINT.into());
        }
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

    /// A channel layout is a template (UX.md decision 5): it loads the
    /// lanes' instruments, and asks before it changes the hardware profile.
    fn apply_layout(&mut self, cx: &mut EventContext, i: usize) {
        let Some(l) = chip::presets::LAYOUTS.get(i) else { return };
        let want = if l.vrc6 { SoundChip::WithVrc6 } else { SoundChip::Base2A03 };
        let cur = self.s.chip.get_untracked();
        if cur != want {
            let silent = if want == SoundChip::Base2A03 { self.vrc6_clip_count() } else { 0 };
            let extra = if silent > 0 {
                format!("\n\nWith 2A03 alone, the {silent} clip(s) on the VRC6 lanes are dimmed and silent until you switch back.")
            } else {
                String::new()
            };
            let text = format!(
                "The layout \"{}\" is made for {}. This project uses {}.\n\nSwitch the hardware profile to {}?{extra}",
                l.name,
                want.label(),
                cur.label(),
                want.label()
            );
            let switch = format!("Switch to {}", want.label());
            let keep = format!("Keep {}", cur.label());
            match self.ask(cx, &text, &switch, Some(&keep)) {
                Some(true) => self.set_chip(want),
                Some(false) => {}
                None => return,
            }
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
        // Clips on lanes that just went silent leave the selection (F-04).
        if chip == SoundChip::Base2A03 {
            self.s.arrangement.update(|a| {
                let off: Vec<u32> = a.clips.iter().filter(|c| state::lane_off(chip, c.lane)).map(|c| c.id).collect();
                a.selection.retain(|id| !off.contains(id));
            });
        }
        let on = chip == SoundChip::WithVrc6;
        self.send(Command::Vrc6(on));
        if !self.filter_manual {
            self.filter = FilterMode::default_for(on);
            self.send(Command::Filter(self.filter));
            self.s.filter_label.set(self.filter.label());
            self.s.filter_state.set((self.filter, true));
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

    fn log_format(&self) {
        if dev::enabled() {
            let (r, b) = self.s.project_format.get_untracked();
            eprintln!("dev: project format {r} {b}");
        }
    }

    /// Sends the track volumes to the engine when they changed (a fader
    /// drag, a typed value, undo, a project load).
    fn sync_gains(&mut self) {
        let g = self.s.arrangement.with_untracked(|a| a.gains());
        if g != self.sent_gains {
            self.sent_gains = g;
            self.send(Command::LaneGains(g));
        }
    }

    /// One TAP: the tempo from the last four intervals; a sequence is one
    /// undo step (the first change commits, later ones amend it).
    fn tap(&mut self) {
        let now = Instant::now();
        let t = self.tap.tap(now);
        if t.restarted {
            self.tap_step = None;
        }
        let Some(bpm) = t.bpm else { return };
        let bpm = (bpm * 100.0).round() / 100.0;
        let bpm = bpm.clamp(Transport::MIN_BPM, Transport::MAX_BPM);
        self.send(Command::Bpm(bpm as f64));
        self.s.transport.update(|t| t.bpm = bpm);
        let d = self.doc();
        match self.tap_step {
            // Nothing else was committed since this sequence's step.
            Some(steps) if self.history.steps() == steps => {
                self.history.amend(d);
            }
            _ => {
                self.history.commit(d);
                self.tap_step = Some(self.history.steps());
            }
        }
        self.update_dirty();
        if dev::enabled() {
            eprintln!("dev: tap bpm {bpm:.2} undo steps {:?}", self.history.steps());
        }
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
        self.poll_export();
        if self.audition_until.is_some_and(|t| Instant::now() >= t) {
            self.audition(None);
        }
        // MEM: the process's private memory, once a second (idle stays idle).
        if self.last_mem.is_none_or(|t| t.elapsed() >= Duration::from_secs(1)) {
            self.last_mem = Some(Instant::now());
            let frozen = dev::var("CHIPDAW_FROZEN_METERS").is_some_and(|v| v == "1");
            self.s.mem_mb.set_if_changed(if frozen { Some(100) } else { perf::process_memory().map(|b| (b / (1024 * 1024)) as u32) });
        }
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
        if let Some(l) = self.s.arrangement.with_untracked(|a| if a.selection.is_empty() { a.track_sel.first().copied() } else { None }) {
            if l != self.s.lane_sel.get_untracked() {
                self.s.dials.set(self.dial_values(l));
                self.s.lane_sel.set(l);
            }
        }
        let any = self.s.arrangement.with_untracked(|a| !a.selection.is_empty() || !a.track_sel.is_empty());
        self.s.inspector.set_if_changed(any);
        // Which dials an instrument overrides (cheap; set only on change).
        let lane = self.s.lane_sel.get_untracked();
        let st = self.s.arrangement.with_untracked(|a| state::dial_states(lane, &self.params, a.instruments.get(a.current_instrument(lane))));
        self.s.dial_state.set_if_changed(st);
        self.sync_gains();
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
            let mut moved_to = None;
            if running && generation > self.audio_generation {
                if self.audio_generation > 0 {
                    let dev = e.device_info();
                    moved_to = Some(dev.split(" · ").next().unwrap_or("?").to_string());
                }
                self.audio_generation = generation;
            }
            // Persistent problems stay on the status bar until resolved
            // (polish 2 §6.4): no device, a device that won't open, and
            // dropouts (until 10 s pass without one).
            let msg: String = match sh.device_error.load(Relaxed) {
                _ if running => String::new(),
                engine::device_error::NO_DEVICE => "Audio: no output device. Connect one or pick another in Settings ▸ Audio.".into(),
                engine::device_error::REBUILDING => "Audio: the device changed; reconnecting.".into(),
                engine::device_error::BUILD_FAILED => "Audio: can't open the output device. Pick another in Settings ▸ Audio.".into(),
                _ => "Audio: starting.".into(),
            };
            if stats.underruns > self.underruns_seen.0 {
                self.underruns_seen = (stats.underruns, Some(now));
            }
            let dropouts = self.underruns_seen.1.is_some_and(|t| now.duration_since(t) < DROPOUT_HOLD);
            let mut problems = Vec::new();
            // Dev hook: `CHIPDAW_FAKE_PROBLEM=text` shows a persistent problem
            // (the status bar's screenshots and tests).
            if let Some(t) = dev::var("CHIPDAW_FAKE_PROBLEM") {
                problems.push(t);
            }
            if !msg.is_empty() {
                problems.push(msg.clone());
            }
            if dropouts && running {
                problems.push(format!("Audio dropouts: {} buffer underruns so far. A larger buffer (Settings ▸ Audio) helps.", self.underruns_seen.0));
            }
            self.s.problems.set_if_changed(problems);
            let _ = &self.device_notice;
            // The stats change on every refresh and only the F3 overlay shows
            // them: publishing them while it is hidden would draw a frame every
            // 250 ms and keep the pump fast, so the app would never go idle.
            if self.s.stats.with_untracked(|s| s.visible) {
                self.s.audio.set_if_changed(stats);
            }
            self.s.audio_msg.set_if_changed(msg);
            // A non-urgent message: where the sound went after a device change.
            if let Some(name) = moved_to {
                self.set_status(format!("Audio: now playing on {name}"));
            }
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
        let _ = custom;
        self.frame_log.log(ft, &spans, playhead);

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
/// How long an arrow-key audition in the browser sounds.
const AUDITION_BRIEF: Duration = Duration::from_millis(600);
/// Pump interval while nothing animates (meter decay etc. still settles).
const IDLE_PUMP: Duration = Duration::from_millis(50);

/// A tooltip (I4, F-34): every control says what it does on hover, and a
/// disabled one says why.
pub fn tip<'a>(cx: &'a mut Context, text: &str) -> Handle<'a, Tooltip> {
    let text = text.to_string();
    let long = text.chars().count() > TIP_LONG;
    Tooltip::new(cx, move |cx| {
        Label::new(cx, text.clone()).class("tip-text").toggle_class("long", long);
    })
    .class("tip")
}

/// A wrapper that carries a control's tooltip: vizia never hovers a
/// disabled view, so a disabled button's "why" goes on this instead (I4).
/// The status bar's hover help for a view (polish 2 §6.5): one line while
/// the mouse is over it.
pub trait HelpExt {
    fn help(self, text: impl Into<String>) -> Self;
}

impl<V: View> HelpExt for Handle<'_, V> {
    fn help(self, text: impl Into<String>) -> Self {
        let t: String = text.into();
        self.on_hover(move |cx| cx.emit(AppEvent::HoverHelp(t.clone()))).on_hover_out(|cx| cx.emit(AppEvent::HoverHelp(String::new())))
    }
}

pub fn tip_wrap<'a>(cx: &'a mut Context, content: impl FnOnce(&mut Context)) -> Handle<'a, HStack> {
    HStack::new(cx, content).width(Auto).height(Auto).class("tip-wrap")
}

/// A tooltip longer than this (characters) wraps at a fixed width.
const TIP_LONG: usize = 40;

/// How long the pointer rests before a tooltip shows.
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

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
    cx.emit(EnvironmentEvent::SetTooltipDelay(TOOLTIP_DELAY));

    // The default chip; the default output filter follows it.
    let chip = SoundChip::WithVrc6;
    let mut settings = file::load_settings();
    // Dev hook: `CHIPDAW_OVERVIEW=0` starts with the Overview hidden (the
    // earlier scenario suites' coordinates; not saved unless changed).
    if dev::var("CHIPDAW_OVERVIEW").is_some_and(|v| v == "0") {
        settings.show_overview = false;
    }
    let s = Signals {
        // Opens with nothing selected: the full-height arrangement of the prototype.
        arrangement: Signal::new(state::Arrangement {
            selection: Vec::new(),
            snap: Snap::from_label(&settings.arrangement_grid),
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
        piano: Signal::new(PianoState {
            grid: notes::Grid::from_label(&settings.piano_grid),
            follow: settings.follow_playhead,
            ..PianoState::default()
        }),
        piano_open: Signal::new(false),
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
        inspector: Signal::new(false),
        browser_open: Signal::new(settings.browser_open),
        browser_query: Signal::new(String::new()),
        browser_sel: Signal::new(None),
        drag_preset: Signal::new(None),
        user_presets: Signal::new(user_presets::load()),
        export_dlg: Signal::new(None),
        dirty: Signal::new(false),
        modal: Signal::new(None),
        settings: Signal::new(settings.clone()),
        audio_devices: Signal::new(Vec::new()),
        audio_info: Signal::new(String::new()),
        filter_state: Signal::new((FilterMode::default_for(chip == SoundChip::WithVrc6), true)),
        recovery_msg: Signal::new(""),
        rename: Signal::new(None),
        guide: Signal::new(false),
        header: Signal::new(views::header_layout::HeaderLayout::of(&settings)),
        hover_help: Signal::new(String::new()),
        problems: Signal::new(Vec::new()),
        project_format: Signal::new((file::DEFAULT_PROJECT_RATE, file::DEFAULT_BIT_DEPTH)),
        mem_mb: Signal::new(if dev::var("CHIPDAW_FROZEN_METERS").is_some_and(|v| v == "1") { Some(100) } else { perf::process_memory().map(|b| (b / (1024 * 1024)) as u32) }),
    };

    // Dev hook: a persistent problem shown from the start (also kept while
    // the engine reports its own; see `sync_engine`).
    if let Some(t) = dev::var("CHIPDAW_FAKE_PROBLEM") {
        s.problems.set(vec![t]);
    }

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
        cap: FrameCap::from_setting(settings.frame_cap),
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
        export_job: None,
        audition_lane: None,
        text_edit: None,
        editor: Ctx::Arrangement,
        saved: (Doc::of(&s.arrangement.get_untracked(), 150.0, instruments::default_params()), Extras {
            chip,
            filter: FilterMode::default_for(true),
            filter_auto: true,
            loop_start: 0,
            loop_end: 0,
            loop_on: true,
            mute: 0,
            solo: 0,
            format: (file::DEFAULT_PROJECT_RATE, file::DEFAULT_BIT_DEPTH),
        }),
        quitting: false,
        title: String::new(),
        settings,
        #[cfg(feature = "menus")]
        menus: None,
        menus_tried: false,
        audio_switch: None,
        clip_board: None,
        zoom_back: Vec::new(),
        stop_point: 0.0,
        rename_text: String::new(),
        preset_rename_at: (0.0, 0.0),
        scale: 1.0,
        window_logical: (MIN_WINDOW_SIZE.0 as f32, MIN_WINDOW_SIZE.1 as f32),
        audition_until: None,
        colour_backup: None,
        last_mem: None,
        tap: tap::TapTempo::default(),
        tap_step: None,
        bpm_rename_at: (0.0, 0.0, 0.0),
        sent_gains: [1.0; 8],
        underruns_seen: (0, None),
    };
    match state.recovery.as_ref().and_then(|r| r.start_session()) {
        Some(file::LastSession::Crashed) => {
            state.s.recovery_msg.set("Last session didn't close cleanly");
            state.s.recovery_offer.set(true);
        }
        Some(file::LastSession::UnsavedQuit) => {
            state.s.recovery_msg.set("Unsaved changes from your last session");
            state.s.recovery_offer.set(true);
        }
        None => {}
    }
    state.s.filter_label.set(state.filter.label());
    state.s.filter_state.set((state.filter, !state.filter_manual));
    state.start_engine();
    state.apply_test_project();
    state.mark_saved();
    // The start state is the first undo state (nothing to undo yet).
    let d = state.doc();
    state.history.reset(d);
    state.build(cx);

    VStack::new(cx, |cx| {
        top_bar(cx, s);
        // The main area (polish 2 §5): browser, track headers and lanes in
        // the order Settings ▸ Look & Feel picks; rebuilt when it changes.
        let order = Memo::new(move |_| s.settings.with(|st| (st.headers_right, st.browser_right, st.show_overview)));
        Binding::new(cx, order, move |cx| {
            let (headers_right, browser_right, overview) = order.get();
            HStack::new(cx, |cx| {
                let browser_side = |cx: &mut Context| {
                    Element::new(cx)
                        .class("col-divider")
                        .display(s.browser_open.map(|o| if *o { Display::Flex } else { Display::None }));
                };
                if !browser_right {
                    browser(cx, s);
                    browser_side(cx);
                }
                if !headers_right {
                    track_headers(cx, s, overview);
                }
                VStack::new(cx, |cx| {
                    if overview {
                        views::overview::Overview::new(cx, s);
                    }
                    Ruler::new(cx, s).height(Pixels(theme::tokens::RULER_HEIGHT));
                    ArrangementView::new(cx, s);
                })
                .width(Stretch(1.0))
                .overflow(Overflow::Hidden);
                if headers_right {
                    track_headers(cx, s, overview);
                }
                if browser_right {
                    browser_side(cx);
                    browser(cx, s);
                }
            })
            .class("main")
            .height(Memo::new(move |_| {
                if s.piano_open.get() {
                    // Piano roll open: the arrangement gets the height the split
                    // handle sets (its lanes scroll) and the piano roll the rest.
                    let ov = if s.settings.with(|st| st.show_overview) { theme::tokens::OVERVIEW_HEIGHT } else { 0.0 };
                    Pixels(theme::tokens::RULER_HEIGHT + ov + s.piano_pane_h.get())
                } else {
                    Stretch(1.0)
                }
            }))
            .overflow(Overflow::Hidden);
        });
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
        // The status bar along the window's bottom (polish 2 §6).
        if status_bar_shown() {
            views::statusbar::StatusBar::new(cx, s);
        }
        notice(cx, s);
        views::rename::build(cx, s);
        views::export_dialog::build(cx, s);
        views::modal::build(cx, s);
        // Menus open above the modal panels too (Settings' colour grid).
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
        let e = Engine::start(self.engine_config());
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

/// Smallest window (logical px):
/// - width: the inspector's fixed sections (dials, channels) plus the
///   smallest CLIP section and the panel's padding and gaps, or the top
///   bar's three groups if wider (1296: the top bar never clips or touches);
/// - height: 720 (polish pass): the lanes scroll, so a 1080p laptop at 125%
///   (1536×816 logical without the taskbar) fits; five lanes still show
///   above the inspector.
pub const MIN_WINDOW_SIZE: (u32, u32) = (
    // The inspector (two paddings and two gaps), but never narrower than
    // the top bar's fixed controls need.
    max_u32(
        INSPECTOR_PAD * 4 + DIALS_SECTION_W as u32 + CLIP_SECTION_MIN_W + INSTRUMENT_SECTION_MIN_W as u32,
        TOPBAR_MIN_W,
    ),
    MIN_WINDOW_H,
);
/// The height the Overview and the status bar take from the lanes (polish
/// 2 §6, §7), as shown now.
pub fn chrome_bands(s: &Signals) -> f32 {
    let st = s.settings.with_untracked(|st| st.show_overview);
    (if st { theme::tokens::OVERVIEW_HEIGHT } else { 0.0 }) + if status_bar_shown() { theme::tokens::STATUS_BAR_HEIGHT } else { 0.0 }
}

/// The status bar is shown unless the dev hook `CHIPDAW_HIDE_STATUS=1`
/// hides it (the regression gate's screenshots only).
pub fn status_bar_shown() -> bool {
    !dev::var("CHIPDAW_HIDE_STATUS").is_some_and(|v| v == "1")
}

/// The minimum window height (logical px, on the 4 px grid).
const MIN_WINDOW_H: u32 = 720;
/// The device inspector's height, padding and gap; must match `.device` in style.css.
const INSPECTOR_HEIGHT: u32 = 232;
const INSPECTOR_PAD: u32 = 16;
/// Inspector section widths (logical px). Dials and channels are fixed so
/// they never squeeze; the CLIP section takes what's left.
const DIALS_SECTION_W: f32 = MAX_DIALS as f32 * DIAL_W + (MAX_DIALS - 1) as f32 * DIAL_GAP + 24.0;
const CLIP_SECTION_MIN_W: u32 = 240;
/// The INSTRUMENT section's smallest width (its four graphs stay readable).
const INSTRUMENT_SECTION_MIN_W: f32 = 320.0;
// 320, not 480: the room the CHANNELS meters left goes to the instrument editor.
const CLIP_SECTION_MAX_W: f32 = 320.0;
/// The narrowest window the top bar's three groups fit with room between
/// them: keys 297 + display and TAP 628 + spectrum and CPU/MEM 312 + 2 × 16
/// padding = 1269, plus the groups' own 12 px gap on each side, on the 4 px
/// grid (polish pass: at 1272 they touched).
const TOPBAR_MIN_W: u32 = 1296;

const fn max_u32(a: u32, b: u32) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}

/// The open context menu (and its open submenu) over a full-window
/// backdrop that closes it.
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
        Binding::new(cx, s.menu, move |cx| {
            let Some(m) = s.menu.get() else { return };
            let sub = m.sub.clone();
            ContextMenu::new(cx, ContextMenuSpec { sub: None, ..m }, false);
            if let Some(sb) = sub {
                ContextMenu::new(cx, *sb, true);
            }
        });
    });
}

/// A notice longer than this (characters) wraps in a fixed-width capsule.
const NOTICE_LONG: usize = 96;
/// The notice's distance from the window's bottom-right corner (logical px).
const NOTICE_MARGIN: f32 = 16.0;
/// How long a notice stays, and an import report (F-37: long enough to read).
/// A status-bar message stays this long (polish 2 §6.2), then the idle text.
const STATUS_SECS: u64 = 8;
const REPORT_SECS: u64 = 30;

/// Width of the recovery prompt's keys (logical px).
const NOTICE_KEY_W: f32 = 64.0;

/// Messages of varying length (save status, audio device problems, the crash
/// recovery prompt) float in a capsule under the transport instead of living
/// in the top bar, where they would push the fixed-size controls around.
fn notice(cx: &mut Context, s: Signals) {
    // Polish 2 §6: messages and problems live on the status bar; the capsule
    // is left to the recovery offer, a decision. With the status bar hidden
    // (the regression gate's dev hook) messages still show here.
    let bar = status_bar_shown();
    let visible = Memo::new(move |_| s.recovery_offer.get() || (!bar && (!s.status.get().is_empty() || !s.audio_msg.get().is_empty())));
    Binding::new(cx, visible, move |cx| {
        if !visible.get() {
            return;
        }
        HStack::new(cx, |cx| {
            if !bar {
                Label::new(cx, s.audio_msg)
                    .class("notice-text")
                    .toggle_class("hidden", s.audio_msg.map(|m| m.is_empty()));
                Label::new(cx, s.status)
                    .class("notice-text")
                    .toggle_class("long", s.status.map(|m| m.chars().count() > NOTICE_LONG))
                    .toggle_class("hidden", s.status.map(|m| m.is_empty()));
            }
            Binding::new(cx, s.recovery_offer, move |cx| {
                if s.recovery_offer.get() {
                    Label::new(cx, s.recovery_msg).class("notice-text");
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
        // F-37: docked bottom-right, clear of the ruler and the first bars;
        // a click dismisses it.
        .on_press(|cx| cx.emit(AppEvent::Status(String::new())))
        .position_type(PositionType::Absolute)
        .left(Stretch(1.0))
        .right(Pixels(NOTICE_MARGIN))
        .top(Stretch(1.0))
        .bottom(Pixels(NOTICE_MARGIN + if bar { theme::tokens::STATUS_BAR_HEIGHT } else { 0.0 }));
    });
}

/// The top bar (VISUAL-SPEC.md §2.2): view and transport keys on the left,
/// the display and TAP in the middle, the spectrum and CPU / MEM on the
/// right, the three groups spaced between.
fn top_bar(cx: &mut Context, s: Signals) {
    use theme::tokens::{SPECTRUM_W, WELL_H};
    HStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                TopKeyView::new(cx, s, TopKey::Arrangement).help("Arrangement view");
                TopKeyView::new(cx, s, TopKey::Mixer).help("Mixer view: arrives in Batch 4");
            })
            .class("key-group");
            Element::new(cx).class("bar-divider");
            HStack::new(cx, |cx| {
                TopKeyView::new(cx, s, TopKey::Play).help(format!("Play ({}) · the display shows what's playing", shortcuts::label(Action::PlayStop)));
                TopKeyView::new(cx, s, TopKey::Stop).help(format!("Stop ({}) · press again to go back to the start ({})", shortcuts::label(Action::PlayStop), shortcuts::label(Action::Rewind)));
                TopKeyView::new(cx, s, TopKey::Record).help("Record: arrives with MIDI input");
                TopKeyView::new(cx, s, TopKey::Loop).help("Loop: plays the song between the loop brace's ends (the ruler)");
            })
            .class("key-group");
        })
        .class("bar-group");
        Element::new(cx).class("bar-spacer");
        HStack::new(cx, |cx| {
            DisplayWell::new(cx, s).help("Display: time, position, selection start and length, tempo · drag or wheel the BPM (Shift: finer) · double-click it to type");
            TopKeyView::new(cx, s, TopKey::Tap).help(format!("Tap tempo ({}): the average of the last four taps · taps more than 2 s apart start over", shortcuts::label(Action::Tap)));
        })
        .class("bar-group");
        Element::new(cx).class("bar-spacer");
        HStack::new(cx, |cx| {
            SpectrumBox::new(cx, s)
                .width(Pixels(SPECTRUM_W))
                .height(Pixels(WELL_H))
                .help("Spectrum of the mix · right-click for its styles");
            CpuMem::new(cx, s).help("CPU: the sound engine's time per buffer, as a share of it (OFF: no output running) · MEM: this app's private memory");
        })
        .class("bar-group");
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
            let title = Memo::new(move |_| {
                let lane = s.lane_sel.get();
                let name = s.arrangement.with(|a| a.lanes.get(lane).map(|l| l.name.to_uppercase()).unwrap_or_default());
                let hw = state::default_lanes().get(lane).map(|l| l.name.to_uppercase()).unwrap_or_default();
                if name == hw { format!("CHANNEL  ·  {name}") } else { format!("CHANNEL  ·  {name}  ·  {hw}") }
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
                        Dial::new(cx, s, slot, spec).class("dial").tooltip(move |cx| tip(cx, &dial_tip(s, slot, spec.label)));
                    }
                })
                .gap(Pixels(DIAL_GAP))
                .height(Auto)
                .width(Auto);
            });
        })
        .class("device-section")
        .width(Pixels(DIALS_SECTION_W));

        // The clip: its name (renamed in place, UX.md §4) and the clip's
        // instrument commands as visible buttons (I3; disabled ones say why).
        VStack::new(cx, |cx| {
            Label::new(cx, "CLIP").class("section-title");
            let name = s.arrangement.map(|a| a.first_selected().map(|c| c.name.clone()).unwrap_or_else(|| "No clip selected".into()));
            Label::new(cx, name).class("clip-name");
            Label::new(cx, format!("Rename it in place: {} or right-click > Rename.", shortcuts::label(Action::Rename))).class("field-label").class("clip-hint");
            let st = Memo::new(move |_| {
                s.arrangement.with(|a| {
                    let clips: Vec<&state::Clip> = a.selection.iter().filter_map(|id| a.clip(*id)).collect();
                    let none = clips.is_empty();
                    let unique = !none && clips.iter().all(|c| c.instrument.is_some_and(|i| a.instruments.users(i, &a.clips) == 1));
                    let track = !none && clips.iter().all(|c| c.instrument.is_none());
                    (none, unique, track)
                })
            });
            HStack::new(cx, |cx| {
                tip_wrap(cx, |cx| {
                    Button::new(cx, |cx| Label::new(cx, "Make Unique"))
                        .class("btn")
                        .disabled(st.map(|(none, unique, _)| *none || *unique))
                        .on_press(|cx| cx.emit(AppEvent::MakeInstrumentUnique));
                })
                    .tooltip(move |cx| {
                        let (none, unique, _) = st.get_untracked();
                        tip(cx, if none { "Select a clip: Make Unique gives it its own copy of the instrument." } else if unique { edit::WHY_NOT_UNIQUE } else { "Give the selected clip(s) their own copy of this instrument, so editing it changes only them." })
                    });
                tip_wrap(cx, |cx| {
                    Button::new(cx, |cx| Label::new(cx, "Use Track Instrument"))
                        .class("btn")
                        .disabled(st.map(|(none, _, track)| *none || *track))
                        .on_press(|cx| cx.emit(AppEvent::UseLaneInstrument));
                })
                    .tooltip(move |cx| {
                        let (none, _, track) = st.get_untracked();
                        tip(cx, if none { "Select a clip that has its own instrument." } else if track { edit::WHY_TRACK_INST } else { "The selected clip(s) go back to the track's instrument." })
                    });
            })
            .class("inst-actions");
        })
        .class("device-section")
        .width(Stretch(1.0))
        .min_width(Pixels(CLIP_SECTION_MIN_W as f32))
        .max_width(Pixels(CLIP_SECTION_MAX_W));

        // The lane's instrument: its macros, editable.
        VStack::new(cx, |cx| {
            // Tracks both the arrangement and the selected lane: a lane change
            // alone (the selection moving to another lane) must update it too.
            let title = Memo::new(move |_| {
                let lane = s.lane_sel.get();
                s.arrangement.with(|a| {
                    let id = a.current_instrument(lane);
                    let name = a.instruments.get(id).map(|i| i.label()).unwrap_or_default();
                    let whose = if id == a.instruments.lane_id(lane) { "TRACK DEFAULT" } else { "CLIP'S OWN" };
                    let n = a.instruments.users(id, &a.clips);
                    let shared = if n == 1 { "1 CLIP".to_string() } else { format!("{n} CLIPS") };
                    format!("INSTRUMENT  ·  {}  ·  {whose}  ·  {shared}", name.to_uppercase())
                })
            });
            HStack::new(cx, |cx| {
                Label::new(cx, title).class("section-title");
                Element::new(cx).width(Stretch(1.0)).height(Pixels(1.0));
                Button::new(cx, |cx| Label::new(cx, "Save as Preset"))
                    .class("btn")
                    .on_press(|cx| cx.emit(AppEvent::SaveUserPreset))
                    .tooltip(|cx| tip(cx, "Save this instrument to your presets (the browser lists them after the factory presets)."));
            })
            .class("inst-actions");
            InstrumentView::new(cx, s)
                .width(Stretch(1.0))
                .height(Stretch(1.0))
                .tooltip(|cx| tip(cx, "Drag across a graph to draw the macro. Shift-click sets the loop point, Alt-click the release point; double-click a marker to remove it. Drag the vibrato values up or down."));
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

/// A dial's tooltip: what it is, and why it's inert when it is (I4).
fn dial_tip(s: Signals, slot: usize, label: &str) -> String {
    match s.dial_state.with_untracked(|d| d.get(slot).and_then(|d| d.locked)) {
        Some(why) => format!("{label}: inert, {why}. The instrument's macro drives it; clear that macro to use the dial."),
        None => format!("{label}: drag up or down (Shift for fine steps), or use the wheel."),
    }
}

/// The piano roll's key hint, from the shortcut table and the platform's
/// drag modifiers (F-25: never hard-coded key names).
fn piano_hint() -> String {
    let (copy, no_snap) = if shortcuts::MACOS { ("Option", "Cmd") } else { ("Ctrl", "Alt") };
    format!(
        "Double-click add / delete · {} draw · {copy}-drag copy · {no_snap} no snap · Esc close",
        shortcuts::label(Action::DrawMode)
    )
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
            // F-39: room between the title and the controls.
            Element::new(cx).width(Pixels(16.0)).height(Pixels(1.0));
            let draw = s.piano.map(|p| p.draw);
            Button::new(cx, |cx| Label::new(cx, "Draw"))
                .class("btn")
                .toggle_class("checked", draw)
                .on_press(|cx| cx.emit(AppEvent::Run(Action::DrawMode)))
                .tooltip(|cx| tip(cx, &format!("Draw Mode ({}): a click draws a note, a click on a note erases it. Off: a click selects, a drag selects several, a double-click adds or deletes a note.", shortcuts::label(Action::DrawMode))));
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
            Label::new(cx, piano_hint()).class("field-label");
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
        self.s.arrangement.with_untracked(|a| Doc::of(a, bpm, self.params)).with_song(self.song_state())
    }

    /// The undoable song settings (F-33).
    fn song_state(&self) -> history::SongState {
        let x = self.extras();
        history::SongState { chip: x.chip, loop_start: x.loop_start, loop_end: x.loop_end, loop_on: x.loop_on }
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
            self.update_dirty();
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
        self.update_dirty();
    }

    /// Puts a document into the model and the engine (through the usual
    /// snapshot path: the content key changes, `sync_engine` sends it).
    fn apply_doc(&mut self, d: &Doc) {
        self.s.arrangement.update(|a| d.apply_to(a));
        let now = self.song_state();
        if now.chip != d.song.chip {
            self.set_chip(d.song.chip);
        }
        if (now.loop_start, now.loop_end) != (d.song.loop_start, d.song.loop_end) {
            self.set_song_loop(d.song.loop_start as f64, d.song.loop_end as f64, false);
        }
        if now.loop_on != d.song.loop_on {
            self.s.loop_on.set(d.song.loop_on);
            let end = self.s.transport.with_untracked(|t| t.loop_end_ticks);
            self.send(Command::LoopEnd(self.engine_loop_end(end)));
        }
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
            rate: self.s.project_format.get_untracked().0,
            bits: self.s.project_format.get_untracked().1,
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
        self.s.project_format.set((p.rate, p.bits));
        self.log_format();
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
        self.s.filter_label.set(self.filter.label());
        self.s.filter_state.set((self.filter, p.filter_auto));
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
            e if e == file::EXTENSION => self.open_project_file(path),
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
        // A new song nobody has saved yet.
        self.mark_unsaved();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        self.set_status_for(format!("Imported {name}: {}", imp.report), REPORT_SECS);
    }

    /// Asks where to save (the base name of every file), then renders on a
    /// worker thread: never on the UI thread, never on the audio thread.
    fn export_go(&mut self) {
        let Some(dlg) = self.s.export_dlg.get_untracked() else { return };
        if self.export_job.is_some() {
            return;
        }
        self.settings.export = dlg.remembered();
        self.save_settings();
        let cur = file::with_extension(&self.s.file_path.get_untracked());
        let stem = cur.file_stem().and_then(|n| n.to_str()).filter(|n| !n.is_empty()).unwrap_or("song");
        let mut d = rfd::FileDialog::new().add_filter("WAV audio", &["wav"]).set_file_name(format!("{stem}.wav"));
        if let Some(dir) = cur.parent().filter(|d| d.is_dir()) {
            d = d.set_directory(dir);
        }
        // `CHIPDAW_EXPORT_TO=path.wav` (scripted UI tests): no native dialog.
        let path = match crate::dev::var_os("CHIPDAW_EXPORT_TO") {
            Some(p) => std::path::PathBuf::from(p),
            None => match d.save_file() {
                Some(p) => p,
                None => return,
            },
        };
        let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let base = path.file_stem().and_then(|n| n.to_str()).unwrap_or("song").to_string();
        let project = self.project();
        let opts = dlg.options();
        let ctl = std::sync::Arc::new(game_export::Control::default());
        let c2 = ctl.clone();
        let t0 = Instant::now();
        let handle = std::thread::Builder::new()
            .name("export".into())
            .spawn(move || {
                game_export::export(&project, &opts, &dir, &base, &c2).map(|mut r| {
                    r.notes.push(format!("{} file(s) in {:.1} s", r.files.len(), t0.elapsed().as_secs_f32()));
                    r
                })
            })
            .expect("export thread");
        self.export_job = Some((ctl, handle));
        self.s.export_dlg.update(|d| {
            if let Some(d) = d {
                d.running = Some(0);
                d.result.clear();
            }
        });
    }

    /// Pump tick: export progress, and the result when the worker is done.
    fn poll_export(&mut self) {
        let Some((ctl, h)) = &self.export_job else { return };
        let pm = ctl.permille.load(std::sync::atomic::Ordering::Relaxed);
        if !h.is_finished() {
            // Redraw on whole-percent changes only.
            if self.s.export_dlg.with_untracked(|d| d.as_ref().and_then(|d| d.running)).map(|p| p / 10) != Some(pm / 10) {
                self.s.export_dlg.update(|d| {
                    if let Some(d) = d {
                        d.running = Some(pm);
                    }
                });
            }
            return;
        }
        let Some((_, h)) = self.export_job.take() else { return };
        let msg = match h.join() {
            Ok(Ok(r)) => {
                let first = r.files.first().map(|f| f.display().to_string()).unwrap_or_default();
                format!("Exported {} ({}). {}", first, r.files.len(), r.notes.join(" · "))
            }
            Ok(Err(e)) => e,
            Err(_) => "The export failed.".into(),
        };
        self.s.export_dlg.update(|d| {
            if let Some(d) = d {
                d.running = None;
                d.result = msg.clone();
            }
        });
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

    /// Saves to the current path (asks for one if there is none). Returns
    /// whether the project was saved.
    fn save_project(&mut self) -> bool {
        if self.s.file_path.get_untracked().trim().is_empty() {
            match self.file_dialog().save_file() {
                Some(path) => self.s.file_path.set(path.display().to_string()),
                None => return false,
            }
        }
        let text = self.s.file_path.get_untracked();
        let path = file::with_extension(&text);
        self.commit();
        match file::save(&self.project(), &path) {
            Ok(()) => {
                let shown = path.display().to_string();
                self.s.file_path.set(shown.clone());
                self.mark_saved();
                self.add_recent(&shown);
                self.set_status(format!("Saved {shown}"));
                true
            }
            Err(e) => {
                self.set_status(e);
                false
            }
        }
    }

    /// Save As: a native save dialog, then save there.
    fn save_project_as(&mut self) -> bool {
        match self.file_dialog().save_file() {
            Some(path) => {
                self.s.file_path.set(path.display().to_string());
                self.save_project()
            }
            None => false,
        }
    }

    /// Opens a project file (the caller already asked to save).
    fn open_project_file(&mut self, path: &std::path::Path) {
        match file::load(path) {
            Ok(p) => {
                self.load_project(p);
                let shown = path.display().to_string();
                self.s.file_path.set(shown.clone());
                self.mark_saved();
                self.add_recent(&shown);
                self.set_status(format!("Opened {shown}"));
            }
            Err(e) => self.set_status(e),
        }
    }

    fn set_status(&mut self, msg: String) {
        self.set_status_for(msg, STATUS_SECS);
    }

    fn set_status_for(&mut self, msg: String, secs: u64) {
        self.s.status.set(msg);
        self.status_until = Some(Instant::now() + Duration::from_secs(secs));
    }

    fn open_piano(&mut self, id: u32) {
        let chip = self.s.chip.get_untracked();
        if self.s.arrangement.with_untracked(|a| a.clip(id).is_some_and(|c| state::lane_off(chip, c.lane))) {
            return self.set_status(state::VRC6_OFF_HINT.into());
        }
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
        self.editor = Ctx::PianoRoll;
        // Bring the clip's lane into view if it's scrolled out.
        if let Some(lane) = self.s.arrangement.with_untracked(|a| a.clip(id).map(|c| c.lane)) {
            let (top, bottom) = self.s.arrangement.with_untracked(|a| (a.lane_top(lane), a.lane_top(lane + 1)));
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
        self.editor = Ctx::Arrangement;
        self.clamp_lane_scroll();
    }

    /// Keeps the lane scroll inside what the lane viewport allows. The lanes
    /// always scroll when they don't fit (polish pass): under the piano
    /// roll's split, or in a short window (above the inspector, if open).
    fn clamp_lane_scroll(&mut self) {
        let content = self.s.arrangement.with_untracked(|a| a.lanes_height());
        let viewport = if self.s.piano_open.get_untracked() {
            self.s.piano_pane_h.get_untracked()
        } else {
            let inspector = if self.s.inspector.get_untracked() { INSPECTOR_HEIGHT as f32 } else { 0.0 };
            self.window_logical.1 - theme::tokens::TOPBAR_HEIGHT - theme::tokens::RULER_HEIGHT - inspector - chrome_bands(&self.s)
        };
        let max = (content - viewport).max(0.0);
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
        // The arrangement pages along too (F-32: one Follow for both editors).
        if p.follow {
            let (v, w) = self.s.arrangement.with_untracked(|a| (a.view, a.view_w as f64));
            let x = v.tick_to_x(playhead);
            if x < 0.0 || x > w * 0.95 {
                let to = (playhead - v.px_to_ticks(w * 0.05)).max(0.0);
                self.s.arrangement.update(|a| a.view.scroll_ticks = to);
            }
        }
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
        let every = Duration::from_secs(self.settings.autosave_secs as u64);
        if self.settings.autosave_secs > 0 && now - self.last_autosave >= every && !self.s.recovery_offer.get_untracked() {
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
        let opened_arg = std::env::args_os().skip(1).any(|a| std::path::Path::new(&a).is_file());
        if self.settings.open_last && !opened_arg && crate::dev::var("CHIPDAW_PROJECT").is_none() {
            if let Some(last) = self.settings.recent.first().cloned().filter(|p| std::path::Path::new(p).is_file()) {
                self.open_project_file(std::path::Path::new(&last));
            }
        }
        if let Some(path) = crate::dev::var("CHIPDAW_PROJECT") {
            match file::load(std::path::Path::new(&path)) {
                Ok(p) => {
                    let n = p.arrangement.clips.len();
                    self.load_project(p);
                    eprintln!("dev: CHIPDAW_PROJECT {path} ({n} clips)");
                    self.s.file_path.set(path);
                    self.mark_saved();
                }
                Err(e) => eprintln!("CHIPDAW_PROJECT: {e}"),
            }
        }
        if let Some(id) = crate::dev::var("CHIPDAW_OPEN_CLIP").and_then(|v| v.parse().ok()) {
            self.open_piano(id);
            eprintln!("dev: CHIPDAW_OPEN_CLIP {id} (piano roll open: {})", self.s.piano_open.get_untracked());
        }
        // `CHIPDAW_PIANO_ZOOM=px per bar` (the piano-roll benchmark shows
        // all 2,000 notes at 32).
        if let Some(ppb) = crate::dev::var("CHIPDAW_PIANO_ZOOM").and_then(|v| v.parse::<f64>().ok()) {
            let ppb = ppb.clamp(PianoState::MIN_PX_PER_BAR, PianoState::MAX_PX_PER_BAR);
            self.s.piano.update(|p| p.px_per_tick = ppb / TICKS_PER_BAR as f64);
        }
        // `CHIPDAW_PX_PER_BAR=100`: the arrangement's zoom (the design pass's
        // pixel comparison uses the mockup's 100 px per bar).
        if dev::enabled() {
            eprintln!("dev: spectrum style {} (settings)", views::chrome::SpectrumStyle::from_key(&self.settings.spectrum_style).key());
        }
        if let Some(ppb) = crate::dev::var("CHIPDAW_PX_PER_BAR").and_then(|v| v.parse::<f64>().ok()) {
            self.s.arrangement.update(|a| {
                a.view.px_per_tick = ppb / TICKS_PER_BAR as f64;
                a.view.scroll_ticks = 0.0;
            });
            eprintln!("dev: CHIPDAW_PX_PER_BAR {ppb}");
        }
    }
}

// ───────────────────────────── App shell (Batches 1 and 2) ─────────────────────────────

impl AppState {
    /// The audio configuration from Settings (a dev-mode `CHIPDAW_BUFFER`
    /// overrides the buffer for the benchmarks).
    fn engine_config(&self) -> EngineConfig {
        EngineConfig {
            sample_rate: self.settings.sample_rate,
            buffer_frames: self.tests.buffer.or(self.settings.buffer_frames),
            device_name: self.settings.audio_device.clone(),
        }
    }

    fn extras(&self) -> Extras {
        let tr = self.s.transport.get_untracked();
        let (loop_start, loop_end) = if self.clip_looping {
            (self.loop_start_saved, self.loop_end_saved)
        } else {
            (tr.loop_start_ticks, tr.loop_end_ticks)
        };
        Extras {
            chip: self.s.chip.get_untracked(),
            filter: self.filter,
            filter_auto: !self.filter_manual,
            loop_start: loop_start as u32,
            loop_end: loop_end as u32,
            loop_on: self.s.loop_on.get_untracked(),
            mute: self.s.mute.get_untracked(),
            solo: self.s.solo.get_untracked(),
            format: self.s.project_format.get_untracked(),
        }
    }

    /// The document as it is now is the saved one (New, Open, Save).
    fn mark_saved(&mut self) {
        let d = self.doc();
        self.history.commit(d.clone());
        self.saved = (d, self.extras());
        self.update_dirty();
    }

    /// Nothing on disk matches the document (a recovered session, a song
    /// made from a MIDI file).
    fn mark_unsaved(&mut self) {
        // No real document has a negative tempo.
        self.saved.0 = Doc::of(&state::Arrangement::empty(), -1.0, instruments::default_params());
        self.update_dirty();
    }

    /// F-01: dirty when the last committed document or the saved extras
    /// differ from the saved ones. Undoing back to the saved state is clean
    /// again (snapshots compare by content).
    fn update_dirty(&mut self) {
        let dirty = !self.history.current().same_as(&self.saved.0) || self.extras() != self.saved.1;
        self.s.dirty.set_if_changed(dirty);
    }

    /// The project's name for the title bar and prompts.
    fn project_name(&self) -> String {
        let p = self.s.file_path.get_untracked();
        std::path::Path::new(p.trim())
            .file_stem()
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty())
            .unwrap_or("Untitled")
            .to_string()
    }

    /// A native question with two or three buttons (rfd; Windows task
    /// dialog). `Some(true)`: the first button; `Some(false)`: the second
    /// (only when `second` is given); `None`: Cancel or closed.
    fn ask(&mut self, cx: &mut EventContext, text: &str, first: &str, second: Option<&str>) -> Option<bool> {
        use rfd::{MessageButtons, MessageDialogResult as R};
        let buttons = match second {
            Some(s) => MessageButtons::YesNoCancelCustom(first.into(), s.into(), "Cancel".into()),
            None => MessageButtons::OkCancelCustom(first.into(), "Cancel".into()),
        };
        let mut d = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title("ChipDAW")
            .set_description(text)
            .set_buttons(buttons);
        if let Some(w) = cx.window() {
            d = d.set_parent(&*w);
        }
        // The dialog's own loop runs on this thread; the audio keeps going.
        let r = d.show();
        self.last_activity = Instant::now();
        match r {
            R::Yes | R::Ok => Some(true),
            R::No => second.map(|_| false),
            R::Custom(c) if c == first => Some(true),
            R::Custom(c) if second == Some(c.as_str()) => Some(false),
            _ => None,
        }
    }

    /// F-01: before something replaces or closes the document, ask "Save /
    /// Don't Save / Cancel" if it has unsaved changes. No prompt when clean.
    fn ask_save(&mut self, cx: &mut EventContext, doing: &str) -> Proceed {
        // An edit still in flight counts.
        self.commit();
        self.update_dirty();
        if !self.s.dirty.get_untracked() {
            return Proceed::Clean;
        }
        let name = self.project_name();
        let text = format!("Do you want to save the changes to \"{name}\" before {doing}?\n\nIf you don't save, your changes will be lost.");
        match self.ask(cx, &text, "Save", Some("Don't Save")) {
            Some(true) => {
                if self.save_project() {
                    Proceed::Clean
                } else {
                    Proceed::Stop
                }
            }
            Some(false) => Proceed::Discard,
            None => Proceed::Stop,
        }
    }

    /// The window is closing: autosave once more and end the session. After
    /// Don't Save with "Offer recovery after an unsaved quit" on, the
    /// recovery file stays and is offered at the next start.
    fn end_session(&mut self, discarded: bool) {
        let Some(r) = self.recovery.as_ref() else { return };
        // A recovery still on offer (never answered) is kept as it is.
        if self.s.recovery_offer.get_untracked() {
            return;
        }
        if discarded && self.settings.offer_recovery {
            r.end_session_unsaved(&self.project());
        } else {
            r.end_session(&self.project());
        }
    }

    /// Clips on the VRC6 lanes (silent under the 2A03 profile).
    fn vrc6_clip_count(&self) -> usize {
        self.s.arrangement.with_untracked(|a| a.clips.iter().filter(|c| c.lane >= chip::driver::LANE_V1).count())
    }

    /// Project Settings: the hardware profile. Switching VRC6 off while its
    /// lanes have clips asks first (F-04, principle 5).
    fn set_profile(&mut self, cx: &mut EventContext, chip: SoundChip) {
        if chip == self.s.chip.get_untracked() {
            return;
        }
        let n = self.vrc6_clip_count();
        if chip == SoundChip::Base2A03 && n > 0 {
            let text = format!(
                "Switch the hardware profile to 2A03?\n\nThe VRC6 lanes have {n} clip(s). They stay in the project, dimmed and silent, until you switch back to 2A03 + VRC6."
            );
            if self.ask(cx, &text, "Switch to 2A03", None) != Some(true) {
                return;
            }
        }
        self.set_chip(chip);
        self.commit();
    }

    /// Project Settings: the output filter (`None`: follow the profile).
    fn set_filter(&mut self, m: Option<FilterMode>) {
        match m {
            None => {
                self.filter_manual = false;
                self.filter = FilterMode::default_for(self.s.chip.get_untracked() == SoundChip::WithVrc6);
            }
            Some(m) => {
                self.filter_manual = true;
                self.filter = m;
            }
        }
        self.send(Command::Filter(self.filter));
        self.s.filter_label.set(self.filter.label());
        self.s.filter_state.set((self.filter, !self.filter_manual));
        self.last_audio_stats = None;
        self.update_dirty();
    }

    fn save_settings(&mut self) {
        file::save_settings(&self.settings);
        self.s.settings.set(self.settings.clone());
    }

    fn add_recent(&mut self, path: &str) {
        self.settings.add_recent(path);
        self.save_settings();
    }

    /// File > Open Recent > n. A file that has gone is removed, with a notice.
    fn open_recent(&mut self, cx: &mut EventContext, i: usize) {
        let Some(path) = self.settings.recent.get(i).cloned() else { return };
        if !std::path::Path::new(&path).is_file() {
            self.settings.recent.remove(i);
            self.save_settings();
            self.set_status(format!("{path} was moved or deleted, so it was removed from Open Recent."));
            return;
        }
        if self.ask_save(cx, "opening another project") == Proceed::Stop {
            return;
        }
        self.open_project_file(std::path::Path::new(&path));
    }

    fn open_modal(&mut self, m: Modal) {
        self.s.menu.set(None);
        if m == Modal::Settings(SettingsTab::Audio) {
            self.s.audio_devices.set(Engine::output_devices());
            self.refresh_audio_info(None);
        }
        self.s.modal.set(Some(m));
    }

    fn refresh_audio_info(&mut self, gap_ms: Option<f64>) {
        let info = match self.engine.as_ref() {
            Some(e) => format!("Now playing on {}", e.device_info()),
            None => "Audio is off in this session.".to_string(),
        };
        let info = match gap_ms {
            Some(g) => format!("{info}  ·  last change: {g:.0} ms between callbacks"),
            None => info,
        };
        self.s.audio_info.set(info);
    }

    /// One change from the Settings panel: applied at once and saved.
    fn set_setting(&mut self, cx: &mut EventContext, v: Set) {
        let st = &mut self.settings;
        let mut audio = false;
        match v {
            Set::Device(d) => {
                st.audio_device = d;
                audio = true;
            }
            Set::Buffer(b) => {
                st.buffer_frames = b;
                audio = true;
            }
            Set::Rate(r) => {
                st.sample_rate = r;
                audio = true;
            }
            Set::FrameCap(c) => {
                st.frame_cap = c;
                self.cap = FrameCap::from_setting(c);
                self.apply_frame_cap(cx);
            }
            Set::UiScale(p) => {
                st.ui_scale = p;
                let f = st.ui_scale_factor();
                cx.emit(WindowEvent::SetUserScaleFactor(f));
            }
            Set::DarkMenus(b) => {
                st.dark_menus = b;
                #[cfg(feature = "menus")]
                if let Some(m) = &self.menus {
                    m.set_dark(b);
                }
            }
            Set::ArrangementGrid(g) => {
                let snap = Snap::from_label(&g);
                st.arrangement_grid = g;
                self.s.arrangement.update(|a| a.snap = snap);
            }
            Set::PianoGrid(g) => {
                let grid = notes::Grid::from_label(&g);
                st.piano_grid = g;
                self.s.piano.update(|p| p.grid = grid);
            }
            Set::Follow(f) => {
                st.follow_playhead = f;
                self.s.piano.update(|p| p.follow = f);
            }
            Set::AutoTrackColours(b) => st.auto_track_colours = b,
            Set::ReducedAutoColours(b) => st.reduced_auto_colours = b,
            Set::NewClipColour(v) => st.new_clip_colour = v,
            Set::Autosave(secs) => st.autosave_secs = secs,
            Set::OfferRecovery(b) => st.offer_recovery = b,
            Set::OpenLast(b) => st.open_last = b,
            Set::Unsilence(id) => st.silenced.retain(|x| *x != id),
            Set::UnsilenceAll => st.silenced.clear(),
            Set::HeadersRight(b) => st.headers_right = b,
            Set::StripByClips(b) => st.strip_by_clips = b,
            Set::BrowserRight(b) => st.browser_right = b,
            Set::ShowOverview(b) => st.show_overview = b,
            Set::HeaderWidth(w) => st.header_width = views::header_layout::clamp_width(w),
        }
        self.s.header.set_if_changed(views::header_layout::HeaderLayout::of(&self.settings));
        if dev::enabled() {
            let st = &self.settings;
            eprintln!("dev: look headers_right={} strip_by_clips={} browser_right={} overview={} width={}", st.headers_right, st.strip_by_clips, st.browser_right, st.show_overview, st.header_width);
        }
        self.save_settings();
        if audio {
            self.apply_audio();
        }
    }

    /// Settings > Audio changed: switch the running engine through its
    /// rebuild path, and measure the gap a moment later.
    fn apply_audio(&mut self) {
        let cfg = self.engine_config();
        let Some(e) = self.engine.as_mut() else { return };
        e.shared().max_gap_ns.store(0, std::sync::atomic::Ordering::Relaxed);
        if e.reconfigure(cfg) {
            self.audio_switch = Some(Instant::now());
            self.s.audio_load.set((0, 0));
            self.last_load = None;
        }
    }

    /// Runs an action from the keyboard (`menu: false`) or a menu (`true`).
    /// Returns whether it was handled (the key is then consumed).
    fn run_action(&mut self, cx: &mut EventContext, a: Action, menu: bool) -> bool {
        let piano = self.piano_active();
        if let Some(handled) = self.run_b3(cx, a) {
            return handled;
        }
        match a {
            Action::New => cx.emit(AppEvent::NewProject),
            Action::Open => cx.emit(AppEvent::OpenProject),
            Action::Save => {
                self.save_project();
            }
            Action::SaveAs => {
                self.save_project_as();
            }
            Action::ExportAudio => cx.emit(AppEvent::ExportWav),
            Action::Quit => cx.emit(WindowEvent::WindowClose),
            Action::Undo => self.undo(false),
            Action::Redo => self.undo(true),
            // The Edit menu while the piano roll is the active editor: the
            // piano roll does it (its keys reach it directly when focused).
            Action::Cut | Action::Copy | Action::Paste | Action::Duplicate | Action::Delete | Action::SelectAll | Action::Quantize
                if menu && piano =>
            {
                cx.emit_custom(Event::new(PianoCmd(a)).target(Entity::root()).propagate(Propagation::Subtree));
            }
            Action::Cut | Action::Copy | Action::Paste | Action::Duplicate | Action::Delete | Action::Quantize => return false,
            // Keys: the focused editor handles these itself.
            Action::SelectAll | Action::OpenClip if !menu => return false,
            Action::SelectAll => {
                let chip = self.s.chip.get_untracked();
                self.s.arrangement.update(|a| a.selection = a.clips.iter().filter(|c| !state::lane_off(chip, c.lane)).map(|c| c.id).collect());
            }
            Action::OpenClip => return false,
            Action::Settings => self.open_modal(Modal::Settings(SettingsTab::Audio)),
            Action::InsertClip => self.insert_clips(),
            Action::LoopSelection => self.run_menu_action(cx, MenuAction::Cmd(Action::LoopSelection)),
            Action::PlayStop => cx.emit(AppEvent::TogglePlay),
            Action::Tap => cx.emit(AppEvent::Tap),
            Action::Rewind => {
                let lane = self.s.arrangement.with_untracked(|a| a.insert.1);
                self.set_insert(0, lane);
                if self.s.transport.with_untracked(|t| t.playing) {
                    cx.emit(AppEvent::SetPlayhead(0.0));
                }
            }
            Action::ToggleBrowser => cx.emit(AppEvent::ToggleBrowser),
            Action::ShowOverview => {
                let on = !self.settings.show_overview;
                self.set_setting(cx, Set::ShowOverview(on));
                self.clamp_lane_scroll();
            }
            Action::Follow => cx.emit(AppEvent::TogglePianoFollow),
            Action::Escape => {
                if self.s.menu.with_untracked(|m| m.is_some()) {
                    self.s.menu.set(None);
                } else if self.s.guide.get_untracked() {
                    self.s.guide.set(false);
                } else if self.s.piano_open.get_untracked() {
                    self.close_piano();
                } else {
                    self.s.arrangement.update(|a| {
                        a.selection.clear();
                        a.track_sel.clear();
                    });
                    self.s.time_sel.set(None);
                }
            }
            // Enter in the export dialog exports (E2).
            Action::ConfirmDialog => {
                if self.s.modal.with_untracked(|m| m.is_none()) && self.s.export_dlg.with_untracked(|d| d.is_some()) {
                    self.export_go();
                } else {
                    return false;
                }
            }
            Action::Shorten | Action::Lengthen | Action::PrevNote | Action::NextNote => return false,
            Action::CloseDialog => {
                if self.s.modal.with_untracked(|m| m.is_some()) {
                    self.s.modal.set(None);
                } else if self.export_job.is_none() {
                    cx.emit(AppEvent::ExportClose);
                }
            }
            Action::TransposeUp
            | Action::TransposeDown
            | Action::OctaveUp
            | Action::OctaveDown
            | Action::NudgeLeft
            | Action::NudgeRight
            | Action::CutTime
            | Action::CopyTime
            | Action::PasteTime
            | Action::DuplicateTime
            | Action::DeleteTime
            | Action::InsertSilence
            | Action::Split
            | Action::Consolidate
            | Action::Rename
            | Action::Continue
            | Action::ZoomIn
            | Action::ZoomOut
            | Action::ZoomToSelection
            | Action::ZoomBack
            | Action::ZoomToFit
            | Action::FitSong
            | Action::DrawMode
            | Action::GridNarrow
            | Action::GridWiden
            | Action::GridTriplet
            | Action::GridSnap
            | Action::GridAdaptive
            | Action::MarkerLeft
            | Action::MarkerRight
            | Action::TrackUp
            | Action::TrackDown
            | Action::GettingStarted
            | Action::TextConfirm
            | Action::TextCancel
            | Action::TextNext
            | Action::TextSelectAll
            | Action::TextCopy
            | Action::TextCut
            | Action::TextPaste => return false,
            Action::DevStats => cx.emit(AppEvent::ToggleStats),
            Action::DevStress => cx.emit(AppEvent::ToggleStress),
            Action::DevReloadStyles => cx.emit(WindowEvent::ReloadStyles),
            Action::DevFrameCap => cx.emit(AppEvent::CycleFrameCap),
            Action::DevFilter => cx.emit(AppEvent::CycleFilter),
        }
        true
    }

    fn run_menu_cmd(&mut self, cx: &mut EventContext, c: menus::Cmd) {
        use menus::Cmd;
        match c {
            Cmd::Action(a) => {
                self.run_action(cx, a, true);
            }
            Cmd::NewFromMidi => cx.emit(AppEvent::ImportMidi),
            Cmd::ImportAudio => cx.emit(AppEvent::ImportAudio),
            Cmd::ProjectSettings => self.open_modal(Modal::ProjectSettings),
            Cmd::OpenRecent(i) => self.open_recent(cx, i),
            Cmd::ClearRecent => {
                self.settings.recent.clear();
                self.save_settings();
            }
            Cmd::About => self.open_modal(Modal::About),
            Cmd::ShortcutSheet => self.open_modal(Modal::Shortcuts),
        }
    }

    /// What the menu bar shows (enabled items, check marks, Open Recent).
    fn menu_state(&self) -> menus::MenuState {
        let (can_undo, can_redo) = (self.history.can_undo(), self.history.can_redo());
        let piano = self.s.piano_open.get_untracked() && self.editor == Ctx::PianoRoll;
        let notes_selected = self.s.piano.with_untracked(|p| !p.sel.is_empty());
        let clips = self.s.arrangement.with_untracked(|a| !a.selection.is_empty());
        menus::MenuState {
            can_undo,
            can_redo,
            piano,
            notes_selected,
            can_paste: if piano { !self.clipboard.is_empty() } else { self.clip_board.is_some() },
            selection: if piano { notes_selected } else { clips },
            time_selection: self.s.time_sel.with_untracked(|t| t.is_some()),
            browser: self.s.browser_open.get_untracked(),
            follow: self.s.piano.with_untracked(|p| p.follow),
            can_paste_time: self.clip_board.is_some(),
            range: self.time_range().is_some(),
            can_rename: self.s.arrangement.with_untracked(|a| a.selection.len() == 1 || (a.selection.is_empty() && a.track_sel.len() == 1)),
            can_split: if piano { notes_selected } else { self.time_range().is_some() || self.s.arrangement.with_untracked(|a| a.clips.iter().any(|c| c.lane == a.insert.1 && c.start_ticks < a.insert.0 && c.end_ticks() > a.insert.0)) },
            can_consolidate: if piano { notes_selected } else { self.s.time_sel.with_untracked(|t| t.is_some()) || self.s.arrangement.with_untracked(|a| a.selection.len() > 1) },
            can_zoom_back: if piano { true } else { !self.zoom_back.is_empty() },
            draw: self.s.piano.with_untracked(|p| p.draw),
            overview: self.settings.show_overview,
            recent: self
                .settings
                .recent
                .iter()
                .map(|p| {
                    let path = std::path::Path::new(p);
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or(p);
                    let dir = path.parent().map(|d| d.display().to_string()).unwrap_or_default();
                    if shortcuts::MACOS {
                        format!("{name} — {dir}")
                    } else {
                        format!("{name}\t{dir}")
                    }
                })
                .collect(),
        }
    }

    /// Once, on the first tick with a window: the native menu bar.
    #[cfg(feature = "menus")]
    fn install_menus(&mut self, cx: &mut EventContext) {
        if self.menus_tried {
            return;
        }
        let Some(win) = cx.window() else { return };
        self.menus_tried = true;
        // Missing files leave Open Recent, with a notice.
        let gone = self.settings.prune_recent();
        if !gone.is_empty() {
            self.save_settings();
            self.set_status(format!("Removed {} missing project(s) from Open Recent.", gone.len()));
        }
        let m = menus::NativeMenus::build(&self.menu_state());
        m.set_dark(self.settings.dark_menus);
        // Menu commands arrive on the UI thread (muda's window subclass) and
        // go through the usual event path.
        let proxy = std::sync::Mutex::new(cx.get_proxy());
        muda::MenuEvent::set_event_handler(Some(std::sync::Arc::new(move |e: muda::MenuEvent| {
            if let Some(c) = menus::Cmd::from_id(&e.id.0) {
                if let Ok(mut p) = proxy.lock() {
                    let _ = p.emit(AppEvent::MenuCmd(c));
                }
            }
        })));
        #[cfg(target_os = "windows")]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            let hwnd = win.window_handle().ok().and_then(|h| match h.as_raw() {
                RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
                _ => None,
            });
            if let Some(hwnd) = hwnd {
                // The bar takes its height from the client area: grow the
                // window by it so the content keeps its size.
                let before = win.inner_size();
                match m.attach_hwnd(hwnd) {
                    Ok(()) => {
                        let _ = win.request_inner_size(before);
                    }
                    Err(e) => eprintln!("menu bar: {e}"),
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            let _ = &win;
            m.attach_nsapp();
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let _ = &win;
        self.menus = Some(m);
    }

    #[cfg(not(feature = "menus"))]
    fn install_menus(&mut self, _cx: &mut EventContext) {}

    /// Per tick: the window title (name, unsaved mark), the menu bar's
    /// state, and the audio switch measurement.
    fn sync_shell(&mut self, cx: &mut EventContext) {
        let mark = if self.s.dirty.get_untracked() { "*" } else { "" };
        let title = format!("{mark}{} — ChipDAW", self.project_name());
        if title != self.title {
            self.title = title.clone();
            cx.emit(WindowEvent::SetTitle(title));
        }
        #[cfg(feature = "menus")]
        if self.menus.is_some() {
            let st = self.menu_state();
            if let Some(m) = self.menus.as_mut() {
                m.update(&st);
            }
        }
        // The gap a switch caused: the largest interval between callbacks
        // since the switch (it includes the silence while rebuilding).
        if let Some(t) = self.audio_switch {
            let running = self.engine.as_ref().is_some_and(|e| e.shared().stream_running.load(std::sync::atomic::Ordering::Relaxed));
            if running && t.elapsed() > Duration::from_millis(1500) {
                self.audio_switch = None;
                let gap = self.engine.as_ref().map_or(0, |e| e.shared().max_gap_ns.load(std::sync::atomic::Ordering::Relaxed));
                let ms = gap as f64 / 1e6;
                eprintln!("audio-switch: gap_ms={ms:.1} config={:?}", self.engine.as_ref().map(|e| e.config().clone()));
                self.refresh_audio_info(Some(ms));
                let dev = self.engine.as_ref().map(|e| e.device_info()).unwrap_or_default();
                self.set_status(format!("Audio: {dev} · switched with a {ms:.0} ms gap"));
            } else if t.elapsed() > Duration::from_secs(10) {
                self.audio_switch = None;
                self.refresh_audio_info(None);
            }
        }
    }
}
