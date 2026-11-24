//! Design tokens. Single source of truth for colors, radii and spacing used by
//! procedurally drawn views. The stylesheet (`style.css`) mirrors these values
//! as CSS variables; keep the two in sync when retuning.

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

// Surfaces
pub const BG_0: Rgba = Rgba::hex(0x0E0F13);
pub const BG_1: Rgba = Rgba::hex(0x16181F);
pub const BG_2: Rgba = Rgba::hex(0x1E2129);
pub const BG_INSET: Rgba = Rgba::hex(0x0A0B0E);
pub const LINE: Rgba = Rgba::hex(0x2A2E38);

// Text
pub const TEXT_HI: Rgba = Rgba::hex(0xE8EAF0);
pub const TEXT_LO: Rgba = Rgba::hex(0x8A90A0);

// Accent
pub const ACCENT: Rgba = Rgba::hex(0x7C5CFF);
pub const RECORD: Rgba = Rgba::hex(0xFF4D5E);

// Lanes
pub const LANE_PULSE: Rgba = Rgba::hex(0xFF5C8A);
pub const LANE_TRI: Rgba = Rgba::hex(0x4CC9F0);
pub const LANE_NOISE: Rgba = Rgba::hex(0xB8C0CC);
pub const LANE_DPCM: Rgba = Rgba::hex(0xF4A261);
pub const LANE_VRC6_PULSE: Rgba = Rgba::hex(0x9B5DE5);
pub const LANE_VRC6_SAW: Rgba = Rgba::hex(0x00F5A0);

// Meter gradient stops
pub const METER_GREEN: Rgba = Rgba::hex(0x3DDC84);
pub const METER_AMBER: Rgba = Rgba::hex(0xF4B740);
pub const METER_RED: Rgba = Rgba::hex(0xFF4D5E);

// Radii (logical px)
pub const RADIUS_CLIP: f32 = 6.0;
pub const RADIUS_FIELD: f32 = 6.0;
pub const RADIUS_BUTTON: f32 = 8.0;
pub const RADIUS_PANEL: f32 = 10.0;

// Spacing scale (logical px)
pub const SP_1: f32 = 4.0;
pub const SP_2: f32 = 8.0;
pub const SP_3: f32 = 12.0;
pub const SP_4: f32 = 16.0;
pub const SP_5: f32 = 24.0;

// Arrangement metrics (logical px)
pub const LANE_HEIGHT: f32 = 64.0;
pub const RULER_HEIGHT: f32 = 30.0;
pub const TRACK_HEADER_WIDTH: f32 = 208.0;
pub const CLIP_V_INSET: f32 = 5.0;
pub const CLIP_HEADER_HEIGHT: f32 = 17.0;
pub const CLIP_RESIZE_ZONE: f32 = 7.0;

// Motion
/// Time constant for hover/select/drag easing (seconds). Exponential approach
/// reaches ~95% in 3τ ≈ 150 ms.
pub const EASE_TAU: f32 = 0.05;
