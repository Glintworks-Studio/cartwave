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
    fn max_pane_h(cx: &EventContext) -> f32 {
        let s = cx.scale_factor();
        let win_h = cx.cache.get_bounds(Entity::root()).h / s;
        let room = win_h - TOPBAR_HEIGHT - RULER_HEIGHT - SPLIT_HANDLE_H - PIANO_MIN_H;
        let room = (room / SP_1).floor() * SP_1;
        room.clamp(crate::state::PIANO_PANE_MIN, 8.0 * crate::state::LANE_H_NORMAL)
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
// VISUAL-SPEC.md §2.4. Positions are logical px from the row's top-left
// (the row is 72 px plus its 1 px bottom hairline); they match the mockup's
// layout boxes (docs/design/mockup-boxes.jsonl), snapped to whole px.

/// The channel number: Inter 800 italic 40, centred in 28 px after the strip
/// and a 12 px gap, on a 40 px line box from y 16; tracking −.02em.
const NUMBER_PX: f32 = 40.0;
const NUMBER_X: f32 = 28.0;
const NUMBER_W: f32 = 28.0;
const NUMBER_TOP: f32 = 16.0;
const NUMBER_TRACKING: f32 = -0.02;
/// Name: Inter 600 14 from x 68, its line box (normal line height) from y 14.5.
const NAME_X: f32 = 68.0;
const NAME_PX: f32 = 14.0;
const NAME_TOP: f32 = 14.5;
/// The name column ends where the control block's 12 px gap starts.
const NAME_RIGHT: f32 = 236.0;
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
/// The control block: 120 px wide, 12 px from the row's right edge, its
/// 40 px (keys, 12 px gap, sliders) centred in the row.
pub const CTRL_W: f32 = 120.0;
pub const CTRL_RIGHT: f32 = 12.0;
pub const CTRL_TOP: f32 = 16.0;
/// Volume meter-fader and pan: track widths, track height, the gap between.
pub const FADER_W: f32 = 68.0;
pub const PAN_W: f32 = 40.0;
pub const SLIDER_H: f32 = 4.0;
pub const SLIDER_GAP: f32 = 12.0;
/// The thumb: 8 × 12, centred on the track; the slider views are thumb-tall.
pub const THUMB_W: f32 = 8.0;
pub const THUMB_H: f32 = 12.0;
/// Where the slider views sit (their top) below the keys.
pub const SLIDER_VIEW_TOP: f32 = CTRL_TOP + KEY_SIZE + 8.0;
/// The strip's lit look (`inset 0 0 10px rgba(255,255,255,.55)`, and a
/// `0 0 10px` spill in the track colour at 35%).
const STRIP_GLOW_A: f32 = 0.55;
const STRIP_GLOW_R: f32 = 10.0;
const STRIP_SPILL_A: f32 = 0.35;

/// The strip's colours: the palette recipe (theme::palette::strip).
fn strip_colours(color: Rgba, light: f32) -> ([Rgba; 3], Rgba, Rgba) {
    crate::theme::palette::strip(color, light)
}

/// One track row in the header column: colour strip (lights on each
/// note-on), channel number, name, profile badge and preset, and the
/// control block (M/S keys, the volume meter-fader, the pan slider: child
/// views).
pub struct TrackRow {
    s: Signals,
    lane: usize,
    hover: bool,
    /// The lit strip, pre-rendered: (colour, height, scale) and the image.
    lit: LitCache,
    /// The row's text and badge, pre-rendered.
    text: RefCell<Option<(RowTextKey, vg::Image)>>,
}

/// The lit strip: (colour, height, scale) and its image.
type LitCache = RefCell<Option<((u32, i32, u32), vg::Image)>>;

/// What a row's text layer shows.
#[derive(Clone, Debug, PartialEq)]
struct RowTextKey {
    selected: bool,
    name: String,
    inst: Option<String>,
    w: i32,
    h: i32,
    s: u32,
}

/// A row's number, name, profile badge and preset on a transparent layer
/// (grayscale-antialiased text composites exactly as if drawn in place).
fn render_row_text(lane: usize, k: &RowTextKey, s: f32) -> Option<vg::Image> {
    let mut surface = vg::surfaces::raster_n32_premul((k.w.max(1), k.h.max(1)))?;
    let canvas = surface.canvas();
    canvas.clear(vg::Color::TRANSPARENT);
    let r = vg::Rect::from_xywh(0.0, 0.0, k.w as f32, k.h as f32);
    let hl = gfx::hairline(s);
    let at = |x: f32, y: f32| (r.left + x * s, r.top + y * s);
    let (selected, name) = (k.selected, &k.name);
        // The channel number.
        let nf = gfx::font(Face::HeavyItalic, NUMBER_PX * s);
        let num = format!("{}", lane + 1);
        let ntr = NUMBER_TRACKING * NUMBER_PX * s;
        let nw = gfx::tracked_width(&num, &nf, ntr);
        let (nx, ny) = at(NUMBER_X + NUMBER_W * 0.5, NUMBER_TOP);
        let nb = gfx::css_baseline(&nf, ny, NUMBER_PX * s).round();
        gfx::draw_text_tracked(canvas, &num, (nx - nw * 0.5).round(), nb, &nf, ntr, NUMBER_INK.lighten(if selected { 0.04 } else { 0.0 }));

        // Name.
        let f = gfx::font(Face::SemiBold, NAME_PX * s);
        let (x, y) = at(NAME_X, NAME_TOP);
        let (_, m) = f.metrics();
        // Skia's hinting puts the ink a pixel higher than the browser's.
        let base = (y - m.ascent).round() + s.round();
        let max_w = (NAME_RIGHT - NAME_X) * s;
        gfx::draw_text_clipped(canvas, name, x.round(), base, max_w, &f, TEXT_HI);

        // The profile badge and the preset.
        let chip = if lane >= chip::driver::LANE_V1 { "VRC6" } else { "2A03" };
        let bf = gfx::font(Face::SemiBold, BADGE_PX * s);
        let bw = gfx::text_width(&bf, chip);
        let (bx, by) = at(NAME_X, BADGE_Y);
        let badge = vg::Rect::from_xywh(bx.round(), by.round(), (bw + (2.0 + 3.0 * BADGE_PAD + LOCK_W) * s).round(), BADGE_H * s);
        gfx::ring(canvas, &gfx::rrect(badge, RADIUS_KEY * s), hl, &gfx::fill(STROKE));
        let lock_o = (badge.left + hl + BADGE_PAD * s, (badge.top + (BADGE_H - LOCK_H) * 0.5 * s).round());
        draw_lock(canvas, lock_o, s, BADGE_INK);
        let bb = (r.top + BADGE_BASELINE * s).round();
        gfx::draw_text(canvas, chip, (lock_o.0 + (LOCK_W + BADGE_PAD) * s).round(), bb, &bf, BADGE_INK);
        if let Some(text) = &k.inst {
            let fi = gfx::font(Face::Medium, PRESET_PX * s);
            let ix = badge.right + PRESET_GAP * s;
            gfx::draw_text_clipped(canvas, text, ix.round(), bb, r.left + NAME_RIGHT * s - ix, &fi, if selected { TEXT_MID } else { TEXT_LO });
        }

    Some(surface.image_snapshot())
}

impl TrackRow {
    pub fn new(cx: &mut Context, s: Signals, lane: usize) -> Handle<'_, Self> {
        let seen = RefCell::new((false, String::new()));
        // Redraw on this lane's light only (PERF.md: only what changed).
        let light = Cell::new(0u8);
        Self { s, lane, hover: false, lit: RefCell::new(None), text: RefCell::new(None) }
            .build(cx, move |cx| {
                let ctrl = if crate::shortcuts::MACOS { "Cmd" } else { "Ctrl" };
                let left = TRACK_HEADER_WIDTH - CTRL_RIGHT - CTRL_W;
                Key::new(cx, s, "M", KeyAction::Mute(lane))
                    .position_type(PositionType::Absolute)
                    .left(Pixels(left + CTRL_W - 2.0 * KEY_SIZE - 8.0))
                    .top(Pixels(CTRL_TOP))
                    .tooltip(|cx| crate::tip(cx, "Mute this track."));
                Key::new(cx, s, "S", KeyAction::Solo(lane))
                    .position_type(PositionType::Absolute)
                    .left(Pixels(left + CTRL_W - KEY_SIZE))
                    .top(Pixels(CTRL_TOP))
                    .tooltip(move |cx| crate::tip(cx, &format!("Solo this track only. {ctrl}-click to solo several.")));
                VolumeFader::new(cx, s, lane)
                    .position_type(PositionType::Absolute)
                    .left(Pixels(left - THUMB_W * 0.5))
                    .top(Pixels(SLIDER_VIEW_TOP))
                    .tooltip(move |cx| crate::tip(cx, &VolumeFader::tip(s, lane)));
                PanSlider::new(cx)
                    .position_type(PositionType::Absolute)
                    .left(Pixels(left + FADER_W + SLIDER_GAP - THUMB_W * 0.5))
                    .top(Pixels(SLIDER_VIEW_TOP))
                    .tooltip(|cx| crate::tip(cx, PanSlider::WHY));
            })
            .class("track-row")
            .bind(s.meters, move |mut h| {
                let l = s.meters.with_untracked(|m| (m.light[lane] * 64.0).round() as u8);
                if light.get() != l {
                    light.set(l);
                    h.needs_redraw();
                }
            })
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
            .tooltip(move |cx| {
                // The full name, profile and preset (both texts truncate in the row).
                let text = s.arrangement.map(move |a| {
                    let inst = a.instruments.lane(lane).map(|i| i.label()).unwrap_or_default();
                    format!("{} · {} · {inst}", a.lanes[lane].name, if lane >= chip::driver::LANE_V1 { "VRC6" } else { "2A03" })
                });
                Tooltip::new(cx, move |cx| {
                    Label::new(cx, text).class("tip-text").class("long");
                })
                .class("tip")
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
        let selected = self
            .s
            .arrangement
            .with_untracked(|a| a.first_selected().map(|c| c.lane) == Some(self.lane) || a.track_sel.contains(&self.lane));
        let (color, name) = self.s.arrangement.with_untracked(|a| (a.lanes[self.lane].color, a.lanes[self.lane].name.clone()));
        let light = self.s.meters.with_untracked(|m| m.light[self.lane]);

        // Row surface (selection lifts it), and the 1 px bottom hairline.
        let body = vg::Rect::from_ltrb(r.left, r.top, r.right, r.bottom - hl);
        canvas.draw_rect(body, &gfx::fill(if selected { BG_0.lighten(0.03) } else { BG_0 }));
        canvas.draw_rect(vg::Rect::from_ltrb(r.left, r.bottom - hl, r.right, r.bottom), &gfx::fill(LINE));

        // The colour strip: full height, square, light left edge → dark
        // right edge; lit by note-ons (inner glow and a soft spill).
        let strip = vg::Rect::from_ltrb(r.left, body.top, (r.left + LANE_STRIP_W * s).round(), body.bottom);
        // The unlit strip, then the lit look over it at `light`: the lit
        // fill, edges, inner glow and spill are rendered once per colour,
        // height and scale, and blending them at `light` is exactly
        // mix(unlit, lit, light). No blur runs per frame (PERF.md).
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
                canvas.save();
                canvas.clip_rect(body, None, None);
                canvas.draw_image(img, (strip.left, strip.top), Some(&p));
                canvas.restore();
            }
        }

        // The number, name, badge and preset: one image per content and
        // scale (a row redraws every frame while its strip or meter moves).
        let inst = self.s.arrangement.with_untracked(|a| a.instruments.lane(self.lane).map(|i| if i.is_kit() { format!("{}  (kit)", i.label()) } else { i.label() }));
        let key = RowTextKey { selected, name: name.clone(), inst: inst.clone(), w: r.width().round() as i32, h: r.height().round() as i32, s: (s * 100.0).round() as u32 };
        {
            let mut text = self.text.borrow_mut();
            if text.as_ref().is_none_or(|(k, _)| *k != key) {
                *text = render_row_text(self.lane, &key, s).map(|img| (key, img));
            }
            if let Some((_, img)) = text.as_ref() {
                canvas.draw_image(img, (r.left, r.top), None);
            }
        }
        let bb = (r.top + BADGE_BASELINE * s).round();

        // F-04: a VRC6 lane under the 2A03 profile: dimmed, and it says why.
        if crate::state::lane_off(self.s.chip.get_untracked(), self.lane) {
            canvas.draw_rect(body, &gfx::fill(Rgba::black(super::arrangement::OFF_DIM)));
            let fh = gfx::font(Face::Medium, PRESET_PX * s);
            let text = if self.hover { "Off: turn on VRC6 in File > Project Settings…" } else { "Off in the 2A03 profile" };
            let hx = (r.left + NAME_X * s).round();
            let max_w = r.right - CTRL_RIGHT * s - hx;
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
    }
}

/// The fully lit strip (fill, edges, inner glow) plus its spill to the
/// right, as an image whose top-left is the strip's.
pub fn render_lit_strip(color: Rgba, w: f32, h: f32, s: f32) -> Option<vg::Image> {
    let spill_pad = (2.0 * STRIP_GLOW_R * s).ceil();
    let mut surface = vg::surfaces::raster_n32_premul(((w + spill_pad).ceil() as i32, h.ceil().max(1.0) as i32))?;
    let c = surface.canvas();
    c.clear(vg::Color::TRANSPARENT);
    let strip = vg::Rect::from_xywh(0.0, 0.0, w, h);
    let hl = gfx::hairline(s);
    let mut spill = gfx::fill(color.with_alpha(STRIP_SPILL_A));
    spill.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, STRIP_GLOW_R * 0.5 * s, false));
    c.draw_rect(strip, &spill);
    let (fill, edge_l, edge_r) = strip_colours(color, 1.0);
    c.draw_rect(strip, &gfx::shaded(gfx::linear((0.0, 0.0), (w, 0.0), &fill, Some(&[0.0, 0.55, 1.0]))));
    c.draw_rect(vg::Rect::from_xywh(0.0, 0.0, hl, h), &gfx::fill(edge_l));
    c.draw_rect(vg::Rect::from_xywh(w - hl, 0.0, hl, h), &gfx::fill(edge_r));
    let pad = gfx::rrect(strip.with_inset((hl, 0.0)), 0.0);
    gfx::css_inset_shadow(c, &pad, 0.0, 0.0, STRIP_GLOW_R * s, 0.0, Rgba::white(STRIP_GLOW_A));
    Some(surface.image_snapshot())
}

/// The badge's lock (the mockup's 9 × 11 icon).
fn draw_lock(canvas: &Canvas, o: (f32, f32), s: f32, ink: Rgba) {
    let body = vg::Rect::from_xywh(o.0 + 0.5 * s, o.1 + 4.5 * s, 8.0 * s, 6.0 * s);
    canvas.draw_rrect(gfx::rrect(body, 1.2 * s), &gfx::fill(ink));
    let shackle = gfx::svg_path("M2.3 4.5V3a2.2 2.2 0 0 1 4.4 0v1.5", o, s);
    canvas.draw_path(&shackle, &gfx::stroke(ink, 1.2 * s));
}

/// The slider track: `#050505` with `inset 0 1px 0 #000` and `0 1px 0 #262626` below.
fn slider_track(canvas: &Canvas, t: vg::Rect, s: f32) {
    let hl = gfx::hairline(s);
    canvas.draw_rect(vg::Rect::from_xywh(t.left, t.bottom, t.width(), hl), &gfx::fill(Rgba::hex(0x262626)));
    canvas.draw_rect(t, &gfx::fill(Rgba::hex(0x050505)));
    canvas.draw_rect(vg::Rect::from_xywh(t.left, t.top, t.width(), hl), &gfx::fill(Rgba::black(1.0)));
}

/// The 8 × 12 thumb: square, `#4A4A4A` border, `#5A5A5A → #2E2E2E`, a
/// `#A0A0A0` inner top line.
fn slider_thumb(canvas: &Canvas, th: vg::Rect, s: f32, lift: f32) {
    let hl = gfx::hairline(s);
    canvas.draw_rect(th, &gfx::vgrad(th, Rgba::hex(0x5A5A5A).lighten(lift), Rgba::hex(0x2E2E2E).lighten(lift)));
    gfx::ring(canvas, &gfx::rrect(th, 0.0), hl, &gfx::fill(Rgba::hex(0x4A4A4A)));
    canvas.draw_rect(vg::Rect::from_xywh(th.left + hl, th.top + hl, th.width() - 2.0 * hl, hl), &gfx::fill(Rgba::hex(0xA0A0A0)));
}

/// The track's volume meter-fader: a 68 × 4 track whose fill is a
/// post-fader level meter (cool → hot over the full 68 px, never past the
/// thumb), and the 8 × 12 thumb that sets the channel volume (the
/// inspector's Volume dial: undoable, saved). The view is the thumb's
/// height and the track plus half a thumb on each side, so the thumb can
/// be grabbed at both ends.
pub struct VolumeFader {
    s: Signals,
    lane: usize,
    drag: bool,
    hover: bool,
    wheel: crate::input::WheelSteps,
}

impl VolumeFader {
    pub fn new(cx: &mut Context, s: Signals, lane: usize) -> Handle<'_, Self> {
        let shown = Cell::new((u16::MAX, 0u16, false));
        // `CHIPDAW_FROZEN_METERS=1` (dev): the meters don't follow the level
        // (the performance attribution in DESIGNPASS.md §6).
        let frozen = crate::dev::var("CHIPDAW_FROZEN_METERS").is_some_and(|v| v == "1");
        Self { s, lane, drag: false, hover: false, wheel: Default::default() }
            .build(cx, |_| {})
            .class("fader")
            .width(Pixels(FADER_W + THUMB_W))
            .height(Pixels(THUMB_H))
            .bind(s.meters, move |mut h| {
                let v = Self::state(&s, lane);
                if !frozen && shown.get() != v {
                    shown.set(v);
                    h.needs_redraw();
                }
            })
            .bind(s.faders, |mut h| h.needs_redraw())
    }

    /// (fill px ×4, thumb px ×4, disabled): what the view shows.
    fn state(s: &Signals, lane: usize) -> (u16, u16, bool) {
        let f = s.faders.with_untracked(|f| f[lane]);
        let level = if s.mute.get_untracked() & (1 << lane) != 0 { 0.0 } else { s.meters.with_untracked(|m| m.level[lane]) };
        let fill = crate::state::meter_fill(level, f.pos, FADER_W);
        ((fill * 4.0).round() as u16, (f.pos * FADER_W * 4.0).round() as u16, f.reason.is_some())
    }

    pub fn tip(s: Signals, lane: usize) -> String {
        match s.faders.with_untracked(|f| f[lane].reason) {
            Some(why) => why.into(),
            None => "Volume: drag the thumb or use the wheel. The bar shows the track's level after the fader.".into(),
        }
    }

    fn set_from_x(&self, cx: &mut EventContext) {
        let x = local_mouse(cx).0 - THUMB_W * 0.5;
        cx.emit(AppEvent::SetLaneVolume(self.lane, x / FADER_W));
    }
}

impl View for VolumeFader {
    fn element(&self) -> Option<&'static str> {
        Some("fader")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let off = self.s.faders.with_untracked(|f| f[self.lane].reason.is_some());
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) if !off => {
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
            WindowEvent::MouseScroll(_, y) if !off => {
                let steps = self.wheel.steps(*y);
                if steps != 0 {
                    let pos = self.s.faders.with_untracked(|f| f[self.lane].pos);
                    let (lo, hi, _) = crate::state::fader_param(self.lane).map_or((0, 15, 0), |p| p.range(self.lane));
                    cx.emit(AppEvent::SetLaneVolume(self.lane, pos + steps as f32 / (hi - lo) as f32));
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
            // Keep a press on the fader from selecting the track.
            WindowEvent::MouseDown(MouseButton::Left) => meta.consume(),
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Rows);
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        let (fill4, thumb4, off) = Self::state(&self.s, self.lane);
        let left = (r.left + THUMB_W * 0.5 * s).round();
        let track = vg::Rect::from_xywh(left, (r.center_y() - SLIDER_H * 0.5 * s).round(), (FADER_W * s).round(), (SLIDER_H * s).round());
        slider_track(canvas, track, s);
        let fill = fill4 as f32 / 4.0;
        if fill > 0.0 {
            // The gradient spans the full track; the fill reveals its left part.
            let stops: Vec<Rgba> = FADER_STOPS.iter().map(|(_, c)| *c).collect();
            let pos: Vec<f32> = FADER_STOPS.iter().map(|(x, _)| x / FADER_W).collect();
            let paint = gfx::shaded(gfx::linear((track.left, 0.0), (track.right, 0.0), &stops, Some(&pos)));
            canvas.draw_rect(vg::Rect::from_xywh(track.left, track.top, (fill * s).round(), track.height()), &paint);
        }
        let tx = (left + thumb4 as f32 / 4.0 * s - THUMB_W * 0.5 * s).round();
        let th = vg::Rect::from_xywh(tx, r.top, THUMB_W * s, THUMB_H * s);
        let lift = if (self.hover || self.drag) && !off { 0.08 } else { 0.0 };
        slider_thumb(canvas, th, s, lift);
    }
}

/// The pan slider: drawn (40 × 4, a centre detent) but disabled until
/// stereo mixing arrives.
pub struct PanSlider;

impl PanSlider {
    pub const WHY: &'static str = "Pan arrives with stereo mixing (Batch 4).";

    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self.build(cx, |_| {}).class("pan").width(Pixels(PAN_W + THUMB_W)).height(Pixels(THUMB_H))
    }
}

impl View for PanSlider {
    fn element(&self) -> Option<&'static str> {
        Some("pan")
    }

    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        // Disabled: a press does nothing (and doesn't select the track).
        event.map(|we, meta| {
            if let WindowEvent::MouseDown(_) = we {
                meta.consume();
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        let left = (r.left + THUMB_W * 0.5 * s).round();
        let track = vg::Rect::from_xywh(left, (r.center_y() - SLIDER_H * 0.5 * s).round(), (PAN_W * s).round(), (SLIDER_H * s).round());
        slider_track(canvas, track, s);
        // The centre detent: 1 × 8 at 19.5 px (the mockup's half-pixel, snapped).
        let dx = (track.left + 19.5 * s).round();
        canvas.draw_rect(vg::Rect::from_xywh(dx, (r.center_y() - 4.0 * s).round(), gfx::hairline(s), 8.0 * s), &gfx::fill(Rgba::hex(0x4A4A4A)));
        let th = vg::Rect::from_xywh((track.left + (PAN_W * 0.5 - THUMB_W * 0.5) * s).round(), r.top, THUMB_W * s, THUMB_H * s);
        slider_thumb(canvas, th, s, 0.0);
    }
}
