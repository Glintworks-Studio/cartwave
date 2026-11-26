//! Design tokens. Single source of truth for colors, radii and spacing used by
//! procedurally drawn views. The stylesheet (`style.css`) mirrors these values
//! as CSS variables; keep the two in sync when retuning.
//!
//! Every spacing, padding and size used for layout is a multiple of 4 logical
//! px. Hairlines (1 px strokes) are the only exception.

use vizia::vg;

/// An RGBA color in 0..=1 floats, convenient for Skia and for lighten/darken math.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as f32 / 255.0,
            g: ((hex >> 8) & 0xFF) as f32 / 255.0,
            b: (hex & 0xFF) as f32 / 255.0,
            a: 1.0,
        }
    }

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn white(a: f32) -> Self {
        Self::new(1.0, 1.0, 1.0, a)
    }

    pub const fn black(a: f32) -> Self {
        Self::new(0.0, 0.0, 0.0, a)
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Mix towards white by `t` (0..1).
    pub fn lighten(self, t: f32) -> Self {
        self.mix(Self::white(self.a), t)
    }

    /// Mix towards black by `t` (0..1).
    pub fn darken(self, t: f32) -> Self {
        self.mix(Self::black(self.a), t)
    }

    pub fn mix(self, o: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: self.r + (o.r - self.r) * t,
            g: self.g + (o.g - self.g) * t,
            b: self.b + (o.b - self.b) * t,
            a: self.a + (o.a - self.a) * t,
        }
    }

    pub fn c4f(self) -> vg::Color4f {
        vg::Color4f::new(self.r, self.g, self.b, self.a)
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Palette, sampled from the reference prototype (docs/design/reference.png).
// Neutral greys with no blue tint; colour is reserved for lanes and the accent.
// ───────────────────────────────────────────────────────────────────────────

// Surfaces
/// Window / track column background (deepest).
pub const BG_0: Rgba = Rgba::hex(0x0A0A0A);
/// Raised panel surface (bottom panel sections).
pub const BG_1: Rgba = Rgba::hex(0x141414);
/// Capsule / box interiors (transport, meter box).
pub const BG_2: Rgba = Rgba::hex(0x151515);
/// Sunk wells (fields, meters).
pub const BG_INSET: Rgba = Rgba::hex(0x0C0C0C);
/// Hairline dividers between bands and rows.
pub const LINE: Rgba = Rgba::hex(0x1A1A1A);
/// Capsule / box outline.
pub const STROKE: Rgba = Rgba::hex(0x323232);
/// Key (M/S) side and bottom stroke.
pub const KEY_STROKE: Rgba = Rgba::hex(0x383838);
/// Brightest point of a key's top-edge highlight (fades to the corners).
pub const KEY_SHEEN: Rgba = Rgba::hex(0x999999);

/// Band gradients (top → bottom).
pub const TOPBAR_TOP: Rgba = Rgba::hex(0x191919);
pub const TOPBAR_BOTTOM: Rgba = Rgba::hex(0x0A0A0A);
pub const BAND_TOP: Rgba = Rgba::hex(0x121212);
pub const BAND_BOTTOM: Rgba = Rgba::hex(0x0A0A0A);
/// The arrangement canvas: lit from the top, falling to black.
pub const CANVAS_TOP: Rgba = Rgba::hex(0x191919);
pub const CANVAS_BOTTOM: Rgba = Rgba::hex(0x0A0A0A);
/// Key bodies are lit from below (dark top, lighter bottom). Option D:
/// `#161616 → #424242` (docs/design/VISUAL-SPEC.md §1).
pub const KEY_TOP: Rgba = Rgba::hex(0x161616);
pub const KEY_BOTTOM: Rgba = Rgba::hex(0x424242);
/// Label segment of a split pill (lighter than the value segment).
pub const PILL_LABEL_TOP: Rgba = Rgba::hex(0x323232);
pub const PILL_LABEL_BOTTOM: Rgba = Rgba::hex(0x545454);

// Text
pub const TEXT_HI: Rgba = Rgba::hex(0xE6E6E6);
/// Values in the display.
pub const TEXT_VALUE: Rgba = Rgba::hex(0xEDEDED);
/// Key icons: idle and dim (the unselected view, disabled keys).
pub const ICON_IDLE: Rgba = Rgba::hex(0xD6D6D6);
pub const ICON_DIM: Rgba = Rgba::hex(0x8C8C8C);
/// Track names, key glyphs. >= 4.5:1 on every surface incl. key faces (#2D).
pub const TEXT_MID: Rgba = Rgba::hex(0x9A9A9A);
/// Secondary information (labels, hints, read-outs): >= 4.5:1 on every panel
/// background (#0A..#15). Raised from #5C (2.7:1) in STABILIZE.md §A4.4.
pub const TEXT_LO: Rgba = Rgba::hex(0x808080);
/// Decoration only (watermarks), never information: 1.6:1, below WCAG AA.
pub const TEXT_GHOST: Rgba = Rgba::hex(0x3A3A3A);
/// The big time read-out (gradient top → bottom).
pub const READOUT_TOP: Rgba = Rgba::hex(0xDEDEDE);
pub const READOUT_BOTTOM: Rgba = Rgba::hex(0xA6A6A6);

// Lime: switched-on states only (VISUAL-SPEC.md §1): lit M/S glyphs, the
// display's active indicators, the selected view icon, the loop brace while
// looping. Everything else is neutral.
pub const ACCENT: Rgba = Rgba::hex(0xC7D74E);
/// The neutral stand-in for the old accent: playhead, focus, dial arcs,
/// checked buttons.
pub const NEUTRAL_HI: Rgba = Rgba::hex(0xE6E6E6);
/// The display's indicators while inactive.
pub const INDICATOR_OFF: Rgba = Rgba::hex(0x4A4A4A);
/// Hairlines between the display's cells.
pub const CELL_DIVIDER: Rgba = Rgba::hex(0x2A2A2A);
/// The record glyph (drawn, disabled until MIDI input arrives).
pub const RECORD_GLYPH: Rgba = Rgba::hex(0xB9534C);
pub const RECORD: Rgba = Rgba::hex(0xE5484D);

// Lanes (pulse, triangle, noise and VRC6 pulse from the Option D mockup's
// tracks; the design pass). Formerly: the bright end of each colour strip (the strip darkens to ~65% on
// its left). Pulse lime and triangle periwinkle come from the prototype; the
// rest are picked at the same lightness and saturation.
pub const LANE_PULSE: Rgba = Rgba::hex(0xC9D24A);
pub const LANE_TRI: Rgba = Rgba::hex(0x7F86E0);
pub const LANE_NOISE: Rgba = Rgba::hex(0xB5AC98);
pub const LANE_DPCM: Rgba = Rgba::hex(0xE39A42);
pub const LANE_VRC6_PULSE: Rgba = Rgba::hex(0xD9579A);
pub const LANE_VRC6_SAW: Rgba = Rgba::hex(0x3CC4A6);

// The track volume meter-fader: cool-to-hot stops over its full 68 px
// (px from the left, colour).
pub const FADER_STOPS: [(f32, Rgba); 5] = [
    (0.0, Rgba::hex(0x2F6FD6)),
    (22.0, Rgba::hex(0x2FB7B0)),
    (44.0, Rgba::hex(0xC7D74E)),
    (56.0, Rgba::hex(0xF0A23A)),
    (68.0, Rgba::hex(0xE0483B)),
];

// Arrangement surfaces (Option D)
/// Ruler band and the TRACKS cell beside it.
pub const RULER_BG: Rgba = Rgba::hex(0x0F0F0F);
/// The one hairline under the ruler, and the header column's right edge.
pub const LINE_SOFT: Rgba = Rgba::hex(0x1E1E1E);
/// Ruler bar lines and beat ticks; sixteenth ticks.
pub const RULER_TICK: Rgba = Rgba::hex(0x3A3A3A);
pub const RULER_TICK_MINOR: Rgba = Rgba::hex(0x262626);
/// Lane rows alternate between these.
pub const ROW_A: Rgba = Rgba::hex(0x171717);
pub const ROW_B: Rgba = Rgba::hex(0x141414);
/// Lane grid: bars and beats.
pub const GRID_BAR: Rgba = Rgba::hex(0x262626);
pub const GRID_BEAT: Rgba = Rgba::hex(0x1D1D1D);
/// The big italic channel number in the track header.
pub const NUMBER_INK: Rgba = Rgba::hex(0x222222);
/// The profile badge's lock and chip name.
pub const BADGE_INK: Rgba = Rgba::hex(0xA8A8A8);
/// Clip header text.
pub const CLIP_INK: Rgba = Rgba::hex(0x111111);

// Meter gradient stops
pub const METER_GREEN: Rgba = Rgba::hex(0x9CCB3B);
pub const METER_AMBER: Rgba = Rgba::hex(0xE3B341);
pub const METER_RED: Rgba = Rgba::hex(0xE5484D);

// Radii (logical px)
pub const RADIUS_CLIP: f32 = 4.0;
pub const RADIUS_KEY: f32 = 4.0;
pub const RADIUS_FIELD: f32 = 4.0;
pub const RADIUS_BUTTON: f32 = 4.0;
pub const RADIUS_BOX: f32 = 8.0;
pub const RADIUS_PANEL: f32 = 8.0;

// Spacing scale (logical px). Every layout distance is a multiple of 4.
pub const SP_1: f32 = 4.0;
pub const SP_2: f32 = 8.0;
pub const SP_3: f32 = 12.0;
pub const SP_4: f32 = 16.0;
pub const SP_5: f32 = 24.0;

// Shell metrics (logical px)
pub const TOPBAR_HEIGHT: f32 = 76.0;
/// The display well and the spectrum well.
pub const DISPLAY_W: f32 = 560.0;
pub const WELL_H: f32 = 56.0;
pub const SPECTRUM_W: f32 = 216.0;
/// View and transport keys; the TAP key.
pub const KEY_BIG: f32 = 40.0;
pub const KEY_TAP: f32 = 56.0;
/// Corner radius of the 40 / 56 px keys and of the wells.
pub const RADIUS_KEY_BIG: f32 = 8.0;
pub const KEY_SIZE: f32 = 24.0;
pub const LANE_STRIP_W: f32 = 16.0;

// Arrangement metrics (logical px)
/// A track row: 72 px plus its 1 px bottom hairline (the lane pitch is
/// `state::LANE_H_NORMAL`, 73).
pub const LANE_HEIGHT: f32 = 72.0;
pub const RULER_HEIGHT: f32 = 28.0;
/// The Overview strip above the ruler (polish 2 §7) and the status bar
/// along the window's bottom (§6).
pub const OVERVIEW_HEIGHT: f32 = 32.0;
pub const STATUS_BAR_HEIGHT: f32 = 24.0;
/// The header column including its right hairline.
pub const TRACK_HEADER_WIDTH: f32 = 380.0;
pub const CLIP_V_INSET: f32 = 2.0;
pub const CLIP_HEADER_HEIGHT: f32 = 16.0;
pub const CLIP_RESIZE_ZONE: f32 = 8.0;

// Motion
/// Time constant for hover/select/drag easing (seconds). Exponential approach
/// reaches ~95% in 3τ ≈ 150 ms.
pub const EASE_TAU: f32 = 0.05;
