//! The window chrome from the reference prototype (docs/design/reference.png):
//! keys (M/S and friends), split stat pills, the transport capsule with the
//! time read-out, the spectrum box, and the track rows.
//!
//! Everything is drawn procedurally. The prototype's look lives in its 1 px
//! *gradient strokes* (a centre-bright sheen on the top edge of keys and
//! pills, bright-left / dark-right edges on colour strips), so those are
//! drawn as rings inside each shape with `gfx::ring*` / `gfx::top_sheen`.

use std::cell::{Cell, RefCell};

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::state::{Transport, PPQ};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

/// Logical mouse position relative to the view.
fn local_mouse(cx: &EventContext) -> (f32, f32) {
    let b = cx.bounds();
    let s = cx.scale_factor();
    ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s)
}

fn mouse_inside(cx: &EventContext) -> bool {
    let b = cx.bounds();
    let (x, y) = (cx.mouse().cursor_x, cx.mouse().cursor_y);
    x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h
}

/// A soft glow behind a path or text: the same shape blurred in `color`.
fn glow_paint(color: Rgba, blur: f32) -> vg::Paint {
    let mut p = gfx::fill(color);
    p.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, blur, false));
    p
}

// ───────────────────────────────── Keys ─────────────────────────────────

/// What a key does and what lights it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    Mute(usize),
    Solo(usize),
    Recover,
    Discard,
}

impl KeyAction {
    fn event(self) -> AppEvent {
        match self {
            KeyAction::Mute(i) => AppEvent::ToggleMute(i),
            KeyAction::Solo(i) => AppEvent::ToggleSolo(i),
            KeyAction::Recover => AppEvent::RecoverAccept,
            KeyAction::Discard => AppEvent::RecoverDiscard,
        }
    }
}

/// Glyph size of single-letter keys and of word keys (logical px).
const KEY_LETTER_PX: f32 = 13.0;
const KEY_WORD_PX: f32 = 9.0;

/// A small hardware-style key: lit from below (dark top, lighter bottom),
/// a plain stroke with a centre-bright sheen along the top edge. Lit keys
/// (mute/solo on) get an accent glyph with a soft glow, like the loop light.
pub struct Key {
    s: Signals,
    label: &'static str,
    action: KeyAction,
    pressed: bool,
    hover: bool,
}

impl Key {
    pub fn new<'a>(
        cx: &'a mut Context,
        s: Signals,
        label: &'static str,
        action: KeyAction,
    ) -> Handle<'a, Self> {
        Self {
            s,
            label,
            action,
            pressed: false,
            hover: false,
        }
        .build(cx, |_| {})
        .class("key")
        .cursor(CursorIcon::Hand)
        .bind(s.mute, |mut h| h.needs_redraw())
        .bind(s.solo, |mut h| h.needs_redraw())
    }

    fn lit(&self) -> bool {
        match self.action {
            KeyAction::Mute(i) => self.s.mute.get_untracked() & (1 << i) != 0,
            KeyAction::Solo(i) => self.s.solo.get_untracked() & (1 << i) != 0,
            _ => false,
        }
    }
}

impl View for Key {
    fn element(&self) -> Option<&'static str> {
        Some("key")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.pressed = true;
                cx.capture();
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.pressed => {
                self.pressed = false;
                cx.release();
                if mouse_inside(cx) {
                    cx.emit(self.action.event());
                }
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseEnter => {
                self.hover = true;
                cx.needs_redraw();
            }
            WindowEvent::MouseLeave => {
                self.hover = false;
                cx.needs_redraw();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Chrome);
        let b = cx.bounds();
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        let rr = gfx::rrect(r, RADIUS_KEY * s);
        let lit = self.lit();
        let lift = if self.hover && !self.pressed { 0.05 } else { 0.0 };

        // Body: lit from below; pressed keys flatten and darken.
        let (top, bottom) = if self.pressed {
            (KEY_TOP.darken(0.25), KEY_BOTTOM.darken(0.3))
        } else {
            (KEY_TOP.lighten(lift), KEY_BOTTOM.lighten(lift))
        };
        canvas.draw_rrect(rr, &gfx::vgrad(r, top, bottom));
        if lit {
            // A faint accent wash from the bottom, as if the glyph lit the key.
            let mut wash = vg::Paint::default();
            wash.set_anti_alias(true);
            wash.set_shader(gfx::linear(
                (r.left, r.top),
                (r.left, r.bottom),
                &[ACCENT.with_alpha(0.0), ACCENT.with_alpha(0.10)],
                None,
            ));
            canvas.draw_rrect(rr, &wash);
        }

        // Stroke: plain sides and bottom, a sheen along the top edge.
        gfx::ring(canvas, &rr, gfx::hairline(s), &gfx::fill(KEY_STROKE));
        let sheen = if self.pressed {
            KEY_STROKE.lighten(0.1)
        } else if lit {
            ACCENT.mix(KEY_SHEEN, 0.35)
        } else {
            KEY_SHEEN.lighten(lift)
        };
        gfx::top_sheen(canvas, &rr, gfx::hairline(s), KEY_STROKE, sheen);

        // Glyph, nudged down 1 px while pressed.
        let px = if self.label.chars().count() == 1 {
            KEY_LETTER_PX
        } else {
            KEY_WORD_PX
        };
        let face = if px > KEY_WORD_PX { Face::SemiBold } else { Face::Bold };
        let f = gfx::font(face, px * s);
        let w = gfx::text_width(&f, self.label);
        let x = (r.left + (r.width() - w) * 0.5).round();
        let press_dy = if self.pressed { s } else { 0.0 };
        let base = gfx::baseline_for_center(&f, r.center_y()).round() + press_dy;
        if lit {
            canvas.draw_str(self.label, (x, base), &f, &glow_paint(ACCENT.with_alpha(0.8), 3.0 * s));
            gfx::draw_text(canvas, self.label, x, base, &f, ACCENT);
        } else {
            let ink = if self.hover { TEXT_MID.lighten(0.2) } else { TEXT_MID };
            gfx::draw_text(canvas, self.label, x, base, &f, ink);
        }
    }
}

// ─────────────────────────────── Stat pills ───────────────────────────────

/// What a split pill shows, and what it does when used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillKind {
    /// Audio callback load, % of the buffer period.
    Cpu,
    /// Output sample rate.
    Khz,
    /// Tempo: drag vertically or use the wheel to change it.
    Bpm,
}

/// BPM change per logical pixel of vertical drag (Shift: 10x finer).
const BPM_PER_PX: f32 = 0.25;
/// Pill glyph size (logical px).
const PILL_TEXT_PX: f32 = 10.0;

/// A split pill: a lighter label segment and a darker value segment under
/// one stroke with a top sheen (the prototype's CPU / kHz read-outs).
pub struct StatPill {
    s: Signals,
    kind: PillKind,
    /// Width of the label segment (logical px).
    label_w: f32,
    /// Vertical BPM scrub in progress: (press y, BPM at press).
    scrub: Option<(f32, f32)>,
    pressed: bool,
    hover: bool,
    wheel: crate::input::WheelSteps,
}

impl StatPill {
    pub fn new(cx: &mut Context, s: Signals, kind: PillKind, label_w: f32) -> Handle<'_, Self> {
        let h = Self {
            s,
            kind,
            label_w,
            scrub: None,
            pressed: false,
            hover: false,
            wheel: Default::default(),
        }
        .build(cx, |_| {})
        .class("pill")
        .cursor(match kind {
            PillKind::Bpm => CursorIcon::NsResize,
            _ => CursorIcon::Default,
        });
        // Redraw only when the shown text changes.
        let shown = RefCell::new(Self::value(s, kind));
        let redraw_on_change = move |mut h: Handle<'_, Self>| {
            let v = Self::value(s, kind);
            if *shown.borrow() != v {
                *shown.borrow_mut() = v;
                h.needs_redraw();
            }
        };
        match kind {
            PillKind::Cpu | PillKind::Khz => h.bind(s.audio_load, redraw_on_change),
            PillKind::Bpm => h.bind(s.transport, redraw_on_change),
        }
    }

    fn label(&self) -> &'static str {
        match self.kind {
            PillKind::Cpu => "CPU",
            PillKind::Khz => "kHz",
            PillKind::Bpm => "BPM",
        }
    }

    fn value(s: Signals, kind: PillKind) -> String {
        match kind {
            PillKind::Cpu => {
                // F-38: no running output says so instead of "0%".
                let (pct, hz) = s.audio_load.get_untracked();
                if hz == 0 {
                    "OFF".into()
                } else {
                    format!("{pct}%")
                }
            }
            PillKind::Khz => {
                let (_, hz) = s.audio_load.get_untracked();
                if hz == 0 {
                    "--".into()
                } else {
                    format!("{:.1}", hz as f32 / 1000.0)
                }
            }
            PillKind::Bpm => format!("{:.1}", s.transport.with_untracked(|t| t.bpm)),
        }
    }
}

impl View for StatPill {
    fn element(&self) -> Option<&'static str> {
        Some("stat-pill")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                match self.kind {
                    PillKind::Bpm => {
                        let bpm = self.s.transport.with_untracked(|t| t.bpm);
                        self.scrub = Some((local_mouse(cx).1, bpm));
                        cx.emit(AppEvent::GestureBegin);
                    }
                    _ => return,
                }
                cx.capture();
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) => {
                if let Some((y0, bpm0)) = self.scrub {
                    let fine = if cx.modifiers().shift() { 0.1 } else { 1.0 };
                    let dy = y0 - local_mouse(cx).1;
                    let bpm = bpm0 + dy * BPM_PER_PX * fine;
                    // Whole tenths, as displayed.
                    cx.emit(AppEvent::SetBpm((bpm * 10.0).round() / 10.0));
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.scrub.take().is_some() {
                    cx.emit(AppEvent::GestureEnd);
                    cx.release();
                }
                cx.needs_redraw();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) if self.kind == PillKind::Bpm => {
                cx.emit(AppEvent::SetBpm(Transport::DEFAULT_BPM));
                cx.emit(AppEvent::Commit);
                meta.consume();
            }
            WindowEvent::MouseScroll(_, y) if self.kind == PillKind::Bpm => {
                let steps = self.wheel.steps(*y);
                if steps != 0 {
                    let bpm = self.s.transport.with_untracked(|t| t.bpm);
                    cx.emit(AppEvent::SetBpm(bpm.round() + steps as f32));
                    cx.emit(AppEvent::Commit);
                }
                meta.consume();
            }
            WindowEvent::MouseEnter => {
                self.hover = true;
                cx.needs_redraw();
            }
            WindowEvent::MouseLeave => {
                self.hover = false;
                cx.needs_redraw();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Pills);
        let b = cx.bounds();
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        let rr = gfx::rrect(r, r.height() * 0.5);
        let interactive = !matches!(self.kind, PillKind::Cpu | PillKind::Khz);
        let active = self.scrub.is_some() || self.pressed;
        let lift = if interactive && self.hover && !active { 0.05 } else { 0.0 };
        let split = (r.left + self.label_w * s).round();

        canvas.save();
        canvas.clip_rrect(rr, vg::ClipOp::Intersect, true);
        let lab = vg::Rect::from_ltrb(r.left, r.top, split, r.bottom);
        canvas.draw_rect(lab, &gfx::vgrad(lab, PILL_LABEL_TOP, PILL_LABEL_BOTTOM));
        let val = vg::Rect::from_ltrb(split, r.top, r.right, r.bottom);
        let (vt, vb) = if active {
            (KEY_TOP.darken(0.3), KEY_BOTTOM.darken(0.3))
        } else {
            (KEY_TOP.lighten(lift), KEY_BOTTOM.lighten(lift))
        };
        canvas.draw_rect(val, &gfx::vgrad(val, vt, vb));
        canvas.restore();

        gfx::ring(canvas, &rr, gfx::hairline(s), &gfx::fill(KEY_STROKE));
        gfx::top_sheen(canvas, &rr, gfx::hairline(s), KEY_STROKE, KEY_SHEEN.lighten(lift));

        let f = gfx::font(Face::Bold, PILL_TEXT_PX * s);
        let base = gfx::baseline_for_center(&f, r.center_y()).round();
        let label = self.label();
        let lw = gfx::text_width(&f, label);
        gfx::draw_text(canvas, label, (r.left + (split - r.left - lw) * 0.5).round(), base, &f, TEXT_HI);
        let v = Self::value(self.s, self.kind);
        let vw = gfx::text_width(&f, &v);
        let ink = if self.scrub.is_some() { ACCENT } else { Rgba::white(1.0) };
        gfx::draw_text(canvas, &v, (split + (r.right - split - vw) * 0.5).round(), base, &f, ink);
    }
}

// ──────────────────────────── Transport capsule ────────────────────────────

/// Hit zones at the capsule's ends (logical px): play on the left, loop on the right.
const CAPSULE_END_ZONE: f32 = 32.0;
/// The time read-out's glyph size (logical px) and horizontal squeeze: the
/// prototype's read-out is a condensed face; Inter Bold at 88% width matches it.
const READOUT_PX: f32 = 24.0;
const READOUT_SCALE_X: f32 = 0.88;
/// Logical px of horizontal drag per sixteenth note when scrubbing the time.
const PX_PER_SIXTEENTH: f32 = 4.0;
/// Play and loop glyph boxes, from the capsule's top-left / top-right (logical px).
const GLYPH_INSET_X: f32 = 8.0;
const GLYPH_INSET_Y: f32 = 4.0;
const GLYPH_BOX: f32 = 16.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CapsuleZone {
    Play,
    Time,
    Loop,
}

/// The centre capsule: play light, the time read-out (m:ss.cc, or
/// bars.beats.sixteenths), loop light. Click the play light to play/pause,
/// the loop light to toggle looping; drag the time to scrub. The format is
/// in View > Clock Shows Bars and Settings; Home rewinds (UX.md §4).
pub struct TimeCapsule {
    s: Signals,
    scrub: Option<(f32, f64)>,
    pressed: Option<CapsuleZone>,
    hover: Option<CapsuleZone>,
}

impl TimeCapsule {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        let shown = RefCell::new(Self::time_text(s));
        Self {
            s,
            scrub: None,
            pressed: None,
            hover: None,
        }
        .build(cx, |_| {})
        .class("capsule")
        // Redraw when the text or the play state changes, not per tick.
        .bind(s.transport, move |mut h| {
            let t = Self::time_text(s);
            if *shown.borrow() != t {
                *shown.borrow_mut() = t;
                h.needs_redraw();
            }
        })
        .bind(s.loop_on, |mut h| h.needs_redraw())
        .bind(s.clock_bars, |mut h| h.needs_redraw())
    }

    /// m:ss.cc (minutes, seconds, hundredths) or bars.beats.sixteenths of
    /// the playhead. The playing flag rides along so a play/pause change also
    /// redraws.
    fn time_text(s: Signals) -> String {
        let (ticks, bpm, playing) =
            s.transport.with_untracked(|t| (t.playhead_ticks, t.bpm, t.playing));
        let pad = if playing { "" } else { " " };
        if s.clock_bars.get_untracked() {
            return format!("{}{pad}", crate::state::Transport::position_string(ticks));
        }
        format!("{}{pad}", clock_string(ticks / PPQ as f64 * 60.0 / bpm.max(1.0) as f64))
    }

    fn zone_at(&self, x: f32, w: f32) -> CapsuleZone {
        if x < CAPSULE_END_ZONE {
            CapsuleZone::Play
        } else if x > w - CAPSULE_END_ZONE {
            CapsuleZone::Loop
        } else {
            CapsuleZone::Time
        }
    }
}

impl View for TimeCapsule {
    fn element(&self) -> Option<&'static str> {
        Some("time-capsule")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let (x, _) = local_mouse(cx);
                let w = cx.bounds().w / cx.scale_factor();
                let zone = self.zone_at(x, w);
                if zone == CapsuleZone::Time {
                    let ph = self.s.transport.with_untracked(|t| t.playhead_ticks);
                    self.scrub = Some((x, ph));
                }
                self.pressed = Some(zone);
                cx.capture();
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) => {
                let (x, _) = local_mouse(cx);
                if let Some((x0, t0)) = self.scrub {
                    let sixteenth = PPQ as f64 / 4.0;
                    let steps = ((x - x0) / PX_PER_SIXTEENTH).round() as f64;
                    cx.emit(AppEvent::SetPlayhead((t0 + steps * sixteenth).max(0.0)));
                } else if self.pressed.is_none() {
                    let w = cx.bounds().w / cx.scale_factor();
                    let z = Some(self.zone_at(x, w));
                    if z != self.hover {
                        self.hover = z;
                        cx.needs_redraw();
                        cx.emit(WindowEvent::SetCursor(match z {
                            Some(CapsuleZone::Time) => CursorIcon::EwResize,
                            _ => CursorIcon::Hand,
                        }));
                    }
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                self.scrub = None;
                if let Some(zone) = self.pressed.take() {
                    cx.release();
                    if mouse_inside(cx) {
                        match zone {
                            CapsuleZone::Play => cx.emit(AppEvent::TogglePlay),
                            CapsuleZone::Loop => cx.emit(AppEvent::ToggleLoop),
                            CapsuleZone::Time => {}
                        }
                    }
                }
                cx.needs_redraw();
            }
            // No hidden double-click: Home rewinds (F-38).
            WindowEvent::MouseDoubleClick(MouseButton::Left) => meta.consume(),
            WindowEvent::MouseLeave => {
                self.hover = None;
                cx.emit(WindowEvent::SetCursor(CursorIcon::Default));
                cx.needs_redraw();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Chrome);
        let b = cx.bounds();
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        let rr = gfx::rrect(r, RADIUS_BOX * s);
        draw_box(canvas, &rr, s);

        let playing = self.s.transport.with_untracked(|t| t.playing);
        let loop_on = self.s.loop_on.get_untracked();

        // Play / pause light, top-left.
        let g = vg::Rect::from_xywh(
            r.left + GLYPH_INSET_X * s,
            r.top + GLYPH_INSET_Y * s,
            GLYPH_BOX * s,
            GLYPH_BOX * s,
        );
        let press = |z| if self.pressed == Some(z) { s } else { 0.0 };
        let hover = |z| self.hover == Some(z) && self.pressed.is_none();
        let g = g.with_offset((0.0, press(CapsuleZone::Play)));
        let path = if playing { pause_path(g, s) } else { play_path(g, s) };
        if playing {
            canvas.draw_path(&path, &glow_paint(ACCENT.with_alpha(0.75), 3.0 * s));
            canvas.draw_path(&path, &gfx::fill(ACCENT));
        } else {
            let lift = if hover(CapsuleZone::Play) { 0.15 } else { 0.0 };
            canvas.draw_path(
                &path,
                &gfx::vgrad(g, READOUT_TOP.lighten(lift), READOUT_BOTTOM.lighten(lift)),
            );
        }

        // Loop light, top-right.
        let lg = vg::Rect::from_xywh(
            r.right - (GLYPH_INSET_X + GLYPH_BOX) * s,
            r.top + GLYPH_INSET_Y * s,
            GLYPH_BOX * s,
            GLYPH_BOX * s,
        )
        .with_offset((0.0, press(CapsuleZone::Loop)));
        let lp = loop_path(lg, s);
        let mut stroke = gfx::stroke(ACCENT, 2.0 * s);
        stroke.set_stroke_cap(vg::PaintCap::Round);
        stroke.set_stroke_join(vg::PaintJoin::Round);
        stroke.set_path_effect(vg::PathEffect::corner_path(3.0 * s));
        if loop_on {
            let mut glow = glow_paint(ACCENT.with_alpha(0.8), 3.0 * s);
            glow.set_style(vg::PaintStyle::Stroke);
            glow.set_stroke_width(3.0 * s);
            glow.set_path_effect(vg::PathEffect::corner_path(3.0 * s));
            canvas.draw_path(&lp, &glow);
        } else {
            stroke.set_color4f(
                if hover(CapsuleZone::Loop) { TEXT_MID } else { TEXT_LO }.c4f(),
                None,
            );
        }
        canvas.draw_path(&lp, &stroke);
        let head = loop_heads(lg, s);
        let mut hp = gfx::fill(if loop_on {
            ACCENT
        } else if hover(CapsuleZone::Loop) {
            TEXT_MID
        } else {
            TEXT_LO
        });
        hp.set_anti_alias(true);
        canvas.draw_path(&head, &hp);

        // Time read-out: tabular cells so digits never shift.
        let text = Self::time_text(self.s);
        let text = text.trim_end();
        let mut f = gfx::font(Face::Bold, READOUT_PX * s);
        f.set_scale_x(READOUT_SCALE_X);
        let cell = ('0'..='9')
            .map(|d| gfx::text_width(&f, d.encode_utf8(&mut [0; 4])))
            .fold(0.0f32, f32::max);
        let width_of = |c: char| if c.is_ascii_digit() { cell } else { gfx::text_width(&f, c.encode_utf8(&mut [0; 4])) };
        let total: f32 = text.chars().map(width_of).sum();
        let mut x = (r.center_x() - total * 0.5).round();
        let base = gfx::baseline_for_center(&f, r.center_y()).round();
        let (top, bottom) = if self.scrub.is_some() {
            (ACCENT.lighten(0.2), ACCENT.darken(0.15))
        } else {
            (READOUT_TOP, READOUT_BOTTOM)
        };
        let mut buf = [0u8; 4];
        for c in text.chars() {
            let cw = width_of(c);
            let g = c.encode_utf8(&mut buf);
            let gw = gfx::text_width(&f, g);
            gfx::draw_text_vgrad(canvas, g, x + (cw - gw) * 0.5, base, &f, top, bottom);
            x += cw;
        }
    }
}

/// m:ss.cc: minutes, seconds and hundredths (a full stop before the
/// hundredths, as audio editors write it).
pub fn clock_string(secs: f64) -> String {
    let cs = (secs.max(0.0) * 100.0).floor() as u64;
    format!("{}:{:02}.{:02}", cs / 6000, (cs / 100) % 60, cs % 100)
}

/// A capsule / box: flat interior, flat 1 px stroke (prototype values).
fn draw_box(canvas: &Canvas, rr: &vg::RRect, s: f32) {
    canvas.draw_rrect(*rr, &gfx::fill(BG_2));
    gfx::ring(canvas, rr, gfx::hairline(s), &gfx::fill(STROKE));
}

fn play_path(g: vg::Rect, s: f32) -> vg::Path {
    // A 10 x 12 triangle, left-aligned in the glyph box like the prototype.
    let (x, y) = (g.left + 2.0 * s, g.top + 2.0 * s);
    let mut pb = vg::PathBuilder::new();
    pb.move_to((x, y));
    pb.line_to((x + 10.0 * s, y + 6.0 * s));
    pb.line_to((x, y + 12.0 * s));
    pb.close();
    pb.detach()
}

fn pause_path(g: vg::Rect, s: f32) -> vg::Path {
    let (x, y) = (g.left + 2.0 * s, g.top + 2.0 * s);
    let mut pb = vg::PathBuilder::new();
    for bx in [x, x + 7.0 * s] {
        pb.move_to((bx, y));
        pb.line_to((bx + 4.0 * s, y));
        pb.line_to((bx + 4.0 * s, y + 12.0 * s));
        pb.line_to((bx, y + 12.0 * s));
        pb.close();
    }
    pb.detach()
}

/// The two rails of the repeat glyph (stroked).
fn loop_path(g: vg::Rect, s: f32) -> vg::Path {
    let (l, t) = (g.left + 2.0 * s, g.top + 3.0 * s);
    let (w, h) = (12.0 * s, 10.0 * s);
    let mut pb = vg::PathBuilder::new();
    // Upper rail: from the left end down-turn, across to before the arrowhead.
    pb.move_to((l, t + h * 0.55));
    pb.line_to((l, t + 1.0 * s));
    pb.line_to((l + w - 3.0 * s, t + 1.0 * s));
    // Lower rail, mirrored.
    pb.move_to((l + w, t + h * 0.45));
    pb.line_to((l + w, t + h - 1.0 * s));
    pb.line_to((l + 3.0 * s, t + h - 1.0 * s));
    pb.detach()
}

/// The two arrowheads of the repeat glyph (filled).
fn loop_heads(g: vg::Rect, s: f32) -> vg::Path {
    let (l, t) = (g.left + 2.0 * s, g.top + 3.0 * s);
    let (w, h) = (12.0 * s, 10.0 * s);
    let a = 3.5 * s;
    let mut pb = vg::PathBuilder::new();
    // Right-pointing head at the end of the upper rail.
    let (hx, hy) = (l + w - 3.0 * s, t + 1.0 * s);
    pb.move_to((hx - 0.5 * s, hy - a));
    pb.line_to((hx + a, hy));
    pb.line_to((hx - 0.5 * s, hy + a));
    pb.close();
    // Left-pointing head at the end of the lower rail.
    let (hx, hy) = (l + 3.0 * s, t + h - 1.0 * s);
    pb.move_to((hx + 0.5 * s, hy - a));
    pb.line_to((hx - a, hy));
    pb.line_to((hx + 0.5 * s, hy + a));
    pb.close();
    pb.detach()
}

// ───────────────────────────── Spectrum box ─────────────────────────────

/// Bars in the spectrum box: 4 px bars on an 8 px pitch fill the 204 px box
/// with 8 px padding on both sides.
const SPECTRUM_BARS: usize = 24;
const SPECTRUM_BAR_W: f32 = 4.0;
const SPECTRUM_PITCH: f32 = 8.0;
const SPECTRUM_PAD: f32 = 8.0;
/// Peak dash height and its gap above the bar area (logical px).
const PEAK_H: f32 = 2.0;
/// Attack / release time constants (s) and peak hold / fall.
const SPEC_ATTACK: f32 = 0.02;
const SPEC_RELEASE: f32 = 0.16;
const PEAK_HOLD: f32 = 0.6;
const PEAK_FALL_PER_SEC: f32 = 0.5;
/// Bar ink, top of each bar → its foot.
const BAR_TOP: Rgba = Rgba::hex(0xD6D6D6);
const BAR_FOOT: Rgba = Rgba::hex(0x464646);
const PEAK_INK: Rgba = Rgba::hex(0xBFBFBF);

/// Where each channel sits in the spectrum: (centre bar, spread in bars,
/// gain). Low channels left, noise spread wide on the right.
const CHANNEL_BANDS: [(f32, f32, f32); 8] = [
    (10.0, 3.0, 0.85), // Pulse 1
    (12.0, 3.0, 0.8),  // Pulse 2
    (3.0, 2.5, 1.0),   // Triangle
    (19.0, 5.0, 0.75), // Noise
    (6.0, 4.0, 0.7),   // DPCM
    (14.0, 3.0, 0.75), // VRC6 Pulse 1
    (16.0, 3.0, 0.7),  // VRC6 Pulse 2
    (7.0, 3.5, 0.9),   // VRC6 Saw
];

#[derive(Default)]
struct SpectrumState {
    bars: [f32; SPECTRUM_BARS],
    peaks: [f32; SPECTRUM_BARS],
    hold: [f32; SPECTRUM_BARS],
    /// Shimmer phase, advanced by time so the bars breathe while held notes ring.
    phase: f32,
}

/// The spectrum box: per-channel levels spread into frequency bands, with
/// peak-hold dashes. Grey bars, lit from the top, as in the prototype.
pub struct SpectrumBox {
    s: Signals,
    st: RefCell<SpectrumState>,
    last_draw: Cell<Option<Instant>>,
    animating: std::rc::Rc<Cell<bool>>,
}

impl SpectrumBox {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        let animating = std::rc::Rc::new(Cell::new(false));
        Self {
            s,
            st: RefCell::new(SpectrumState::default()),
            last_draw: Cell::new(None),
            animating: animating.clone(),
        }
        .build(cx, |_| {})
        .class("spectrum")
        .hoverable(false)
        .bind(s.meters, |mut h| h.needs_redraw())
        .bind(s.frame, move |mut h| {
            if animating.get() {
                h.needs_redraw();
            }
        })
    }

    fn step(&self, dt: f32) -> bool {
        // Stopped: let everything fall (the engine can report a held DC
        // level on a silent channel).
        let playing = self.s.transport.with_untracked(|t| t.playing);
        let level = if playing {
            self.s.meters.with_untracked(|m| m.level)
        } else {
            [0.0; 8]
        };
        let mut st = self.st.borrow_mut();
        st.phase = (st.phase + dt * 7.0) % 1000.0;
        let phase = st.phase;
        let mut moving = false;
        for k in 0..SPECTRUM_BARS {
            let kf = k as f32;
            let mut e = 0.0;
            for (c, (centre, spread, gain)) in CHANNEL_BANDS.iter().enumerate() {
                let d = (kf - centre) / spread;
                e += level[c] * gain * (-d * d).exp();
            }
            // A little per-bar movement so sustained notes don't look frozen.
            let shimmer = 0.88 + 0.12 * (phase * (1.0 + 0.37 * kf) + kf * 2.1).sin();
            let target = (e * shimmer).clamp(0.0, 1.0);
            let tau = if target > st.bars[k] { SPEC_ATTACK } else { SPEC_RELEASE };
            st.bars[k] = gfx::approach(st.bars[k], target, dt, tau);
            if st.bars[k] < 0.002 && target == 0.0 {
                st.bars[k] = 0.0;
            }
            if st.bars[k] >= st.peaks[k] {
                st.peaks[k] = st.bars[k];
                st.hold[k] = PEAK_HOLD;
            } else if st.hold[k] > 0.0 {
                st.hold[k] -= dt;
            } else {
                st.peaks[k] = (st.peaks[k] - PEAK_FALL_PER_SEC * dt).max(st.bars[k]);
            }
            moving |= st.bars[k] > 0.0 || st.peaks[k] > 0.0;
        }
        moving
    }
}

impl View for SpectrumBox {
    fn element(&self) -> Option<&'static str> {
        Some("spectrum-box")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Spectrum);
        let b = cx.bounds();
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        let dt = gfx::frame_dt(&self.last_draw);
        self.animating.set(self.step(dt));

        let rr = gfx::rrect(r, RADIUS_BOX * s);
        draw_box(canvas, &rr, s);

        let st = self.st.borrow();
        let top = r.top + SPECTRUM_PAD * s;
        let foot = r.bottom - SPECTRUM_PAD * s * 0.5;
        // The dashes need room above the tallest bar.
        let bar_area = foot - top - (PEAK_H + 2.0) * s;
        for k in 0..SPECTRUM_BARS {
            let x = (r.left + (SPECTRUM_PAD + k as f32 * SPECTRUM_PITCH) * s).round();
            let w = (SPECTRUM_BAR_W * s).round();
            let h = (st.bars[k] * bar_area).round();
            if h >= 1.0 {
                let bar = vg::Rect::from_ltrb(x, foot - h, x + w, foot);
                canvas.draw_rect(bar, &gfx::vgrad(bar, BAR_TOP, BAR_FOOT));
            }
            if st.peaks[k] > 0.01 {
                let py = (foot - st.peaks[k] * bar_area - (PEAK_H + 2.0) * s).round();
                canvas.draw_rect(
                    vg::Rect::from_xywh(x, py, w, (PEAK_H * s).round()),
                    &gfx::vgrad(vg::Rect::from_xywh(x, py, w, PEAK_H * s), PEAK_INK, PEAK_INK.darken(0.3)),
                );
            }
        }
    }
}

// ─────────────────────────────── Track rows ───────────────────────────────

// ─────────────────────────────── Context menu ───────────────────────────────

/// Menu geometry (logical px).
const MENU_W: f32 = 240.0;
const MENU_ROW_H: f32 = 28.0;
const MENU_SEP_H: f32 = 8.0;
const MENU_PAD: f32 = 4.0;
const MENU_TEXT_X: f32 = 12.0;
const MENU_TEXT_PX: f32 = 12.0;
/// The reason line under a menu with disabled rows (F-24): two lines of text.
const MENU_REASON_H: f32 = 40.0;
const MENU_REASON_PX: f32 = 11.0;
/// Colour swatch size and the room the tick takes.
const MENU_SWATCH: f32 = 12.0;
const MENU_CHECK_W: f32 = 16.0;

fn row_h(it: &crate::state::MenuItem) -> f32 {
    if it.action == crate::state::MenuAction::Separator {
        MENU_SEP_H
    } else {
        MENU_ROW_H
    }
}

/// A menu's size (logical px).
pub fn menu_size_of(items: &[crate::state::MenuItem]) -> (f32, f32) {
    let rows: f32 = items.iter().map(row_h).sum();
    let reason = if items.iter().any(|i| !i.enabled) { MENU_REASON_H } else { 0.0 };
    (MENU_W, MENU_PAD * 2.0 + rows + reason)
}

/// Where to open a menu for a click at (x, y) so it stays inside a window of
/// `win` (logical px): flipped left / up near the edges.
pub fn menu_origin_in(win: (f32, f32), x: f32, y: f32, items: &[crate::state::MenuItem]) -> (f32, f32) {
    let (w, h) = menu_size_of(items);
    let x = if x + w > win.0 - SP_1 { (x - w).max(0.0) } else { x };
    let y = if y + h > win.1 - SP_1 { (win.1 - SP_1 - h).max(0.0) } else { y };
    (x, y)
}

/// Catches clicks outside the menu and closes it.
pub struct MenuBackdrop;

impl MenuBackdrop {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self.build(cx, |_| {})
    }
}

impl View for MenuBackdrop {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| {
            if let WindowEvent::MouseDown(_) = we {
                cx.emit(AppEvent::CloseMenu);
                meta.consume();
            }
        });
    }
}

/// A right-click menu in the capsule style: label left, shortcut right,
/// the hovered row lit in the accent. Disabled rows are dimmed and say why
/// in the line at the bottom; a row ending in ▸ opens its submenu.
pub struct ContextMenu {
    items: Vec<crate::state::MenuItem>,
    hover: Option<usize>,
    /// This is a submenu (no submenus of its own).
    nested: bool,
}

impl ContextMenu {
    pub fn new(cx: &mut Context, m: crate::state::ContextMenuSpec, nested: bool) -> Handle<'_, Self> {
        let (w, h) = menu_size_of(&m.items);
        let (x, y) = (m.x, m.y);
        Self { items: m.items, hover: None, nested }
            .build(cx, |_| {})
            .position_type(PositionType::Absolute)
            .left(Pixels(x))
            .top(Pixels(y))
            .width(Pixels(w))
            .height(Pixels(h))
    }

    /// The row under the mouse and its top (logical px from the menu's top).
    fn row_at(&self, cx: &EventContext) -> Option<(usize, f32)> {
        if !mouse_inside(cx) {
            return None;
        }
        let (_, y) = local_mouse(cx);
        let mut top = MENU_PAD;
        for (i, it) in self.items.iter().enumerate() {
            let h = row_h(it);
            if y >= top && y < top + h {
                return (it.action != crate::state::MenuAction::Separator).then_some((i, top));
            }
            top += h;
        }
        None
    }
}

impl View for ContextMenu {
    fn element(&self) -> Option<&'static str> {
        Some("context-menu")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        use crate::state::MenuAction;
        event.map(|we, meta| match we {
            WindowEvent::MouseMove(_, _) => {
                let r = self.row_at(cx);
                if r.map(|r| r.0) != self.hover {
                    self.hover = r.map(|r| r.0);
                    cx.needs_redraw();
                    // Hovering a submenu row opens it beside the row.
                    if let (Some((i, top)), false) = (r, self.nested) {
                        if let MenuAction::Sub(sub) = self.items[i].action {
                            let b = cx.bounds();
                            let s = cx.scale_factor();
                            cx.emit(AppEvent::OpenSub(sub, b.right() / s - MENU_PAD, b.y / s + top - MENU_PAD));
                        }
                    }
                }
            }
            WindowEvent::MouseLeave => {
                self.hover = None;
                cx.needs_redraw();
            }
            WindowEvent::MouseDown(_) => meta.consume(),
            WindowEvent::MouseUp(MouseButton::Left) => {
                if let Some((i, _)) = self.row_at(cx) {
                    let it = &self.items[i];
                    if it.enabled && !matches!(it.action, MenuAction::Sub(_)) {
                        cx.emit(AppEvent::Menu(it.action));
                    }
                }
                meta.consume();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        let rr = gfx::rrect(r, RADIUS_BOX * s);
        gfx::drop_shadow(canvas, &rr, 0.0, 6.0 * s, 16.0 * s, 0.0, Rgba::black(0.6));
        draw_box(canvas, &rr, s);
        gfx::top_sheen(canvas, &rr, gfx::hairline(s), STROKE, KEY_SHEEN.with_alpha(0.6));
        let f = gfx::font(Face::Medium, MENU_TEXT_PX * s);
        let mut top = MENU_PAD;
        for (i, item) in self.items.iter().enumerate() {
            let h = row_h(item);
            let row = vg::Rect::from_xywh(r.left + MENU_PAD * s, r.top + top * s, r.width() - 2.0 * MENU_PAD * s, h * s);
            top += h;
            if item.action == crate::state::MenuAction::Separator {
                let y = row.center_y().round();
                canvas.draw_rect(vg::Rect::from_xywh(row.left + 8.0 * s, y, row.width() - 16.0 * s, gfx::hairline(s)), &gfx::fill(LINE));
                continue;
            }
            let hot = self.hover == Some(i) && item.enabled;
            if hot {
                canvas.draw_rrect(gfx::rrect(row, RADIUS_KEY * s), &gfx::vgrad(row, ACCENT.lighten(0.05), ACCENT.darken(0.15)));
            }
            let base = gfx::baseline_for_center(&f, row.center_y()).round();
            let ink = if hot {
                Rgba::hex(0x0A0A0A)
            } else if item.enabled {
                TEXT_HI
            } else {
                TEXT_LO
            };
            let mut x = row.left + MENU_TEXT_X * s;
            if item.checked {
                gfx::draw_text(canvas, "\u{2713}", x.round(), base, &f, ink);
            }
            if self.items.iter().any(|i| i.checked) {
                x += MENU_CHECK_W * s;
            }
            if let Some(c) = item.swatch {
                let sw = vg::Rect::from_xywh(x, row.center_y() - MENU_SWATCH * 0.5 * s, MENU_SWATCH * s, MENU_SWATCH * s);
                let srr = gfx::rrect(gfx::snap(sw), 3.0 * s);
                canvas.draw_rrect(srr, &gfx::vgrad(sw, c.lighten(0.08), c.darken(0.12)));
                gfx::ring(canvas, &srr, gfx::hairline(s), &gfx::fill(Rgba::black(0.4)));
                x += (MENU_SWATCH + 8.0) * s;
            }
            gfx::draw_text(canvas, &item.label, x.round(), base, &f, ink);
            let ink = if hot { Rgba::hex(0x0A0A0A).with_alpha(0.6) } else { TEXT_LO };
            if matches!(item.action, crate::state::MenuAction::Sub(_)) {
                // A small right-pointing triangle: the submenu opens on hover.
                let (tx, ty) = (row.right - MENU_TEXT_X * s - 3.0 * s, row.center_y());
                let mut pb = vg::PathBuilder::new();
                pb.move_to((tx - 3.0 * s, ty - 4.0 * s));
                pb.line_to((tx + 3.0 * s, ty));
                pb.line_to((tx - 3.0 * s, ty + 4.0 * s));
                pb.close();
                canvas.draw_path(&pb.detach(), &gfx::fill(ink));
            } else {
                let sw = gfx::text_width(&f, &item.shortcut);
                gfx::draw_text(canvas, &item.shortcut, (row.right - MENU_TEXT_X * s - sw).round(), base, &f, ink);
            }
        }
        // The reason a hovered row is disabled (F-24).
        if self.items.iter().any(|i| !i.enabled) {
            let area = vg::Rect::from_ltrb(r.left + MENU_PAD * s, r.top + top * s, r.right - MENU_PAD * s, r.bottom - MENU_PAD * s);
            canvas.draw_rect(vg::Rect::from_xywh(area.left + 8.0 * s, area.top.round(), area.width() - 16.0 * s, gfx::hairline(s)), &gfx::fill(LINE));
            let text = match self.hover.map(|h| &self.items[h]) {
                Some(it) if !it.enabled => it.reason.as_str(),
                _ => "Dimmed items say why on hover.",
            };
            let fr = gfx::font(Face::Medium, MENU_REASON_PX * s);
            gfx::draw_text_wrapped(canvas, text, area.left + MENU_TEXT_X * s, area.top + 6.0 * s, area.width() - 2.0 * MENU_TEXT_X * s, &fr, TEXT_MID, 2);
        }
    }
}

// ─────────────────────────────── Split handle ───────────────────────────────

/// Height of the handle band (logical px) and its grip.
pub const SPLIT_HANDLE_H: f32 = 8.0;
const GRIP_W: f32 = 32.0;
const GRIP_H: f32 = 4.0;
/// The piano roll never gets shorter than this while dragging (logical px).
const PIANO_MIN_H: f32 = 240.0;

/// The draggable band between the arrangement and the piano roll, as in
/// It sets how much of the arrangement is visible (in 4 px steps).
/// Lanes keep their height; when they don't all fit they scroll vertically.
/// Double-click restores the default split.
pub struct SplitHandle {
    s: Signals,
    /// Drag in progress: (window y at press, viewport height at press).
    drag: Option<(f32, f32)>,
    hover: bool,
}

impl SplitHandle {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self {
            s,
            drag: None,
            hover: false,
        }
        .build(cx, |_| {})
        .class("split-handle")
        .height(Pixels(SPLIT_HANDLE_H))
        .width(Stretch(1.0))
        .cursor(CursorIcon::NsResize)
    }

    /// The tallest lane viewport that still leaves the piano roll its
    /// minimum height (never taller than all 8 lanes).
    fn max_pane_h(cx: &EventContext) -> f32 {
        let s = cx.scale_factor();
        let win_h = cx.cache.get_bounds(Entity::root()).h / s;
        let room = win_h - TOPBAR_HEIGHT - RULER_HEIGHT - SPLIT_HANDLE_H - PIANO_MIN_H;
        let room = (room / SP_1).floor() * SP_1;
        room.clamp(crate::state::PIANO_PANE_MIN, 8.0 * LANE_HEIGHT)
    }
}

impl View for SplitHandle {
    fn element(&self) -> Option<&'static str> {
        Some("split-handle")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                // Window y: the handle itself moves while it's dragged.
                let y = cx.mouse().cursor_y / cx.scale_factor();
                self.drag = Some((y, self.s.piano_pane_h.get_untracked()));
                cx.capture();
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) => {
                if let Some((y0, h0)) = self.drag {
                    let y = cx.mouse().cursor_y / cx.scale_factor();
                    let h = ((h0 + y - y0) / SP_1).round() * SP_1;
                    let h = h.clamp(crate::state::PIANO_PANE_MIN, Self::max_pane_h(cx));
                    if h != self.s.piano_pane_h.get_untracked() {
                        cx.emit(AppEvent::SetPianoPaneHeight(h));
                    }
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.drag.is_some() => {
                self.drag = None;
                cx.release();
                cx.needs_redraw();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                let h = crate::state::PIANO_PANE_DEFAULT.min(Self::max_pane_h(cx));
                cx.emit(AppEvent::SetPianoPaneHeight(h));
                meta.consume();
            }
            WindowEvent::MouseEnter => {
                self.hover = true;
                cx.needs_redraw();
            }
            WindowEvent::MouseLeave => {
                self.hover = false;
                cx.needs_redraw();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        let hl = gfx::hairline(s);
        // The band: the panel gradient under a hairline.
        canvas.draw_rect(r, &gfx::vgrad(r, BAND_TOP, BAND_BOTTOM));
        canvas.draw_rect(vg::Rect::from_xywh(r.left, r.top, r.width(), hl), &gfx::fill(LINE));
        // Grip: a small sunk groove that lights up when used.
        let g = gfx::snap(vg::Rect::from_xywh(
            r.center_x() - GRIP_W * 0.5 * s,
            r.center_y() - GRIP_H * 0.5 * s,
            GRIP_W * s,
            GRIP_H * s,
        ));
        let grr = gfx::rrect(g, g.height() * 0.5);
        let active = self.drag.is_some();
        if active || self.hover {
            let c = if active { ACCENT } else { TEXT_MID };
            if active {
                canvas.draw_rrect(grr, &glow_paint(ACCENT.with_alpha(0.5), 3.0 * s));
            }
            canvas.draw_rrect(grr, &gfx::vgrad(g, c.lighten(0.1), c.darken(0.2)));
        } else {
            canvas.draw_rrect(grr, &gfx::fill(BG_INSET));
            gfx::ring_vgrad(canvas, &grr, hl, Rgba::black(0.6), KEY_STROKE);
        }
    }
}

/// Track name size and its baseline below the row top (logical px).
const ROW_NAME_PX: f32 = 13.0;
const ROW_NAME_BASELINE: f32 = 20.0;
/// Chip badge geometry (logical px).
const BADGE_Y: f32 = 32.0;
const BADGE_W: f32 = 52.0;
const BADGE_H: f32 = 16.0;
const BADGE_PX: f32 = 11.0;
/// Instrument name in the track row, and the room kept free for the M/S keys.
const INST_PX: f32 = 11.0;
const INST_RIGHT_RESERVE: f32 = 80.0;
/// The big italic lane number: size, centre x and baseline (logical px).
const WATERMARK_PX: f32 = 48.0;
const WATERMARK_X: f32 = 180.0;
const WATERMARK_BASELINE: f32 = 48.0;
/// Text column start: strip + 8.
const ROW_TEXT_X: f32 = LANE_STRIP_W + SP_2;

/// One track row in the header column: colour strip (pulses with the
/// channel's notes), name, chip badge, a big ghosted lane number, and the
/// M/S keys (child views, laid out by the stylesheet).
pub struct TrackRow {
    s: Signals,
    lane: usize,
    hover: bool,
}

impl TrackRow {
    pub fn new(cx: &mut Context, s: Signals, lane: usize) -> Handle<'_, Self> {
        let seen = RefCell::new((false, String::new()));
        Self { s, lane, hover: false }
        .build(cx, move |cx| {
            let ctrl = if crate::shortcuts::MACOS { "Cmd" } else { "Ctrl" };
            Key::new(cx, s, "M", KeyAction::Mute(lane)).tooltip(|cx| crate::tip(cx, "Mute this track."));
            Key::new(cx, s, "S", KeyAction::Solo(lane)).tooltip(move |cx| crate::tip(cx, &format!("Solo this track only. {ctrl}-click to solo several.")));
        })
        .class("track-row")
        .bind(s.meters, |mut h| h.needs_redraw())
        .bind(s.lane_h, |mut h| h.needs_redraw())
        .bind(s.drag_preset, |mut h| h.needs_redraw())
        .bind(s.chip, |mut h| h.needs_redraw())
        .bind(s.arrangement, move |mut h| {
            let now = s.arrangement.with_untracked(|a| {
                (
                    a.first_selected().map(|c| c.lane) == Some(lane) || a.track_sel.contains(&lane),
                    format!("{}\u{1}{}\u{1}{:?}", a.instruments.lane(lane).map(|i| i.label()).unwrap_or_default(), a.lanes[lane].name, a.lanes[lane].color),
                )
            });
            if *seen.borrow() != now {
                *seen.borrow_mut() = now;
                h.needs_redraw();
            }
        })
    }
}

impl View for TrackRow {
    fn element(&self) -> Option<&'static str> {
        Some("track-row")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| {
            match we {
                WindowEvent::MouseEnter => {
                    self.hover = true;
                    cx.needs_redraw();
                }
                WindowEvent::MouseLeave => {
                    self.hover = false;
                    cx.needs_redraw();
                }
                // T1: a click selects the track (Shift / Ctrl / Cmd adds).
                WindowEvent::MouseDown(MouseButton::Left) if !crate::state::lane_off(self.s.chip.get_untracked(), self.lane) => {
                    let m = *cx.modifiers();
                    cx.emit(AppEvent::EditorFocus(crate::shortcuts::Ctx::Arrangement));
                    cx.emit(AppEvent::SelectTrack(self.lane, m.shift() || crate::input::command(&m)));
                    meta.consume();
                }
                // T2: a double-click renames it.
                WindowEvent::MouseDoubleClick(MouseButton::Left) if !crate::state::lane_off(self.s.chip.get_untracked(), self.lane) => {
                    cx.emit(AppEvent::OpenRename(crate::edit::RenameTarget::Track(self.lane)));
                    meta.consume();
                }
                // T3: its menu.
                WindowEvent::MouseDown(MouseButton::Right) => {
                    cx.emit(AppEvent::SelectTrack(self.lane, false));
                    let s = cx.scale_factor();
                    cx.emit(AppEvent::ContextMenu(crate::edit::MenuSite::Track(self.lane), cx.mouse().cursor_x / s, cx.mouse().cursor_y / s));
                    meta.consume();
                }
                _ => {}
            }
            // The header column scrolls with the lanes.
            if let WindowEvent::MouseScroll(_, sy) = we {
                if self.s.lane_scroll_max.get_untracked() > 0.0 && *sy != 0.0 {
                    cx.emit(AppEvent::ScrollLanes(
                        -*sy * super::arrangement::LANE_SCROLL_PER_NOTCH,
                    ));
                    meta.consume();
                }
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Rows);
        let b = cx.bounds();
        super::browser::register_lane_row(self.lane, b);
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        // Below this there's no room for the badge under the name.
        let compact = r.height() < 56.0 * s;
        let selected = self
            .s
            .arrangement
            .with_untracked(|a| a.first_selected().map(|c| c.lane) == Some(self.lane) || a.track_sel.contains(&self.lane));
        let (color, name) = self
            .s
            .arrangement
            .with_untracked(|a| (a.lanes[self.lane].color, a.lanes[self.lane].name.clone()));
        let light = self.s.meters.with_untracked(|m| m.light[self.lane]);

        // Row surface and its top hairline.
        let lift = if selected { 0.03 } else { 0.0 };
        canvas.draw_rect(r, &gfx::vgrad(r, BAND_TOP.lighten(lift), BAND_BOTTOM.lighten(lift)));
        let hl = gfx::hairline(s);
        canvas.draw_rect(vg::Rect::from_xywh(r.left, r.top, r.width(), hl), &gfx::fill(LINE));
        let body = vg::Rect::from_ltrb(r.left, r.top + hl, r.right, r.bottom);

        // Watermark lane number, ghosted into the surface.
        canvas.save();
        canvas.clip_rect(body, None, None);
        let wf = gfx::font(
            Face::HeavyItalic,
            if compact { (r.height() / s - 8.0).min(WATERMARK_PX) } else { WATERMARK_PX } * s,
        );
        let num = format!("{}", self.lane + 1);
        let ww = gfx::text_width(&wf, &num);
        let wb = if compact {
            gfx::baseline_for_center(&wf, r.center_y())
        } else {
            r.top + WATERMARK_BASELINE * s
        };
        let ghost = if selected { 0.05 } else { 0.0 };
        gfx::draw_text_vgrad(
            canvas,
            &num,
            (r.left + WATERMARK_X * s - ww * 0.5).round(),
            wb.round(),
            &wf,
            Rgba::hex(0x1C1C1C).lighten(ghost),
            Rgba::hex(0x0E0E0E),
        );
        canvas.restore();

        // Colour strip: dark-left → bright-right fill, with a gradient stroke
        // (bright left edge, dark right edge). Note-ons brighten it and spill
        // a little light into the row.
        let strip = vg::Rect::from_ltrb(r.left, body.top, r.left + LANE_STRIP_W * s, r.bottom);
        let c = color.lighten(0.22 * light);
        let mut fill = vg::Paint::default();
        fill.set_anti_alias(true);
        fill.set_shader(gfx::linear(
            (strip.left, strip.top),
            (strip.right, strip.top),
            &[c.darken(0.35), c],
            None,
        ));
        canvas.draw_rect(strip, &fill);
        let srr = gfx::rrect(strip, 0.0);
        // Edge values sampled from the prototype's strips: the left edge sits
        // just under the bright end, the right edge falls to ~half.
        gfx::ring_hgrad(canvas, &srr, gfx::hairline(s), c.lighten(0.1).darken(0.08), c.darken(0.5));
        if light > 0.01 {
            let spill = vg::Rect::from_ltrb(strip.right, body.top, strip.right + 48.0 * s, r.bottom);
            let mut p = vg::Paint::default();
            p.set_shader(gfx::linear(
                (spill.left, spill.top),
                (spill.right, spill.top),
                &[color.with_alpha(0.16 * light), color.with_alpha(0.0)],
                None,
            ));
            canvas.draw_rect(spill, &p);
        }

        // Name and chip badge.
        let x = r.left + ROW_TEXT_X * s;
        let f = gfx::font(Face::SemiBold, ROW_NAME_PX * s);
        let base = if compact {
            gfx::baseline_for_center(&f, r.center_y())
        } else {
            r.top + ROW_NAME_BASELINE * s
        };
        let ink = if selected { TEXT_HI } else { TEXT_MID };
        // Stop before the M/S keys (a long name is cut with an ellipsis).
        gfx::draw_text_clipped(canvas, &name, x.round(), base.round(), r.right - INST_RIGHT_RESERVE * s - x, &f, ink);

        if !compact {
            let badge = vg::Rect::from_xywh(x, r.top + BADGE_Y * s, BADGE_W * s, BADGE_H * s);
            let brr = gfx::rrect(badge, badge.height() * 0.5);
            gfx::ring(canvas, &brr, gfx::hairline(s), &gfx::fill(STROKE));
            let chip = if self.lane >= 5 { "VRC6" } else { "2A03" };
            let bf = gfx::font(Face::SemiBold, BADGE_PX * s);
            let bw = gfx::text_width(&bf, chip);
            let bb = gfx::baseline_for_center(&bf, badge.center_y());
            gfx::draw_text(
                canvas,
                chip,
                (badge.center_x() - bw * 0.5).round(),
                bb.round(),
                &bf,
                if selected { TEXT_MID } else { TEXT_LO },
            );
        }

        // The lane's instrument, after the name (or the badge).
        let inst = self
            .s
            .arrangement
            .with_untracked(|a| a.instruments.lane(self.lane).map(|i| (i.label(), i.is_kit())));
        if let Some((label, kit)) = inst {
            let fi = gfx::font(Face::Medium, INST_PX * s);
            let (ix, ib) = if compact {
                (x + gfx::text_width(&f, &name) + 12.0 * s, base)
            } else {
                (x + BADGE_W * s + 8.0 * s, gfx::baseline_for_center(&fi, r.top + (BADGE_Y + BADGE_H * 0.5) * s))
            };
            // Stop before the M/S keys.
            let max_w = r.right - INST_RIGHT_RESERVE * s - ix;
            let text = if kit { format!("{label}  (kit)") } else { label };
            gfx::draw_text_clipped(canvas, &text, ix.round(), ib.round(), max_w, &fi, if selected { ACCENT } else { TEXT_LO });
        }

        // F-04: a VRC6 lane under the 2A03 profile: dimmed, and it says why.
        if crate::state::lane_off(self.s.chip.get_untracked(), self.lane) {
            canvas.draw_rect(r, &gfx::fill(Rgba::black(super::arrangement::OFF_DIM)));
            let fh = gfx::font(Face::Medium, INST_PX * s);
            let text = if self.hover { "Off: turn on VRC6 in File > Project Settings…" } else { "Off in the 2A03 profile" };
            let hb = if compact { r.bottom - 6.0 * s } else { gfx::baseline_for_center(&fh, r.top + (BADGE_Y + BADGE_H * 0.5) * s) };
            let hx = r.left + ROW_TEXT_X * s + if compact { 0.0 } else { BADGE_W * s + 8.0 * s };
            let max_w = r.right - INST_RIGHT_RESERVE * s - hx;
            // An opaque band under the text so the instrument name below it doesn't show through.
            let band = vg::Rect::from_ltrb(hx - 2.0 * s, hb - 11.0 * s, hx + max_w, hb + 4.0 * s);
            canvas.draw_rect(band, &gfx::fill(BAND_BOTTOM));
            gfx::draw_text_clipped(canvas, text, hx.round(), hb.round(), max_w, &fh, TEXT_HI);
        }

        // Dragging a preset from the browser: dim the lanes it can't play
        // on, outline the one it would land on.
        if let Some((fits, target)) = super::browser::drag_state(&self.s, self.lane) {
            if !fits {
                canvas.draw_rect(body, &gfx::fill(Rgba::black(0.62)));
            } else if target {
                canvas.draw_rect(body, &gfx::fill(ACCENT.with_alpha(0.08)));
                gfx::ring(canvas, &gfx::rrect(body, 0.0), 2.0 * s, &gfx::fill(ACCENT.with_alpha(0.8)));
            }
        }
    }
}
