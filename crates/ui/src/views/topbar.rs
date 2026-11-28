//! The Option D top bar (docs/design/VISUAL-SPEC.md §2; the approved mockup
//! is docs/design/mockup-D.dc.html): view keys, transport keys, the display
//! well with its read-out cells, the TAP key, and the CPU / MEM read-out.
//! The spectrum well is `chrome::SpectrumBox`.
//!
//! Everything is drawn procedurally. Icon shapes are the mockup's SVG path
//! data, placed on the same pixel grid as the browser lays them out.

use std::cell::RefCell;

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face, KeyLook};
use crate::state::{Transport, PPQ, TICKS_PER_BAR};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

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

/// A CSS `drop-shadow(0 0 r color)` / `text-shadow` look: the same shape
/// blurred (CSS blur radius r → Gaussian sigma r/2).
pub fn glow(color: Rgba, css_radius: f32) -> vg::Paint {
    let mut p = gfx::fill(color);
    p.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, css_radius * 0.5, false));
    p
}

/// The lime glow of switched-on icons (the indicators: `.65`).
const ICON_GLOW_A: f32 = 0.65;
/// The selected view icon's glow (the mockup's Arrangement key: `.55`).
const VIEW_GLOW_A: f32 = 0.55;
/// Icon glow radius (CSS px).
const ICON_GLOW_R: f32 = 3.0;

// ───────────────────────────────── Icons ─────────────────────────────────

/// The mockup's icons: (SVG viewBox w, h) and how each is painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Arrangement,
    Mixer,
    Play,
    Stop,
    Record,
    Loop,
    /// Batch 5: an envelope (Automation Arm), and an envelope with a
    /// return arrow (Re-Enable Automation).
    AutoArm,
    ReEnable,
}

impl Icon {
    pub fn size(self) -> (f32, f32) {
        match self {
            Icon::Arrangement => (18.0, 14.0),
            Icon::Mixer => (18.0, 16.0),
            Icon::Play => (12.0, 14.0),
            Icon::Stop => (12.0, 12.0),
            Icon::Record => (14.0, 14.0),
            Icon::Loop => (18.0, 14.0),
            Icon::AutoArm => (18.0, 14.0),
            Icon::ReEnable => (18.0, 14.0),
        }
    }

    /// Draws the icon with its top-left at `o` (device px), in `ink`, with
    /// an optional glow (alpha of the lime drop shadow).
    pub fn draw(self, canvas: &Canvas, o: (f32, f32), s: f32, ink: Rgba, glow_a: Option<f32>) {
        self.draw_glowing(canvas, o, s, ink, glow_a.map(|a| ACCENT.with_alpha(a)));
    }

    /// `draw` with the glow's own colour (Batch 5: the automation keys never
    /// glow lime).
    pub fn draw_glowing(self, canvas: &Canvas, o: (f32, f32), s: f32, ink: Rgba, glow_c: Option<Rgba>) {
        let glow_a = glow_c.map(|c| c.a);
        let glow_rgb = glow_c.unwrap_or(ACCENT);
        let p = |d: &str| gfx::svg_path(d, o, s);
        let fill = |path: &vg::Path| {
            if let Some(a) = glow_a {
                canvas.draw_path(path, &glow(glow_rgb.with_alpha(a), ICON_GLOW_R * s));
            }
            canvas.draw_path(path, &gfx::fill(ink));
        };
        let stroke = |path: &vg::Path, w: f32, color: Rgba| {
            let mut st = gfx::stroke(color, w * s);
            st.set_stroke_cap(vg::PaintCap::Round);
            st.set_stroke_join(vg::PaintJoin::Round);
            if let Some(a) = glow_a {
                let mut g = glow(glow_rgb.with_alpha(a), ICON_GLOW_R * s);
                g.set_style(vg::PaintStyle::Stroke);
                g.set_stroke_width(w * s);
                canvas.draw_path(path, &g);
            }
            canvas.draw_path(path, &st);
        };
        match self {
            Icon::Arrangement => {
                let mut pb = vg::PathBuilder::new();
                for (x, y, w, h) in [(1.0, 1.5, 9.0, 3.0), (5.0, 5.5, 12.0, 3.0), (2.0, 9.5, 7.0, 3.0)] {
                    let r = vg::Rect::from_xywh(o.0 + x * s, o.1 + y * s, w * s, h * s);
                    pb.add_rrect(vg::RRect::new_rect_xy(r, 0.5 * s, 0.5 * s), None, None);
                }
                fill(&pb.detach());
            }
            Icon::Mixer => {
                stroke(&p("M3 1V15M9 1V15M15 1V15"), 1.5, ink);
                let mut pb = vg::PathBuilder::new();
                for (x, y) in [(1.0, 9.0), (7.0, 4.0), (13.0, 7.0)] {
                    let r = vg::Rect::from_xywh(o.0 + x * s, o.1 + y * s, 4.0 * s, 3.0 * s);
                    pb.add_rrect(vg::RRect::new_rect_xy(r, 0.5 * s, 0.5 * s), None, None);
                }
                fill(&pb.detach());
            }
            Icon::Play => fill(&p("M1 1L11 7L1 13Z")),
            Icon::Stop => {
                let r = vg::Rect::from_xywh(o.0 + s, o.1 + s, 10.0 * s, 10.0 * s);
                let mut pb = vg::PathBuilder::new();
                pb.add_rrect(vg::RRect::new_rect_xy(r, 1.5 * s, 1.5 * s), None, None);
                fill(&pb.detach());
            }
            Icon::Record => {
                let mut pb = vg::PathBuilder::new();
                pb.add_circle((o.0 + 7.0 * s, o.1 + 7.0 * s), 5.5 * s, None);
                stroke(&pb.detach(), 2.0, ink);
            }
            Icon::Loop => {
                stroke(&p("M4 4H13a3 3 0 0 1 3 3v0a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3"), 1.8, ink);
                stroke(&p("M5 1.5L3 4L5 6.5"), 1.8, ink);
            }
            Icon::AutoArm => {
                stroke(&p("M1.5 11L6 4.5L11 8.5L16.5 2.5"), 1.6, ink);
                let mut pb = vg::PathBuilder::new();
                for (x, y) in [(6.0, 4.5), (11.0, 8.5)] {
                    pb.add_circle((o.0 + x * s, o.1 + y * s), 1.9 * s, None);
                }
                fill(&pb.detach());
            }
            Icon::ReEnable => {
                stroke(&p("M1.5 11L6 6L10 9"), 1.6, ink);
                stroke(&p("M13 12.5a4 4 0 1 0-1-7.2"), 1.6, ink);
                stroke(&p("M10.2 3.6L12 5.4L14.2 4"), 1.6, ink);
            }
        }
    }
}

// ───────────────────────────────── Keys ─────────────────────────────────

/// The top bar's keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopKey {
    Arrangement,
    Mixer,
    Play,
    Stop,
    Record,
    Loop,
    Tap,
    /// Batch 5: Automation Arm (a toggle) and Re-Enable Automation (a
    /// momentary button, lit while a control is overridden).
    AutoArm,
    ReEnable,
}

impl TopKey {
    pub fn size(self) -> f32 {
        if self == TopKey::Tap {
            KEY_TAP
        } else {
            KEY_BIG
        }
    }

    /// Drawn but not usable yet (VISION.md principle 3: the reason shows on hover).
    pub fn disabled(self) -> bool {
        matches!(self, TopKey::Record)
    }

    fn icon(self) -> Option<Icon> {
        Some(match self {
            TopKey::Arrangement => Icon::Arrangement,
            TopKey::Mixer => Icon::Mixer,
            TopKey::Play => Icon::Play,
            TopKey::Stop => Icon::Stop,
            TopKey::Record => Icon::Record,
            TopKey::Loop => Icon::Loop,
            TopKey::AutoArm => Icon::AutoArm,
            TopKey::ReEnable => Icon::ReEnable,
            TopKey::Tap => return None,
        })
    }

    pub fn id(self) -> &'static str {
        match self {
            TopKey::Arrangement => "key-arrangement",
            TopKey::Mixer => "key-mixer",
            TopKey::Play => "key-play",
            TopKey::Stop => "key-stop",
            TopKey::Record => "key-record",
            TopKey::Loop => "key-loop",
            TopKey::Tap => "key-tap",
            TopKey::AutoArm => "key-auto-arm",
            TopKey::ReEnable => "key-re-enable",
        }
    }
}

/// TAP's label: Inter 700 11, tracking .08em, `#C8C8C8`.
const TAP_INK: Rgba = Rgba::hex(0xC8C8C8);
const TAP_PX: f32 = 11.0;
const TAP_TRACKING: f32 = 0.08;

/// A key in the key material (VISION-SPEC §1): icon-only view and transport
/// keys (40 px, radius 8) and the TAP key (56 px). Transport icons stay
/// neutral: the display shows the state.
pub struct TopKeyView {
    s: Signals,
    key: TopKey,
    pressed: bool,
    hover: bool,
}

impl TopKeyView {
    pub fn new(cx: &mut Context, s: Signals, key: TopKey) -> Handle<'_, Self> {
        let size = key.size();
        Self { s, key, pressed: false, hover: false }
            .build(cx, |_| {})
            .class("top-key")
            .id(key.id())
            .width(Pixels(size))
            .height(Pixels(size))
            .cursor(if key.disabled() { CursorIcon::Default } else { CursorIcon::Hand })
            .bind(s.view_mixer, |mut h| h.needs_redraw())
            .bind(s.auto_arm, |mut h| h.needs_redraw())
            .bind(Memo::new(move |_| s.arrangement.with(|a| !a.automation.overridden.is_empty())), |mut h| h.needs_redraw())
    }
}

impl View for TopKeyView {
    fn element(&self) -> Option<&'static str> {
        Some("top-key")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                meta.consume();
                if self.key.disabled() {
                    return;
                }
                self.pressed = true;
                cx.capture();
                cx.needs_redraw();
                // TAP counts on the press, as a tapped key should.
                if self.key == TopKey::Tap {
                    cx.emit(AppEvent::Tap);
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.pressed => {
                self.pressed = false;
                cx.release();
                if mouse_inside(cx) {
                    let playing = self.s.transport.with_untracked(|t| t.playing);
                    match self.key {
                        // Play starts playback; while playing it does nothing (ours).
                        TopKey::Play if !playing => cx.emit(AppEvent::TogglePlay),
                        TopKey::Stop => cx.emit(AppEvent::Stop),
                        TopKey::Loop => cx.emit(AppEvent::ToggleLoop),
                        TopKey::Arrangement => cx.emit(AppEvent::ShowMixer(false)),
                        TopKey::Mixer => cx.emit(AppEvent::ShowMixer(true)),
                        TopKey::AutoArm => cx.emit(AppEvent::Run(crate::shortcuts::Action::AutomationArm)),
                        TopKey::ReEnable => cx.emit(AppEvent::Run(crate::shortcuts::Action::ReEnableAutomation)),
                        _ => {}
                    }
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
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        let live = !self.key.disabled();
        let look = KeyLook {
            lift: if live && self.hover && !self.pressed { 0.05 } else { 0.0 },
            pressed: self.pressed,
        };
        gfx::key_material(canvas, r, RADIUS_KEY_BIG * s, s, look);
        let dy = if self.pressed { s } else { 0.0 };
        match self.key.icon() {
            Some(icon) => {
                let (w, h) = icon.size();
                let o = (
                    (r.left + (r.width() - w * s) * 0.5).round(),
                    (r.top + (r.height() - h * s) * 0.5).round() + dy,
                );
                let (ink, glow_a) = match self.key {
                    // The view keys: the shown view is lit (a toggle on-state).
                    TopKey::Arrangement | TopKey::Mixer if (self.key == TopKey::Mixer) == self.s.view_mixer.get_untracked() => (ACCENT, Some(VIEW_GLOW_A)),
                    TopKey::Arrangement | TopKey::Mixer => (if look.lift > 0.0 { ICON_IDLE.lighten(0.3) } else { ICON_IDLE }, None),
                    TopKey::Record => (RECORD_GLYPH, None),
                    _ => (if look.lift > 0.0 { ICON_IDLE.lighten(0.3) } else { ICON_IDLE }, None),
                };
                // Batch 5: the automation keys light in their own colours
                // (warm white armed; orange while a control is overridden).
                let lit = match self.key {
                    TopKey::AutoArm if self.s.auto_arm.get_untracked() => Some(AUTO_LINE),
                    TopKey::ReEnable if self.s.arrangement.with_untracked(|a| !a.automation.overridden.is_empty()) => Some(AUTO_REENABLE),
                    _ => None,
                };
                match lit {
                    Some(c) => icon.draw_glowing(canvas, o, s, c, Some(c.with_alpha(ICON_GLOW_A))),
                    None => icon.draw(canvas, o, s, ink, glow_a),
                }
            }
            None => {
                let f = gfx::font(Face::Bold, TAP_PX * s);
                let tr = TAP_TRACKING * TAP_PX * s;
                let w = gfx::tracked_width("TAP", &f, tr);
                let x = r.left + (r.width() - w) * 0.5;
                let base = gfx::css_baseline(&f, r.center_y() - TAP_PX * s * 0.65, TAP_PX * 1.3 * s).round() + dy;
                let ink = if look.lift > 0.0 { TAP_INK.lighten(0.3) } else { TAP_INK };
                gfx::draw_text_tracked(canvas, "TAP", x, base, &f, tr, ink);
            }
        }
    }
}

// ─────────────────────────────── The display ───────────────────────────────

/// Cell widths, left to right (logical px), and the divider between cells:
/// 1 px with 8 px of margin on each side.
pub const CELL_W: [f32; 6] = [72.0, 96.0, 80.0, 64.0, 64.0, 64.0];
const DIVIDER_MARGIN: f32 = 8.0;
const DIVIDER_H: f32 = 32.0;
/// Every value sits on one baseline and every label on another (logical px
/// below the display's top edge). Chosen so each cell's ink (value cap top
/// to label baseline) centres on the display's middle within ±0.5 px for
/// both faces (VISUAL-SPEC.md §2.2, DESIGNPASS.md §display alignment).
pub const VALUE_BASELINE: f32 = 27.0;
pub const LABEL_BASELINE: f32 = 43.0;
/// Inter 600 20 (TIME, BPM); JetBrains Mono 600 18 (POSITION, START, LENGTH).
const VALUE_PX: f32 = 20.0;
const MONO_PX: f32 = 18.0;
/// Labels: Inter 600 10, tracking .1em, #808080 (the 10 px exception, UX.md).
const LABEL_PX: f32 = 10.0;
const LABEL_TRACKING: f32 = 0.1;
/// Indicators: a 2×2 grid, gaps 8×8; Inter 700 11 text.
const IND_GAP: f32 = 8.0;
const IND_TEXT_PX: f32 = 11.0;
/// The indicator icons' top (logical px below the display's top edge).
const IND_ICON_TOP: f32 = 11.0;
/// Text glow of lit indicator text (`0 0 6px .75`).
const IND_TEXT_GLOW_A: f32 = 0.75;
const IND_TEXT_GLOW_R: f32 = 6.0;
/// BPM drag: change per logical px (Shift: 10× finer).
const BPM_PER_PX: f32 = 0.25;

/// What the display shows (one value per cell, plus the indicator states).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DisplayText {
    pub playing: bool,
    pub looping: bool,
    /// Lit while the audio output runs.
    pub format_on: bool,
    pub rate: String,
    pub bits: String,
    pub time: String,
    pub position: String,
    pub start: String,
    pub length: String,
    pub bpm: String,
}

/// bar.beat.sixteenth of a position, 1-based, unpadded ("17.2.3").
pub fn position_text(ticks: f64) -> String {
    let t = ticks.max(0.0) as u32;
    format!("{}.{}.{}", t / TICKS_PER_BAR + 1, (t % TICKS_PER_BAR) / PPQ + 1, (t % PPQ) / (PPQ / 4) + 1)
}

/// bars.beats.sixteenths of a length, 0-based ("2.0.0").
pub fn length_text(ticks: u32) -> String {
    format!("{}.{}.{}", ticks / TICKS_PER_BAR, (ticks % TICKS_PER_BAR) / PPQ, (ticks % PPQ) / (PPQ / 4))
}

/// The sample-rate indicator: "48k", "44.1k".
pub fn rate_text(hz: u32) -> String {
    if hz.is_multiple_of(1000) {
        format!("{}k", hz / 1000)
    } else {
        format!("{:.1}k", hz as f32 / 1000.0)
    }
}

impl DisplayText {
    pub fn of(s: &Signals) -> Self {
        let (ticks, bpm, playing) = s.transport.with_untracked(|t| (t.playhead_ticks, t.bpm, t.playing));
        let secs = ticks / PPQ as f64 * 60.0 / bpm.max(1.0) as f64;
        let range = s.time_sel.get_untracked().map(|t| (t.start, t.end)).or_else(|| s.arrangement.with_untracked(|a| a.selection_span()));
        let (rate, bits) = s.project_format.get_untracked();
        Self {
            playing,
            looping: s.loop_on.get_untracked(),
            format_on: s.audio_load.get_untracked().1 != 0,
            rate: rate_text(rate),
            bits: format!("{bits}-bit"),
            time: crate::views::chrome::clock_string(secs),
            position: position_text(ticks),
            start: range.map_or("\u{2013}".into(), |(a, _)| position_text(a as f64)),
            length: range.map_or("\u{2013}".into(), |(a, b)| length_text(b.saturating_sub(a))),
            bpm: format!("{bpm:.2}"),
        }
    }
}

/// The overhang shadow over the display (VISUAL-SPEC §2.2), rendered once
/// per size and scale.
struct Overhang {
    key: (i32, i32, u32),
    img: vg::Image,
}

/// The display well: indicators, TIME, POSITION, START, LENGTH and BPM,
/// under an overhanging shadow. Read-only except BPM: drag or wheel it,
/// double-click it to type.
pub struct DisplayWell {
    s: Signals,
    scrub: Option<(f32, f32)>,
    wheel: crate::input::WheelSteps,
    hover_bpm: bool,
    overhang: RefCell<Option<Overhang>>,
    /// The indicators cell, pre-rendered per state and scale.
    indicators: RefCell<Option<(IndKey, vg::Image)>>,
    /// The well, dividers and labels, pre-rendered per size and scale.
    static_layer: gfx::ImageCache,
}

/// The display's static layer: the well, the cell dividers and the labels.
fn render_display_static(w: i32, h: i32, s: f32) -> Option<vg::Image> {
    let hl = gfx::hairline(s);
    let mut surface = vg::surfaces::raster_n32_premul((w, h + hl as i32))?;
    let canvas = surface.canvas();
    canvas.clear(vg::Color::TRANSPARENT);
    let r = vg::Rect::from_xywh(0.0, 0.0, w as f32, h as f32);
    gfx::well(canvas, r, RADIUS_KEY_BIG * s, s);
    let lefts = DisplayWell::cell_lefts();
    for l in &lefts[1..] {
        let x = ((l - DIVIDER_MARGIN - 1.0) * s).round();
        let y = (r.center_y() - DIVIDER_H * 0.5 * s).round();
        canvas.draw_rect(vg::Rect::from_xywh(x, y, hl, DIVIDER_H * s), &gfx::fill(CELL_DIVIDER));
    }
    let lf = gfx::font(Face::SemiBold, LABEL_PX * s);
    let ltr = LABEL_TRACKING * LABEL_PX * s;
    let lb = (LABEL_BASELINE * s).round();
    for (k, label) in ["TIME", "POSITION", "START", "LENGTH", "BPM"].into_iter().enumerate() {
        let cx_ = (lefts[k + 1] + CELL_W[k + 1] * 0.5) * s;
        let lw = gfx::tracked_width(label, &lf, ltr);
        gfx::draw_text_tracked(canvas, label, (cx_ - lw * 0.5).round(), lb, &lf, ltr, TEXT_LO);
    }
    Some(surface.image_snapshot())
}

/// What the indicators cell shows: playing, looping, format lit, rate,
/// bits, scale (×100).
type IndKey = (bool, bool, bool, String, String, u32);

/// Room around the indicators cell for its glows (logical px).
const IND_PAD: f32 = 8.0;

/// The indicators cell (its 72 px plus `IND_PAD` on each side, the well's
/// height): the play and loop icons over the rate and bit depth, lime and
/// glowing while active, `#4A4A4A` while not.
fn render_indicators(t: &DisplayText, h: f32, s: f32) -> Option<vg::Image> {
    let pad = (IND_PAD * s).round();
    let mut surface = vg::surfaces::raster_n32_premul(((CELL_W[0] * s + 2.0 * pad).ceil() as i32, h.ceil() as i32))?;
    let canvas = surface.canvas();
    canvas.clear(vg::Color::TRANSPARENT);
    let x0 = pad;
    let tf = gfx::font(Face::Bold, IND_TEXT_PX * s);
    let (rw, bw) = (gfx::text_width(&tf, &t.rate), gfx::text_width(&tf, &t.bits));
    let (pw, lw) = (Icon::Play.size().0, Icon::Loop.size().0);
    let c1 = (rw / s).max(pw);
    let c2 = (bw / s).max(lw);
    let gx = x0 + (CELL_W[0] - c1 - IND_GAP - c2) * 0.5 * s;
    let col = [gx, gx + (c1 + IND_GAP) * s];
    let cw = [c1, c2];
    let iy = (IND_ICON_TOP * s).round();
    for (k, (icon, on)) in [(Icon::Play, t.playing), (Icon::Loop, t.looping)].into_iter().enumerate() {
        let w = icon.size().0;
        let o = ((col[k] + (cw[k] - w) * 0.5 * s).round(), iy);
        icon.draw(canvas, o, s, if on { ACCENT } else { INDICATOR_OFF }, on.then_some(ICON_GLOW_A));
    }
    let lb = (LABEL_BASELINE * s).round();
    for (k, (text, w)) in [(&t.rate, rw), (&t.bits, bw)].into_iter().enumerate() {
        let x = (col[k] + (cw[k] * s - w) * 0.5).round();
        if t.format_on {
            canvas.draw_str(text, (x, lb), &tf, &glow(ACCENT.with_alpha(IND_TEXT_GLOW_A), IND_TEXT_GLOW_R * s));
        }
        gfx::draw_text(canvas, text, x, lb, &tf, if t.format_on { ACCENT } else { INDICATOR_OFF });
    }
    Some(surface.image_snapshot())
}

impl DisplayWell {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        let shown = RefCell::new(DisplayText::of(&s));
        let redraw = move |mut h: Handle<'_, Self>| {
            let t = DisplayText::of(&s);
            if *shown.borrow() != t {
                *shown.borrow_mut() = t;
                h.needs_redraw();
            }
        };
        Self { s, scrub: None, wheel: Default::default(), hover_bpm: false, overhang: RefCell::new(None), indicators: RefCell::new(None), static_layer: RefCell::new(None) }
            .build(cx, |_| {})
            .class("display")
            .id("display")
            .width(Pixels(DISPLAY_W))
            .height(Pixels(WELL_H))
            // Sampled once per presented frame (the pump's tick), not on
            // every model change: a clip drag changes the arrangement on each
            // mouse move, and START / LENGTH follow it (polish pass §2.5).
            .bind(s.frame, redraw)
    }

    /// The cells' left edges (logical px from the display's left), centred
    /// as a group and snapped to whole px.
    pub fn cell_lefts() -> [f32; 6] {
        let total: f32 = CELL_W.iter().sum::<f32>() + 5.0 * (1.0 + 2.0 * DIVIDER_MARGIN);
        // The mockup centres the group at 17.5 px; its dividers land on
        // the whole pixel below (floor).
        let mut x = ((DISPLAY_W - total) * 0.5).floor();
        let mut out = [0.0; 6];
        for (i, w) in CELL_W.iter().enumerate() {
            out[i] = x;
            x += w + 1.0 + 2.0 * DIVIDER_MARGIN;
        }
        out
    }

    fn over_bpm(x: f32) -> bool {
        let l = Self::cell_lefts()[5];
        x >= l - DIVIDER_MARGIN && x <= l + CELL_W[5] + DIVIDER_MARGIN
    }

    fn overhang(&self, canvas: &Canvas, inner: vg::RRect, s: f32) {
        let r = *inner.rect();
        let key = (r.width().round() as i32, r.height().round() as i32, (s * 100.0).round() as u32);
        let mut cache = self.overhang.borrow_mut();
        if cache.as_ref().is_none_or(|o| o.key != key) {
            *cache = render_overhang(key.0, key.1, inner.radii(vg::rrect::Corner::UpperLeft).x, s).map(|img| Overhang { key, img });
        }
        if let Some(o) = cache.as_ref() {
            canvas.draw_image(&o.img, (r.left, r.top), None);
        }
    }
}

/// The overhang's reach and its curve (VISUAL-SPEC.md §13, G11; the
/// `fixups` board): black at alpha `0.6 · (1 − y/48)^2.2`, easing out over
/// 48 px (it was a 34 px linear-ish ramp), sampled every 4 px as the board's
/// gradient is.
pub const OVERHANG_REACH: f32 = 48.0;
const OVERHANG_ALPHA: f32 = 0.6;
const OVERHANG_EASE: f32 = 2.2;
const OVERHANG_STEP: f32 = 4.0;

/// The overhang's alpha `y` CSS px below the display's inner top.
pub fn overhang_alpha(y: f32) -> f32 {
    OVERHANG_ALPHA * (1.0 - (y / OVERHANG_REACH).clamp(0.0, 1.0)).powf(OVERHANG_EASE)
}

/// The overhang: the eased gradient above plus the softer inset
/// `0 16px 24px −12px` black at 0.55, in the display's padding box (radius 7).
fn render_overhang(w: i32, h: i32, radius: f32, s: f32) -> Option<vg::Image> {
    let mut surface = vg::surfaces::raster_n32_premul((w.max(1), h.max(1)))?;
    let c = surface.canvas();
    c.clear(vg::Color::TRANSPARENT);
    let r = vg::Rect::from_xywh(0.0, 0.0, w as f32, h as f32);
    let rr = gfx::rrect(r, radius);
    c.save();
    c.clip_rrect(rr, vg::ClipOp::Intersect, true);
    let n = (OVERHANG_REACH / OVERHANG_STEP) as usize;
    let ys: Vec<f32> = (0..=n).map(|k| k as f32 * OVERHANG_STEP).collect();
    let colors: Vec<Rgba> = ys.iter().map(|y| Rgba::black(overhang_alpha(*y))).collect();
    let pos: Vec<f32> = ys.iter().map(|y| y / OVERHANG_REACH).collect();
    c.draw_rect(
        vg::Rect::from_xywh(0.0, 0.0, w as f32, OVERHANG_REACH * s),
        &gfx::shaded(gfx::linear((0.0, 0.0), (0.0, OVERHANG_REACH * s), &colors, Some(&pos))),
    );
    c.restore();
    gfx::css_inset_shadow(c, &rr, 0.0, 16.0 * s, 24.0 * s, -12.0 * s, Rgba::black(0.55));
    Some(surface.image_snapshot())
}

#[cfg(test)]
mod overhang_tests {
    /// G11: the board's stops (`fixups`: .600 at 0, .495 at 4, .183 at 20,
    /// .028 at 36, 0 at 48).
    #[test]
    fn the_overhang_eases_out_over_48_px() {
        for (y, a) in [(0.0, 0.600), (4.0, 0.495), (20.0, 0.183), (36.0, 0.028), (48.0, 0.0)] {
            assert!((super::overhang_alpha(y) - a).abs() < 0.0015, "{y}: {}", super::overhang_alpha(y));
        }
    }
}

impl View for DisplayWell {
    fn element(&self) -> Option<&'static str> {
        Some("display")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) if Self::over_bpm(local_mouse(cx).0) => {
                let bpm = self.s.transport.with_untracked(|t| t.bpm);
                self.scrub = Some((local_mouse(cx).1, bpm));
                cx.emit(AppEvent::GestureBegin);
                cx.capture();
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) => {
                let (x, y) = local_mouse(cx);
                if let Some((y0, bpm0)) = self.scrub {
                    let fine = if cx.modifiers().shift() { 0.1 } else { 1.0 };
                    let bpm = bpm0 + (y0 - y) * BPM_PER_PX * fine;
                    cx.emit(AppEvent::SetBpm((bpm * 100.0).round() / 100.0));
                } else {
                    let over = Self::over_bpm(x);
                    if over != self.hover_bpm {
                        self.hover_bpm = over;
                        cx.emit(WindowEvent::SetCursor(if over { CursorIcon::NsResize } else { CursorIcon::Default }));
                    }
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.scrub.take().is_some() {
                    cx.emit(AppEvent::GestureEnd);
                    cx.release();
                }
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) if Self::over_bpm(local_mouse(cx).0) => {
                let s = cx.scale_factor();
                let b = cx.bounds();
                let l = Self::cell_lefts()[5];
                cx.emit(AppEvent::EditBpm(b.x / s + l - DIVIDER_MARGIN, b.y / s + (WELL_H - crate::edit::RENAME_H) * 0.5, CELL_W[5] + 2.0 * DIVIDER_MARGIN));
                meta.consume();
            }
            WindowEvent::MouseScroll(_, y) if Self::over_bpm(local_mouse(cx).0) => {
                let steps = self.wheel.steps(*y);
                if steps != 0 {
                    let bpm = self.s.transport.with_untracked(|t| t.bpm);
                    cx.emit(AppEvent::SetBpm(bpm.round() + steps as f32));
                    cx.emit(AppEvent::Commit);
                }
                meta.consume();
            }
            WindowEvent::MouseLeave if self.hover_bpm => {
                self.hover_bpm = false;
                cx.emit(WindowEvent::SetCursor(CursorIcon::Default));
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Chrome);
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        // The well, its dividers and the labels: one image per size and scale.
        {
            let key = (r.width().round() as i32, r.height().round() as i32, (s * 100.0).round() as u32);
            let mut cache = self.static_layer.borrow_mut();
            if cache.as_ref().is_none_or(|(k, _)| *k != key) {
                *cache = render_display_static(key.0, key.1, s).map(|img| (key, img));
            }
            match cache.as_ref() {
                Some((_, img)) => {
                    canvas.draw_image(img, (r.left, r.top), None);
                }
                None => gfx::well(canvas, r, RADIUS_KEY_BIG * s, s),
            }
        }
        let t = DisplayText::of(&self.s);
        let lefts = Self::cell_lefts();
        let vb = (r.top + VALUE_BASELINE * s).round();

        // 1. Indicators: play and loop icons over the format. Their glows are
        // blurs, so the cell is rendered once per state, not per time tick.
        {
            let key = (t.playing, t.looping, t.format_on, t.rate.clone(), t.bits.clone(), (s * 100.0).round() as u32);
            let mut cache = self.indicators.borrow_mut();
            if cache.as_ref().is_none_or(|(k, _)| *k != key) {
                *cache = render_indicators(&t, r.height(), s).map(|img| (key, img));
            }
            if let Some((_, img)) = cache.as_ref() {
                let pad = (IND_PAD * s).round();
                canvas.draw_image(img, ((r.left + lefts[0] * s).round() - pad, r.top), None);
            }
        }

        // 2–6. Value over label.
        let inter = gfx::font(Face::SemiBold, VALUE_PX * s);
        let mono_font = gfx::font(Face::MonoSemiBold, MONO_PX * s);
        // The widest digit, once per face (tabular figures).
        let (inter_cell, mono_cell) = (digit_cell(&inter), digit_cell(&mono_font));
        let cells: [(&str, &str, bool); 5] = [
            (&t.time, "TIME", false),
            (&t.position, "POSITION", true),
            (&t.start, "START", true),
            (&t.length, "LENGTH", true),
            (&t.bpm, "BPM", false),
        ];
        for (k, (value, label, mono)) in cells.into_iter().enumerate() {
            let (cl, cw) = (r.left + lefts[k + 1] * s, CELL_W[k + 1] * s);
            let cx_ = cl + cw * 0.5;
            // Tabular figures: every digit in the widest digit's cell.
            let (mut f, mut cell) = if mono { (mono_font.clone(), mono_cell) } else { (inter.clone(), inter_cell) };
            // A long position (bar 100+) shrinks to fit its cell and margins.
            let room = cw + DIVIDER_MARGIN * s;
            let mut w = tabular_width(&f, cell, value);
            if w > room {
                f.set_size(f.size() * room / w);
                cell = digit_cell(&f);
                w = tabular_width(&f, cell, value);
            }
            let ink = if k == 4 && (self.scrub.is_some() || self.hover_bpm) { Rgba::white(1.0) } else { TEXT_VALUE };
            draw_tabular(canvas, value, cx_ - w * 0.5, vb, &f, cell, ink);
            let _ = label;
        }

        // The overhang, over the values (inside the border).
        let hl = gfx::hairline(s);
        let inner = gfx::rrect(r.with_inset((hl, hl)), (RADIUS_KEY_BIG - 1.0) * s);
        self.overhang(canvas, inner, s);
    }
}

/// The widest digit's advance: tabular figures without OpenType features.
fn digit_cell(f: &vg::Font) -> f32 {
    ('0'..='9').map(|d| gfx::text_width(f, d.encode_utf8(&mut [0; 4]))).fold(0.0f32, f32::max)
}

fn tabular_width(f: &vg::Font, cell: f32, text: &str) -> f32 {
    text.chars().map(|c| if c.is_ascii_digit() { cell } else { gfx::text_width(f, c.encode_utf8(&mut [0; 4])) }).sum()
}

fn draw_tabular(canvas: &Canvas, text: &str, x: f32, base: f32, f: &vg::Font, cell: f32, ink: Rgba) {
    let mut x = x;
    let mut buf = [0u8; 4];
    for c in text.chars() {
        let g = c.encode_utf8(&mut buf);
        let gw = gfx::text_width(f, g);
        let w = if c.is_ascii_digit() { cell } else { gw };
        gfx::draw_text(canvas, g, x + (w - gw) * 0.5, base, f, ink);
        x += w;
    }
}

// ─────────────────────────────── CPU / MEM ───────────────────────────────

/// The read-out's width (the mockup's grid, 83.44 px, on the 4 px grid).
pub const CPU_MEM_W: f32 = 84.0;
const STAT_PX: f32 = 11.0;
const STAT_TRACKING: f32 = 0.06;
const STAT_ROW_GAP: f32 = 8.0;
const STAT_LINE: f32 = 12.0;

/// CPU (the audio callback's load) and MEM (the process's private memory),
/// a 2×2 grid: labels `#808080`, values `#E6E6E6` right-aligned.
pub struct CpuMem {
    s: Signals,
}

impl CpuMem {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        let shown = RefCell::new(Self::values(&s));
        Self { s }
            .build(cx, |_| {})
            .class("cpu-mem")
            .id("cpu-mem")
            .hoverable(true)
            .width(Pixels(CPU_MEM_W))
            .height(Pixels(2.0 * STAT_LINE + STAT_ROW_GAP))
            .bind(s.mem_mb, |mut h| h.needs_redraw())
            .bind(s.audio_load, move |mut h| {
                let v = Self::values(&s);
                if *shown.borrow() != v {
                    *shown.borrow_mut() = v;
                    h.needs_redraw();
                }
            })
    }

    fn values(s: &Signals) -> (String, String) {
        let (pct, hz) = s.audio_load.get_untracked();
        // F-38: no running output says so instead of "0%".
        let cpu = if hz == 0 { "OFF".to_string() } else { format!("{pct}%") };
        let mem = s.mem_mb.get_untracked().map_or("--".to_string(), |mb| format!("{mb} MB"));
        (cpu, mem)
    }
}

impl View for CpuMem {
    fn element(&self) -> Option<&'static str> {
        Some("cpu-mem")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Pills);
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        let f = gfx::font(Face::SemiBold, STAT_PX * s);
        let tr = STAT_TRACKING * STAT_PX * s;
        let (cpu, mem) = Self::values(&self.s);
        for (row, (label, value)) in [("CPU", cpu), ("MEM", mem)].into_iter().enumerate() {
            let top = r.top + row as f32 * (STAT_LINE + STAT_ROW_GAP) * s;
            let base = gfx::css_baseline(&f, top, STAT_LINE * s).round();
            gfx::draw_text_tracked(canvas, label, r.left, base, &f, tr, TEXT_LO);
            let w = gfx::tracked_width(&value, &f, tr);
            gfx::draw_text_tracked(canvas, &value, (r.right - w).round(), base, &f, tr, TEXT_HI);
        }
    }
}

/// The BPM a text field entry asks for, if it is a number in range.
pub fn parse_bpm(text: &str) -> Option<f32> {
    let v: f32 = text.trim().replace(',', ".").parse().ok()?;
    (v.is_finite()).then(|| v.clamp(Transport::MIN_BPM, Transport::MAX_BPM))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(position_text(0.0), "1.1.1");
        let t = (16 * TICKS_PER_BAR + PPQ + 2 * (PPQ / 4)) as f64;
        assert_eq!(position_text(t), "17.2.3");
        assert_eq!(length_text(2 * TICKS_PER_BAR), "2.0.0");
        assert_eq!(rate_text(48_000), "48k");
        assert_eq!(rate_text(44_100), "44.1k");
        assert_eq!(parse_bpm(" 128,5 "), Some(128.5));
        assert_eq!(parse_bpm("abc"), None);
    }

    #[test]
    fn cells_fill_the_display_symmetrically() {
        let l = DisplayWell::cell_lefts();
        let right = l[5] + CELL_W[5];
        // Centred within a pixel.
        assert!((l[0] - (DISPLAY_W - right)).abs() <= 1.0, "{l:?} {right}");
    }
}
