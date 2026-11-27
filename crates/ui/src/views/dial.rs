//! Procedural rotary dial: domed brushed body, glowing value arc, pointer.

use std::cell::Cell;
use std::rc::Rc;

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::state::DialSpec;
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

/// Sweep of the dial in degrees (gap at the bottom).
const SWEEP_DEG: f32 = 270.0;
/// Skia angle (degrees, clockwise from +x) of the minimum position.
const START_DEG: f32 = 135.0;
/// Vertical drag distance for full range, logical px (Shift = fine).
const DRAG_RANGE_PX: f32 = 220.0;
const FINE_FACTOR: f32 = 8.0;
/// Fraction of range per wheel notch for continuous dials.
const WHEEL_STEP: f32 = 0.02;

const BODY_R: f32 = 19.0;
const ARC_R: f32 = 25.0;
const ARC_W: f32 = 3.5;
const DIAL_CY: f32 = 50.0;

/// A studio track's voice control on the inspector's instrument section
/// (Batch 4b §5): Unison, Detune or Drift of the selected track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniField {
    Voices,
    Detune,
    Drift,
}

impl UniField {
    pub fn get(self, u: crate::state::Unison) -> u8 {
        match self {
            UniField::Voices => u.voices,
            UniField::Detune => u.detune,
            UniField::Drift => u.drift,
        }
    }
}

pub struct Dial {
    s: Signals,
    index: usize,
    /// `Some`: a studio voice control (its value lives on the track), not a
    /// chip dial slot.
    uni: Option<UniField>,
    spec: DialSpec,
    dragging: bool,
    /// Unquantised normalised value while dragging (so stepped dials feel smooth).
    drag_norm: f32,
    last_y: f32,
    hovered: bool,
    wheel: crate::input::WheelSteps,
    /// Animated display state.
    shown_norm: Cell<f32>,
    active_t: Cell<f32>,
    last_draw: Cell<Option<Instant>>,
    /// Set by `draw` while the display is still easing.
    animating: Rc<Cell<bool>>,
}

impl Dial {
    /// `index` is the dial slot (`Signals::dials`); `spec` describes the
    /// hardware control it shows for the selected lane.
    pub fn new(cx: &mut Context, s: Signals, index: usize, spec: DialSpec) -> Handle<'_, Self> {
        let n = spec.to_norm(s.dials.get_untracked()[index]);
        let animating = Rc::new(Cell::new(false));
        Self {
            s,
            index,
            uni: None,
            spec,
            dragging: false,
            drag_norm: n,
            last_y: 0.0,
            hovered: false,
            wheel: Default::default(),
            shown_norm: Cell::new(n),
            active_t: Cell::new(0.0),
            last_draw: Cell::new(None),
            animating: animating.clone(),
        }
        .build(cx, |_| {})
        .bind(s.dials, |mut h| h.needs_redraw())
        .bind(s.dial_state, |mut h| h.needs_redraw())
        .bind(s.frame, move |mut h| {
            // Note: NOT `Handle::modify` here, it emits a full-window redraw.
            if animating.get() {
                h.needs_redraw();
            }
        })
    }

    fn uni_value(s: &Signals, f: UniField) -> f32 {
        let lane = s.lane_sel.get_untracked();
        s.arrangement.with_untracked(|a| a.lanes.get(lane).map_or(0.0, |l| f.get(l.unison) as f32))
    }

    fn value(&self) -> f32 {
        match self.uni {
            Some(f) => Self::uni_value(&self.s, f),
            None => self.s.dials.get_untracked()[self.index],
        }
    }

    fn state(&self) -> crate::state::DialState {
        match self.uni {
            Some(_) => crate::state::DialState::default(),
            None => self.s.dial_state.get_untracked()[self.index],
        }
    }

    fn set_value(&self, cx: &mut EventContext, v: f32) {
        match self.uni {
            Some(f) => cx.emit(AppEvent::SetUnison(f, v.round().max(0.0) as u8)),
            None => cx.emit(AppEvent::SetDial(self.index, v)),
        }
    }

    fn set_norm(&self, cx: &mut EventContext, n: f32) {
        self.set_value(cx, self.spec.from_norm(n));
    }
}

impl View for Dial {
    fn element(&self) -> Option<&'static str> {
        Some("dial")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        // Overridden by the instrument: inert (no drag, wheel or reset).
        let locked = self.state().locked.is_some() && !self.dragging;
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) | WindowEvent::MouseDoubleClick(MouseButton::Left) | WindowEvent::MouseScroll(..)
                if locked =>
            {
                meta.consume();
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.dragging = true;
                self.drag_norm = self.spec.to_norm(self.value());
                self.last_y = cx.mouse().cursor_y;
                cx.capture();
                cx.set_active(true);
                // One undo step per drag.
                cx.emit(AppEvent::GestureBegin);
                meta.consume();
            }
            WindowEvent::MouseMove(_, y) if self.dragging => {
                let s = cx.scale_factor();
                let dy = (self.last_y - *y) / s;
                self.last_y = *y;
                let range = if cx.modifiers().shift() {
                    DRAG_RANGE_PX * FINE_FACTOR
                } else {
                    DRAG_RANGE_PX
                };
                self.drag_norm = (self.drag_norm + dy / range).clamp(0.0, 1.0);
                self.set_norm(cx, self.drag_norm);
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.dragging => {
                self.dragging = false;
                cx.release();
                cx.set_active(false);
                cx.emit(AppEvent::GestureEnd);
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                self.set_value(cx, self.spec.default);
                meta.consume();
            }
            WindowEvent::MouseScroll(_, sy) => {
                let step = if self.spec.steps > 0 {
                    1.0 / self.spec.steps as f32
                } else {
                    WHEEL_STEP
                };
                let fine = if cx.modifiers().shift() { 0.25 } else { 1.0 };
                let steps = self.wheel.steps(*sy);
                if steps != 0 {
                    let n = self.spec.to_norm(self.value()) + steps as f32 * step * fine;
                    self.set_norm(cx, n);
                }
                meta.consume();
            }
            WindowEvent::MouseEnter => {
                self.hovered = true;
                cx.needs_redraw();
            }
            WindowEvent::MouseLeave => {
                self.hovered = false;
                cx.needs_redraw();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Dials);
        let b = cx.bounds();
        let s = cx.scale_factor();
        let dt = gfx::frame_dt(&self.last_draw);

        let value = self.value();
        let target = self.spec.to_norm(value);
        // While dragging show the value immediately; otherwise ease (e.g. double-click reset).
        // `CARTWAVE_FROZEN_METERS=1` (dev, the regression screenshots): no
        // easing, so the pointer lands on its value whatever the frame timing.
        let n = if self.dragging || crate::dev::var("CARTWAVE_FROZEN_METERS").is_some_and(|v| v == "1") {
            target
        } else {
            gfx::approach(self.shown_norm.get(), target, dt, 0.06)
        };
        self.shown_norm.set(n);
        let act_target = if self.dragging {
            1.0
        } else if self.hovered {
            0.45
        } else {
            0.0
        };
        let act = gfx::approach(self.active_t.get(), act_target, dt, EASE_TAU);
        self.active_t.set(act);
        let settled = (n - target).abs() < 0.001 && (act - act_target).abs() < 0.002;
        self.animating.set(!settled);

        let cxp = b.x + b.w * 0.5;
        let cyp = b.y + DIAL_CY * s;

        let state = self.state();
        let locked = state.locked.is_some();
        // A locked dial is drawn faded (one layer, only while locked), without the accent.
        if locked {
            canvas.save_layer_alpha_f(vg::Rect::from_xywh(b.x, b.y, b.w, b.h), 0.35);
        }

        // Label.
        let lf = gfx::font(Face::SemiBold, 10.0 * s);
        let label = state.label.unwrap_or(self.spec.label).to_uppercase();
        let lw = gfx::text_width(&lf, &label);
        gfx::draw_text(
            canvas,
            &label,
            cxp - lw * 0.5,
            b.y + 12.0 * s,
            &lf,
            TEXT_LO.mix(TEXT_HI, act * 0.6),
        );

        // Inset track.
        let arc_r = ARC_R * s;
        let oval = vg::Rect::from_xywh(cxp - arc_r, cyp - arc_r, arc_r * 2.0, arc_r * 2.0);
        let mut track = gfx::stroke(Rgba::white(0.05), (ARC_W + 2.0) * s);
        track.set_stroke_cap(vg::PaintCap::Round);
        let mut bevel = oval;
        bevel.offset((0.0, 1.0 * s));
        canvas.draw_arc(bevel, START_DEG, SWEEP_DEG, false, &track);
        track.set_color4f(BG_INSET.c4f(), None);
        canvas.draw_arc(oval, START_DEG, SWEEP_DEG, false, &track);
        let mut track_in = gfx::stroke(Rgba::hex(0x171717), ARC_W * 0.4 * s);
        track_in.set_stroke_cap(vg::PaintCap::Round);
        canvas.draw_arc(
            oval.with_offset((0.0, 0.6 * s)),
            START_DEG,
            SWEEP_DEG,
            false,
            &track_in,
        );

        // Value arc (bipolar dials grow from the top centre).
        let bipolar = self.spec.min < 0.0;
        let (a0, sweep) = if bipolar {
            let mid = START_DEG + SWEEP_DEG * 0.5;
            (mid, (n - 0.5) * SWEEP_DEG)
        } else {
            (START_DEG, n * SWEEP_DEG)
        };
        if sweep.abs() > 0.5 && !locked {
            let col = NEUTRAL_HI.lighten(0.1 + 0.15 * act);
            let mut glow = gfx::stroke(NEUTRAL_HI.with_alpha(0.45 + 0.35 * act), (ARC_W + 2.0) * s);
            glow.set_stroke_cap(vg::PaintCap::Round);
            glow.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 3.5 * s, false));
            canvas.draw_arc(oval, a0, sweep, false, &glow);
            let mut arc = vg::Paint::default();
            arc.set_anti_alias(true);
            arc.set_style(vg::PaintStyle::Stroke);
            arc.set_stroke_width(ARC_W * s);
            arc.set_stroke_cap(vg::PaintCap::Round);
            arc.set_dither(gfx::dither_on());
            arc.set_shader(gfx::linear(
                (cxp, cyp - arc_r),
                (cxp, cyp + arc_r),
                &[col.lighten(0.2), col.darken(0.1)],
                None,
            ));
            canvas.draw_arc(oval, a0, sweep, false, &arc);
        }

        // Body: soft shadow, domed radial gradient, brushed sheen, rim.
        let r = BODY_R * s;
        let body = vg::Rect::from_xywh(cxp - r, cyp - r, r * 2.0, r * 2.0);
        let rr = vg::RRect::new_oval(body);
        gfx::drop_shadow(canvas, &rr, 0.0, 3.0 * s, 9.0 * s, 0.0, Rgba::black(0.6));
        gfx::drop_shadow(canvas, &rr, 0.0, 1.0 * s, 2.0 * s, 0.0, Rgba::black(0.5));
        let mut dome = vg::Paint::default();
        dome.set_anti_alias(true);
        let lift = 0.05 * act;
        dome.set_dither(gfx::dither_on());
        dome.set_shader(gfx::radial(
            (cxp - r * 0.35, cyp - r * 0.45),
            r * 1.7,
            &[
                Rgba::hex(0x505050).lighten(lift),
                Rgba::hex(0x2F2F2F).lighten(lift),
                Rgba::hex(0x1C1C1C),
            ],
            Some(&[0.0, 0.5, 1.0]),
        ));
        canvas.draw_circle((cxp, cyp), r, &dome);
        // Brushed-metal sheen: a sweep gradient of faint light/dark bands.
        let mut sheen = vg::Paint::default();
        sheen.set_anti_alias(true);
        let bands = [
            Rgba::white(0.0),
            Rgba::white(0.07),
            Rgba::white(0.0),
            Rgba::black(0.06),
            Rgba::white(0.0),
            Rgba::white(0.06),
            Rgba::white(0.0),
            Rgba::black(0.06),
            Rgba::white(0.0),
        ];
        let c: Vec<vg::Color4f> = bands.iter().map(|c| c.c4f()).collect();
        let g = vg::gradient::Gradient::new(
            vg::gradient::Colors::new(&c, None, vg::TileMode::Clamp, None),
            vg::gradient::Interpolation::default(),
        );
        sheen.set_dither(gfx::dither_on());
        sheen.set_shader(vg::gradient::shaders::sweep_gradient(
            (cxp, cyp),
            (0.0, 360.0),
            &g,
            None,
        ));
        canvas.draw_circle((cxp, cyp), r * 0.92, &sheen);
        // Rim: bright top edge, dark bottom edge.
        let mut rim = vg::Paint::default();
        rim.set_anti_alias(true);
        rim.set_style(vg::PaintStyle::Stroke);
        rim.set_stroke_width(1.0 * s);
        rim.set_dither(gfx::dither_on());
        rim.set_shader(gfx::linear(
            (cxp, cyp - r),
            (cxp, cyp + r),
            &[Rgba::white(0.22), Rgba::black(0.45)],
            None,
        ));
        canvas.draw_circle((cxp, cyp), r - 0.5 * s, &rim);
        // Specular highlight upper-left.
        let mut spec = vg::Paint::default();
        spec.set_anti_alias(true);
        spec.set_dither(gfx::dither_on());
        spec.set_shader(gfx::radial(
            (cxp - r * 0.42, cyp - r * 0.5),
            r * 0.6,
            &[Rgba::white(0.20), Rgba::white(0.0)],
            None,
        ));
        canvas.draw_circle((cxp - r * 0.42, cyp - r * 0.5), r * 0.6, &spec);

        // Pointer.
        let ang = (START_DEG + n * SWEEP_DEG).to_radians();
        let (ca, sa) = (ang.cos(), ang.sin());
        let p0 = (cxp + ca * r * 0.38, cyp + sa * r * 0.38);
        let p1 = (cxp + ca * r * 0.80, cyp + sa * r * 0.80);
        let mut pglow = gfx::stroke(NEUTRAL_HI.lighten(0.3).with_alpha(0.5 * act), 3.0 * s);
        pglow.set_stroke_cap(vg::PaintCap::Round);
        pglow.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 2.0 * s, false));
        canvas.draw_line(p0, p1, &pglow);
        let mut ptr = gfx::stroke(Rgba::hex(0xEDEDED), 2.0 * s);
        ptr.set_stroke_cap(vg::PaintCap::Round);
        canvas.draw_line(p0, p1, &ptr);

        if locked {
            canvas.restore();
        }

        // Value readout (mono, tabular), or why the dial is inert.
        let vy = b.y + (DIAL_CY + ARC_R + 22.0) * s;
        if let Some(why) = state.locked {
            // Two short lines ("SET BY" / "INSTRUMENT"), clipped to the dial:
            // one line is wider than the dial, and text drawn outside a view's
            // bounds is left behind by later partial repaints.
            let f = gfx::font(Face::SemiBold, 9.0 * s);
            let t = why.to_uppercase();
            let (a, b2) = t.split_once(" BY ").map_or((t.as_str(), ""), |(a, b)| (a, b));
            canvas.save();
            canvas.clip_rect(vg::Rect::from_xywh(b.x, b.y, b.w, b.h), None, Some(true));
            for (k, line) in [format!("{a} BY"), b2.to_string()].iter().enumerate() {
                let w = gfx::text_width(&f, line);
                gfx::draw_text(canvas, line, cxp - w * 0.5, vy - 4.0 * s + k as f32 * 11.0 * s, &f, TEXT_LO);
            }
            canvas.restore();
            return;
        }
        let vf = gfx::font(Face::MonoMedium, 11.0 * s);
        let txt = self.spec.format(value);
        let vw = gfx::text_width(&vf, &txt);
        gfx::draw_text(
            canvas,
            &txt,
            cxp - vw * 0.5,
            vy,
            &vf,
            TEXT_HI.mix(TEXT_LO, 0.35 * (1.0 - act)),
        );

        // Floating value bubble while dragging.
        if act > 0.5 && self.dragging {
            let a = ((act - 0.5) * 2.0).clamp(0.0, 1.0);
            let bw = vw + 14.0 * s;
            let pill = vg::Rect::from_xywh(cxp - bw * 0.5, b.y + 0.0 * s, bw, 18.0 * s);
            let prr = gfx::rrect(pill, 9.0 * s);
            gfx::drop_shadow(
                canvas,
                &prr,
                0.0,
                2.0 * s,
                8.0 * s,
                0.0,
                Rgba::black(0.5 * a),
            );
            canvas.draw_rrect(
                prr,
                &gfx::vgrad(
                    pill,
                    NEUTRAL_HI.lighten(0.08).with_alpha(a),
                    NEUTRAL_HI.darken(0.2).with_alpha(a),
                ),
            );
            gfx::top_highlight(canvas, &prr, s, Rgba::white(0.3 * a));
            gfx::draw_text(
                canvas,
                &txt,
                cxp - vw * 0.5,
                pill.top + 13.0 * s,
                &vf,
                Rgba::white(a),
            );
        }
    }
}
