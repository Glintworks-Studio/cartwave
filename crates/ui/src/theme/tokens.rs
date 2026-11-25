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
/// Key bodies are lit from below (dark top, lighter bottom).
pub const KEY_TOP: Rgba = Rgba::hex(0x191919);
pub const KEY_BOTTOM: Rgba = Rgba::hex(0x424242);
/// Label segment of a split pill (lighter than the value segment).
pub const PILL_LABEL_TOP: Rgba = Rgba::hex(0x323232);
pub const PILL_LABEL_BOTTOM: Rgba = Rgba::hex(0x545454);

// Text
pub const TEXT_HI: Rgba = Rgba::hex(0xD9D9D9);
/// Track names, key glyphs.
pub const TEXT_MID: Rgba = Rgba::hex(0x888888);
pub const TEXT_LO: Rgba = Rgba::hex(0x5C5C5C);
/// Recessive ink (chip badges): barely above the surface.
pub const TEXT_GHOST: Rgba = Rgba::hex(0x3A3A3A);
/// The big time read-out (gradient top → bottom).
pub const READOUT_TOP: Rgba = Rgba::hex(0xDEDEDE);
pub const READOUT_BOTTOM: Rgba = Rgba::hex(0xA6A6A6);

// Accent: the lime of the loop light.
pub const ACCENT: Rgba = Rgba::hex(0xC7D74E);
pub const RECORD: Rgba = Rgba::hex(0xE5484D);

// Lanes: the bright end of each colour strip (the strip darkens to ~65% on
// its left). Pulse lime and triangle periwinkle come from the prototype; the
// rest are picked at the same lightness and saturation.
pub const LANE_PULSE: Rgba = Rgba::hex(0xBAC51A);
pub const LANE_TRI: Rgba = Rgba::hex(0x828BEE);
pub const LANE_NOISE: Rgba = Rgba::hex(0xB3AA94);
pub const LANE_DPCM: Rgba = Rgba::hex(0xE39A42);
pub const LANE_VRC6_PULSE: Rgba = Rgba::hex(0xD5609E);
pub const LANE_VRC6_SAW: Rgba = Rgba::hex(0x3CC4A6);

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
pub const TOPBAR_HEIGHT: f32 = 60.0;
/// Transport capsule and meter box.
pub const BOX_W: f32 = 204.0;
pub const BOX_H: f32 = 44.0;
/// CPU / kHz split pills.
pub const PILL_W: f32 = 72.0;
pub const PILL_H: f32 = 20.0;
pub const KEY_SIZE: f32 = 24.0;
pub const LANE_STRIP_W: f32 = 16.0;

// Arrangement metrics (logical px)
pub const LANE_HEIGHT: f32 = 60.0;
pub const RULER_HEIGHT: f32 = 40.0;
pub const TRACK_HEADER_WIDTH: f32 = 356.0;
pub const CLIP_V_INSET: f32 = 4.0;
pub const CLIP_HEADER_HEIGHT: f32 = 16.0;
pub const CLIP_RESIZE_ZONE: f32 = 8.0;

// Motion
/// Time constant for hover/select/drag easing (seconds). Exponential approach
/// reaches ~95% in 3τ ≈ 150 ms.
pub const EASE_TAU: f32 = 0.05;
