//! Standalone host for the ChipDAW GUI prototype.
//!
//! Optional: `--size WxH` sets the initial logical window size (useful for
//! capturing at 150%/200% display scaling on a 1440p screen).

use vizia::prelude::*;

const DEFAULT_SIZE: (u32, u32) = (1440, 900);

fn parse_size() -> (u32, u32) {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "--size")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.split_once('x'))
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or(DEFAULT_SIZE)
}

fn main() -> Result<(), ApplicationError> {
    let size = parse_size();
    Application::new(|cx| {
        ui::build_ui(cx);
    })
    .title("ChipDAW — GUI prototype")
    .inner_size(size)
    // Present on vblank; the UI advances its model once per presented frame.
    // `CHIPDAW_VSYNC=0` turns it off (an experiment knob for PERF measurements).
    .vsync(std::env::var("CHIPDAW_VSYNC").map_or(true, |v| v != "0"))
    .run()
}
