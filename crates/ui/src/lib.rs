//! ChipDAW GUI prototype. A library crate so the same UI can later be hosted
//! inside a plugin window (baseview) as well as the standalone app.

pub mod input;
pub mod mock;
pub mod perf;
pub mod state;
pub mod theme;
pub mod views;

use vizia::prelude::*;

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
    pub dials: Signal<[f32; DIAL_COUNT]>,
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
}

impl Model for AppState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| match e {
            AppEvent::Tick => self.tick(cx),
            AppEvent::TogglePlay => self.s.transport.update(|t| t.playing = !t.playing),
            AppEvent::Stop => self.s.transport.update(|t| {
                if !t.playing {
                    t.playhead_ticks = 0.0;
                }
                t.playing = false;
            }),
            AppEvent::ToggleRecord => self
                .s
                .transport
                .update(|t| t.record_armed = !t.record_armed),
            AppEvent::SetBpm(b) => self
                .s
                .transport
                .update(|t| t.bpm = b.clamp(Transport::MIN_BPM, Transport::MAX_BPM)),
            AppEvent::SetPlayhead(p) => self.s.transport.update(|t| t.playhead_ticks = p.max(0.0)),
            AppEvent::SetDial(i, v) => self.s.dials.update(|d| d[*i] = *v),
            AppEvent::CycleChip => self.s.chip.update(|c| {
                *c = match c {
                    SoundChip::Base2A03 => SoundChip::WithVrc6,
                    SoundChip::WithVrc6 => SoundChip::Base2A03,
                }
            }),
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
        // Re-applies the cap if the window moved to a display with another rate.
        self.apply_frame_cap(cx);
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
            let ticks_per_sec = tr.bpm as f64 / 60.0 * PPQ as f64;
            tr.playhead_ticks += ticks_per_sec * dt as f64;
            if tr.playhead_ticks >= tr.loop_end_ticks {
                tr.playhead_ticks -= tr.loop_end_ticks;
            }
            self.s.transport.set(tr);
        }

        let rng = &mut self.rng;
        let (arr, meters) = (self.s.arrangement, self.s.meters);
        arr.with_untracked(|a| {
            // Only notify (and so redraw) when something actually moved.
            let mut m = meters.get_untracked();
            mock::step_meters(&mut m, a, tr.playhead_ticks, tr.playing, dt, rng);
            meters.set_if_changed(m);
        });

        self.record_frame_stats(ft, tr.playhead_ticks);

        self.s.frame.update(|f| *f += 1);
    }
}

impl AppState {
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
        let fast = playing || now - self.last_activity < STAY_FAST;
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
        dials: Signal::new(DIALS.map(|d| d.default)),
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

    AppState {
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
    }
    .build(cx);

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
            let title = s.arrangement.map(|a| {
                let lane = a.first_selected().map(|c| c.lane).unwrap_or(0);
                format!("DEVICE  ·  {}", a.lanes[lane].name.to_uppercase())
            });
            Label::new(cx, title).class("section-title");
            HStack::new(cx, |cx| {
                for i in 0..DIAL_COUNT {
                    Dial::new(cx, s, i).class("dial");
                }
            })
            .gap(Pixels(DIAL_GAP))
            .height(Auto)
            .width(Auto);
        })
        .class("device-section")
        .width(Pixels(
            DIAL_COUNT as f32 * DIAL_W + (DIAL_COUNT - 1) as f32 * DIAL_GAP + 24.0,
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
