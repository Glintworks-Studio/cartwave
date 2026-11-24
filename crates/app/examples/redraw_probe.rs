//! Probe: CPU cost of vizia redrawing one tiny view at ~64 Hz.
use vizia::prelude::*;

struct Tiny {
    frame: Signal<u64>,
}
impl View for Tiny {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let b = cx.bounds();
        let mut p = vizia::vg::Paint::default();
        let f = self.frame.get_untracked();
        p.set_color(vizia::vg::Color::from_rgb((f % 255) as u8, 80, 200));
        canvas.draw_rect(vizia::vg::Rect::from_xywh(b.x, b.y, b.w, b.h), &p);
    }
}

#[cfg(windows)]
#[link(name = "winmm")]
unsafe extern "system" {
    fn timeBeginPeriod(u: u32) -> u32;
}

fn main() -> Result<(), ApplicationError> {
    #[cfg(windows)]
    if std::env::var("PROBE_TBP").is_ok() {
        unsafe { timeBeginPeriod(1) };
    }
    let last = std::cell::Cell::new(Instant::now());
    let acc = std::cell::Cell::new((0u32, 0.0f64));
    Application::new(move |cx| {
        let frame = Signal::new(0u64);
        let full = std::env::args().any(|a| a == "--full");
        let us: u64 = std::env::var("PROBE_US")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15000);
        let t = cx.add_timer(Duration::from_micros(us), None, move |_cx, a| {
            if let TimerAction::Tick(_) = a {
                frame.update(|f| *f += 1);
                let now = Instant::now();
                let dt = (now - last.get()).as_secs_f64() * 1000.0;
                last.set(now);
                let (n, sum) = acc.get();
                if n == 200 {
                    eprintln!("avg interval {:.2} ms", sum / n as f64);
                    acc.set((0, 0.0));
                } else {
                    acc.set((n + 1, sum + dt));
                }
            }
        });
        cx.start_timer(t);
        let size = if full { Stretch(1.0) } else { Pixels(8.0) };
        Tiny { frame }
            .build(cx, |_| {})
            .size(size)
            .bind(frame, |mut h| h.needs_redraw());
    })
    .inner_size((1440, 900))
    .run()
}
