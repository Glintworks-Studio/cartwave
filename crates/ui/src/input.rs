//! Platform input conventions, kept in one place.
//!
//! | Action                                   | Windows / Linux      | macOS                |
//! |------------------------------------------|----------------------|----------------------|
//! | Command (select all, copy, undo, save…)  | Ctrl                 | Cmd                  |
//! | Duplicate while dragging (clips, notes)  | Ctrl                 | Option               |
//! | Bypass snap while dragging / drawing     | Alt                  | Cmd                  |
//! | Zoom with the wheel                      | Ctrl or Cmd          | Cmd or Ctrl          |
//! | Marquee in the piano roll                | Shift-drag           | Shift-drag           |
//! | Redo                                     | Ctrl+Shift+Z, Ctrl+Y | Cmd+Shift+Z, Cmd+Y   |
//!
//! The macOS column follows the common convention for DAWs on the Mac
//! (Option-drag copies, Cmd suspends the grid). Every shortcut in the app
//! goes through `command` / `duplicate_drag` / `bypass_snap` / `zoom_wheel`;
//! the full map is in PIANOROLL.md §2.

use vizia::prelude::Modifiers;

const MACOS: bool = cfg!(target_os = "macos");

/// The platform's command modifier: Cmd on macOS, Ctrl elsewhere.
pub fn command(m: &Modifiers) -> bool {
    if MACOS {
        m.logo()
    } else {
        m.ctrl()
    }
}

/// Dragging a clip with this held drags copies instead.
pub fn duplicate_drag(m: &Modifiers) -> bool {
    if MACOS {
        m.alt()
    } else {
        m.ctrl()
    }
}

/// Held while dragging (clips, playhead), positions aren't snapped to the grid.
pub fn bypass_snap(m: &Modifiers) -> bool {
    if MACOS {
        m.logo()
    } else {
        m.alt()
    }
}

/// Wheel with this held zooms the arrangement (either key, on every platform).
pub fn zoom_wheel(m: &Modifiers) -> bool {
    m.ctrl() || m.logo()
}

/// Turns wheel deltas into whole steps.
///
/// A mouse wheel notch is ±1 line, which gives one step, as before. Trackpads
/// (and smooth-scrolling mice) send many small fractional deltas per gesture;
/// stepping once per event made a single swipe jump many steps. The remainder
/// is kept between events.
#[derive(Default)]
pub struct WheelSteps {
    acc: f32,
}

impl WheelSteps {
    pub fn steps(&mut self, delta: f32) -> i32 {
        // A direction change starts a new gesture.
        if self.acc != 0.0 && self.acc.signum() != delta.signum() {
            self.acc = 0.0;
        }
        self.acc += delta;
        let n = self.acc.trunc();
        self.acc -= n;
        n as i32
    }
}
