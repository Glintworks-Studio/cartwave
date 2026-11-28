//! The in-instrument preset picker (PRESETS_PROMPT.md §6; VISION.md "The
//! preset picker inside the instrument"): the device's preset field opens a
//! dropdown with **the same folders as the browser**, plus **Show in
//! Browser** and **Save Preset As…**. On a locked track it lists the
//! authentic presets for that channel first; a studio-only preset asks
//! before it unlocks the track (`preset_ui`).
//!
//! One custom-drawn list under a backdrop that closes it (a click outside,
//! Esc). A click on a preset loads it on the selected track at once.

use std::collections::HashSet;

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::bank::{self, Folder, Item};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

const W: f32 = 320.0;
const MAX_H: f32 = 432.0;
/// The picker's size (logical px): the list scrolls inside it.
pub const WIDTH: f32 = W;
pub const HEIGHT: f32 = MAX_H;
const ROW_H: f32 = 24.0;
const PAD: f32 = 4.0;
const INDENT: f32 = 16.0;
const WHEEL_PX: f32 = 48.0;
/// The two command rows at the bottom.
const CMDS: [&str; 2] = ["Show in Browser", "Save Preset As…"];

#[derive(Clone, Debug, PartialEq)]
enum Row {
    Folder { key: String, name: String, depth: usize, open: bool },
    Item { item: Item, depth: usize },
}

/// Catches clicks outside the picker and closes it.
pub struct PickerBackdrop;

impl View for PickerBackdrop {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| {
            if let WindowEvent::MouseDown(_) = we {
                cx.emit(AppEvent::ClosePicker);
                meta.consume();
            }
        });
    }
}

pub struct PickerView {
    s: Signals,
    roots: Vec<(String, Folder)>,
    open: HashSet<String>,
    rows: Vec<Row>,
    scroll: f32,
    hover: Option<usize>,
    hover_cmd: Option<usize>,
    current: Option<usize>,
}

impl PickerView {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        let lane = s.lane_sel.get_untracked();
        let lib = bank::library();
        let (voice, lock) = s.arrangement.with_untracked(|a| a.lanes.get(lane).map_or((chip::voice::VoiceKind::Pulse, None), |l| (l.voice, l.lock)));
        let mut roots: Vec<(String, Folder)> = Vec::new();
        // A locked track: its channel's authentic presets first (§6.3).
        if let Some(c) = lock {
            let mut f = Folder { name: format!("For this channel ({})", crate::tracks::channel_name(c)), ..Default::default() };
            f.items = bank::all().iter().enumerate().filter(|(_, p)| p.authentic() && p.voice == voice).map(|(i, _)| Item::Preset(i)).collect();
            roots.push(("Channel".into(), f));
        }
        let user = Folder { name: "User Library".into(), items: (0..s.user_presets.with_untracked(|u| u.len())).map(Item::User).collect(), ..Default::default() };
        for (k, f) in [("Sounds", &lib.sounds), ("Drums", &lib.drums), ("Instruments", &lib.instruments), ("Hardware", &lib.hardware)] {
            roots.push((k.into(), f.clone()));
        }
        if !user.items.is_empty() {
            roots.push(("User Library".into(), user));
        }
        let lane_name = s.arrangement.with_untracked(|a| a.instruments.lane(lane).map(|i| i.name.clone())).unwrap_or_default();
        let current = bank::index_of(&lane_name);
        let mut open = HashSet::new();
        if lock.is_some() {
            open.insert("Channel".to_string());
        }
        // The current preset's folder, open.
        if let Some((path, _)) = current.and_then(bank::home_of) {
            let mut key = String::new();
            for (k, part) in path.iter().enumerate() {
                key = if k == 0 { part.clone() } else { format!("{key}/{part}") };
                open.insert(key.clone());
            }
        }
        let mut v = Self { s, roots, open, rows: Vec::new(), scroll: 0.0, hover: None, hover_cmd: None, current };
        v.refresh();
        if let Some(i) = v.current.and_then(|c| v.rows.iter().position(|r| matches!(r, Row::Item { item: Item::Preset(p), .. } if *p == c))) {
            v.scroll = (i as f32 * ROW_H - 4.0 * ROW_H).max(0.0);
        }
        let h = v.height();
        if crate::dev::enabled() {
            eprintln!("dev: picker open: roots {} current {:?}", v.roots.iter().map(|r| r.1.name.clone()).collect::<Vec<_>>().join("|"), v.current);
        }
        let (x, y, _) = s.picker.get_untracked().unwrap_or((0.0, 0.0, W));
        v.build(cx, |_| {})
            .position_type(PositionType::Absolute)
            .left(Pixels(x))
            .top(Pixels(y))
            .width(Pixels(W))
            .height(Pixels(h))
            .on_build(|cx| cx.focus())
    }

    fn height(&self) -> f32 {
        MAX_H
    }

    fn list_h(&self) -> f32 {
        self.height() - 2.0 * PAD - 8.0 - CMDS.len() as f32 * ROW_H
    }

    fn refresh(&mut self) {
        let mut rows = Vec::new();
        for (key, f) in &self.roots {
            let o = self.open.contains(key);
            rows.push(Row::Folder { key: key.clone(), name: f.name.clone(), depth: 0, open: o });
            if o {
                push(&mut rows, f, key, 1, &self.open);
            }
        }
        self.rows = rows;
    }

    fn local(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s)
    }

    fn row_at(&self, p: (f32, f32)) -> Option<usize> {
        if p.1 < PAD || p.1 >= PAD + self.list_h() {
            return None;
        }
        let i = ((p.1 - PAD + self.scroll) / ROW_H).floor() as usize;
        (i < self.rows.len()).then_some(i)
    }

    fn cmd_at(&self, p: (f32, f32)) -> Option<usize> {
        let top = PAD + self.list_h() + 8.0;
        (p.1 >= top).then(|| ((p.1 - top) / ROW_H) as usize).filter(|i| *i < CMDS.len())
    }
}

fn push(rows: &mut Vec<Row>, f: &Folder, key: &str, depth: usize, open: &HashSet<String>) {
    for sub in &f.folders {
        let k = format!("{key}/{}", sub.name);
        let o = open.contains(&k);
        rows.push(Row::Folder { key: k.clone(), name: sub.name.clone(), depth, open: o });
        if o {
            push(rows, sub, &k, depth + 1, open);
        }
    }
    for it in &f.items {
        rows.push(Row::Item { item: *it, depth });
    }
}

fn name_of(s: &Signals, it: Item) -> String {
    match it {
        Item::Preset(i) => bank::get(i).map_or(String::new(), |p| p.name.clone()),
        Item::User(u) => s.user_presets.with_untracked(|v| v.get(u).map_or(String::new(), |p| p.name.clone())),
        Item::Band(i) => chip::presets::LAYOUTS.get(i).map_or(String::new(), |l| l.name.to_string()),
        _ => String::new(),
    }
}

impl View for PickerView {
    fn element(&self) -> Option<&'static str> {
        Some("preset-picker")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::KeyDown(Code::Escape, _) => {
                cx.emit(AppEvent::ClosePicker);
                meta.consume();
            }
            WindowEvent::MouseScroll(_, sy) => {
                let max = (self.rows.len() as f32 * ROW_H - self.list_h()).max(0.0);
                self.scroll = (self.scroll - sy * WHEEL_PX).clamp(0.0, max);
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseMove(..) => {
                let p = Self::local(cx);
                let (h, hc) = (self.row_at(p), self.cmd_at(p));
                if (h, hc) != (self.hover, self.hover_cmd) {
                    self.hover = h;
                    self.hover_cmd = hc;
                    let help = match h.map(|i| self.rows[i].clone()) {
                        Some(Row::Item { item: Item::Preset(i), .. }) => bank::get(i).map_or(String::new(), |p| format!("{}: {} · click to load it on this track", p.name, p.blurb)),
                        Some(Row::Item { item, .. }) => format!("{}: click to load it", name_of(&self.s, item)),
                        Some(Row::Folder { .. }) => "Click to open or close the folder".into(),
                        None => match hc {
                            Some(0) => "Show this preset in the browser".into(),
                            Some(_) => "Save this instrument as a preset in your User Library".into(),
                            None => String::new(),
                        },
                    };
                    cx.emit(AppEvent::HoverHelp(help));
                    cx.needs_redraw();
                }
                meta.consume();
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                let p = Self::local(cx);
                if let Some(i) = self.row_at(p) {
                    match self.rows[i].clone() {
                        Row::Folder { key, .. } => {
                            if !self.open.remove(&key) {
                                self.open.insert(key);
                            }
                            self.refresh();
                            cx.needs_redraw();
                        }
                        Row::Item { item, .. } => {
                            cx.emit(AppEvent::ClosePicker);
                            match item {
                                Item::Preset(i) => cx.emit(AppEvent::AssignPreset { lane: None, preset: i }),
                                Item::User(u) => cx.emit(AppEvent::AssignPreset { lane: None, preset: bank::len() + u }),
                                Item::Band(i) => cx.emit(AppEvent::ApplyLayout(i)),
                                _ => {}
                            }
                        }
                    }
                } else if let Some(c) = self.cmd_at(p) {
                    if c == 0 {
                        cx.emit(AppEvent::ShowInBrowser(self.current));
                    } else {
                        cx.emit(AppEvent::SavePresetAs);
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
        let rr = gfx::rrect(r, 6.0 * s);
        gfx::drop_shadow(canvas, &rr, 0.0, 6.0 * s, 16.0 * s, 0.0, Rgba::black(0.6));
        canvas.draw_rrect(rr, &gfx::fill(Rgba::hex(0x141414)));
        gfx::ring(canvas, &rr, gfx::hairline(s), &gfx::fill(STROKE));
        let fi = gfx::font(Face::Medium, 12.0 * s);
        let ff = gfx::font(Face::SemiBold, 12.0 * s);
        canvas.save();
        let list = vg::Rect::from_xywh(r.left, r.top + PAD * s, r.width(), self.list_h() * s);
        canvas.clip_rect(list, None, None);
        for (i, row) in self.rows.iter().enumerate() {
            let top = list.top + (i as f32 * ROW_H - self.scroll) * s;
            if top + ROW_H * s < list.top || top > list.bottom {
                continue;
            }
            let rw = vg::Rect::from_xywh(r.left + PAD * s, top.round(), r.width() - 2.0 * PAD * s, (ROW_H * s).round());
            match row {
                Row::Folder { name, depth, open, .. } => {
                    if self.hover == Some(i) {
                        canvas.draw_rrect(gfx::rrect(rw, 4.0 * s), &gfx::fill(Rgba::white(0.04)));
                    }
                    let x = rw.left + (8.0 + *depth as f32 * INDENT) * s;
                    let (d, ink) = if *open { ("M1 2L4 6L7 2", Rgba::hex(0x9A9A9A)) } else { ("M2 1L6 4L2 7", Rgba::hex(0x808080)) };
                    let mut st = gfx::stroke(ink, 1.4 * s);
                    st.set_stroke_cap(vg::PaintCap::Butt);
                    canvas.draw_path(&gfx::svg_path(d, (x.round(), (rw.center_y() - 4.0 * s).round()), s), &st);
                    gfx::draw_text_clipped(canvas, name, (x + 16.0 * s).round(), gfx::baseline_for_center(&ff, rw.center_y()).round(), rw.right - x - 24.0 * s, &ff, Rgba::hex(0xD0D0D0));
                }
                Row::Item { item, depth } => {
                    let cur = matches!(item, Item::Preset(p) if Some(*p) == self.current);
                    if cur {
                        canvas.draw_rrect(gfx::rrect(rw, 4.0 * s), &gfx::fill(Rgba::hex(0xBDBDBD)));
                    } else if self.hover == Some(i) {
                        canvas.draw_rrect(gfx::rrect(rw, 4.0 * s), &gfx::fill(Rgba::white(0.06)));
                    }
                    let ink = if cur { Rgba::hex(0x111111) } else { Rgba::hex(0xB0B0B0) };
                    let x = rw.left + (8.0 + *depth as f32 * INDENT) * s;
                    canvas.draw_circle((x + 4.0 * s, rw.center_y()), 2.0 * s, &gfx::fill(Rgba::hex(0x5A5A5A)));
                    let mut tx = x + 16.0 * s;
                    let base = gfx::baseline_for_center(&fi, rw.center_y());
                    if matches!(item, Item::Preset(p) if bank::get(*p).is_some_and(|p| p.authentic())) {
                        super::device::lock_glyph(canvas, (tx.round(), (base - 8.0 * s).round()), s, if cur { Rgba::hex(0x333333) } else { Rgba::hex(0x9A9A9A) });
                        tx += 12.0 * s;
                    }
                    gfx::draw_text_clipped(canvas, &name_of(&self.s, *item), tx.round(), base.round(), rw.right - tx - 8.0 * s, &fi, ink);
                }
            }
        }
        canvas.restore();
        // The commands, under a hairline.
        let top = r.top + (PAD + self.list_h() + 4.0) * s;
        canvas.draw_rect(vg::Rect::from_xywh(r.left + 8.0 * s, top.round(), r.width() - 16.0 * s, gfx::hairline(s)), &gfx::fill(LINE));
        for (k, label) in CMDS.iter().enumerate() {
            let rw = vg::Rect::from_xywh(r.left + PAD * s, top + 4.0 * s + k as f32 * ROW_H * s, r.width() - 2.0 * PAD * s, ROW_H * s);
            if self.hover_cmd == Some(k) {
                canvas.draw_rrect(gfx::rrect(rw, 4.0 * s), &gfx::fill(Rgba::white(0.06)));
            }
            gfx::draw_text(canvas, label, (rw.left + 8.0 * s).round(), gfx::baseline_for_center(&fi, rw.center_y()).round(), &fi, TEXT_HI);
        }
    }
}

/// The picker while it's open (built at the shell's top, over everything).
pub fn build(cx: &mut Context, s: Signals) {
    Binding::new(cx, s.picker, move |cx| {
        if s.picker.get().is_none() {
            return;
        }
        PickerBackdrop.build(cx, |_| {}).position_type(PositionType::Absolute).width(Stretch(1.0)).height(Stretch(1.0));
        PickerView::new(cx, s);
    });
}
