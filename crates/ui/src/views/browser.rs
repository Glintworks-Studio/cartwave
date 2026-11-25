//! The browser sidebar (left, Ctrl+Alt+B): channel layouts and the preset
//! library. Click a preset to hear it, double-click to load it onto the
//! selected channel, or drag it onto a track header or lane. While dragging,
//! channels the preset can't play on are dimmed.
//!
//! The list is one custom-drawn view (no child per row): it scrolls, hovers
//! and hit-tests itself, so 60+ rows cost one draw call.

use std::cell::RefCell;

use chip::presets::{lane_kind, Category, K_DMC, K_NOISE, K_PULSE, K_SAW, K_TRI, K_VPULSE, LAYOUTS, PRESETS};
use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

/// Sidebar width (logical px); a multiple of 4.
pub const BROWSER_W: f32 = 240.0;
const ROW_H: f32 = 28.0;
const HEADER_H: f32 = 32.0;
const PAD_X: f32 = 12.0;
const DOT: f32 = 6.0;
const DRAG_START_PX: f32 = 4.0;
const WHEEL_PX: f32 = 56.0;

/// What is being dragged out of the browser, and where (physical window px).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresetDrag {
    pub preset: usize,
    pub x: f32,
    pub y: f32,
    /// The lane under the pointer, if any (whether or not the preset fits).
    pub target: Option<usize>,
}

thread_local! {
    /// Each track row's bounds as last drawn (physical px): drop targets.
    static LANE_ROWS: RefCell<[Option<BoundingBox>; 8]> = const { RefCell::new([None; 8]) };
}

/// Called by the track rows when they draw.
pub fn register_lane_row(lane: usize, b: BoundingBox) {
    LANE_ROWS.with(|r| {
        if let Some(slot) = r.borrow_mut().get_mut(lane) {
            *slot = Some(b);
        }
    });
}

/// The lane at a window position: over its track header or anywhere to the
/// right of it (the arrangement lane).
pub fn lane_at(x: f32, y: f32) -> Option<usize> {
    LANE_ROWS.with(|r| {
        r.borrow()
            .iter()
            .position(|b| b.is_some_and(|b| x >= b.x && y >= b.y && y < b.y + b.h))
    })
}

/// The vertical span of a lane's row (physical px), for the arrangement's dimming.
pub fn lane_span(lane: usize) -> Option<(f32, f32)> {
    LANE_ROWS.with(|r| r.borrow().get(lane).copied().flatten().map(|b| (b.y, b.y + b.h)))
}

/// Kind bits in display order, with their lane colour.
const KINDS: [(u8, Rgba); 6] = [
    (K_PULSE, LANE_PULSE),
    (K_TRI, LANE_TRI),
    (K_NOISE, LANE_NOISE),
    (K_DMC, LANE_DPCM),
    (K_VPULSE, LANE_VRC6_PULSE),
    (K_SAW, LANE_VRC6_SAW),
];

/// The colour a preset is shown in: its first channel kind's.
pub fn preset_color(kinds: u8) -> Rgba {
    KINDS.iter().find(|(k, _)| kinds & k != 0).map_or(ACCENT, |(_, c)| *c)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
    Header(&'static str, usize),
    Layout(usize),
    Preset(usize),
    /// A filled user DPCM slot.
    Sample(usize),
    ImportAudio,
}

fn items(query: &str, samples: &[Option<String>]) -> Vec<Item> {
    let q = query.trim().to_lowercase();
    let hit = |name: &str, blurb: &str, cat: &str| {
        q.is_empty() || name.to_lowercase().contains(&q) || blurb.to_lowercase().contains(&q) || cat.to_lowercase().contains(&q)
    };
    let mut out = Vec::new();
    let layouts: Vec<usize> = (0..LAYOUTS.len())
        .filter(|&i| hit(LAYOUTS[i].name, LAYOUTS[i].blurb, "layout"))
        .collect();
    if !layouts.is_empty() {
        out.push(Item::Header("Channel Layouts", layouts.len()));
        out.extend(layouts.into_iter().map(Item::Layout));
    }
    let filled: Vec<usize> = (0..samples.len())
        .filter(|&i| samples[i].as_ref().is_some_and(|n| hit(n, "", "dpcm sample audio")))
        .collect();
    if q.is_empty() || !filled.is_empty() || "dpcm samples audio import wav".contains(&q) {
        out.push(Item::Header("DPCM Samples", filled.len()));
        out.extend(filled.into_iter().map(Item::Sample));
        out.push(Item::ImportAudio);
    }
    for cat in Category::ALL {
        let ps: Vec<usize> = (0..PRESETS.len())
            .filter(|&i| PRESETS[i].category == cat && hit(PRESETS[i].name, PRESETS[i].blurb, cat.label()))
            .collect();
        if !ps.is_empty() {
            out.push(Item::Header(cat.label(), ps.len()));
            out.extend(ps.into_iter().map(Item::Preset));
        }
    }
    out
}

fn item_h(it: &Item) -> f32 {
    match it {
        Item::Header(..) => HEADER_H,
        _ => ROW_H,
    }
}

pub struct PresetList {
    s: Signals,
    items: Vec<Item>,
    scroll: f32,
    hover: Option<usize>,
    /// Pressed item and the press position (physical).
    press: Option<(usize, f32, f32)>,
    dragging: bool,
}

impl PresetList {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s, items: Vec::new(), scroll: 0.0, hover: None, press: None, dragging: false }
            .build(cx, |_| {})
            .class("preset-list")
            .bind(s.browser_query, |h| {
                h.modify(|v: &mut PresetList| {
                    v.refresh();
                    v.scroll = 0.0;
                })
                .needs_redraw();
            })
            .bind(s.arrangement, |h| {
                h.modify(|v: &mut PresetList| v.refresh());
            })
            .bind(s.browser_sel, |mut h| h.needs_redraw())
            .bind(s.lane_sel, |mut h| h.needs_redraw())
            .bind(s.arrangement, |mut h| h.needs_redraw())
    }

    fn refresh(&mut self) {
        let names: Vec<Option<String>> = self
            .s
            .arrangement
            .with_untracked(|a| a.samples.iter().map(|s| s.as_ref().map(|s| s.name.clone())).collect());
        self.items = items(&self.s.browser_query.get_untracked(), &names);
    }

    fn content_h(&self) -> f32 {
        self.items.iter().map(item_h).sum()
    }

    /// Item index at a logical y inside the view.
    fn item_at(&self, y: f32) -> Option<usize> {
        let mut top = -self.scroll;
        for (i, it) in self.items.iter().enumerate() {
            let h = item_h(it);
            if y >= top && y < top + h {
                return Some(i);
            }
            top += h;
        }
        None
    }

    fn local_y(cx: &EventContext) -> f32 {
        (cx.mouse().cursor_y - cx.bounds().y) / cx.scale_factor()
    }

    fn inside(cx: &EventContext) -> bool {
        let b = cx.bounds();
        let (x, y) = (cx.mouse().cursor_x, cx.mouse().cursor_y);
        x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h
    }
}

impl View for PresetList {
    fn element(&self) -> Option<&'static str> {
        Some("preset-list")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseScroll(_, sy) => {
                let max = (self.content_h() - cx.bounds().h / cx.scale_factor()).max(0.0);
                self.scroll = (self.scroll - sy * WHEEL_PX).clamp(0.0, max);
                self.hover = self.item_at(Self::local_y(cx));
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                let Some(i) = self.item_at(Self::local_y(cx)) else { return };
                match self.items[i] {
                    Item::Preset(p) => {
                        self.press = Some((i, cx.mouse().cursor_x, cx.mouse().cursor_y));
                        self.dragging = false;
                        cx.capture();
                        cx.emit(AppEvent::BrowserSelect(Some(p)));
                        cx.emit(AppEvent::Audition(Some(p)));
                    }
                    Item::Layout(l) => cx.emit(AppEvent::ApplyLayout(l)),
                    Item::Sample(slot) => {
                        let x = (cx.mouse().cursor_x - cx.bounds().x) / cx.scale_factor();
                        if x > cx.bounds().w / cx.scale_factor() - REMOVE_ZONE {
                            cx.emit(AppEvent::RemoveSample(slot));
                        } else {
                            self.press = Some((SAMPLE_PRESS, 0.0, 0.0));
                            cx.capture();
                            cx.emit(AppEvent::PreviewSample(Some(slot)));
                        }
                    }
                    Item::ImportAudio => cx.emit(AppEvent::ImportAudio),
                    Item::Header(..) => {}
                }
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                if let Some(Item::Preset(p)) = self.item_at(Self::local_y(cx)).map(|i| self.items[i]) {
                    cx.emit(AppEvent::AssignPreset { lane: None, preset: p });
                }
                meta.consume();
            }
            WindowEvent::MouseMove(x, y) => {
                if let Some((i, px, py)) = self.press.filter(|p| p.0 != SAMPLE_PRESS) {
                    let s = cx.scale_factor();
                    if !self.dragging && ((x - px).abs() > DRAG_START_PX * s || (y - py).abs() > DRAG_START_PX * s) {
                        self.dragging = true;
                        cx.emit(AppEvent::Audition(None));
                    }
                    if self.dragging {
                        if let Item::Preset(p) = self.items[i] {
                            let target = lane_at(*x, *y).filter(|_| !Self::inside(cx));
                            self.s.drag_preset.set(Some(PresetDrag { preset: p, x: *x, y: *y, target }));
                        }
                    }
                } else {
                    let h = Self::inside(cx).then(|| self.item_at(Self::local_y(cx))).flatten();
                    if h != self.hover {
                        self.hover = h;
                        cx.needs_redraw();
                    }
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.press.is_some_and(|p| p.0 == SAMPLE_PRESS) => {
                cx.release();
                cx.emit(AppEvent::PreviewSample(None));
                self.press = None;
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.press.is_some() => {
                cx.release();
                if self.dragging {
                    if let Some(d) = self.s.drag_preset.get_untracked() {
                        if let Some(lane) = d.target {
                            cx.emit(AppEvent::AssignPreset { lane: Some(lane), preset: d.preset });
                        }
                    }
                    self.s.drag_preset.set(None);
                } else {
                    cx.emit(AppEvent::Audition(None));
                }
                self.press = None;
                self.dragging = false;
                meta.consume();
            }
            WindowEvent::MouseLeave if self.hover.is_some() => {
                self.hover = None;
                cx.needs_redraw();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let rect: vg::Rect = b.into();
        canvas.save();
        canvas.clip_rect(rect, None, None);
        let sel = self.s.browser_sel.get_untracked();
        let lane = self.s.lane_sel.get_untracked();
        let lane_names: Vec<String> = self
            .s
            .arrangement
            .with_untracked(|a| a.instruments.iter().map(|i| i.name.clone()).collect());
        let fh = gfx::font(Face::SemiBold, 10.0 * s);
        let fr = gfx::font(Face::Medium, 12.0 * s);
        let fs = gfx::font(Face::Regular, 10.0 * s);
        let mut top = rect.top - self.scroll * s;
        for (i, it) in self.items.iter().enumerate() {
            let h = item_h(it) * s;
            if top + h < rect.top || top > rect.bottom {
                top += h;
                continue;
            }
            let row = vg::Rect::from_xywh(rect.left, top.round(), rect.width(), h.round());
            let x = rect.left + PAD_X * s;
            match *it {
                Item::Header(label, n) => {
                    let base = row.bottom - 9.0 * s;
                    gfx::draw_text(canvas, &label.to_uppercase(), x.round(), base.round(), &fh, TEXT_LO);
                    let count = n.to_string();
                    let cw = gfx::text_width(&fh, &count);
                    gfx::draw_text(canvas, &count, (row.right - PAD_X * s - cw).round(), base.round(), &fh, TEXT_LO);
                    let hl = gfx::hairline(s);
                    canvas.draw_rect(
                        vg::Rect::from_xywh(x, row.bottom - hl, row.width() - 2.0 * PAD_X * s, hl),
                        &gfx::fill(LINE),
                    );
                }
                Item::Sample(slot) => {
                    let hovered = self.hover == Some(i);
                    let card = vg::Rect::from_ltrb(row.left + 4.0 * s, row.top + 2.0 * s, row.right - 4.0 * s, row.bottom - 2.0 * s);
                    if hovered {
                        canvas.draw_rrect(gfx::rrect(card, RADIUS_KEY * s), &gfx::vgrad(card, Rgba::white(0.06), Rgba::white(0.025)));
                    }
                    let info = self.s.arrangement.with_untracked(|a| {
                        a.samples.get(slot).and_then(|s| s.as_ref()).map(|s| (s.name.clone(), s.seconds(), s.hz()))
                    });
                    if let Some((name, secs, hz)) = info {
                        // A tiny waveform glyph in the DPCM lane colour.
                        let gy = row.center_y();
                        for (k, h) in [3.0, 8.0, 5.0, 2.0].iter().enumerate() {
                            let g = vg::Rect::from_xywh(x + k as f32 * 2.0 * s, gy - h * s * 0.5, s, h * s);
                            canvas.draw_rect(g, &gfx::fill(LANE_DPCM));
                        }
                        let base = gfx::baseline_for_center(&fr, row.center_y());
                        // The row is named after the sample now; say which slot.
                        let label = format!("{name}  ·  slot {}", slot + 1);
                        gfx::draw_text_clipped(canvas, &label, (x + 16.0 * s).round(), base.round(), row.width() - 120.0 * s, &fr, TEXT_MID);
                        let meta = if hovered { "remove ×".to_string() } else { format!("{secs:.2}s · {:.1}k", hz / 1000.0) };
                        let mw = gfx::text_width(&fs, &meta);
                        gfx::draw_text(canvas, &meta, (row.right - PAD_X * s - mw).round(), base.round(), &fs, if hovered { RECORD } else { TEXT_LO });
                    }
                }
                Item::ImportAudio => {
                    let hovered = self.hover == Some(i);
                    let card = vg::Rect::from_ltrb(row.left + 8.0 * s, row.top + 3.0 * s, row.right - 8.0 * s, row.bottom - 3.0 * s);
                    let rr = gfx::rrect(card, RADIUS_KEY * s);
                    let mut dash = gfx::stroke(if hovered { ACCENT } else { STROKE }, gfx::hairline(s));
                    if let Some(e) = vg::PathEffect::dash(&[4.0 * s, 3.0 * s], 0.0) {
                        dash.set_path_effect(e);
                    }
                    canvas.draw_rrect(rr, &dash);
                    let free = self.s.arrangement.with_untracked(|a| crate::dpcm::free_bytes(&a.samples));
                    let t = format!("+  Import audio as DPCM  ·  {:.1} KB free", free as f32 / 1024.0);
                    let tw = gfx::text_width(&fs, &t);
                    let base = gfx::baseline_for_center(&fs, row.center_y());
                    gfx::draw_text(canvas, &t, (card.center_x() - tw * 0.5).round(), base.round(), &fs, if hovered { ACCENT } else { TEXT_LO });
                }
                Item::Layout(l) | Item::Preset(l) => {
                    let is_preset = matches!(it, Item::Preset(_));
                    let selected = is_preset && sel == Some(l);
                    let hovered = self.hover == Some(i);
                    let card = vg::Rect::from_ltrb(row.left + 4.0 * s, row.top + 2.0 * s, row.right - 4.0 * s, row.bottom - 2.0 * s);
                    let rr = gfx::rrect(card, RADIUS_KEY * s);
                    if selected {
                        canvas.draw_rrect(rr, &gfx::vgrad(card, ACCENT.with_alpha(0.14), ACCENT.with_alpha(0.06)));
                        gfx::ring(canvas, &rr, gfx::hairline(s), &gfx::fill(ACCENT.with_alpha(0.35)));
                    } else if hovered {
                        canvas.draw_rrect(rr, &gfx::vgrad(card, Rgba::white(0.06), Rgba::white(0.025)));
                    }
                    let (name, kinds, current) = if is_preset {
                        let p = &PRESETS[l];
                        let fits_sel = lane < 8 && p.fits(lane);
                        (p.name, p.kinds, fits_sel && lane_names.get(lane).is_some_and(|n| n == p.name))
                    } else {
                        (LAYOUTS[l].name, 0, false)
                    };
                    // Leading glyph: kind colour square for presets, a stack for layouts.
                    let gx = x;
                    let gy = row.center_y();
                    if is_preset {
                        let c = preset_color(kinds);
                        let g = vg::Rect::from_xywh(gx, gy - 4.0 * s, 8.0 * s, 8.0 * s);
                        canvas.draw_rrect(gfx::rrect(g, 2.0 * s), &gfx::vgrad(g, c, c.darken(0.35)));
                    } else {
                        for k in 0..3 {
                            let g = vg::Rect::from_xywh(gx, gy - 5.0 * s + k as f32 * 4.0 * s, 8.0 * s, 2.0 * s);
                            canvas.draw_rect(g, &gfx::fill(ACCENT.with_alpha(0.9 - 0.25 * k as f32)));
                        }
                    }
                    let ink = if selected || current { TEXT_HI } else { TEXT_MID };
                    let base = gfx::baseline_for_center(&fr, row.center_y());
                    gfx::draw_text(canvas, name, (x + 16.0 * s).round(), base.round(), &fr, ink);
                    let mut rx = row.right - PAD_X * s;
                    if is_preset {
                        // Channel-kind dots, right-aligned; lit if the kind is supported.
                        for (k, c) in KINDS.iter().rev() {
                            rx -= DOT * s;
                            let on = kinds & k != 0;
                            let d = vg::Rect::from_xywh(rx, gy - DOT * 0.5 * s, DOT * s, DOT * s);
                            let paint = if on { gfx::fill(c.with_alpha(0.85)) } else { gfx::fill(Rgba::white(0.06)) };
                            canvas.draw_oval(d, &paint);
                            rx -= 2.0 * s;
                        }
                        if current {
                            let t = "ON";
                            let tw = gfx::text_width(&fs, t);
                            gfx::draw_text(canvas, t, (rx - 4.0 * s - tw).round(), base.round(), &fs, ACCENT);
                        }
                    } else {
                        let t = if LAYOUTS[l].vrc6 { "+VRC6" } else { "2A03" };
                        let tw = gfx::text_width(&fs, t);
                        gfx::draw_text(canvas, t, (rx - tw).round(), base.round(), &fs, TEXT_LO);
                    }
                }
            }
            top += h;
        }
        // Scroll fades at the edges.
        for (y0, y1, a, b2) in [
            (rect.top, rect.top + 12.0 * s, BG_0, BG_0.with_alpha(0.0)),
            (rect.bottom - 16.0 * s, rect.bottom, BG_0.with_alpha(0.0), BG_0),
        ] {
            if (y0 == rect.top && self.scroll > 0.0) || y0 != rect.top {
                let r = vg::Rect::from_ltrb(rect.left, y0, rect.right, y1);
                canvas.draw_rect(r, &gfx::vgrad(r, a, b2));
            }
        }
        canvas.restore();
    }
}

/// Width of the "remove" zone at the right of a sample row (logical px).
const REMOVE_ZONE: f32 = 64.0;
/// `press` marker for a held sample audition (not a preset drag).
const SAMPLE_PRESS: usize = usize::MAX;

/// Info about the selected preset, under the list.
fn info_text(sel: Option<usize>) -> String {
    match sel.and_then(|i| PRESETS.get(i)) {
        Some(p) => {
            let kinds: Vec<&str> = [
                (K_PULSE, "Pulse"),
                (K_TRI, "Triangle"),
                (K_NOISE, "Noise"),
                (K_DMC, "DPCM"),
                (K_VPULSE, "VRC6 Pulse"),
                (K_SAW, "VRC6 Saw"),
            ]
            .iter()
            .filter(|(k, _)| p.kinds & k != 0)
            .map(|(_, n)| *n)
            .collect();
            format!("{}\n{}\nPlays on: {}", p.name, p.blurb, kinds.join(", "))
        }
        None => "Click a preset to hear it. Double-click loads it on the selected channel; or drag it onto a track.".into(),
    }
}

/// The sidebar.
pub fn browser(cx: &mut Context, s: Signals) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Label::new(cx, "BROWSER").class("section-title");
            Element::new(cx).width(Stretch(1.0)).height(Pixels(1.0));
            Label::new(cx, "Ctrl+Alt+B").class("browser-hint");
        })
        .class("browser-head");
        HStack::new(cx, |cx| {
            Textbox::new(cx, s.browser_query)
                .placeholder("Search instruments, sounds…")
                .on_edit(move |_cx, t| s.browser_query.set(t))
                .width(Stretch(1.0));
        })
        .class("field")
        .class("browser-search");
        PresetList::new(cx, s).height(Stretch(1.0)).width(Stretch(1.0));
        Label::new(cx, s.browser_sel.map(|sel| info_text(*sel)))
            .class("browser-info")
            .text_wrap(true)
            // No room under the list while the piano roll takes the height.
            .display(s.piano_open.map(|o| if *o { Display::None } else { Display::Flex }));
    })
    .class("browser")
    .width(Pixels(BROWSER_W))
    .display(s.browser_open.map(|o| if *o { Display::Flex } else { Display::None }));
}

/// The preset following the pointer while dragging.
pub struct DragGhost {
    s: Signals,
}

impl DragGhost {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s }
            .build(cx, |_| {})
            .position_type(PositionType::Absolute)
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .hoverable(false)
            .bind(s.drag_preset, |mut h| h.needs_redraw())
    }
}

impl View for DragGhost {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let Some(d) = self.s.drag_preset.get_untracked() else { return };
        let s = cx.scale_factor();
        let p = &PRESETS[d.preset];
        let fits = d.target.map(|l| p.fits(l));
        let f = gfx::font(Face::SemiBold, 12.0 * s);
        let label = match (d.target, fits) {
            (Some(l), Some(true)) => format!("{}  →  {}", p.name, crate::state::default_lanes()[l].name),
            (Some(_), Some(false)) => format!("{}  ·  can't play here", p.name),
            _ => p.name.to_string(),
        };
        let w = gfx::text_width(&f, &label) + 32.0 * s;
        let r = vg::Rect::from_xywh(d.x + 12.0 * s, d.y + 8.0 * s, w, 28.0 * s);
        let rr = gfx::rrect(r, RADIUS_BOX * s);
        gfx::drop_shadow(canvas, &rr, 0.0, 6.0 * s, 16.0 * s, 0.0, Rgba::black(0.55));
        canvas.draw_rrect(rr, &gfx::vgrad(r, Rgba::hex(0x262626), Rgba::hex(0x161616)));
        let edge = match fits {
            Some(true) => ACCENT,
            Some(false) => RECORD,
            None => STROKE,
        };
        gfx::ring(canvas, &rr, gfx::hairline(s), &gfx::fill(edge.with_alpha(0.7)));
        gfx::top_sheen(canvas, &rr, gfx::hairline(s), KEY_STROKE, KEY_SHEEN);
        let c = preset_color(p.kinds);
        let g = vg::Rect::from_xywh(r.left + 12.0 * s, r.center_y() - 4.0 * s, 8.0 * s, 8.0 * s);
        canvas.draw_rrect(gfx::rrect(g, 2.0 * s), &gfx::fill(c));
        let base = gfx::baseline_for_center(&f, r.center_y());
        gfx::draw_text(canvas, &label, (r.left + 24.0 * s).round(), base.round(), &f, TEXT_HI);
    }
}

/// While a preset is dragged: how a lane looks. `Some(true)` = the target and
/// it fits, `Some(false)` = can't play the preset (dimmed), `None` = normal.
pub fn drag_state(s: &Signals, lane: usize) -> Option<(bool, bool)> {
    let d = s.drag_preset.get_untracked()?;
    let fits = PRESETS[d.preset].kinds & lane_kind(lane) != 0;
    Some((fits, d.target == Some(lane)))
}
