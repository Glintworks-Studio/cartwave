//! Scrubbable numeric fields (BPM, position). The inset look, focus glow and
//! transitions come from the stylesheet (`.field`); this view only paints the
//! text and handles click-drag scrubbing.

use vizia::prelude::*;

use super::gfx::{self, Face};
use crate::state::{Transport, PPQ};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Bpm,
    Position,
}

/// BPM change per logical pixel of vertical drag (Shift = 10x finer).
const BPM_PER_PX: f32 = 0.25;
/// Pixels of drag per sixteenth note when scrubbing the position.
const PX_PER_SIXTEENTH: f32 = 4.0;

pub struct ScrubField {
    s: Signals,
    kind: FieldKind,
    dragging: bool,
    press: (f32, f32),
    start_value: f64,
    wheel: crate::input::WheelSteps,
}

impl ScrubField {
    pub fn new(cx: &mut Context, s: Signals, kind: FieldKind) -> Handle<'_, Self> {
        Self {
            s,
            kind,
            dragging: false,
            press: (0.0, 0.0),
            start_value: 0.0,
            wheel: Default::default(),
        }
        .build(cx, |_| {})
        .class("field")
        .cursor(CursorIcon::NsResize)
        // Redraw only when the displayed text changes (POS changes ~4x per
        // beat, BPM rarely), not on every playhead step.
        .bind(s.transport, {
            let shown = std::cell::RefCell::new(String::new());
            move |mut h| {
                let text = Self::format(s, kind);
                if *shown.borrow() != text {
                    *shown.borrow_mut() = text;
                    h.needs_redraw();
                }
            }
        })
    }

    fn format(s: Signals, kind: FieldKind) -> String {
        let t = s.transport.get_untracked();
        match kind {
            FieldKind::Bpm => format!("{:.1}", t.bpm),
            FieldKind::Position => Transport::position_string(t.playhead_ticks),
        }
    }

    fn text(&self) -> String {
        Self::format(self.s, self.kind)
    }
}

impl View for ScrubField {
    fn element(&self) -> Option<&'static str> {
        Some("scrub-field")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.dragging = true;
                self.press = (cx.mouse().cursor_x, cx.mouse().cursor_y);
                let t = self.s.transport.get_untracked();
                self.start_value = match self.kind {
                    FieldKind::Bpm => t.bpm as f64,
                    FieldKind::Position => t.playhead_ticks,
                };
                cx.capture();
                cx.set_active(true);
                meta.consume();
            }
            WindowEvent::MouseMove(x, y) if self.dragging => {
                let s = cx.scale_factor();
                // Up and right both increase.
                let d = ((self.press.1 - *y) + (*x - self.press.0)) / s;
                match self.kind {
                    FieldKind::Bpm => {
                        let k = if cx.modifiers().shift() {
                            BPM_PER_PX * 0.1
                        } else {
                            BPM_PER_PX
                        };
                        let v = (self.start_value as f32 + d * k * 10.0).round() / 10.0;
                        cx.emit(AppEvent::SetBpm(v));
                    }
                    FieldKind::Position => {
                        let six = (PPQ / 4) as f64;
                        let steps = (d / PX_PER_SIXTEENTH).round() as f64;
                        let base = (self.start_value / six).round() * six;
                        cx.emit(AppEvent::SetPlayhead((base + steps * six).max(0.0)));
                    }
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.dragging => {
                self.dragging = false;
                cx.release();
                cx.set_active(false);
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                match self.kind {
                    FieldKind::Bpm => cx.emit(AppEvent::SetBpm(150.0)),
                    FieldKind::Position => cx.emit(AppEvent::SetPlayhead(0.0)),
                }
                meta.consume();
            }
            WindowEvent::MouseScroll(_, sy) => {
                let steps = self.wheel.steps(*sy);
                let t = self.s.transport.get_untracked();
                match self.kind {
                    _ if steps == 0 => {}
                    FieldKind::Bpm => cx.emit(AppEvent::SetBpm(t.bpm + steps as f32)),
                    FieldKind::Position => cx.emit(AppEvent::SetPlayhead(
                        t.playhead_ticks + steps as f64 * PPQ as f64,
                    )),
                }
                meta.consume();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Fields);
        // Stylesheet-driven background + inset/outer shadows.
        cx.draw_background(canvas);
        cx.draw_shadows(canvas);

        let b = cx.bounds();
        let s = cx.scale_factor();
        let f = gfx::font(Face::MonoMedium, 13.0 * s);
        let txt = self.text();
        let w = gfx::text_width(&f, &txt);
        let base = gfx::baseline_for_center(&f, b.y + b.h * 0.5);
        let x = b.x + b.w - w - 10.0 * s;
        let col = if self.dragging {
            Rgba::hex(0xD9D0FF)
        } else {
            TEXT_HI
        };
        gfx::draw_text(canvas, &txt, x, base, &f, col);
    }
}
