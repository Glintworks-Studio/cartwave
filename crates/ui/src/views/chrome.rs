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
const KEY_LETTER_PX: f32 = 12.0;
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
            KeyAction::Mute(i) => self.s.arrangement.with_untracked(|a| a.lanes.get(i).is_some_and(|l| self.s.mute.with_untracked(|m| m.contains(l.id)))),
            KeyAction::Solo(i) => self.s.arrangement.with_untracked(|a| a.lanes.get(i).is_some_and(|l| self.s.solo.with_untracked(|m| m.contains(l.id)))),
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
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        // The keys repaint whenever their row does (every frame while the
        // meters move): each look is rendered once and blitted (polish pass).
        let key = (self.label, self.lit(), self.hover, self.pressed, r.width() as i32, r.height() as i32, (s * 100.0).round() as u32);
        let img = KEY_IMAGES.with(|c| {
            c.borrow_mut()
                .entry(key)
                .or_insert_with(|| {
                    // A surface like the window's (GPU): the cached key looks
                    // exactly like one drawn in place.
                    let mut surface = canvas.new_surface(&vg::ImageInfo::new_n32_premul((key.4.max(1), key.5.max(1)), None), None)?;
                    let lc = surface.canvas();
                    lc.clear(vg::Color::TRANSPARENT);
                    self.draw_key(lc, vg::Rect::from_xywh(0.0, 0.0, key.4 as f32, key.5 as f32), s);
                    Some(surface.image_snapshot())
                })
                .clone()
        });
        match img {
            Some(img) => {
                canvas.draw_image(&img, (r.left, r.top), None);
            }
            None => self.draw_key(canvas, r, s),
        }
    }
}

/// A key look: (label, lit, hover, pressed, w, h, scale×100).
type KeyLookKey = (&'static str, bool, bool, bool, i32, i32, u32);

thread_local! {
    /// Rendered key looks: (label, lit, hover, pressed, w, h, scale×100).
    static KEY_IMAGES: RefCell<std::collections::HashMap<KeyLookKey, Option<vg::Image>>> = RefCell::new(std::collections::HashMap::new());
}

impl Key {
    fn draw_key(&self, canvas: &Canvas, r: vg::Rect, s: f32) {
        let lit = self.lit();
        let lift = if self.hover && !self.pressed { 0.05 } else { 0.0 };
        // The key material (VISUAL-SPEC.md §1): unchanged when lit; only
        // the glyph turns lime.
        gfx::key_material(canvas, r, RADIUS_KEY * s, s, gfx::KeyLook { lift, pressed: self.pressed });

        // Glyph (Inter 700 12), nudged down 1 px while pressed.
        let px = if self.label.chars().count() == 1 { KEY_LETTER_PX } else { KEY_WORD_PX };
        let f = gfx::font(Face::Bold, px * s);
        let w = gfx::text_width(&f, self.label);
        let x = (r.left + (r.width() - w) * 0.5).round();
        let press_dy = if self.pressed { s } else { 0.0 };
        let base = gfx::baseline_for_center(&f, r.center_y()).round() + press_dy;
        if lit {
            // text-shadow: 0 0 6px lime .85, 0 0 2px lime .9.
            canvas.draw_str(self.label, (x, base), &f, &super::topbar::glow(ACCENT.with_alpha(0.85), 6.0 * s));
            canvas.draw_str(self.label, (x, base), &f, &super::topbar::glow(ACCENT.with_alpha(0.9), 2.0 * s));
            gfx::draw_text(canvas, self.label, x, base, &f, ACCENT);
        } else {
            let ink = if self.hover { TEXT_MID.lighten(0.2) } else { TEXT_MID };
            gfx::draw_text(canvas, self.label, x, base, &f, ink);
        }
    }
}

/// A menu panel: flat interior, flat 1 px stroke (prototype values).
fn draw_box(canvas: &Canvas, rr: &vg::RRect, s: f32) {
    canvas.draw_rrect(*rr, &gfx::fill(BG_2));
    gfx::ring(canvas, rr, gfx::hairline(s), &gfx::fill(STROKE));
}

/// m:ss.cc: minutes, seconds and hundredths (a full stop before the
/// hundredths, as audio editors write it).
pub fn clock_string(secs: f64) -> String {
    let cs = (secs.max(0.0) * 100.0).floor() as u64;
    format!("{}:{:02}.{:02}", cs / 6000, (cs / 100) % 60, cs % 100)
}

// ───────────────────────────── Spectrum well ─────────────────────────────

/// The spectrum well (VISUAL-SPEC.md §2.2): 24 square bars, 4 px wide on an
/// 8 px pitch, 12 px side padding, 8 px bottom padding.
const SPECTRUM_BARS: usize = 24;
const SPECTRUM_BAR_W: f32 = 4.0;
const SPECTRUM_PITCH: f32 = 8.0;
const SPECTRUM_PAD_X: f32 = 12.0;
const SPECTRUM_PAD_BOTTOM: f32 = 8.0;
/// The peak mark: 2 px, 4 px above its bar.
const PEAK_H: f32 = 2.0;
const PEAK_GAP: f32 = 4.0;
/// Attack / release time constants (s) and peak hold / fall.
const SPEC_ATTACK: f32 = 0.02;
const SPEC_RELEASE: f32 = 0.16;
const PEAK_HOLD: f32 = 0.6;
const PEAK_FALL_PER_SEC: f32 = 0.5;
/// Bar gradient, foot → 60% → top (the mockup's `0deg` gradient), and the
/// peak mark (`#C8C8C8` at 55%).
const BAR_FOOT: Rgba = Rgba::hex(0x4A4A4A);
const BAR_MID: Rgba = Rgba::hex(0x9A9A9A);
const BAR_TOP: Rgba = Rgba::hex(0xD6D6D6);
const PEAK_INK: Rgba = Rgba::hex(0xC8C8C8);
const PEAK_ALPHA: f32 = 0.55;
/// LED segments: 2 px lit, 2 px gap.
const LED_H: f32 = 2.0;
const LED_PITCH: f32 = 4.0;

/// How the spectrum draws (its right-click menu; settings.json).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpectrumStyle {
    Bars,
    Leds,
    Line,
    Dots,
}

impl SpectrumStyle {
    pub const ALL: [SpectrumStyle; 4] = [SpectrumStyle::Bars, SpectrumStyle::Leds, SpectrumStyle::Line, SpectrumStyle::Dots];

    pub fn key(self) -> &'static str {
        match self {
            SpectrumStyle::Bars => "bars",
            SpectrumStyle::Leds => "leds",
            SpectrumStyle::Line => "line",
            SpectrumStyle::Dots => "dots",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SpectrumStyle::Bars => "Bars",
            SpectrumStyle::Leds => "LED Segments",
            SpectrumStyle::Line => "Line",
            SpectrumStyle::Dots => "Peak Dots",
        }
    }

    /// Unknown keys fall back to Bars (settings.json never fails).
    pub fn from_key(k: &str) -> Self {
        Self::ALL.into_iter().find(|s| s.key() == k).unwrap_or(SpectrumStyle::Bars)
    }
}

/// Where each channel sits in the spectrum: (centre bar, spread in bars,
/// gain). Low channels left, noise spread wide on the right. The bars are
/// the whole mix: the channels sum into one neutral display.
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

/// The spectrum well: the mix's levels spread into frequency bands, with
/// peak marks. Right-click for its styles.
pub struct SpectrumBox {
    s: Signals,
    st: RefCell<SpectrumState>,
    last_draw: Cell<Option<Instant>>,
    animating: std::rc::Rc<Cell<bool>>,
}

impl SpectrumBox {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        let animating = std::rc::Rc::new(Cell::new(false));
        let style = RefCell::new(s.settings.with_untracked(|st| st.spectrum_style.clone()));
        Self {
            s,
            st: RefCell::new(SpectrumState::default()),
            last_draw: Cell::new(None),
            animating: animating.clone(),
        }
        .build(cx, |_| {})
        .class("spectrum")
        .id("spectrum")
        .bind(s.meters, |mut h| h.needs_redraw())
        .bind(s.settings, move |mut h| {
            let now = s.settings.with_untracked(|st| st.spectrum_style.clone());
            if *style.borrow() != now {
                *style.borrow_mut() = now;
                h.needs_redraw();
            }
        })
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
        // Each band follows the tracks of its channel type (a studio Pulse
        // track lights the pulse band).
        let level = if playing {
            let mut l = [0.0f32; 8];
            self.s.arrangement.with_untracked(|a| {
                self.s.meters.with_untracked(|m| {
                    for (i, t) in a.lanes.iter().enumerate() {
                        let c = t.chan();
                        l[c] = l[c].max(m.level_of(i));
                    }
                })
            });
            l
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

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| {
            if let WindowEvent::MouseDown(MouseButton::Right) = we {
                let s = cx.scale_factor();
                cx.emit(AppEvent::ContextMenu(crate::edit::MenuSite::Spectrum, cx.mouse().cursor_x / s, cx.mouse().cursor_y / s));
                meta.consume();
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Spectrum);
        let b = cx.bounds();
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        let dt = gfx::frame_dt(&self.last_draw);
        self.animating.set(self.step(dt));
        gfx::well_cached(canvas, r, RADIUS_KEY_BIG * s, s);
        let style = SpectrumStyle::from_key(&self.s.settings.with_untracked(|st| st.spectrum_style.clone()));

        let st = self.st.borrow();
        let hl = gfx::hairline(s);
        let foot = (r.bottom - hl - SPECTRUM_PAD_BOTTOM * s).round();
        // The tallest bar leaves room for its peak mark under the top border.
        let full = foot - (r.top + hl) - (PEAK_H + PEAK_GAP) * s;
        let w = (SPECTRUM_BAR_W * s).round();
        let x_of = |k: usize| (r.left + hl + (SPECTRUM_PAD_X + k as f32 * SPECTRUM_PITCH) * s).round();
        let bar_paint = |top: f32| {
            gfx::shaded(gfx::linear((0.0, foot), (0.0, top), &[BAR_FOOT, BAR_MID, BAR_TOP], Some(&[0.0, 0.6, 1.0])))
        };
        let peak = gfx::fill(PEAK_INK.with_alpha(PEAK_ALPHA));
        let mut line = vg::PathBuilder::new();
        for k in 0..SPECTRUM_BARS {
            let x = x_of(k);
            let h = (st.bars[k] * full).round();
            let ph = (st.peaks[k] * full).round();
            match style {
                SpectrumStyle::Bars => {
                    if h >= 1.0 {
                        canvas.draw_rect(vg::Rect::from_ltrb(x, foot - h, x + w, foot), &bar_paint(foot - h));
                    }
                }
                SpectrumStyle::Leds => {
                    // Segments share one gradient over the full height, so
                    // taller columns reach the brighter LEDs.
                    let paint = bar_paint(foot - full);
                    let n = (h / (LED_PITCH * s)).floor() as i32;
                    for i in 0..n {
                        let y = foot - (i as f32 + 1.0) * LED_PITCH * s + (LED_PITCH - LED_H) * s;
                        canvas.draw_rect(vg::Rect::from_xywh(x, y.round(), w, (LED_H * s).round()), &paint);
                    }
                }
                SpectrumStyle::Line => {
                    let p = (x + w * 0.5, foot - h);
                    if k == 0 {
                        line.move_to(p);
                    } else {
                        line.line_to(p);
                    }
                }
                SpectrumStyle::Dots => {
                    if h >= 1.0 {
                        canvas.draw_rect(vg::Rect::from_xywh(x, foot - h, w, w), &gfx::fill(BAR_TOP));
                    }
                }
            }
            // The peak mark (not on the line, which is its own peak).
            if style != SpectrumStyle::Line && st.peaks[k] > 0.01 {
                let top = foot - ph.max(h) - (PEAK_GAP + PEAK_H) * s;
                canvas.draw_rect(vg::Rect::from_xywh(x, top.round(), w, (PEAK_H * s).round()), &peak);
            }
        }
        if style == SpectrumStyle::Line && st.bars.iter().any(|b| *b > 0.0) {
            let path = line.detach();
            // A faint fill under the line, then the line.
            let mut area = vg::PathBuilder::new();
            area.add_path(&path);
            area.line_to((x_of(SPECTRUM_BARS - 1) + w * 0.5, foot));
            area.line_to((x_of(0) + w * 0.5, foot));
            area.close();
            canvas.draw_path(&area.detach(), &gfx::fill(BAR_MID.with_alpha(0.15)));
            let mut p = gfx::stroke(BAR_TOP, 1.5 * s);
            p.set_stroke_join(vg::PaintJoin::Round);
            canvas.draw_path(&path, &p);
        }
    }
}

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

/// The colour grid inside a menu (polish §3.4): 16 px cells, 4 px apart,
/// 14 to a row, 8 px in from the menu's padding.
const GRID_CELL: f32 = 16.0;
const GRID_GAP: f32 = 4.0;
const GRID_COLS: usize = crate::theme::palette::COLS;
const GRID_PAD: f32 = 8.0;

fn row_h(it: &crate::state::MenuItem) -> f32 {
    if it.grid {
        0.0
    } else if it.action == crate::state::MenuAction::Separator {
        MENU_SEP_H
    } else {
        MENU_ROW_H
    }
}

/// The grid block's size (logical px) for `n` cells.
fn grid_size(n: usize) -> (f32, f32) {
    if n == 0 {
        return (0.0, 0.0);
    }
    let rows = n.div_ceil(GRID_COLS) as f32;
    let cols = n.min(GRID_COLS) as f32;
    (2.0 * GRID_PAD + cols * GRID_CELL + (cols - 1.0) * GRID_GAP, 2.0 * GRID_PAD + rows * GRID_CELL + (rows - 1.0) * GRID_GAP)
}

/// A menu's size (logical px).
pub fn menu_size_of(items: &[crate::state::MenuItem]) -> (f32, f32) {
    let rows: f32 = items.iter().map(row_h).sum();
    let reason = if items.iter().any(|i| !i.enabled) { MENU_REASON_H } else { 0.0 };
    let (gw, gh) = grid_size(items.iter().filter(|i| i.grid).count());
    (MENU_W.max(gw + 2.0 * MENU_PAD), MENU_PAD * 2.0 + rows + gh + reason)
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

    /// The row (or grid cell) under the mouse and its top (logical px from
    /// the menu's top).
    fn row_at(&self, cx: &EventContext) -> Option<(usize, f32)> {
        if !mouse_inside(cx) {
            return None;
        }
        let (x, y) = local_mouse(cx);
        let mut top = MENU_PAD;
        for (i, it) in self.items.iter().enumerate() {
            if it.grid {
                continue;
            }
            let h = row_h(it);
            if y >= top && y < top + h {
                return (it.action != crate::state::MenuAction::Separator).then_some((i, top));
            }
            top += h;
        }
        // The grid below the rows.
        self.items.iter().enumerate().filter(|(_, it)| it.grid).enumerate().find_map(|(k, (i, _))| {
            let c = Self::cell(k, top);
            (x >= c.0 && x < c.0 + GRID_CELL && y >= c.1 && y < c.1 + GRID_CELL).then_some((i, c.1))
        })
    }

    /// Cell `k`'s top-left (logical px from the menu's top-left), the grid
    /// starting at `top`.
    fn cell(k: usize, top: f32) -> (f32, f32) {
        let (col, row) = ((k % GRID_COLS) as f32, (k / GRID_COLS) as f32);
        (MENU_PAD + GRID_PAD + col * (GRID_CELL + GRID_GAP), top + GRID_PAD + row * (GRID_CELL + GRID_GAP))
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
                    let was_grid = self.hover.is_some_and(|h| self.items[h].grid);
                    self.hover = r.map(|r| r.0);
                    cx.needs_redraw();
                    // A colour cell previews its colour on the selection.
                    match self.hover.filter(|h| self.items[*h].grid) {
                        Some(h) => cx.emit(AppEvent::ColourPreview(Some(self.items[h].action))),
                        None if was_grid => cx.emit(AppEvent::ColourPreview(None)),
                        None => {}
                    }
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
                if self.hover.is_some_and(|h| self.items[h].grid) {
                    cx.emit(AppEvent::ColourPreview(None));
                }
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
            if item.grid {
                continue;
            }
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
                canvas.draw_rrect(gfx::rrect(row, RADIUS_KEY * s), &gfx::vgrad(row, NEUTRAL_HI.lighten(0.05), NEUTRAL_HI.darken(0.15)));
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
                let (st, sb) = crate::theme::palette::swatch(c);
                canvas.draw_rrect(srr, &gfx::vgrad(sw, st, sb));
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
        // The colour grid: swatches in the menu's swatch material; the
        // current colour has a neutral 2 px ring, the hovered one a hairline.
        let grid: Vec<(usize, &crate::state::MenuItem)> = self.items.iter().enumerate().filter(|(_, it)| it.grid).collect();
        if !grid.is_empty() {
            if top > MENU_PAD {
                canvas.draw_rect(vg::Rect::from_xywh(r.left + (MENU_PAD + 8.0) * s, (r.top + top * s).round(), r.width() - 2.0 * (MENU_PAD + 8.0) * s, gfx::hairline(s)), &gfx::fill(LINE));
            }
            for (k, (i, it)) in grid.iter().enumerate() {
                let (gx, gy) = Self::cell(k, top);
                let cell = gfx::snap(vg::Rect::from_xywh(r.left + gx * s, r.top + gy * s, GRID_CELL * s, GRID_CELL * s));
                let c = it.swatch.unwrap_or(STROKE);
                let crr = gfx::rrect(cell, 3.0 * s);
                let (st, sb) = crate::theme::palette::swatch(c);
                canvas.draw_rrect(crr, &gfx::vgrad(cell, st, sb));
                gfx::ring(canvas, &crr, gfx::hairline(s), &gfx::fill(Rgba::black(0.4)));
                if it.checked || self.hover == Some(*i) {
                    let hl = gfx::hairline(s);
                    let w = if it.checked { 2.0 * hl } else { hl };
                    let outer = gfx::rrect(cell.with_outset((w + hl, w + hl)), 4.0 * s);
                    gfx::ring(canvas, &outer, w, &gfx::fill(if it.checked { NEUTRAL_HI } else { TEXT_MID }));
                }
            }
            top += grid_size(grid.len()).1;
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
    fn max_pane_h(&self, cx: &EventContext) -> f32 {
        let s = cx.scale_factor();
        let win_h = cx.cache.get_bounds(Entity::root()).h / s;
        let room = win_h - TOPBAR_HEIGHT - RULER_HEIGHT - SPLIT_HANDLE_H - PIANO_MIN_H - crate::chrome_bands(&self.s);
        let room = (room / SP_1).floor() * SP_1;
        let content = self.s.arrangement.with_untracked(|a| a.lanes_height());
        room.clamp(crate::state::PIANO_PANE_MIN, content.max(crate::state::PIANO_PANE_MIN))
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
                    let h = h.clamp(crate::state::PIANO_PANE_MIN, self.max_pane_h(cx));
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
                let h = crate::state::PIANO_PANE_DEFAULT.min(self.max_pane_h(cx));
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
            let c = if active { NEUTRAL_HI } else { TEXT_MID };
            if active {
                canvas.draw_rrect(grr, &glow_paint(NEUTRAL_HI.with_alpha(0.5), 3.0 * s));
            }
            canvas.draw_rrect(grr, &gfx::vgrad(g, c.lighten(0.1), c.darken(0.2)));
        } else {
            canvas.draw_rrect(grr, &gfx::fill(BG_INSET));
            gfx::ring_vgrad(canvas, &grr, hl, Rgba::black(0.6), KEY_STROKE);
        }
    }
}

// ─────────────────────────────── Track rows ───────────────────────────────
//
// Polish 2 (E1): VISUAL-SPEC.md §4. Positions are logical px from the row's
// top-left; the header layout (views/header_layout.rs) places the parts for
// the header's width, side and strip position. The number, name block and
// column A keep a 72 px band at the top of a taller row; only the fader
// stretches (`row::fader_len`).

use super::header_layout::{self as hl_, row, HeaderLayout, COL_A_W, COL_B_W, STRIP_W};

/// The channel number: Inter 800 italic 40, centred in its 28 px column on
/// a 40 px line box from y 16; tracking −.02em.
const NUMBER_PX: f32 = 40.0;
const NUMBER_TOP: f32 = 16.0;
const NUMBER_TRACKING: f32 = -0.02;
/// Name: Inter 600 14, its line box (normal line height) from y 14.5.
const NAME_PX: f32 = 14.0;
const NAME_TOP: f32 = 14.5;
/// The profile badge: 16 px inside a 1 px border (18 px), radius 4, a lock
/// and the chip name (Inter 600 11, #A8A8A8) with 4 px padding and gap.
const BADGE_Y: f32 = 40.0;
const BADGE_H: f32 = 18.0;
const BADGE_PAD: f32 = 4.0;
const BADGE_PX: f32 = 11.0;
const LOCK_W: f32 = 9.0;
const LOCK_H: f32 = 11.0;
/// Badge text and preset name share this baseline.
const BADGE_BASELINE: f32 = 53.0;
/// The preset name: Inter 500 12, #808080, 8 px after the badge.
const PRESET_PX: f32 = 12.0;
const PRESET_GAP: f32 = 8.0;
/// The pan and dB values: Inter 600 10 on a 12 px line, tracking .02em,
/// #9A9A9A, centred in their column.
const VALUE_PX: f32 = 10.0;
const VALUE_TRACKING: f32 = 0.02;
const VALUE_INK: Rgba = Rgba::hex(0x9A9A9A);
/// dB scale labels: Inter 600 9 on a 10 px line, #808080; ticks #3A3A3A.
const SCALE_PX: f32 = 9.0;
const SCALE_INK: Rgba = Rgba::hex(0x808080);
const SCALE_TICK: Rgba = Rgba::hex(0x3A3A3A);
/// The pan fill from the centre.
const PAN_FILL: Rgba = Rgba::hex(0x8A8A8A);
/// The bottom-edge band that resizes a row (logical px), and the height
/// change per Alt+wheel notch (three 4 px steps).
pub const ROW_EDGE_GRAB: f32 = 4.0;
pub const ROW_WHEEL_STEP: f32 = 12.0;
/// The header meter's fill: the pan grey (VISUAL-SPEC.md §13, G8; it was a
/// cool-to-hot gradient with a lime stop). Lime stays for toggles.
pub const METER_FILL: Rgba = PAN_FILL;
/// The clip mark above each header meter (G8): 4 × 2 px at y 1, clear of
/// the thumb at +6 dB (its top is y 4). Unlit: the well's dark; lit: bright
/// white with a soft glow, held until clicked.
pub const CLIP_Y: f32 = 1.0;
pub const CLIP_H: f32 = 2.0;
/// The press band for the clip mark (the fader's view starts at the row's
/// top; below it a press moves the fader).
pub const CLIP_HIT_H: f32 = 4.0;
pub const CLIP_OFF: Rgba = Rgba::hex(0x2A2A2A);
pub const CLIP_ON: Rgba = Rgba::hex(0xFFFFFF);
const CLIP_GLOW_R: f32 = 4.0;
/// The strip's lit look: `inset 0 0 10px` white and a `0 0 10px` spill in
/// the track colour (alphas: the palette's strip recipe).
const STRIP_GLOW_R: f32 = 10.0;

/// The strip's colours: the palette recipe (theme::palette::strip).
fn strip_colours(color: Rgba, light: f32) -> ([Rgba; 3], Rgba, Rgba) {
    crate::theme::palette::strip(color, light)
}

/// The rects (logical, row-relative) a row's value fields occupy: the pan
/// value and the dB value (double-click types a value, polish 2 §3).
pub fn pan_value_rect(l: &HeaderLayout) -> (f32, f32, f32, f32) {
    (l.col_a(), row::PAN_VALUE_Y, COL_A_W, row::VALUE_LINE)
}

pub fn db_value_rect(l: &HeaderLayout, h: f32) -> (f32, f32, f32, f32) {
    (l.col_b(), row::db_value_y(h), COL_B_W, row::VALUE_LINE)
}

fn inside(r: (f32, f32, f32, f32), x: f32, y: f32) -> bool {
    x >= r.0 && x < r.0 + r.2 && y >= r.1 - 2.0 && y < r.1 + r.3 + 2.0
}

/// The hover-help line for a point in a row (polish 2 §6.5).
pub fn row_help(l: &HeaderLayout, h: f32, x: f32, y: f32) -> &'static str {
    if y >= h + 1.0 - ROW_EDGE_GRAB {
        "Track height: drag the row's bottom edge · Alt+wheel over a track"
    } else if inside(db_value_rect(l, h), x, y) || (x >= l.col_b() && x < l.col_b() + COL_B_W) {
        VolumeFader::HELP
    } else if inside(pan_value_rect(l), x, y) || (x >= l.col_a() - PanSlider::PAD && x < l.col_a() + COL_A_W + PanSlider::PAD && (row::PAN_WELL_Y - 6.0..row::PAN_WELL_Y + 10.0).contains(&y)) {
        PanSlider::HELP
    } else if (row::KEYS_Y..row::KEYS_Y + KEY_SIZE).contains(&y) && x >= l.col_a() && x < l.col_a() + KEY_SIZE {
        "Mute this track"
    } else if (row::KEYS_Y..row::KEYS_Y + KEY_SIZE).contains(&y) && x >= l.col_a() + KEY_SIZE + row::KEY_GAP && x < l.col_a() + 2.0 * KEY_SIZE + row::KEY_GAP {
        if crate::shortcuts::MACOS { "Solo this track only · Cmd-click to solo several" } else { "Solo this track only · Ctrl-click to solo several" }
    } else {
        "Track: click to select · double-click the name to rename · right-click for its menu"
    }
}

/// One track row in the header column: colour strip (lights on each
/// note-on), channel number, name, profile badge and preset, and the
/// control block (M/S keys, pan, the vertical volume meter-fader: child
/// views, placed by the header layout).
pub struct TrackRow {
    s: Signals,
    lane: usize,
    hover: bool,
    /// The lit strip, pre-rendered: (colour, height, scale) and the image.
    lit: LitCache,
    /// The row's text, values and scale marks, pre-rendered.
    text: RefCell<Option<(RowTextKey, vg::Image)>>,
    /// A bottom-edge resize in progress: (press y in window logical px, height then).
    resize: Option<(f32, f32)>,
    edge_hover: bool,
    /// A press on the row (window logical y, on the badge): a click or the
    /// start of a header drag (T4).
    press: Option<(f32, bool)>,
    badge_hover: bool,
    /// ⌥+wheel row height in whole steps. A trackpad sends many small deltas
    /// per swipe, and one step per event took a short swipe from 72 to 300 px
    /// (MAC-CATCHUP.md, MC-D14); a mouse notch is still exactly one step.
    #[cfg(target_os = "macos")]
    wheel: crate::input::WheelSteps,
}

/// The lit strip: (colour, height, scale) and its image.
type LitCache = RefCell<Option<((u32, i32, u32), vg::Image)>>;

/// What a row's text layer shows.
#[derive(Clone, Debug, PartialEq)]
struct RowTextKey {
    selected: bool,
    name: String,
    /// The badge (Batch 4a §5.3): the lock glyph and the channel ("2A03 P1"),
    /// or the voice type without a lock ("PULSE"); `warn`: the track breaks
    /// the profile (a warning mark).
    badge: (&'static str, bool, bool),
    inst: Option<String>,
    pan: String,
    db: String,
    layout: (i32, bool, bool, i32),
    w: i32,
    h: i32,
    s: u32,
}

/// A row's number, name, badge, preset, values and dB scale on a
/// transparent layer (grayscale-antialiased text composites exactly as if
/// drawn in place).
fn render_row_text(lane: usize, k: &RowTextKey, l: &HeaderLayout, row_h: f32, s: f32) -> Option<vg::Image> {
    let mut surface = vg::surfaces::raster_n32_premul((k.w.max(1), k.h.max(1)))?;
    let canvas = surface.canvas();
    canvas.clear(vg::Color::TRANSPARENT);
    let r = vg::Rect::from_xywh(0.0, 0.0, k.w as f32, k.h as f32);
    let hl = gfx::hairline(s);
    let at = |x: f32, y: f32| (r.left + x * s, r.top + y * s);
    let selected = k.selected;

    // The channel number.
    let nf = gfx::font(Face::HeavyItalic, NUMBER_PX * s);
    let num = format!("{}", lane + 1);
    let ntr = NUMBER_TRACKING * NUMBER_PX * s;
    let nw = gfx::tracked_width(&num, &nf, ntr);
    let (nx, ny) = at(l.number_x() + l.num_w * 0.5, NUMBER_TOP);
    let nb = gfx::css_baseline(&nf, ny, NUMBER_PX * s).round();
    gfx::draw_text_tracked(canvas, &num, (nx - nw * 0.5).round(), nb, &nf, ntr, NUMBER_INK.lighten(if selected { 0.04 } else { 0.0 }));

    // Name (left-aligned in its column, "…" where it doesn't fit).
    let name_x = l.name_x();
    let name_right = name_x + l.name_w();
    let f = gfx::font(Face::SemiBold, NAME_PX * s);
    let (x, y) = at(name_x, NAME_TOP);
    let (_, m) = f.metrics();
    // Skia's hinting puts the ink a pixel higher than the browser's.
    let base = (y - m.ascent).round() + s.round();
    gfx::draw_text_clipped(canvas, &k.name, x.round(), base, l.name_w() * s, &f, TEXT_HI);

    // The badge (Batch 4a §5.3) and the preset.
    let _ = lane;
    let badge = badge_rect(l, &k.badge, s);
    let badge = vg::Rect::from_xywh(r.left + badge.left, r.top + badge.top, badge.width(), badge.height());
    gfx::ring(canvas, &gfx::rrect(badge, RADIUS_KEY * s), hl, &gfx::fill(if k.badge.2 { WARN_INK } else { STROKE }));
    let bf = gfx::font(Face::SemiBold, BADGE_PX * s);
    let bb = (r.top + BADGE_BASELINE * s).round();
    let mut tx = badge.left + hl + BADGE_PAD * s;
    if k.badge.2 {
        draw_warn_mark(canvas, (tx.round(), (badge.top + (BADGE_H - WARN_H) * 0.5 * s).round()), s);
        tx += (WARN_W + BADGE_PAD) * s;
    }
    if k.badge.1 {
        let lock_o = (tx, (badge.top + (BADGE_H - LOCK_H) * 0.5 * s).round());
        draw_lock(canvas, lock_o, s, BADGE_INK);
        tx += (LOCK_W + BADGE_PAD) * s;
    }
    gfx::draw_text(canvas, k.badge.0, tx.round(), bb, &bf, BADGE_INK);
    if let Some(text) = &k.inst {
        let fi = gfx::font(Face::Medium, PRESET_PX * s);
        let ix = badge.right + PRESET_GAP * s;
        gfx::draw_text_clipped(canvas, text, ix.round(), bb, r.left + name_right * s - ix, &fi, if selected { TEXT_MID } else { TEXT_LO });
    }

    // The pan and dB values, centred in their columns.
    let vf = gfx::font(Face::SemiBold, VALUE_PX * s);
    let vtr = VALUE_TRACKING * VALUE_PX * s;
    let value = |text: &str, rect: (f32, f32, f32, f32)| {
        let w = gfx::tracked_width(text, &vf, vtr);
        let (x, y) = at(rect.0 + rect.2 * 0.5, rect.1);
        let base = gfx::css_baseline(&vf, y, row::VALUE_LINE * s).round();
        gfx::draw_text_tracked(canvas, text, (x - w * 0.5).round(), base, &vf, vtr, VALUE_INK);
    };
    value(&k.pan, pan_value_rect(l));
    if k.badge.1 {
        // Locked (§5.3): a small lock glyph instead of the dB value.
        let rect = db_value_rect(l, row_h);
        let (x, y) = at(rect.0 + (rect.2 - LOCK_W) * 0.5, rect.1 + (row::VALUE_LINE - LOCK_H) * 0.5);
        draw_lock(canvas, (x.round(), y.round()), s, VALUE_INK);
    } else {
        value(&k.db, db_value_rect(l, row_h));
    }

    // dB scale marks beside a tall fader (≥ 96 px rows): ticks end 2 px
    // left of the thumb, labels 2 px left of the ticks. Nothing under the thumb.
    if row_h >= crate::state::SCALE_MIN_ROW_H {
        let len = row::fader_len(row_h);
        let well_x = l.col_b() + (COL_B_W - row::FADER_WELL_W) * 0.5;
        let sf = gfx::font(Face::SemiBold, SCALE_PX * s);
        for (y, label, show) in crate::state::scale_marks(len) {
            let ty = r.top + (row::FADER_TOP + y) * s;
            canvas.draw_rect(vg::Rect::from_xywh(r.left + (well_x + row::TICK_X) * s, ty.round(), row::TICK_W * s, hl), &gfx::fill(SCALE_TICK));
            if show {
                let lw = gfx::text_width(&sf, label);
                let base = gfx::css_baseline(&sf, ty - row::LABEL_LINE * 0.5 * s, row::LABEL_LINE * s).round();
                gfx::draw_text(canvas, label, (r.left + (well_x + row::LABEL_RIGHT) * s - lw).round(), base, &sf, SCALE_INK);
            }
        }
    }

    Some(surface.image_snapshot())
}

impl TrackRow {
    pub fn new(cx: &mut Context, s: Signals, lane: usize) -> Handle<'_, Self> {
        let seen = RefCell::new((false, String::new()));
        // Redraw on this lane's light or meter (PERF.md: only what changed).
        // The meter is the row's child, but redrawing the whole row keeps the
        // damage one contiguous column (rows touch) instead of eight small
        // rects that each cost the renderer a pass (polish pass, POLISH.md).
        let light = Cell::new((0u8, (u16::MAX, 0u16)));
        Self {
            s,
            lane,
            hover: false,
            lit: RefCell::new(None),
            text: RefCell::new(None),
            resize: None,
            edge_hover: false,
            press: None,
            badge_hover: false,
            #[cfg(target_os = "macos")]
            wheel: Default::default(),
        }
            .build(cx, move |cx| {
                Key::new(cx, s, "M", KeyAction::Mute(lane))
                    .position_type(PositionType::Absolute)
                    .left(s.header.map(|l| Pixels(l.col_a())))
                    .top(Pixels(row::KEYS_Y));
                Key::new(cx, s, "S", KeyAction::Solo(lane))
                    .position_type(PositionType::Absolute)
                    .left(s.header.map(|l| Pixels(l.col_a() + KEY_SIZE + row::KEY_GAP)))
                    .top(Pixels(row::KEYS_Y));
                PanSlider::new(cx, s, lane)
                    .position_type(PositionType::Absolute)
                    .left(s.header.map(|l| Pixels(l.col_a() - PanSlider::PAD)))
                    .top(Pixels(row::PAN_WELL_Y + row::PAN_WELL_H * 0.5 - row::HTHUMB_H * 0.5));

                VolumeFader::new(cx, s, lane)
                    .position_type(PositionType::Absolute)
                    .left(s.header.map(|l| Pixels(l.col_b())))
                    .top(Pixels(0.0));
            })
            .class("track-row")
            .bind(s.meters, move |mut h| {
                let l = (s.meters.with_untracked(|m| (m.light_of(lane) * 64.0).round() as u8), VolumeFader::state(&s, lane));
                if light.get() != l {
                    light.set(l);
                    h.needs_redraw();
                }
            })
            .bind(s.header, |mut h| h.needs_redraw())
            .bind(s.drag_preset, |mut h| h.needs_redraw())
            .bind(s.track_drag, |mut h| h.needs_redraw())
            .bind(s.chip, |mut h| h.needs_redraw())
            .bind(s.arrangement, move |mut h| {
                let now = s.arrangement.with_untracked(|a| {
                    let Some(l) = a.lanes.get(lane) else { return (false, String::new()) };
                    (
                        a.first_selected().map(|c| c.lane) == Some(lane) || a.track_sel.contains(&lane),
                        format!("{}\u{1}{}\u{1}{:?}\u{1}{}\u{1}{}\u{1}{}\u{1}{:?}\u{1}{:?}", a.instruments.lane(lane).map(|i| i.label()).unwrap_or_default(), l.name, l.color, l.height, l.pan, l.vol_db, l.lock, l.voice),
                    )
                });
                if *seen.borrow() != now {
                    *seen.borrow_mut() = now;
                    h.needs_redraw();
                }
            })
            .tooltip(move |cx| {
                // The full name, profile and preset (both texts truncate in the row).
                let text = s.arrangement.map(move |a| {
                    let inst = a.instruments.lane(lane).map(|i| i.label()).unwrap_or_default();
                    let Some(l) = a.lanes.get(lane) else { return String::new() };
                    let kind = match l.lock {
                        Some(c) => format!("locked to the {}", crate::tracks::channel_name(c)),
                        None => format!("studio {}", crate::tracks::kind_name(l.voice)),
                    };
                    format!("{} · {kind} · {inst}", l.name)
                });
                Tooltip::new(cx, move |cx| {
                    Label::new(cx, text).class("tip-text").class("long");
                })
                .class("tip")
            })
    }

    fn row_h(&self) -> f32 {
        self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).map_or(crate::state::ROW_H_MIN, |l| l.height))
    }

    /// Whether the press point is on the badge (it opens the lock menu).
    fn on_badge(&self, cx: &EventContext) -> bool {
        let (x, y) = local_mouse(cx);
        let l = self.s.header.get_untracked();
        let b = badge_rect(&l, &badge_of(&self.s, self.lane), 1.0);
        x >= b.left && x < b.right && y >= b.top && y < b.bottom
    }

    fn on_edge(&self, cx: &EventContext) -> bool {
        let (_, y) = local_mouse(cx);
        y >= self.row_h() + crate::state::ROW_HAIRLINE - ROW_EDGE_GRAB
    }
}

impl View for TrackRow {
    fn element(&self) -> Option<&'static str> {
        Some("track-row")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let off = self.s.arrangement.with_untracked(|a| crate::state::lane_off(self.s.chip.get_untracked(), a, self.lane));
        event.map(|we, meta| {
            match we {
                WindowEvent::MouseEnter => {
                    self.hover = true;
                    cx.needs_redraw();
                }
                WindowEvent::MouseLeave => {
                    self.hover = false;
                    if self.resize.is_none() {
                        self.edge_hover = false;
                        cx.emit(WindowEvent::SetCursor(CursorIcon::Default));
                    }
                    cx.emit(AppEvent::HoverHelp(String::new()));
                    cx.needs_redraw();
                }
                WindowEvent::MouseMove(_, _) if self.press.is_some_and(|p| !p.1) => {
                    // Dragging the header: the track under the pointer is where it lands.
                    let y = cx.mouse().cursor_y / cx.scale_factor();
                    let (y0, _) = self.press.unwrap_or_default();
                    if self.s.track_drag.get_untracked().is_some() || (y - y0).abs() >= TRACK_DRAG_START {
                        if self.s.track_drag.get_untracked().is_none() {
                            cx.capture();
                        }
                        let b = cx.bounds();
                        let top = b.y / cx.scale_factor();
                        let to = self.s.arrangement.with_untracked(|a| a.lane_at_clamped(a.lane_top(self.lane) + (y - top)));
                        if self.s.track_drag.get_untracked() != Some((self.lane, to)) {
                            self.s.track_drag.set(Some((self.lane, to)));
                        }
                        cx.emit(WindowEvent::SetCursor(CursorIcon::Grabbing));
                    }
                }
                WindowEvent::MouseMove(_, _) => {
                    if self.badge_hover != self.on_badge(cx) {
                        self.badge_hover = !self.badge_hover;
                        cx.emit(WindowEvent::SetCursor(if self.badge_hover { CursorIcon::Hand } else { CursorIcon::Default }));
                    }
                    if let Some((y0, h0)) = self.resize {
                        let y = cx.mouse().cursor_y / cx.scale_factor();
                        // Alt while dragging: every track takes the height (P2-R20).
                        if cx.modifiers().alt() {
                            let n = self.s.arrangement.with_untracked(|a| a.lanes.len());
                            for l in 0..n {
                                cx.emit(AppEvent::SetLaneHeight(l, h0 + (y - y0)));
                            }
                        } else {
                            cx.emit(AppEvent::SetLaneHeight(self.lane, h0 + (y - y0)));
                        }
                    } else {
                        let edge = self.on_edge(cx);
                        if edge != self.edge_hover {
                            self.edge_hover = edge;
                            cx.emit(WindowEvent::SetCursor(if edge { CursorIcon::NsResize } else { CursorIcon::Default }));
                        }
                    }
                    let (x, y) = local_mouse(cx);
                    let l = self.s.header.get_untracked();
                    cx.emit(AppEvent::HoverHelp(row_help(&l, self.row_h(), x, y).into()));
                }
                // §4.2: drag a track's bottom edge to change its height (one undo step).
                WindowEvent::MouseDown(MouseButton::Left) if self.on_edge(cx) => {
                    self.resize = Some((cx.mouse().cursor_y / cx.scale_factor(), self.row_h()));
                    cx.emit(AppEvent::GestureBegin);
                    cx.capture();
                    meta.consume();
                }
                WindowEvent::MouseUp(MouseButton::Left) if self.resize.is_some() => {
                    self.resize = None;
                    cx.release();
                    cx.emit(AppEvent::GestureEnd);
                    meta.consume();
                }
                // The badge (§5.2): its lock menu opens on release (a menu
                // opened on press takes the release on its first row).
                WindowEvent::MouseDown(MouseButton::Left) if self.on_badge(cx) => {
                    self.press = Some((cx.mouse().cursor_y / cx.scale_factor(), true));
                    cx.capture();
                    meta.consume();
                }
                // T1: a click selects the track (Shift / Ctrl / Cmd adds); a
                // drag moves it (T4, R88).
                WindowEvent::MouseDown(MouseButton::Left) if !off => {
                    let m = *cx.modifiers();
                    cx.emit(AppEvent::EditorFocus(crate::shortcuts::Ctx::Arrangement));
                    cx.emit(AppEvent::SelectTrack(self.lane, m.shift() || crate::input::command(&m)));
                    // Keys (Delete, Ctrl+D, Ctrl+R) now act on the track: take the
                    // keyboard focus from wherever it was (a text field keeps Delete).
                    cx.focus();
                    // Captured only once it turns into a drag: the row's M / S keys
                    // and sliders still get their own release.
                    self.press = Some((cx.mouse().cursor_y / cx.scale_factor(), false));
                    meta.consume();
                }
                WindowEvent::MouseUp(MouseButton::Left) if self.press.is_some() => {
                    let (_, badge) = self.press.take().unwrap_or_default();
                    if badge || self.s.track_drag.get_untracked().is_some() {
                        cx.release();
                    }
                    if let Some((from, to)) = self.s.track_drag.get_untracked() {
                        self.s.track_drag.set(None);
                        // Keys go on to act on the moved track (Delete right after
                        // the drop): keep the keyboard focus here.
                        cx.focus();
                        if from != to {
                            cx.emit(AppEvent::MoveTrack(from, to));
                        }
                    } else if badge && self.on_badge(cx) {
                        let sc = cx.scale_factor();
                        let b = cx.bounds();
                        let l = self.s.header.get_untracked();
                        let br = badge_rect(&l, &badge_of(&self.s, self.lane), 1.0);
                        cx.emit(AppEvent::SelectTrack(self.lane, false));
                        cx.emit(AppEvent::ContextMenu(crate::edit::MenuSite::Badge(self.lane), b.x / sc + br.left, b.y / sc + br.bottom + 2.0));
                    }
                    meta.consume();
                }
                // A double-click on a value types it (§3); elsewhere it renames (T2).
                WindowEvent::MouseDoubleClick(MouseButton::Left) if !off => {
                    let (x, y) = local_mouse(cx);
                    let l = self.s.header.get_untracked();
                    let target = if inside(db_value_rect(&l, self.row_h()), x, y) {
                        crate::edit::RenameTarget::Volume(self.lane)
                    } else if inside(pan_value_rect(&l), x, y) {
                        crate::edit::RenameTarget::Pan(self.lane)
                    } else {
                        crate::edit::RenameTarget::Track(self.lane)
                    };
                    cx.emit(AppEvent::OpenRename(target));
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
            if let WindowEvent::MouseScroll(_, sy) = we {
                // §4.2: Alt+wheel over a track changes its height (R35, P2-R).
                if cx.modifiers().alt() && *sy != 0.0 {
                    #[cfg(target_os = "macos")]
                    let steps = self.wheel.steps(*sy) as f32;
                    #[cfg(not(target_os = "macos"))]
                    let steps = sy.signum();
                    if steps != 0.0 {
                        cx.emit(AppEvent::SetLaneHeight(self.lane, self.row_h() + steps * ROW_WHEEL_STEP));
                        cx.emit(AppEvent::Commit);
                    }
                    meta.consume();
                } else if self.s.lane_scroll_max.get_untracked() > 0.0 && *sy != 0.0 {
                    // The header column scrolls with the lanes.
                    cx.emit(AppEvent::ScrollLanes(-*sy * super::arrangement::LANE_SCROLL_PER_NOTCH));
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
        let hl = gfx::hairline(s);
        let lay = self.s.header.get_untracked();
        let selected = self
            .s
            .arrangement
            .with_untracked(|a| a.first_selected().map(|c| c.lane) == Some(self.lane) || a.track_sel.contains(&self.lane));
        let Some((color, name, row_h, pan, vol)) = self.s.arrangement.with_untracked(|a| {
            let l = a.lanes.get(self.lane)?;
            Some((l.color, l.name.clone(), l.height, l.pan, l.vol_db))
        }) else {
            return;
        };
        let light = self.s.meters.with_untracked(|m| m.light_of(self.lane));

        // Row surface (selection lifts it) and the 1 px bottom hairline (the
        // header's divider is its own view beside the rows).
        let body = vg::Rect::from_ltrb(r.left, r.top, r.right, r.bottom - hl);
        canvas.draw_rect(body, &gfx::fill(if selected { BG_0.lighten(0.03) } else { BG_0 }));
        let full = body;
        canvas.draw_rect(vg::Rect::from_ltrb(r.left, r.bottom - hl, r.right, r.bottom), &gfx::fill(LINE));

        // The colour strip: full height, square, light left edge → dark
        // right edge; lit by note-ons (inner glow and a soft spill toward
        // the header's inside).
        let sx = (r.left + lay.strip_x() * s).round();
        let strip = vg::Rect::from_ltrb(sx, full.top, (sx + STRIP_W * s).round(), full.bottom);
        let (fill, edge_l, edge_r) = strip_colours(color, 0.0);
        canvas.draw_rect(
            strip,
            &gfx::shaded(gfx::linear((strip.left, strip.top), (strip.right, strip.top), &fill, Some(&[0.0, 0.55, 1.0]))),
        );
        canvas.draw_rect(vg::Rect::from_xywh(strip.left, strip.top, hl, strip.height()), &gfx::fill(edge_l));
        canvas.draw_rect(vg::Rect::from_xywh(strip.right - hl, strip.top, hl, strip.height()), &gfx::fill(edge_r));
        if light > 0.01 {
            let key = (crate::state::rgb_hex(color), strip.height().round() as i32, (s * 100.0).round() as u32);
            let mut lit = self.lit.borrow_mut();
            if lit.as_ref().is_none_or(|(k, _)| *k != key) {
                *lit = render_lit_strip(color, strip.width(), strip.height(), s).map(|img| (key, img));
            }
            if let Some((_, img)) = lit.as_ref() {
                let mut p = vg::Paint::default();
                p.set_alpha_f(light.clamp(0.0, 1.0));
                let pad = lit_pad(s);
                canvas.save();
                // The spill shows only toward the header's inside.
                let clip = vg::Rect::from_ltrb(body.left.min(strip.left), body.top, body.right.max(strip.right), body.bottom);
                canvas.clip_rect(clip, None, None);
                if !lay.spill_right() {
                    canvas.clip_rect(vg::Rect::from_ltrb(clip.left, clip.top, strip.right, clip.bottom), None, None);
                } else {
                    canvas.clip_rect(vg::Rect::from_ltrb(strip.left, clip.top, clip.right, clip.bottom), None, None);
                }
                canvas.draw_image(img, (strip.left - pad, strip.top), Some(&p));
                canvas.restore();
            }
        }

        // The number, name, badge, preset, values and scale: one image per
        // content and scale (a row redraws every frame while its strip or
        // meter moves).
        let inst = self.s.arrangement.with_untracked(|a| a.instruments.lane(self.lane).map(|i| if i.is_kit() { format!("{}  (kit)", i.label()) } else { i.label() }));
        let badge = badge_of(&self.s, self.lane);
        let key = RowTextKey {
            selected,
            name,
            badge,
            inst,
            pan: crate::state::format_pan(pan),
            db: crate::state::format_db(vol),
            layout: (lay.w as i32, lay.right, lay.by_clips, lay.num_w as i32),
            w: r.width().round() as i32,
            h: r.height().round() as i32,
            s: (s * 100.0).round() as u32,
        };
        {
            let mut text = self.text.borrow_mut();
            if text.as_ref().is_none_or(|(k, _)| *k != key) {
                *text = render_row_text(self.lane, &key, &lay, row_h, s).map(|img| (key, img));
            }
            if let Some((_, img)) = text.as_ref() {
                canvas.draw_image(img, (r.left, r.top), None);
            }
        }
        // The pan slider, where its (hit-only) view sits: drawn here so a
        // row repaint is one view, not two.
        if badge.1 {
            // Locked (§5.3): centred and disabled (a cached image at the
            // disabled opacity, not a layer every frame: B4A-D23).
            let pad = (8.0 * s).ceil();
            let size = (((COL_A_W + 16.0) * s).ceil() as i32 + 2, (20.0 * s).ceil() as i32 + 2);
            draw_disabled(canvas, DisabledPart::Pan, s, size, ((r.left + lay.col_a() * s).round() - pad, (r.top + row::PAN_WELL_Y * s).round() - pad), |c| {
                draw_pan(c, pad, pad, 0, s);
            });
        } else {
            draw_pan(canvas, (r.left + lay.col_a() * s).round(), (r.top + row::PAN_WELL_Y * s).round(), pan, s);
        }
        // A header being dragged (T4): a 2 px line where it would land.
        if let Some((from, to)) = self.s.track_drag.get_untracked() {
            if to == self.lane && from != to {
                let y = if to > from { body.bottom - 2.0 * s } else { body.top };
                canvas.draw_rect(vg::Rect::from_xywh(r.left, y.round(), r.width(), (2.0 * s).round()), &gfx::fill(NEUTRAL_HI));
            }
            if from == self.lane {
                canvas.draw_rect(body, &gfx::fill(Rgba::white(0.04)));
            }
        }

        // F-04: a VRC6 lane under the 2A03 profile: dimmed, and it says why.
        let bb = (r.top + BADGE_BASELINE * s).round();
        if self.s.arrangement.with_untracked(|a| crate::state::lane_off(self.s.chip.get_untracked(), a, self.lane)) {
            canvas.draw_rect(body, &gfx::fill(Rgba::black(super::arrangement::OFF_DIM)));
            let fh = gfx::font(Face::Medium, PRESET_PX * s);
            let text = if self.hover { "Off: turn on VRC6 in File > Project Settings…" } else { "Off in the 2A03 profile" };
            let hx = (r.left + lay.name_x() * s).round();
            let max_w = (lay.name_w() + hl_::CTRL_W) * s;
            // An opaque band under the text so the badge below it doesn't show through.
            let band = vg::Rect::from_ltrb(hx - 2.0 * s, bb - 12.0 * s, hx + max_w, bb + 5.0 * s);
            canvas.draw_rect(band, &gfx::fill(BG_0));
            gfx::draw_text_clipped(canvas, text, hx, bb, max_w, &fh, TEXT_HI);
        }

        // Dragging a preset from the browser: dim the lanes it can't play
        // on, outline the one it would land on (neutral, not lime).
        if let Some((fits, target)) = super::browser::drag_state(&self.s, self.lane) {
            if !fits {
                canvas.draw_rect(body, &gfx::fill(Rgba::black(0.62)));
            } else if target {
                canvas.draw_rect(body, &gfx::fill(Rgba::white(0.05)));
                gfx::ring(canvas, &gfx::rrect(body, 0.0), 2.0 * s, &gfx::fill(NEUTRAL_HI.with_alpha(0.8)));
            }
        }
        // The resize edge under the mouse (or being dragged): a neutral line.
        if self.edge_hover || self.resize.is_some() {
            canvas.draw_rect(vg::Rect::from_ltrb(body.left, r.bottom - 2.0 * hl, body.right, r.bottom), &gfx::fill(NEUTRAL_HI.with_alpha(0.5)));
        }
    }
}

/// The lit strip image's padding on each side (room for the spill).
pub fn lit_pad(s: f32) -> f32 {
    (2.0 * STRIP_GLOW_R * s).ceil()
}

/// The fully lit strip (fill, edges, inner glow) plus its spill on both
/// sides, as an image whose strip starts `lit_pad` px from its left.
pub fn render_lit_strip(color: Rgba, w: f32, h: f32, s: f32) -> Option<vg::Image> {
    use crate::theme::palette::{STRIP_GLOW_A, STRIP_SPILL_A};
    let pad = lit_pad(s);
    let mut surface = vg::surfaces::raster_n32_premul(((w + 2.0 * pad).ceil() as i32, h.ceil().max(1.0) as i32))?;
    let c = surface.canvas();
    c.clear(vg::Color::TRANSPARENT);
    c.translate((pad, 0.0));
    let strip = vg::Rect::from_xywh(0.0, 0.0, w, h);
    let hl = gfx::hairline(s);
    let mut spill = gfx::fill(color.with_alpha(STRIP_SPILL_A));
    spill.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, STRIP_GLOW_R * 0.5 * s, false));
    c.draw_rect(strip, &spill);
    let (fill, edge_l, edge_r) = strip_colours(color, 1.0);
    c.draw_rect(strip, &gfx::shaded(gfx::linear((0.0, 0.0), (w, 0.0), &fill, Some(&[0.0, 0.55, 1.0]))));
    c.draw_rect(vg::Rect::from_xywh(0.0, 0.0, hl, h), &gfx::fill(edge_l));
    c.draw_rect(vg::Rect::from_xywh(w - hl, 0.0, hl, h), &gfx::fill(edge_r));
    let inner = gfx::rrect(strip.with_inset((hl, 0.0)), 0.0);
    gfx::css_inset_shadow(c, &inner, 0.0, 0.0, STRIP_GLOW_R * s, 0.0, Rgba::white(STRIP_GLOW_A));
    Some(surface.image_snapshot())
}

/// The badge's lock (the mockup's 9 × 11 icon).
fn draw_lock(canvas: &Canvas, o: (f32, f32), s: f32, ink: Rgba) {
    let body = vg::Rect::from_xywh(o.0 + 0.5 * s, o.1 + 4.5 * s, 8.0 * s, 6.0 * s);
    canvas.draw_rrect(gfx::rrect(body, 1.2 * s), &gfx::fill(ink));
    let shackle = gfx::svg_path("M2.3 4.5V3a2.2 2.2 0 0 1 4.4 0v1.5", o, s);
    canvas.draw_path(&shackle, &gfx::stroke(ink, 1.2 * s));
}

/// How far a header press moves before it is a drag (logical px).
const TRACK_DRAG_START: f32 = 6.0;

/// The badge's box in row coordinates (device px): x = the name column,
/// y = `BADGE_Y`; its width follows its content.
fn badge_rect(l: &HeaderLayout, b: &(&'static str, bool, bool), s: f32) -> vg::Rect {
    // The badge texts are a few fixed strings: measure each once per scale
    // (building a font and measuring on every row draw and mouse move was
    // this batch's main per-frame cost, B4A-D23).
    thread_local! {
        static WIDTHS: std::cell::RefCell<Vec<(&'static str, u32, f32)>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    let text_w = WIDTHS.with(|c| {
        let mut c = c.borrow_mut();
        let key = s.to_bits();
        match c.iter().find(|(t, k, _)| *t == b.0 && *k == key) {
            Some(&(_, _, w)) => w,
            None => {
                let w = gfx::text_width(&gfx::font(Face::SemiBold, BADGE_PX * s), b.0);
                c.push((b.0, key, w));
                w
            }
        }
    });
    let mut w = text_w + (2.0 + 2.0 * BADGE_PAD) * s;
    if b.1 {
        w += (LOCK_W + BADGE_PAD) * s;
    }
    if b.2 {
        w += (WARN_W + BADGE_PAD) * s;
    }
    vg::Rect::from_xywh((l.name_x() * s).round(), (BADGE_Y * s).round(), w.round(), BADGE_H * s)
}

/// What a track's badge shows (§5.3): locked: the lock and the channel;
/// studio: the voice type; with a warning mark when the track breaks the
/// profile (locked to a channel the profile lacks).
pub(crate) fn badge_of(s: &Signals, lane: usize) -> (&'static str, bool, bool) {
    let chip = s.chip.get_untracked();
    s.arrangement.with_untracked(|a| match a.lanes.get(lane) {
        Some(l) => match l.lock {
            Some(c) => (crate::tracks::channel_badge(c), true, !chip.has_channel(c)),
            None => (crate::tracks::kind_badge(l.voice), false, false),
        },
        None => ("", false, false),
    })
}

/// The amber warning mark (the status bar's, polish 2 §6.4: not lime).
const WARN_W: f32 = 9.0;
const WARN_H: f32 = 8.0;
pub(crate) const WARN_INK: Rgba = Rgba::hex(0xF0A23A);

fn draw_warn_mark(canvas: &Canvas, o: (f32, f32), s: f32) {
    let p = gfx::svg_path("M4.5 0.5L8.5 7.5H0.5Z", o, s);
    canvas.draw_path(&p, &gfx::fill(WARN_INK));
    canvas.draw_rect(vg::Rect::from_xywh(o.0 + 4.0 * s, o.1 + 2.5 * s, s, 2.5 * s), &gfx::fill(Rgba::hex(0x0A0A0A)));
    canvas.draw_rect(vg::Rect::from_xywh(o.0 + 4.0 * s, o.1 + 5.8 * s, s, s), &gfx::fill(Rgba::hex(0x0A0A0A)));
}

/// A locked track's controls are drawn at this opacity (VISUAL-SPEC.md
/// §4b: the disabled style of drawn controls).
pub(crate) const DISABLED_ALPHA: f32 = 0.4;

/// The disabled controls a locked row shows: always the same picture (the
/// pan centred, the thumb at 0 dB), so each is drawn once per scale.
#[derive(Clone, Copy, PartialEq)]
enum DisabledPart {
    Pan,
    Thumb,
}

/// Draws a locked control from a cached image at `DISABLED_ALPHA`, its
/// top-left at `at` (whole pixels). `draw` paints it into the image at full
/// opacity. Replaces an offscreen layer per locked row per frame.
fn draw_disabled(canvas: &Canvas, part: DisabledPart, s: f32, size: (i32, i32), at: (f32, f32), draw: impl FnOnce(&Canvas)) {
    thread_local! {
        static CACHE: std::cell::RefCell<Vec<(DisabledPart, u32, vg::Image)>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    let img = CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if let Some((_, _, img)) = c.iter().find(|(p, k, _)| *p == part && *k == s.to_bits()) {
            return Some(img.clone());
        }
        let mut surface = vg::surfaces::raster_n32_premul(size)?;
        surface.canvas().clear(vg::Color::TRANSPARENT);
        draw(surface.canvas());
        let img = surface.image_snapshot();
        c.push((part, s.to_bits(), img.clone()));
        Some(img)
    });
    if let Some(img) = img {
        let mut p = vg::Paint::default();
        p.set_alpha_f(DISABLED_ALPHA);
        canvas.draw_image(&img, at, Some(&p));
    }
}

/// The slider well: `#050505` with `inset 0 1px 0 #000` and `0 1px 0 #262626` below.
fn slider_track(canvas: &Canvas, t: vg::Rect, s: f32) {
    let hl = gfx::hairline(s);
    canvas.draw_rect(vg::Rect::from_xywh(t.left, t.bottom, t.width(), hl), &gfx::fill(Rgba::hex(0x262626)));
    canvas.draw_rect(t, &gfx::fill(Rgba::hex(0x050505)));
    canvas.draw_rect(vg::Rect::from_xywh(t.left, t.top, t.width(), hl), &gfx::fill(Rgba::black(1.0)));
}

/// A thumb: square, `#4A4A4A` border, `#5A5A5A → #2E2E2E`, a `#A0A0A0`
/// inner top line (8 × 12 on the pan, 12 × 8 on the fader).
fn slider_thumb(canvas: &Canvas, th: vg::Rect, s: f32, lift: f32) {
    let hl = gfx::hairline(s);
    canvas.draw_rect(th, &gfx::vgrad(th, Rgba::hex(0x5A5A5A).lighten(lift), Rgba::hex(0x2E2E2E).lighten(lift)));
    gfx::ring(canvas, &gfx::rrect(th, 0.0), hl, &gfx::fill(Rgba::hex(0x4A4A4A)));
    canvas.draw_rect(vg::Rect::from_xywh(th.left + hl, th.top + hl, th.width() - 2.0 * hl, hl), &gfx::fill(Rgba::hex(0xA0A0A0)));
}

/// The vertical volume meter-fader (polish 2 §3, column B): a 4 px well
/// from y 8, `row height − 32` long, the post-fader meter filling from the
/// bottom on the fader's dB scale (cool to hot, hot at 0 dB, never past the
/// thumb), and the 12 × 8 thumb at `(dB + 48) / 54` of the length. The view
/// is column B from the row's top to the dB value, so the thumb can be
/// grabbed at both ends.
pub struct VolumeFader {
    s: Signals,
    lane: usize,
    /// A drag: (press y, the value then).
    drag: Option<(f32, f32)>,
    hover: bool,
    wheel: crate::input::WheelSteps,
}

/// The "lane" of the Main track's fader (Batch 4a §7).
pub const MAIN: usize = usize::MAX;

impl VolumeFader {
    pub const HELP: &'static str = "Volume: drag to change · double-click the value to type it in dB";
    pub const CLIP_HELP: &'static str = "Clip: lights white when the track goes over 0 dBFS and stays lit · click to reset";
    pub const MAIN_CLIP_HELP: &'static str = "Clip: lights white when the output goes over 0 dBFS and stays lit · click to reset (with PEAK)";
    pub const MAIN_HELP: &'static str = "Main: the gain after the whole mix, −inf … +6 dB (drag or wheel); it works on locked tracks too";
    /// dB per wheel notch.
    const WHEEL_DB: f32 = 1.0;

    pub fn new(cx: &mut Context, s: Signals, lane: usize) -> Handle<'_, Self> {
        let shown = Cell::new((u16::MAX, 0u16));
        // `CHIPDAW_FROZEN_METERS=1` (dev): the meters don't follow the level
        // (the performance attribution in DESIGNPASS.md §6).
        let frozen = crate::dev::var("CHIPDAW_FROZEN_METERS").is_some_and(|v| v == "1");
        Self { s, lane, drag: None, hover: false, wheel: Default::default() }
            .build(cx, |_| {})
            .class("fader")
            .width(Pixels(COL_B_W))
            // A memo: only a height change re-applies it (a scroll or zoom updates
            // the arrangement every frame; re-applying would relayout and redraw
            // eight faders as eight separate damage rects).
            .height(Memo::new(move |_| s.arrangement.with(|a| row::db_value_y(a.lanes.get(lane).map_or(crate::state::ROW_H_MIN, |l| l.height)))).map(|h| Pixels(*h)))
            .bind(s.master_db, move |mut h| {
                if lane == MAIN {
                    h.needs_redraw();
                }
            })
            .bind(s.master_peak, move |mut h| {
                if lane == MAIN {
                    h.needs_redraw();
                }
            })
            .bind(s.meters, move |mut h| {
                let v = Self::state(&s, lane);
                if !frozen && shown.get() != v {
                    shown.set(v);
                    h.needs_redraw();
                }
            })
    }

    /// Whether the clip mark is lit (G8): the track's held peak, or Main's
    /// PEAK, over 0 dBFS.
    pub(crate) fn clipped(s: &Signals, lane: usize) -> bool {
        if lane == MAIN {
            return s.master_peak.get_untracked() > 1.0;
        }
        s.arrangement.with_untracked(|a| a.lanes.get(lane).map(|l| l.id)).is_some_and(|id| s.meters.with_untracked(|m| m.clipped(id)))
    }

    /// (fill px ×4, thumb y ×4 with the clip mark in the top bit): what the
    /// view shows.
    pub(crate) fn state(s: &Signals, lane: usize) -> (u16, u16) {
        let (f, t) = Self::state_parts(s, lane);
        (f, t | if Self::clipped(s, lane) { 0x8000 } else { 0 })
    }

    fn state_parts(s: &Signals, lane: usize) -> (u16, u16) {
        if lane == MAIN {
            let db = s.master_db.get_untracked();
            let len = row::fader_len(crate::state::ROW_H_MIN);
            let fill = crate::state::meter_fill(s.meters.with_untracked(|m| m.main), db, len);
            return ((fill * 4.0).round() as u16, (crate::state::fader_y(db, len) * 4.0).round() as u16);
        }
        let (db, h) = s.arrangement.with_untracked(|a| a.lanes.get(lane).map_or((0.0, crate::state::ROW_H_MIN), |l| (l.vol_db, l.height)));
        let len = row::fader_len(h);
        let muted = s.arrangement.with_untracked(|a| a.lanes.get(lane).is_some_and(|l| s.mute.with_untracked(|m| m.contains(l.id))));
        let level = if muted { 0.0 } else { s.meters.with_untracked(|m| m.level_of(lane)) };
        let fill = crate::state::meter_fill(level, db, len);
        ((fill * 4.0).round() as u16, (crate::state::fader_y(db, len) * 4.0).round() as u16)
    }

    fn db(&self) -> f32 {
        if self.lane == MAIN {
            return self.s.master_db.get_untracked();
        }
        self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).map_or(0.0, |l| l.vol_db))
    }

    /// The value change event: a track's volume, or the Main gain.
    fn set_event(&self, db: f32) -> AppEvent {
        if self.lane == MAIN {
            AppEvent::SetMasterDb(db)
        } else {
            AppEvent::SetLaneVolume(self.lane, db)
        }
    }

    fn len(&self) -> f32 {
        self.s.arrangement.with_untracked(|a| row::fader_len(a.lanes.get(self.lane).map_or(crate::state::ROW_H_MIN, |l| l.height)))
    }

    /// The press point sets the value (the slider gesture the build had).
    fn set_from_y(&self, cx: &mut EventContext) {
        let y = local_mouse(cx).1 - row::FADER_TOP;
        let pos = 1.0 - y / self.len();
        cx.emit(self.set_event(crate::state::pos_to_db(pos)));
    }
}

impl View for VolumeFader {
    fn element(&self) -> Option<&'static str> {
        Some("fader")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let lock = self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).and_then(|l| l.lock));
        event.map(|we, meta| match we {
            // Locked (§5.3): pinned at 0 dB, disabled; the help says why.
            WindowEvent::MouseEnter if lock.is_some() => {
                cx.emit(AppEvent::HoverHelp(crate::tracks::locked_help(lock.unwrap_or(0))));
            }
            // The clip mark (G8): a press on it resets the held peak (locked
            // tracks too: it is a display, not a control).
            WindowEvent::MouseDown(MouseButton::Left) if local_mouse(cx).1 < CLIP_HIT_H => {
                cx.emit(if self.lane == MAIN { AppEvent::ResetPeak } else { AppEvent::ResetTrackPeak(self.lane) });
                meta.consume();
            }
            WindowEvent::MouseMove(..) if self.drag.is_none() && local_mouse(cx).1 < CLIP_HIT_H => {
                cx.emit(AppEvent::HoverHelp(if self.lane == MAIN { Self::MAIN_CLIP_HELP } else { Self::CLIP_HELP }.into()));
            }
            WindowEvent::MouseMove(..) if self.drag.is_none() && self.hover => {
                let help = match lock {
                    Some(c) => crate::tracks::locked_help(c),
                    None if self.lane == MAIN => Self::MAIN_HELP.into(),
                    None => Self::HELP.into(),
                };
                cx.emit(AppEvent::HoverHelp(help));
            }
            WindowEvent::MouseDown(_) | WindowEvent::MouseScroll(..) | WindowEvent::MouseDoubleClick(_) if lock.is_some() => meta.consume(),
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.drag = Some((local_mouse(cx).1, self.db()));
                cx.emit(AppEvent::GestureBegin);
                cx.capture();
                self.set_from_y(cx);
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) if self.drag.is_some() => self.set_from_y(cx),
            WindowEvent::MouseUp(MouseButton::Left) if self.drag.is_some() => {
                self.drag = None;
                cx.release();
                cx.emit(AppEvent::GestureEnd);
                meta.consume();
            }
            WindowEvent::MouseScroll(_, y) if !cx.modifiers().alt() => {
                let steps = self.wheel.steps(*y);
                if steps != 0 {
                    let db = self.db();
                    let base = if db == f32::NEG_INFINITY { crate::state::VOL_DB_FLOOR } else { db };
                    cx.emit(self.set_event((base + steps as f32 * Self::WHEEL_DB).min(crate::state::VOL_DB_MAX)));
                    cx.emit(AppEvent::Commit);
                }
                meta.consume();
            }
            WindowEvent::MouseEnter => {
                self.hover = true;
                cx.emit(AppEvent::HoverHelp(if self.lane == MAIN { Self::MAIN_HELP } else { Self::HELP }.into()));
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
        let _perf = crate::perf::scope(crate::perf::Span::Rows);
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        let (fill4, thumb4) = Self::state_parts(&self.s, self.lane);
        let clipped = Self::clipped(&self.s, self.lane);
        let len = self.len();
        let left = (r.left + (COL_B_W - row::FADER_WELL_W) * 0.5 * s).round();
        let top = (r.top + row::FADER_TOP * s).round();
        let well = vg::Rect::from_xywh(left, top, (row::FADER_WELL_W * s).round(), (len * s).round());
        slider_track(canvas, well, s);
        let fill = fill4 as f32 / 4.0;
        if fill > 0.0 {
            // The pan grey (G8), from the bottom.
            let fh = (fill * s).round();
            canvas.draw_rect(vg::Rect::from_xywh(well.left, well.bottom - fh, well.width(), fh), &gfx::fill(METER_FILL));
        }
        // The clip mark (G8): above the well, white and glowing when lit.
        let mark = vg::Rect::from_xywh(well.left, (r.top + CLIP_Y * s).round(), well.width(), (CLIP_H * s).round());
        if clipped {
            canvas.draw_rect(mark, &glow_paint(CLIP_ON.with_alpha(0.6), CLIP_GLOW_R * s));
            canvas.draw_rect(mark, &gfx::fill(CLIP_ON));
        } else {
            canvas.draw_rect(mark, &gfx::fill(CLIP_OFF));
        }
        let cy = top + thumb4 as f32 / 4.0 * s;
        let th = vg::Rect::from_xywh(
            (left + (row::FADER_WELL_W - row::VTHUMB_W) * 0.5 * s).round(),
            (cy - row::VTHUMB_H * 0.5 * s).round(),
            row::VTHUMB_W * s,
            row::VTHUMB_H * s,
        );
        let lift = if self.hover || self.drag.is_some() { 0.08 } else { 0.0 };
        if self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).is_some_and(|l| l.locked())) {
            // Locked (§5.3): the thumb at 0 dB, disabled.
            let pad = (2.0 * s).ceil();
            let size = ((th.width() + 2.0 * pad).ceil() as i32 + 1, (th.height() + 2.0 * pad).ceil() as i32 + 1);
            let (w, h) = (th.width(), th.height());
            draw_disabled(canvas, DisabledPart::Thumb, s, size, (th.left - pad, th.top - pad), |c| {
                slider_thumb(c, vg::Rect::from_xywh(pad, pad, w, h), s, 0.0);
            });
        } else {
            slider_thumb(canvas, th, s, lift);
        }
    }
}

/// The pan slider (column A, polish 2 §3): a 56 × 4 well with a centre
/// tick, a fill from the centre and an 8 × 12 thumb. Editable and saved;
/// heard once stereo mixing arrives (Batch 4). The row draws it; this view
/// takes the gestures (press and drag, the wheel) and is the well plus half
/// a thumb on each side.
pub struct PanSlider {
    s: Signals,
    lane: usize,
    drag: bool,
    wheel: crate::input::WheelSteps,
}

impl PanSlider {
    pub const HELP: &'static str =
        "Pan: drag to change · double-click the value to type it (C, 25L, 30R) · it applies once stereo mixing arrives";
    pub const PAD: f32 = 4.0;

    pub fn new(cx: &mut Context, s: Signals, lane: usize) -> Handle<'_, Self> {
        Self { s, lane, drag: false, wheel: Default::default() }
            .build(cx, |_| {})
            .class("pan")
            .width(Pixels(COL_A_W + 2.0 * Self::PAD))
            .height(Pixels(row::HTHUMB_H))
    }

    fn set_from_x(&self, cx: &mut EventContext) {
        let x = local_mouse(cx).0 - Self::PAD - COL_A_W * 0.5;
        let v = (x / (COL_A_W * 0.5) * crate::state::PAN_MAX as f32).round();
        cx.emit(AppEvent::SetLanePan(self.lane, v.clamp(-50.0, 50.0) as i8));
    }
}

impl View for PanSlider {
    fn element(&self) -> Option<&'static str> {
        Some("pan")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let lock = self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).and_then(|l| l.lock));
        event.map(|we, meta| match we {
            WindowEvent::MouseEnter if lock.is_some() => {
                cx.emit(AppEvent::HoverHelp(format!("Pan: centred while locked to the {} (the console's output is mono)", crate::tracks::channel_name(lock.unwrap_or(0)))));
            }
            WindowEvent::MouseDown(_) | WindowEvent::MouseScroll(..) if lock.is_some() => meta.consume(),
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.drag = true;
                cx.emit(AppEvent::GestureBegin);
                cx.capture();
                self.set_from_x(cx);
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) if self.drag => self.set_from_x(cx),
            WindowEvent::MouseUp(MouseButton::Left) if self.drag => {
                self.drag = false;
                cx.release();
                cx.emit(AppEvent::GestureEnd);
                meta.consume();
            }
            WindowEvent::MouseScroll(_, y) if !cx.modifiers().alt() => {
                let steps = self.wheel.steps(*y);
                if steps != 0 {
                    let pan = self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).map_or(0, |l| l.pan)) as i32;
                    cx.emit(AppEvent::SetLanePan(self.lane, (pan + steps).clamp(-50, 50) as i8));
                    cx.emit(AppEvent::Commit);
                }
                meta.consume();
            }
            WindowEvent::MouseEnter => cx.emit(AppEvent::HoverHelp(Self::HELP.into())),
            _ => {}
        });
    }

    fn draw(&self, _cx: &mut DrawContext, _canvas: &Canvas) {
        // Drawn by the row (one view repaints, not two).
    }
}

/// The pan slider with its well's top-left at (x, y) (device px).
fn draw_pan(canvas: &Canvas, x: f32, y: f32, pan: i8, s: f32) {
    let hl = gfx::hairline(s);
    let well = vg::Rect::from_xywh(x, y, (COL_A_W * s).round(), (row::PAN_WELL_H * s).round());
    slider_track(canvas, well, s);
    let half = COL_A_W * 0.5;
    let cx = x + (half * s).round();
    // The centre tick: 1 × 8 at 27.5 px (the mockup's half pixel, snapped).
    canvas.draw_rect(vg::Rect::from_xywh(cx, (y - 2.0 * s).round(), hl, 8.0 * s), &gfx::fill(Rgba::hex(0x4A4A4A)));
    let off = (pan as f32 / crate::state::PAN_MAX as f32 * half).round();
    if off != 0.0 {
        let (a, b) = if off < 0.0 { (cx + off * s, cx) } else { (cx, cx + off * s) };
        canvas.draw_rect(vg::Rect::from_ltrb(a.round(), well.top, b.round(), well.bottom), &gfx::fill(PAN_FILL));
    }
    let th = vg::Rect::from_xywh((cx + (off - row::HTHUMB_W * 0.5) * s).round(), (y + (row::PAN_WELL_H - row::HTHUMB_H) * 0.5 * s).round(), row::HTHUMB_W * s, row::HTHUMB_H * s);
    slider_thumb(canvas, th, s, 0.0);
}
