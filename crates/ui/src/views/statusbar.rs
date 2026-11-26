//! The status bar (polish 2 §6; board `E1-header`): 24 px along the window's
//! bottom, `#0F0F0F` with a `#1E1E1E` top hairline, Inter 500 11.
//!
//! - Left (`role=status`): the latest non-urgent message with a small icon,
//!   for 8 s, then the idle text (the project's name and saved state).
//!   Persistent problems (no audio device, dropouts) put a warning icon in
//!   front that stays until they're resolved; a click on it lists them.
//! - Right: one line of hover help for the control under the mouse.
//!
//! Dialogs stay for decisions (save prompts, the MIDI-drop conflict, anything
//! destructive, the recovery offer).

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

const PAD_X: f32 = 16.0;
const TEXT_PX: f32 = 11.0;
const ICON_GAP: f32 = 8.0;
/// The message ink (`#9A9A9A`, 6.4:1 on the bar) and the help ink
/// (`#808080`, 4.9:1).
const MSG_INK: Rgba = Rgba::hex(0x9A9A9A);
const HELP_INK: Rgba = Rgba::hex(0x808080);
/// The warning icon: amber, the meter's hot stop (not lime: not an "on" state).
const WARN_INK: Rgba = Rgba::hex(0xF0A23A);
/// The warning icon's box (logical px) and the gap before the message.
const WARN_W: f32 = 12.0;

pub struct StatusBar {
    s: Signals,
}

/// What the left side shows now: (icon, text).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Check,
    Dot,
}

impl StatusBar {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s }
            .build(cx, |_| {})
            .height(Pixels(STATUS_BAR_HEIGHT))
            .width(Stretch(1.0))
            .role(Role::Status)
            .bind(s.status, |mut h| h.needs_redraw())
            .bind(s.hover_help, |mut h| h.needs_redraw())
            .bind(s.problems, |mut h| h.needs_redraw())
            .bind(s.dirty, |mut h| h.needs_redraw())
            .bind(s.file_path, |mut h| h.needs_redraw())
    }

    /// The idle text: "Saved · song.chipdaw" or "Unsaved changes · …".
    pub fn idle_text(s: &Signals) -> String {
        let path = s.file_path.get_untracked();
        let name = std::path::Path::new(&path).file_name().and_then(|n| n.to_str()).map(str::to_string).unwrap_or_else(|| "Untitled".into());
        if s.dirty.get_untracked() {
            format!("Unsaved changes · {name}")
        } else {
            format!("Saved · {name}")
        }
    }

    fn warn_rect(r: vg::Rect, s: f32) -> vg::Rect {
        vg::Rect::from_xywh(r.left + PAD_X * s, r.top, WARN_W * s, r.height())
    }
}

impl View for StatusBar {
    fn element(&self) -> Option<&'static str> {
        Some("status-bar")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let s = cx.scale_factor();
                let b = cx.bounds();
                let x = (cx.mouse().cursor_x - b.x) / s;
                let problems = self.s.problems.get_untracked();
                if !problems.is_empty() && x < PAD_X + WARN_W + ICON_GAP {
                    // The details of the persistent problems.
                    cx.emit(AppEvent::ShowProblems(cx.mouse().cursor_x / s, b.y / s));
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_, _) => {
                let s = cx.scale_factor();
                let x = (cx.mouse().cursor_x - cx.bounds().x) / s;
                let on_warn = !self.s.problems.with_untracked(|p| p.is_empty()) && x < PAD_X + WARN_W + ICON_GAP;
                cx.emit(WindowEvent::SetCursor(if on_warn { CursorIcon::Hand } else { CursorIcon::Default }));
            }
            WindowEvent::MouseLeave => cx.emit(WindowEvent::SetCursor(CursorIcon::Default)),
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        let hl = gfx::hairline(s);
        canvas.draw_rect(r, &gfx::fill(RULER_BG));
        canvas.draw_rect(vg::Rect::from_xywh(r.left, r.top, r.width(), hl), &gfx::fill(LINE_SOFT));
        let f = gfx::font(Face::Medium, TEXT_PX * s);
        let base = gfx::baseline_for_center(&f, r.center_y() + 0.5 * hl).round();
        let mut x = r.left + PAD_X * s;

        // Persistent problems: an icon that stays until they're resolved.
        if !self.s.problems.with_untracked(|p| p.is_empty()) {
            let wr = Self::warn_rect(r, s);
            draw_warning(canvas, wr.left, r.center_y(), s);
            x = wr.right + ICON_GAP * s;
        }

        // The latest message, else the idle text.
        let msg = self.s.status.get_untracked();
        let (icon, text) = if msg.is_empty() { (Icon::Check, StatusBar::idle_text(&self.s)) } else { (Icon::Dot, msg.lines().next().unwrap_or_default().to_string()) };
        let help = self.s.hover_help.get_untracked();
        let help_w = if help.is_empty() { 0.0 } else { gfx::text_width(&f, &help) };
        let room = (r.right - PAD_X * s - help_w - 24.0 * s - x).max(0.0);
        let iw = draw_icon(canvas, icon, x, r.center_y(), s);
        x += iw + ICON_GAP * s;
        gfx::draw_text_clipped(canvas, &text, x.round(), base, room - iw - ICON_GAP * s, &f, MSG_INK);

        // Hover help, right-aligned.
        if !help.is_empty() {
            gfx::draw_text(canvas, &help, (r.right - PAD_X * s - help_w).round(), base, &f, HELP_INK);
        }
    }
}

/// The 10 × 8 check (the board's) or a 4 px dot for a message; returns its width.
fn draw_icon(canvas: &Canvas, icon: Icon, x: f32, cy: f32, s: f32) -> f32 {
    match icon {
        Icon::Check => {
            let o = (x, (cy - 4.0 * s).round());
            let p = gfx::svg_path("M1 4L4 7L9 1", o, s);
            let mut st = gfx::stroke(MSG_INK, 1.5 * s);
            st.set_stroke_cap(vg::PaintCap::Round);
            st.set_stroke_join(vg::PaintJoin::Round);
            canvas.draw_path(&p, &st);
            10.0 * s
        }
        Icon::Dot => {
            canvas.draw_circle((x + 5.0 * s, cy), 2.0 * s, &gfx::fill(MSG_INK));
            10.0 * s
        }
    }
}

/// A 12 × 11 warning triangle with its mark.
fn draw_warning(canvas: &Canvas, x: f32, cy: f32, s: f32) {
    let o = (x, (cy - 5.5 * s).round());
    let tri = gfx::svg_path("M6 0.5L11.5 10.5H0.5Z", o, s);
    canvas.draw_path(&tri, &gfx::fill(WARN_INK));
    canvas.draw_rect(vg::Rect::from_xywh(o.0 + 5.25 * s, o.1 + 3.5 * s, 1.5 * s, 4.0 * s), &gfx::fill(RULER_BG));
    canvas.draw_rect(vg::Rect::from_xywh(o.0 + 5.25 * s, o.1 + 8.25 * s, 1.5 * s, 1.25 * s), &gfx::fill(RULER_BG));
}
