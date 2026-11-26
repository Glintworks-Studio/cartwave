//! The palette reference sheet (polish §3.2): every palette colour as a lit
//! strip, an unlit strip and a clip, drawn by the app's own code and
//! recipes (theme::palette), on a lane row.
//!
//! usage: cargo run --release -p ui --example palette_sheet -- <out.png>

use ui::theme::palette;
use ui::theme::tokens::*;
use ui::views::{chrome, clip, gfx};
use vizia::vg;

const COLS: usize = 7;
const TILE_W: f32 = 176.0;
const TILE_H: f32 = 96.0;
const PAD: f32 = 12.0;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "palette-reference-sheet.png".into());
    let s = 1.0;
    let n = palette::ROWS * palette::COLS;
    let rows = n.div_ceil(COLS);
    let (w, h) = ((COLS as f32 * TILE_W + 2.0 * PAD) as i32, (rows as f32 * TILE_H + 2.0 * PAD) as i32);
    let mut surface = vg::surfaces::raster_n32_premul((w, h)).expect("surface");
    let canvas = surface.canvas();
    canvas.clear(BG_0.c4f().to_color());
    let notes: Vec<ui::notes::Note> = (0..6).map(|i| ui::notes::Note::new(i, i * 480, 360, 60 + (i as u8 * 3) % 9)).collect();
    for (i, c) in palette::all().enumerate() {
        let (x0, y0) = (PAD + (i % COLS) as f32 * TILE_W, PAD + (i / COLS) as f32 * TILE_H);
        let tile = vg::Rect::from_xywh(x0, y0, TILE_W - 8.0, 72.0);
        canvas.draw_rect(tile, &gfx::fill(ROW_A));
        // Lit strip (the app's pre-rendered lit layer at full light).
        let lit = vg::Rect::from_xywh(x0, y0, LANE_STRIP_W, 72.0);
        let (fill, el, er) = palette::strip(c, 0.0);
        let unlit = vg::Rect::from_xywh(x0 + LANE_STRIP_W + 4.0, y0, LANE_STRIP_W, 72.0);
        for (r, lit_on) in [(lit, true), (unlit, false)] {
            canvas.draw_rect(r, &gfx::shaded(gfx::linear((r.left, r.top), (r.right, r.top), &fill, Some(&[0.0, 0.55, 1.0]))));
            canvas.draw_rect(vg::Rect::from_xywh(r.left, r.top, 1.0, r.height()), &gfx::fill(el));
            canvas.draw_rect(vg::Rect::from_xywh(r.right - 1.0, r.top, 1.0, r.height()), &gfx::fill(er));
            if lit_on {
                if let Some(img) = chrome::render_lit_strip(c, r.width(), r.height(), s) {
                    canvas.draw_image(&img, (r.left, r.top), None);
                }
            }
        }
        // A clip (2 px in from the row, 1 px short at its end).
        let cr = vg::Rect::from_xywh(x0 + 2.0 * LANE_STRIP_W + 12.0, y0 + 2.0, TILE_W - 2.0 * LANE_STRIP_W - 24.0, 68.0);
        let name = format!("#{:06X}", ui::state::rgb_hex(c));
        let p = clip::ClipPaint {
            rect: cr,
            color: c,
            name: &name,
            inst: None,
            notes: &notes,
            len_ticks: 2880,
            anim: clip::ClipAnim::default(),
            px_per_tick: cr.width() / 2880.0,
            visible: (cr.left, cr.right),
            scale: s,
        };
        clip::draw_clip_body(canvas, &p);
        // Its index and the header ink's contrast, under the tile.
        let k = palette::clip(c, 0.0, 0.0);
        let f = gfx::font(gfx::Face::Medium, 10.0);
        let label = format!("{:>2}  {:.1}:1", i + 1, palette::contrast(k.ink, k.header));
        gfx::draw_text(canvas, &label, x0, y0 + 86.0, &f, TEXT_LO);
    }
    let img = surface.image_snapshot();
    let data = img.encode(None, vg::EncodedImageFormat::PNG, None).expect("png");
    std::fs::write(&out, data.as_bytes()).expect("write");
    println!("{out}");
}
