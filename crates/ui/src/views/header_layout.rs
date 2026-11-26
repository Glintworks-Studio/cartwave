//! Where a track header's parts sit (polish 2 §3–§5; boards
//! `docs/design/polish2-ref/E1-*`). All positions are logical px from the
//! header's left edge, for a header `w` px wide including its 1 px hairline
//! on the lanes' side.
//!
//! The parts are laid out "strip first": strip 0–16, number 28–56, name
//! 68 … w − 144, controls w − 132 … w − 12. When the strip sits at the
//! header's right edge the boxes are mirrored as a whole. The control block
//! itself never mirrors: column A (M / S, pan) then column B (the fader).
//!
//! | Headers | Strip        | Left to right                          | Mirrored |
//! |---------|--------------|----------------------------------------|----------|
//! | Left    | Outside      | strip · number · name · controls       | no       |
//! | Left    | By the clips | controls · name · number · strip       | yes      |
//! | Right   | By the clips | strip · number · name · controls       | no       |
//! | Right   | Outside      | controls · name · number · strip       | yes      |

use crate::file::{HEADER_W_MAX, HEADER_W_MIN};

/// The strip, number and name columns (strip-first frame).
pub const STRIP_W: f32 = 16.0;
pub const NUMBER_X: f32 = 28.0;
pub const NUMBER_W: f32 = 28.0;
pub const NAME_X: f32 = 68.0;
/// The name column ends 12 px before the control block.
pub const NAME_END_FROM_RIGHT: f32 = 144.0;
/// The control block: 120 px, 12 px in from the header's far edge; its
/// two columns (56 and 40 px, 16 apart) sit at its right, so column A starts
/// 8 px in.
pub const CTRL_W: f32 = 120.0;
pub const CTRL_EDGE: f32 = 12.0;
pub const COL_A_IN: f32 = 8.0;
pub const COL_A_W: f32 = 56.0;
pub const COL_GAP: f32 = 16.0;
pub const COL_B_W: f32 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeaderLayout {
    /// The header's width including its hairline (380 … 600).
    pub w: f32,
    /// Headers on the right of the lanes.
    pub right: bool,
    /// The strip by the clips (else at the window's edge).
    pub by_clips: bool,
}

impl Default for HeaderLayout {
    fn default() -> Self {
        Self { w: HEADER_W_MIN, right: false, by_clips: false }
    }
}

impl HeaderLayout {
    pub fn of(s: &crate::file::Settings) -> Self {
        Self { w: clamp_width(s.header_width), right: s.headers_right, by_clips: s.strip_by_clips }
    }

    /// The strip sits at the header's right edge (the order table's
    /// mirrored rows).
    pub fn mirrored(&self) -> bool {
        self.right != self.by_clips
    }

    /// A box `x0 … x0 + w` of the strip-first frame in header coordinates.
    pub fn place(&self, x0: f32, w: f32) -> f32 {
        if self.mirrored() {
            self.w - x0 - w
        } else {
            x0
        }
    }

    /// The hairline's x (on the lanes' side).
    pub fn hairline_x(&self) -> f32 {
        if self.right {
            0.0
        } else {
            self.w - 1.0
        }
    }

    /// The strip's left edge.
    pub fn strip_x(&self) -> f32 {
        self.place(0.0, STRIP_W)
    }

    pub fn number_x(&self) -> f32 {
        self.place(NUMBER_X, NUMBER_W)
    }

    /// The name column (text is always left-aligned in it).
    pub fn name_w(&self) -> f32 {
        self.w - NAME_X - NAME_END_FROM_RIGHT
    }

    pub fn name_x(&self) -> f32 {
        self.place(NAME_X, self.name_w())
    }

    /// The control block's left edge.
    pub fn ctrl_x(&self) -> f32 {
        self.place(self.w - CTRL_EDGE - CTRL_W, CTRL_W)
    }

    /// Column A (M / S, pan) and column B (the fader): left edges.
    pub fn col_a(&self) -> f32 {
        self.ctrl_x() + COL_A_IN
    }

    pub fn col_b(&self) -> f32 {
        self.col_a() + COL_A_W + COL_GAP
    }

    /// The strip faces the header's inside on this side: its spill goes there.
    pub fn spill_right(&self) -> bool {
        !self.mirrored()
    }
}

pub fn clamp_width(w: f32) -> f32 {
    if w.is_finite() {
        w.round().clamp(HEADER_W_MIN, HEADER_W_MAX)
    } else {
        HEADER_W_MIN
    }
}

/// Positions inside a row (logical px from its top), from E1-header /
/// E1-row-heights. The number, name block and column A keep a 72 px band at
/// the top; only the fader stretches.
pub mod row {
    /// The fixed band at the top of every row.
    pub const BAND: f32 = 72.0;
    /// M / S keys: 24 × 24, 8 apart, at y 10 (column A centred in the band).
    pub const KEYS_Y: f32 = 10.0;
    pub const KEY_GAP: f32 = 8.0;
    /// The pan value (Inter 600 10/12) and the pan slider's well (56 × 4).
    pub const PAN_VALUE_Y: f32 = 42.0;
    pub const PAN_WELL_Y: f32 = 58.0;
    pub const PAN_WELL_H: f32 = 4.0;
    /// The vertical fader: well 4 px wide at column B's centre, from y 8,
    /// `row height − 32` long; the dB value 4 px under it.
    pub const FADER_TOP: f32 = 8.0;
    pub const FADER_WELL_W: f32 = 4.0;
    pub const FADER_SHORTER: f32 = 32.0;
    pub const DB_GAP: f32 = 4.0;
    pub const VALUE_LINE: f32 = 12.0;
    /// The fader's thumb (12 × 8) and the pan's (8 × 12).
    pub const VTHUMB_W: f32 = 12.0;
    pub const VTHUMB_H: f32 = 8.0;
    pub const HTHUMB_W: f32 = 8.0;
    pub const HTHUMB_H: f32 = 12.0;
    /// dB scale marks (rows ≥ 96 px): ticks 4 px long ending 2 px left of
    /// the thumb (well x −10 … −6), labels right-aligned 2 px left of them
    /// (at well x −12), Inter 600 9/10.
    pub const TICK_X: f32 = -10.0;
    pub const TICK_W: f32 = 4.0;
    pub const LABEL_RIGHT: f32 = -12.0;
    pub const LABEL_LINE: f32 = 10.0;

    /// The fader's well length for a row `h` px tall (without its hairline).
    pub fn fader_len(h: f32) -> f32 {
        h - FADER_SHORTER
    }

    /// The dB value's top.
    pub fn db_value_y(h: f32) -> f32 {
        FADER_TOP + fader_len(h) + DB_GAP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Polish 2 §5.3: the order table, against the side-order board's boxes.
    #[test]
    fn order_table_matches_the_board() {
        let ll = HeaderLayout { w: 380.0, right: false, by_clips: false };
        assert_eq!((ll.strip_x(), ll.number_x(), ll.name_x(), ll.ctrl_x()), (0.0, 28.0, 68.0, 248.0));
        assert_eq!((ll.col_a(), ll.col_b(), ll.hairline_x()), (256.0, 328.0, 379.0));
        let lc = HeaderLayout { by_clips: true, ..ll };
        assert_eq!((lc.strip_x(), lc.number_x(), lc.name_x(), lc.ctrl_x()), (364.0, 324.0, 144.0, 12.0));
        assert_eq!((lc.col_a(), lc.col_b()), (20.0, 92.0), "the block never mirrors inside");
        let rc = HeaderLayout { right: true, by_clips: true, ..ll };
        assert_eq!((rc.strip_x(), rc.number_x(), rc.name_x(), rc.ctrl_x(), rc.hairline_x()), (0.0, 28.0, 68.0, 248.0, 0.0));
        let ro = HeaderLayout { right: true, by_clips: false, ..ll };
        assert_eq!((ro.strip_x(), ro.name_x(), ro.ctrl_x()), (364.0, 144.0, 12.0));
        assert!(!ll.mirrored() && lc.mirrored() && !rc.mirrored() && ro.mirrored());
    }

    /// Polish 2 §5.1: extra width goes only to the name column.
    #[test]
    fn width_goes_to_the_name() {
        let w = HeaderLayout { w: 520.0, ..Default::default() };
        assert_eq!((w.name_x(), w.name_w(), w.ctrl_x()), (68.0, 308.0, 388.0));
        assert_eq!(w.w - w.ctrl_x() - CTRL_W, CTRL_EDGE, "the block stays at its edge");
        assert_eq!(clamp_width(100.0), 380.0);
        assert_eq!(clamp_width(900.0), 600.0);
    }
}
