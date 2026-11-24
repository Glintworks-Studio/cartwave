//! ChipDAW GUI prototype. A library crate so the same UI can later be hosted
//! inside a plugin window (baseview) as well as the standalone app.

pub mod input;
pub mod mock;
pub mod perf;
pub mod state;
pub mod testing;
pub mod theme;
pub mod views;

use vizia::prelude::*;

use engine::{Command, Engine, EngineConfig, FilterMode, Params};

use crate::mock::Rng;
use crate::state::*;
use crate::views::{
    arrangement::{track_headers, Arrangement as ArrangementView},
    dial::Dial,
    meter::MeterBank,
    overlay::StatsOverlay,
    ruler::Ruler,
    text_field::{FieldKind, ScrubField},
    transport::{GlyphKind, TransportGlyph},
    visualizer::ChannelLights,
};

/// All reactive application state. Signals are cheap `Copy` handles.
#[derive(Clone, Copy)]
pub struct Signals {
    pub arrangement: Signal<state::Arrangement>,
    pub transport: Signal<Transport>,
    pub meters: Signal<Meters>,
    /// Values of the dial slots shown for `lane_sel`.
    pub dials: Signal<[f32; MAX_DIALS]>,
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
}

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
    note_ons: [u32; 8],
    /// When each lane's last note-on becomes audible (lights wait for it).
    light_due: [Option<Instant>; 8],
    last_audio_stats: Option<Instant>,
    tests: testing::Tests,
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
    ToggleStats,
    ToggleStress,
    CycleFrameCap,
    RenameSelected(String),
    ToggleMute(usize),
    ToggleSolo(usize),
    CycleFilter,
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
                self.set_param(lane, *i, *v);
            }
            AppEvent::CycleChip => {
                self.s.chip.update(|c| {
                    *c = match c {
                        SoundChip::Base2A03 => SoundChip::WithVrc6,
                        SoundChip::WithVrc6 => SoundChip::Base2A03,
                    }
                });
                let on = self.s.chip.get_untracked() == SoundChip::WithVrc6;
                self.send(Command::Vrc6(on));
            }
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
                self.send(Command::Filter(self.filter));
                self.last_audio_stats = None;
            }
            AppEvent::CycleSnap => self.s.arrangement.update(|a| a.snap = a.snap.next()),
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
                if on {
                    self.s.transport.update(|t| t.playing = true);
                }
            }
            AppEvent::RenameSelected(name) => self.s.arrangement.update(|a| {
                if let Some(id) = a.selection.first().copied() {
                    if let Some(c) = a.clip_mut(id) {
                        c.name = name.clone();
                    }
                }
            }),
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
            if let WindowEvent::KeyDown(code, _) = we {
                match code {
                    Code::Space => {
                        cx.emit(AppEvent::TogglePlay);
                        meta.consume();
                    }
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
                Some((t, true)) => tr.playhead_ticks = t,
                _ => {
                    // No audio (or a command in flight): integrate locally.
                    let ticks_per_sec = tr.bpm as f64 / 60.0 * PPQ as f64;
                    tr.playhead_ticks += ticks_per_sec * dt as f64;
                    if tr.playhead_ticks >= tr.loop_end_ticks {
                        tr.playhead_ticks -= tr.loop_end_ticks;
                    }
                }
            }
            self.s.transport.set(tr);
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
            let running = sh.stream_running.load(Relaxed);
            let stats = AudioStats {
                device: e.device_info(),
                running,
                filter: self.filter.label(),
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
            let msg = match sh.device_error.load(Relaxed) {
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
        cx.set_max_frame_rate(self.cap.max_fps(display_hz));
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

    let s = Signals {
        arrangement: Signal::new(mock::demo_arrangement()),
        transport: Signal::new(Transport {
            playing: false,
            record_armed: false,
            bpm: 150.0,
            playhead_ticks: 0.0,
            loop_end_ticks: (32 * TICKS_PER_BAR) as f64,
        }),
        meters: Signal::new(Meters::default()),
        dials: Signal::new([0.0; MAX_DIALS]),
        lane_sel: Signal::new(usize::MAX),
        mute: Signal::new(0),
        solo: Signal::new(0),
        audio: Signal::new(AudioStats::default()),
        audio_msg: Signal::new(String::new()),
        chip: Signal::new(SoundChip::WithVrc6),
        stats: Signal::new(FrameStats::default()),
        frame: Signal::new(0),
        stress: Signal::new(false),
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
        params: Params::default(),
        song_key: 0,
        song_version: 0,
        filter: tests.filter.unwrap_or_default(),
        note_ons: [0; 8],
        light_due: [None; 8],
        last_audio_stats: None,
        tests,
    };
    state.start_engine();
    state.build(cx);

    VStack::new(cx, |cx| {
        transport_bar(cx, s);
        HStack::new(cx, |cx| {
            track_headers(cx, s);
            VStack::new(cx, |cx| {
                Ruler::new(cx, s).height(Pixels(theme::tokens::RULER_HEIGHT));
                ArrangementView::new(cx, s);
            })
            .width(Stretch(1.0))
            .overflow(Overflow::Hidden);
        })
        .class("panel")
        .height(Stretch(1.0))
        .overflow(Overflow::Hidden);
        device_panel(cx, s);
    })
    .padding(Pixels(10.0))
    .gap(Pixels(10.0));

    StatsOverlay::new(cx, s);
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
        }
        let tr = self.s.transport.get_untracked();
        self.send(Command::Bpm(tr.bpm as f64));
        self.send(Command::LoopEnd(tr.loop_end_ticks));
        self.send(Command::Filter(self.filter));
        if let Some(g) = self.tests.gain {
            self.send(Command::Gain(g));
        }
        let vrc6 = self.s.chip.get_untracked() == SoundChip::WithVrc6;
        self.send(Command::Vrc6(vrc6));
        if self.tests.pilot || self.tests.avsync {
            match engine::testkit::Loopback::start(
                self.tests.pilot.then_some(engine::testkit::PILOT_HZ),
            ) {
                Ok(lb) => self.tests.loopback = Some(lb),
                Err(err) => eprintln!("loopback: {err}"),
            }
            if self.tests.pilot {
                self.send(Command::TestPilot(engine::testkit::PILOT_HZ));
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

fn transport_bar(cx: &mut Context, s: Signals) {
    HStack::new(cx, |cx| {
        let play_checked = s.transport.map(|t| t.playing);
        Button::new(cx, |cx| TransportGlyph::new(cx, GlyphKind::Play, s))
            .class("btn")
            .class("icon")
            .toggle_class("checked", play_checked)
            .on_press(|cx| cx.emit(AppEvent::TogglePlay));
        Button::new(cx, |cx| TransportGlyph::new(cx, GlyphKind::Stop, s))
            .class("btn")
            .class("icon")
            .on_press(|cx| cx.emit(AppEvent::Stop));
        let rec = s.transport.map(|t| t.record_armed);
        Button::new(cx, |cx| TransportGlyph::new(cx, GlyphKind::Record, s))
            .class("btn")
            .class("icon")
            .class("record")
            .toggle_class("checked", rec)
            .on_press(|cx| cx.emit(AppEvent::ToggleRecord));

        Element::new(cx).class("divider-v");

        Label::new(cx, "BPM").class("field-label");
        ScrubField::new(cx, s, FieldKind::Bpm).width(Pixels(84.0));

        Label::new(cx, "POS").class("field-label");
        ScrubField::new(cx, s, FieldKind::Position).width(Pixels(112.0));

        Element::new(cx).class("divider-v");

        Label::new(cx, "SNAP").class("field-label");
        let snap = s.arrangement.map(|a| a.snap.label());
        Button::new(cx, |cx| Label::new(cx, snap))
            .class("btn")
            .width(Pixels(60.0))
            .on_press(|cx| cx.emit(AppEvent::CycleSnap));

        Element::new(cx).width(Stretch(1.0)).height(Pixels(1.0));

        ChannelLights::new(cx, s)
            .width(Pixels(220.0))
            .height(Pixels(32.0));

        Element::new(cx).class("divider-v");

        // Only shows text when the audio device has a problem.
        Label::new(cx, s.audio_msg).class("field-label").class("audio-msg");

        Label::new(cx, "CHIP").class("field-label");
        let chip = s.chip.map(|c| format!("{}  \u{25BE}", c.label()));
        Button::new(cx, |cx| Label::new(cx, chip))
            .class("btn")
            .width(Pixels(128.0))
            .on_press(|cx| cx.emit(AppEvent::CycleChip));
    })
    .class("panel")
    .class("transport");
}

/// Dial cell width and spacing (logical px); must match `.dial` in style.css.
const DIAL_W: f32 = 84.0;
const DIAL_GAP: f32 = 6.0;

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
        .width(Pixels(
            MAX_DIALS as f32 * DIAL_W + (MAX_DIALS - 1) as f32 * DIAL_GAP + 24.0,
        ));

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
            Label::new(cx, "SEARCH").class("section-title");
            let query = Signal::new(String::new());
            HStack::new(cx, |cx| {
                Textbox::new(cx, query)
                    .placeholder("Search clips, instruments…")
                    .on_edit(move |_cx, t| query.set(t));
            })
            .class("field");
        })
        .class("device-section")
        .width(Pixels(300.0));

        // Meters
        VStack::new(cx, |cx| {
            Label::new(cx, "CHANNELS").class("section-title");
            MeterBank::new(cx, s)
                .height(Stretch(1.0))
                .width(Stretch(1.0));
        })
        .class("device-section")
        .width(Stretch(1.0));
    })
    .class("panel")
    .class("device");
}
