//! F3 debug overlay: frame pacing, process CPU, per-phase UI-thread timings
//! (reported by the vizia backend) and per-view custom draw timings.

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::perf::SPANS;
use crate::state::PHASE_NAMES;
use crate::theme::tokens::*;
use crate::Signals;

const WIDTH: f32 = 300.0;
const LINE_H: f32 = 16.0;
const PAD: f32 = 12.0;
/// Header lines + blank + phases + blank + spans + audio.
const LINES: usize = 5 + 1 + PHASE_NAMES.len() + 1 + SPANS.len() + AUDIO_LINES;
const AUDIO_LINES: usize = 5;

pub struct StatsOverlay {
    s: Signals,
}

impl StatsOverlay {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        let visible = s.stats.map(|st| st.visible);
        Self { s }
            .build(cx, |_| {})
            .position_type(PositionType::Absolute)
            .left(Stretch(1.0))
            .right(Pixels(20.0))
            .top(Pixels(76.0))
            .width(Pixels(WIDTH))
            .height(Pixels(PAD * 2.0 + LINES as f32 * LINE_H))
            .hoverable(false)
            .display(visible)
            .bind(s.stats, |mut h| h.needs_redraw())
            .bind(s.audio, |mut h| h.needs_redraw())
    }
}

impl View for StatsOverlay {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Overlay);
        let st = self.s.stats.get_untracked();
        let b = cx.bounds();
        let s = cx.scale_factor();
        let rect: vg::Rect = b.into();
        let rr = gfx::rrect(rect, 8.0 * s);
        gfx::drop_shadow(canvas, &rr, 0.0, 4.0 * s, 14.0 * s, 0.0, Rgba::black(0.5));
        canvas.draw_rrect(rr, &gfx::fill(Rgba::hex(0x0B0C10).with_alpha(0.94)));
        gfx::inner_border(canvas, &rr, s, ACCENT.with_alpha(0.45));

        let f = gfx::font(Face::MonoMedium, 11.0 * s);
        let fps = if st.frame_ms > 0.0 {
            1000.0 / st.frame_ms
        } else {
            0.0
        };
        let ui_total: f32 = st.phases_ms.iter().sum();
        let custom_total: f32 = st.spans_ms.iter().sum();

        let mut lines: Vec<(String, Rgba)> = vec![
            (
                format!("frame  {:6.2} ms {:6.1} fps", st.frame_ms, fps),
                TEXT_HI,
            ),
            (
                format!("worst  {:6.2} ms  (1 s)", st.worst_ms),
                if st.worst_ms > 20.0 {
                    METER_AMBER
                } else {
                    TEXT_HI
                },
            ),
            (format!("cpu    {:6.1} %  process", st.cpu_percent), TEXT_HI),
            (
                format!(
                    "cap    {:<7} display {}",
                    st.cap.label(),
                    st.display_hz.map_or("?".into(), |hz| format!("{hz:.0} Hz"))
                ),
                TEXT_HI,
            ),
            (
                format!("clips  {:6}    frames {}", st.clip_count, st.frames),
                TEXT_LO,
            ),
            (
                format!("ui thread  {:6.2} ms", ui_total),
                ACCENT.lighten(0.4),
            ),
        ];
        for (name, ms) in PHASE_NAMES.iter().zip(st.phases_ms) {
            lines.push((format!("  {:<10} {:6.2}", name, ms), TEXT_HI));
        }
        lines.push((
            format!("custom draw {:5.2} ms", custom_total),
            ACCENT.lighten(0.4),
        ));
        for ((_, name), ms) in SPANS.iter().zip(st.spans_ms) {
            lines.push((
                format!("  {:<10} {:6.2}", name, ms),
                TEXT_LO.mix(TEXT_HI, 0.5),
            ));
        }

        let a = self.s.audio.get_untracked();
        lines.push((
            format!("audio  {}", if a.running { "running" } else { "STOPPED" }),
            ACCENT.lighten(0.4),
        ));
        let dev: String = a.device.chars().take(36).collect();
        lines.push((format!("  {dev}"), TEXT_LO.mix(TEXT_HI, 0.5)));
        lines.push((
            format!(
                "  {} fr/cb  lat {:5.1} ms  filter {}",
                a.frames_per_cb, a.latency_ms, a.filter
            ),
            TEXT_HI,
        ));
        lines.push((
            format!(
                "  cb {:4.2}/{:4.2}/{:4.1} %  mean/p99/max",
                a.cb_mean, a.cb_p99, a.cb_max
            ),
            TEXT_HI,
        ));
        lines.push((
            format!(
                "  xruns {}  overruns {}  edits {}",
                a.underruns, a.overruns, a.edits
            ),
            if a.underruns > 0 { METER_AMBER } else { TEXT_HI },
        ));
        for (i, (l, col)) in lines.iter().enumerate() {
            let y = b.y + (PAD + 11.0 + i as f32 * LINE_H) * s;
            gfx::draw_text(canvas, l, b.x + PAD * s, y, &f, *col);
        }
    }
}
