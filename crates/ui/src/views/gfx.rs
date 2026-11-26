//! Shared procedural-drawing helpers on top of Skia (`vizia::vg`).
//! All sizes passed in are *physical* pixels unless noted; callers multiply by
//! the scale factor.

use std::cell::RefCell;

use vizia::vg;
use vizia::vg::gradient::{shaders, Colors, Gradient, Interpolation};

use crate::theme::tokens::Rgba;

pub const INTER_REGULAR: &[u8] = include_bytes!("../../../../assets/fonts/Inter-Regular.ttf");
pub const INTER_MEDIUM: &[u8] = include_bytes!("../../../../assets/fonts/Inter-Medium.ttf");
pub const INTER_SEMIBOLD: &[u8] = include_bytes!("../../../../assets/fonts/Inter-SemiBold.ttf");
pub const MONO_REGULAR: &[u8] =
    include_bytes!("../../../../assets/fonts/JetBrainsMono-Regular.ttf");
pub const MONO_MEDIUM: &[u8] = include_bytes!("../../../../assets/fonts/JetBrainsMono-Medium.ttf");
/// The display's bar positions (Option D: JetBrains Mono 600).
pub const MONO_SEMIBOLD: &[u8] =
    include_bytes!("../../../../assets/fonts/JetBrainsMono-SemiBold.ttf");
pub const INTER_BOLD: &[u8] = include_bytes!("../../../../assets/fonts/Inter-Bold.ttf");
pub const INTER_XBOLD_ITALIC: &[u8] =
    include_bytes!("../../../../assets/fonts/Inter-ExtraBoldItalic.ttf");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Regular,
    Medium,
    SemiBold,
    Mono,
    MonoMedium,
    Bold,
    /// Watermark numerals in the track rows.
    HeavyItalic,
    /// The display's bar positions.
    MonoSemiBold,
}

struct Faces {
    faces: [vg::Typeface; 8],
}

thread_local! {
    static FACES: RefCell<Option<Faces>> = const { RefCell::new(None) };
}

fn typeface(face: Face) -> vg::Typeface {
    FACES.with(|f| {
        let mut f = f.borrow_mut();
        let faces = f.get_or_insert_with(|| {
            let mgr = vg::FontMgr::new();
            let load = |b: &[u8]| mgr.new_from_data(b, None).expect("bundled font must load");
            Faces {
                faces: [
                    load(INTER_REGULAR),
                    load(INTER_MEDIUM),
                    load(INTER_SEMIBOLD),
                    load(MONO_REGULAR),
                    load(MONO_MEDIUM),
                    load(INTER_BOLD),
                    load(INTER_XBOLD_ITALIC),
                    load(MONO_SEMIBOLD),
                ],
            }
        });
        faces.faces[face as usize].clone()
    })
}

/// A font at a physical pixel size with crisp, subpixel-positioned glyphs.
pub fn font(face: Face, size_px: f32) -> vg::Font {
    let mut f = vg::Font::from_typeface(typeface(face), size_px);
    f.set_edging(vg::font::Edging::AntiAlias);
    f.set_subpixel(true);
    f.set_hinting(vg::FontHinting::Slight);
    f.set_linear_metrics(true);
    f
}

pub fn text_width(font: &vg::Font, s: &str) -> f32 {
    font.measure_str(s, None).0
}

/// Cap-height-centred baseline for vertically centring text at `cy`.
pub fn baseline_for_center(font: &vg::Font, cy: f32) -> f32 {
    let (_, m) = font.metrics();
    let cap = if m.cap_height > 0.0 {
        m.cap_height
    } else {
        -m.ascent * 0.7
    };
    cy + cap * 0.5
}

/// Gradient dithering (design pass §3.8): on, except with
/// `CHIPDAW_NO_DITHER=1` in dev mode (the banding A/B in DESIGNPASS.md).
pub fn dither_on() -> bool {
    thread_local! {
        static ON: bool = !crate::dev::var("CHIPDAW_NO_DITHER").is_some_and(|v| v == "1");
    }
    ON.with(|o| *o)
}

/// An anti-aliased, dithered paint for a gradient shader.
pub fn shaded(shader: Option<vg::Shader>) -> vg::Paint {
    let mut p = vg::Paint::default();
    p.set_anti_alias(true);
    p.set_dither(dither_on());
    p.set_shader(shader);
    p
}

pub fn fill(color: Rgba) -> vg::Paint {
    let mut p = vg::Paint::new(color.c4f(), None);
    p.set_anti_alias(true);
    p
}

pub fn stroke(color: Rgba, width: f32) -> vg::Paint {
    let mut p = fill(color);
    p.set_style(vg::PaintStyle::Stroke);
    p.set_stroke_width(width);
    p
}

pub fn linear(
    p0: (f32, f32),
    p1: (f32, f32),
    colors: &[Rgba],
    pos: Option<&[f32]>,
) -> Option<vg::Shader> {
    linear_tiled(p0, p1, colors, pos, vg::TileMode::Clamp)
}

/// Linear gradient with an explicit tile mode (e.g. `Repeat` for stripes).
pub fn linear_tiled(
    p0: (f32, f32),
    p1: (f32, f32),
    colors: &[Rgba],
    pos: Option<&[f32]>,
    tile: vg::TileMode,
) -> Option<vg::Shader> {
    let c: Vec<vg::Color4f> = colors.iter().map(|c| c.c4f()).collect();
    let g = Gradient::new(Colors::new(&c, pos, tile, None), Interpolation::default());
    shaders::linear_gradient(
        (vg::Point::new(p0.0, p0.1), vg::Point::new(p1.0, p1.1)),
        &g,
        None,
    )
}

pub fn radial(
    center: (f32, f32),
    radius: f32,
    colors: &[Rgba],
    pos: Option<&[f32]>,
) -> Option<vg::Shader> {
    let c: Vec<vg::Color4f> = colors.iter().map(|c| c.c4f()).collect();
    let g = Gradient::new(
        Colors::new(&c, pos, vg::TileMode::Clamp, None),
        Interpolation::default(),
    );
    shaders::radial_gradient(
        (vg::Point::new(center.0, center.1), radius.max(0.01)),
        &g,
        None,
    )
}

/// Paint filled with a vertical linear gradient spanning `rect`.
pub fn vgrad(rect: vg::Rect, top: Rgba, bottom: Rgba) -> vg::Paint {
    let mut p = vg::Paint::default();
    p.set_anti_alias(true);
    p.set_dither(dither_on());
    p.set_shader(linear(
        (rect.left, rect.top),
        (rect.left, rect.bottom),
        &[top, bottom],
        None,
    ));
    p
}

pub fn rrect(rect: vg::Rect, radius: f32) -> vg::RRect {
    vg::RRect::new_rect_xy(rect, radius, radius)
}

/// Blur sigma from a CSS-style blur radius.
fn sigma(blur: f32) -> f32 {
    (blur * 0.5).max(0.01)
}

/// Soft outer shadow / glow behind a rounded rect.
///
/// Drawn from a cached, pre-blurred alpha mask (see [`ShadowMasks`]) tinted by
/// `color`, instead of a blur mask filter: Skia's analytic GPU blur only
/// applies when the rect is larger than `2 * (radius + 3 sigma)`, so small
/// clips fell back to CPU mask rasterization every frame.
pub fn drop_shadow(
    canvas: &vg::Canvas,
    rr: &vg::RRect,
    dx: f32,
    dy: f32,
    blur: f32,
    spread: f32,
    color: Rgba,
) {
    if color.a <= 0.001 {
        return;
    }
    let mut r = *rr;
    r.offset((dx, dy));
    r.outset((spread, spread));
    if !ShadowMasks::draw(canvas, &r, sigma(blur), color) {
        // Non-uniform radii: fall back to a live blur.
        let mut p = fill(color);
        p.set_mask_filter(vg::MaskFilter::blur(
            vg::BlurStyle::Normal,
            sigma(blur),
            false,
        ));
        canvas.draw_rrect(r, &p);
    }
}

/// Cache of pre-blurred rounded-rect alpha masks.
///
/// Rects at least `2 * (radius + margin) + 1` wide/high share one nine-patch
/// mask per (radius, sigma): the corners are drawn 1:1 and a 1 px centre
/// row/column is stretched, which reproduces the blur exactly. Smaller rects
/// get a mask of their own (quantized to 1/4 px) so they stay exact too.
pub struct ShadowMasks;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct MaskKey {
    /// Size in quarter pixels; (0, 0) for the shared nine-patch mask.
    w4: i32,
    h4: i32,
    radius4: i32,
    sigma4: i32,
}

/// Upper bound on cached masks; the cache is simply cleared when exceeded.
const MAX_SHADOW_MASKS: usize = 1024;

thread_local! {
    static SHADOW_MASKS: RefCell<std::collections::HashMap<MaskKey, Option<vg::Image>>> =
        RefCell::new(std::collections::HashMap::new());
}

impl ShadowMasks {
    /// Blur extent beyond the rect, in device px.
    fn margin(sigma: f32) -> f32 {
        (3.0 * sigma).ceil() + 1.0
    }

    /// Renders a blurred rrect of size `w` x `h` centred in an A8 image with `margin` padding.
    fn render(w: f32, h: f32, radius: f32, sigma: f32) -> Option<vg::Image> {
        let m = Self::margin(sigma);
        let iw = (w + 2.0 * m).ceil() as i32;
        let ih = (h + 2.0 * m).ceil() as i32;
        let mut surface = vg::surfaces::raster(&vg::ImageInfo::new_a8((iw, ih)), None, None)?;
        let mut p = vg::Paint::default();
        p.set_anti_alias(true);
        p.set_color(vg::Color::WHITE);
        p.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, sigma, false));
        surface
            .canvas()
            .draw_rrect(rrect(vg::Rect::from_xywh(m, m, w, h), radius), &p);
        Some(surface.image_snapshot())
    }

    fn get(key: MaskKey, make: impl FnOnce() -> Option<vg::Image>) -> Option<vg::Image> {
        SHADOW_MASKS.with(|c| {
            let mut c = c.borrow_mut();
            if c.len() > MAX_SHADOW_MASKS {
                c.clear();
            }
            c.entry(key).or_insert_with(make).clone()
        })
    }

    /// Draws the blurred `rr` tinted with `color`. Returns false if `rr` has non-uniform radii.
    pub fn draw(canvas: &vg::Canvas, rr: &vg::RRect, sigma: f32, color: Rgba) -> bool {
        let radius = rr.radii(vg::rrect::Corner::UpperLeft).x;
        if rr
            .radii_ref()
            .iter()
            .any(|v| v.x != radius || v.y != radius)
        {
            return false;
        }
        let q = |v: f32| (v * 4.0).round() as i32;
        let (sigma, radius) = (q(sigma) as f32 / 4.0, q(radius) as f32 / 4.0);
        let m = Self::margin(sigma);
        let r = *rr.rect();
        let mut paint = fill(color);
        paint.set_anti_alias(false);

        let corner = (radius + m).ceil();
        let patch = 2.0 * corner + 1.0;
        if r.width() >= patch && r.height() >= patch {
            let key = MaskKey {
                w4: 0,
                h4: 0,
                radius4: q(radius),
                sigma4: q(sigma),
            };
            let Some(img) = Self::get(key, || Self::render(patch, patch, radius, sigma)) else {
                return false;
            };
            // Centre pixel of the mask: corner zones are margin + corner wide.
            let c = (m + corner) as i32;
            let center = vg::IRect::from_xywh(c, c, 1, 1);
            canvas.draw_image_nine(
                &img,
                center,
                r.with_outset((m, m)),
                vg::FilterMode::Linear,
                Some(&paint),
            );
        } else {
            let (w, h) = (q(r.width()) as f32 / 4.0, q(r.height()) as f32 / 4.0);
            let key = MaskKey {
                w4: q(w),
                h4: q(h),
                radius4: q(radius),
                sigma4: q(sigma),
            };
            let Some(img) = Self::get(key, || Self::render(w, h, radius, sigma)) else {
                return false;
            };
            let dst = vg::Rect::from_xywh(
                r.left - m,
                r.top - m,
                img.width() as f32,
                img.height() as f32,
            );
            let sampling = vg::SamplingOptions::from(vg::FilterMode::Linear);
            canvas.draw_image_rect_with_sampling_options(&img, None, dst, sampling, &paint);
        }
        true
    }
}

/// Inner shadow along the edge(s) the light comes from: gradient strips
/// clipped to the rounded rect. Visually equivalent to a blurred inset frame
/// at these sizes, but avoids Skia's CPU mask blur for non-rect shapes, which
/// was measurable at 200 clips (see FINDINGS.md).
pub fn inner_shadow(canvas: &vg::Canvas, rr: &vg::RRect, dx: f32, dy: f32, blur: f32, color: Rgba) {
    if color.a <= 0.001 {
        return;
    }
    let r = *rr.rect();
    let clear = color.with_alpha(0.0);
    canvas.save();
    canvas.clip_rrect(rr, vg::ClipOp::Intersect, true);
    let strip = |from: (f32, f32), to: (f32, f32), rect: vg::Rect| {
        let mut p = vg::Paint::default();
        p.set_anti_alias(true);
        p.set_dither(dither_on());
        p.set_shader(linear(from, to, &[color, clear], None));
        canvas.draw_rect(rect, &p);
    };
    let dv = dy.abs() + blur;
    if dy > 0.0 {
        strip(
            (r.left, r.top),
            (r.left, r.top + dv),
            vg::Rect::from_ltrb(r.left, r.top, r.right, r.top + dv),
        );
    } else if dy < 0.0 {
        strip(
            (r.left, r.bottom),
            (r.left, r.bottom - dv),
            vg::Rect::from_ltrb(r.left, r.bottom - dv, r.right, r.bottom),
        );
    }
    let dh = dx.abs() + blur;
    if dx > 0.0 {
        strip(
            (r.left, r.top),
            (r.left + dh, r.top),
            vg::Rect::from_ltrb(r.left, r.top, r.left + dh, r.bottom),
        );
    } else if dx < 0.0 {
        strip(
            (r.right, r.top),
            (r.right - dh, r.top),
            vg::Rect::from_ltrb(r.right - dh, r.top, r.right, r.bottom),
        );
    }
    canvas.restore();
}

/// A 1px highlight that follows only the top edge of a rounded rect, fading out
/// down the corners: the "lit from above" line.
pub fn top_highlight(canvas: &vg::Canvas, rr: &vg::RRect, width: f32, color: Rgba) {
    let r = rr.rect();
    canvas.save();
    canvas.clip_rrect(rr, vg::ClipOp::Intersect, true);
    let mut inner = *rr;
    inner.offset((0.0, width));
    let mut p = vg::Paint::default();
    p.set_anti_alias(true);
    let rad = rr.radii(vg::rrect::Corner::UpperLeft).y.max(1.0);
    p.set_dither(dither_on());
    p.set_shader(linear(
        (r.left, r.top),
        (r.left, r.top + rad + width),
        &[color, color.with_alpha(0.0)],
        Some(&[0.0, 1.0]),
    ));
    canvas.draw_drrect(*rr, inner, &p);
    canvas.restore();
}

/// A 1px inner border (stroke inset by half its width so it stays inside).
pub fn inner_border(canvas: &vg::Canvas, rr: &vg::RRect, width: f32, color: Rgba) {
    let mut r = *rr;
    r.inset((width * 0.5, width * 0.5));
    canvas.draw_rrect(r, &stroke(color, width));
}

// ───────────────────────── Gradient strokes ─────────────────────────
//
// The prototype's surfaces are defined by their 1 px strokes more than by
// their fills: keys and pills carry a sheen along the top edge that is
// brightest in the middle and fades into the plain stroke at the corners;
// colour strips have a bright left edge and a dark right edge. All strokes
// are drawn *inside* the shape as a ring (`draw_drrect`), so they stay crisp
// on whole device pixels at every scale.

/// A logical 1 px hairline in device px: whole pixels at every scale, so
/// strokes stay crisp at 150% (2 px) instead of smearing over 1.5 px.
pub fn hairline(scale: f32) -> f32 {
    scale.round().max(1.0)
}

/// `r` with every edge on a whole device pixel (view bounds can be
/// fractional at 150%).
pub fn snap(r: vg::Rect) -> vg::Rect {
    vg::Rect::from_ltrb(r.left.round(), r.top.round(), r.right.round(), r.bottom.round())
}

/// The `width`-wide ring just inside `rr`.
fn ring_inner(rr: &vg::RRect, width: f32) -> vg::RRect {
    let mut inner = *rr;
    inner.inset((width, width));
    inner
}

/// A 1 px (×`width`) stroke inside `rr`, painted with `paint` (any shader).
pub fn ring(canvas: &vg::Canvas, rr: &vg::RRect, width: f32, paint: &vg::Paint) {
    canvas.draw_drrect(*rr, ring_inner(rr, width), paint);
}

/// A stroke inside `rr` whose colour runs from `top` to `bottom`.
pub fn ring_vgrad(canvas: &vg::Canvas, rr: &vg::RRect, width: f32, top: Rgba, bottom: Rgba) {
    let r = rr.rect();
    ring(canvas, rr, width, &vgrad(*r, top, bottom));
}

/// A stroke inside `rr` whose colour runs from `left` to `right`.
pub fn ring_hgrad(canvas: &vg::Canvas, rr: &vg::RRect, width: f32, left: Rgba, right: Rgba) {
    let r = rr.rect();
    let mut p = vg::Paint::default();
    p.set_anti_alias(true);
    p.set_dither(dither_on());
    p.set_shader(linear((r.left, r.top), (r.right, r.top), &[left, right], None));
    ring(canvas, rr, width, &p);
}

/// The top-edge sheen of keys and pills: a `width` line along the straight
/// top edge of `rr`, `peak` in the middle and fading to `edge` towards both
/// corners. Drawn over the plain stroke.
pub fn top_sheen(canvas: &vg::Canvas, rr: &vg::RRect, width: f32, edge: Rgba, peak: Rgba) {
    let r = *rr.rect();
    let rad = rr.radii(vg::rrect::Corner::UpperLeft).x;
    let mut p = vg::Paint::default();
    p.set_anti_alias(true);
    p.set_dither(dither_on());
    p.set_shader(linear(
        (r.left + rad * 0.5, r.top),
        (r.right - rad * 0.5, r.top),
        &[edge, peak, edge],
        Some(&[0.0, 0.5, 1.0]),
    ));
    canvas.save();
    canvas.clip_rect(
        vg::Rect::from_ltrb(r.left, r.top, r.right, r.top + width),
        vg::ClipOp::Intersect,
        true,
    );
    ring(canvas, rr, width, &p);
    canvas.restore();
}

/// Text painted with a vertical gradient between `top` and `bottom`
/// (spanning the font's cap height above `baseline`).
pub fn draw_text_vgrad(
    canvas: &vg::Canvas,
    s: &str,
    x: f32,
    baseline: f32,
    font: &vg::Font,
    top: Rgba,
    bottom: Rgba,
) {
    let (_, m) = font.metrics();
    let cap = if m.cap_height > 0.0 { m.cap_height } else { -m.ascent * 0.7 };
    let mut p = vg::Paint::default();
    p.set_anti_alias(true);
    p.set_dither(dither_on());
    p.set_shader(linear((x, baseline - cap), (x, baseline), &[top, bottom], None));
    canvas.draw_str(s, (x, baseline), font, &p);
}

pub fn draw_text(
    canvas: &vg::Canvas,
    s: &str,
    x: f32,
    baseline: f32,
    font: &vg::Font,
    color: Rgba,
) {
    canvas.draw_str(s, (x, baseline), font, &fill(color));
}

/// Draw text, truncating with an ellipsis to fit `max_w`.
///
/// The fitted string is cached per (text, font size, width) so the
/// per-character measuring loop runs only when one of those changes.
pub fn draw_text_clipped(
    canvas: &vg::Canvas,
    s: &str,
    x: f32,
    baseline: f32,
    max_w: f32,
    font: &vg::Font,
    color: Rgba,
) {
    if max_w <= 4.0 {
        return;
    }
    let key = (s.to_owned(), (font.size() * 4.0) as u32, max_w as u32);
    let fitted = FITTED.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > MAX_FITTED_STRINGS {
            c.clear();
        }
        c.entry(key)
            .or_insert_with(|| fit_with_ellipsis(s, max_w.floor(), font))
            .clone()
    });
    if let Some(t) = fitted {
        draw_text(canvas, &t, x, baseline, font, color);
    }
}


/// Text wrapped at word breaks into at most `lines` lines inside `max_w`
/// (the last line ends in an ellipsis if the text goes on). `top` is the top
/// of the first line; lines are 1.35 × the font size apart.
#[allow(clippy::too_many_arguments)]
pub fn draw_text_wrapped(canvas: &vg::Canvas, s: &str, x: f32, top: f32, max_w: f32, font: &vg::Font, color: Rgba, lines: usize) {
    let lh = font.size() * 1.35;
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut rest = s.split_whitespace().peekable();
    while let Some(w) = rest.next() {
        let try_line = if cur.is_empty() { w.to_string() } else { format!("{cur} {w}") };
        if text_width(font, &try_line) <= max_w || cur.is_empty() {
            cur = try_line;
        } else {
            out.push(std::mem::take(&mut cur));
            cur = w.to_string();
            if out.len() + 1 == lines {
                // The last line takes everything left (clipped below).
                for w in rest.by_ref() {
                    cur.push(' ');
                    cur.push_str(w);
                }
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    for (i, l) in out.iter().take(lines).enumerate() {
        draw_text_clipped(canvas, l, x, (top + font.size() + i as f32 * lh).round(), max_w, font, color);
    }
}

const MAX_FITTED_STRINGS: usize = 4096;

/// (text, font size in 1/4 px, max width in px) -> fitted text, or None if nothing fits.
type FittedCache = std::collections::HashMap<(String, u32, u32), Option<std::rc::Rc<str>>>;

thread_local! {
    static FITTED: RefCell<FittedCache> =
        RefCell::new(std::collections::HashMap::new());
}

fn fit_with_ellipsis(s: &str, max_w: f32, font: &vg::Font) -> Option<std::rc::Rc<str>> {
    if text_width(font, s) <= max_w {
        return Some(s.into());
    }
    let ell = "…";
    let ell_w = text_width(font, ell);
    let mut end = s.len();
    while end > 0 {
        end = s[..end].char_indices().last().map(|(i, _)| i).unwrap_or(0);
        if text_width(font, &s[..end]) + ell_w <= max_w {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    Some(format!("{}{}", &s[..end], ell).into())
}

/// Time since the previous draw for view-local easing. After an idle gap
/// (no redraws) assume one nominal frame so animations start smoothly.
pub fn frame_dt(last: &std::cell::Cell<Option<vizia::prelude::Instant>>) -> f32 {
    const NOMINAL: f32 = 1.0 / 60.0;
    const IDLE_GAP: f32 = 0.1;
    let now = vizia::prelude::Instant::now();
    let gap = last
        .get()
        .map(|t| (now - t).as_secs_f32())
        .unwrap_or(NOMINAL);
    last.set(Some(now));
    if gap > IDLE_GAP {
        NOMINAL
    } else {
        gap
    }
}

/// Exponential approach towards a target; frame-rate independent.
pub fn approach(cur: f32, target: f32, dt: f32, tau: f32) -> f32 {
    cur + (target - cur) * (1.0 - (-dt / tau).exp())
}

// ───────────────────────── Option D materials ─────────────────────────
//
// docs/design/VISUAL-SPEC.md §1. The mockup draws its keys as a CSS box: a
// 1 px #383838 border over a #161616 → #424242 body gradient that spans the
// border box, and a 1 px sheen *inside* the border (the second pixel row),
// clipped by the inner rounded corners. The wells are flat #151515 under a
// 1 px #323232 border with a faint inner bottom highlight and an outer one.

/// The key sheen's stops: corner, centre, corner (horizontal).
pub const SHEEN_STOPS: [u32; 3] = [0x3E3E3E, 0x999999, 0x3A3A3A];

/// How a key surface is drawn: `lift` brightens it (hover), `pressed`
/// darkens and flattens it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KeyLook {
    pub lift: f32,
    pub pressed: bool,
}

/// The key material: body gradient, 1 px border, inner top sheen.
pub fn key_material(canvas: &vg::Canvas, r: vg::Rect, radius: f32, s: f32, look: KeyLook) {
    use crate::theme::tokens::{KEY_BOTTOM, KEY_STROKE, KEY_TOP};
    let rr = rrect(r, radius);
    let (top, bottom) = if look.pressed {
        (KEY_TOP.darken(0.25), KEY_BOTTOM.darken(0.3))
    } else {
        (KEY_TOP.lighten(look.lift), KEY_BOTTOM.lighten(look.lift))
    };
    canvas.draw_rrect(rr, &vgrad(r, top, bottom));
    let hl = hairline(s);
    ring(canvas, &rr, hl, &fill(KEY_STROKE));
    // The sheen: one hairline under the top border, inside the inner corners.
    let inner = ring_inner(&rr, hl);
    let ir = *inner.rect();
    let line = vg::Rect::from_ltrb(ir.left, ir.top, ir.right, ir.top + hl);
    let [a, b, c] = SHEEN_STOPS.map(Rgba::hex);
    let k = if look.pressed { -0.5 } else { look.lift };
    let tone = |x: Rgba| if k < 0.0 { x.darken(-k) } else { x.lighten(k) };
    canvas.save();
    canvas.clip_rrect(inner, vg::ClipOp::Intersect, true);
    canvas.draw_rect(
        line,
        &shaded(linear((ir.left, ir.top), (ir.right, ir.top), &[tone(a), tone(b), tone(c)], Some(&[0.0, 0.5, 1.0]))),
    );
    canvas.restore();
}

/// A recessed well (the display, the spectrum): fill, border, the inner
/// bottom highlight (`inset 0 -1px 0 rgba(255,255,255,.07)`) and the outer
/// one below it (`0 1px 0 rgba(255,255,255,.04)`).
pub fn well(canvas: &vg::Canvas, r: vg::Rect, radius: f32, s: f32) {
    use crate::theme::tokens::{BG_2, STROKE};
    let hl = hairline(s);
    let rr = rrect(r, radius);
    // Outer highlight: the box shape moved down 1 px, outside the box.
    let mut below = rr;
    below.offset((0.0, hl));
    canvas.save();
    canvas.clip_rrect(rr, vg::ClipOp::Difference, true);
    canvas.draw_rrect(below, &fill(Rgba::white(0.04)));
    canvas.restore();
    canvas.draw_rrect(rr, &fill(BG_2));
    // Inner highlight: inside the border, the padding box's bottom edge.
    let inner = ring_inner(&rr, hl);
    let mut up = inner;
    up.offset((0.0, -hl));
    canvas.save();
    canvas.clip_rrect(inner, vg::ClipOp::Intersect, true);
    canvas.draw_drrect(inner, up, &fill(Rgba::white(0.07)));
    canvas.restore();
    ring(canvas, &rr, hl, &fill(STROKE));
}

/// A CSS inset box shadow (`inset dx dy blur spread color`) inside `rr`:
/// everything outside the hole (`rr` grown by -spread and moved by dx, dy),
/// blurred by blur/2 (CSS blur radius → Gaussian sigma), clipped to `rr`.
pub fn css_inset_shadow(canvas: &vg::Canvas, rr: &vg::RRect, dx: f32, dy: f32, blur: f32, spread: f32, color: Rgba) {
    let r = *rr.rect();
    let mut hole = *rr;
    hole.outset((-spread, -spread));
    hole.offset((dx, dy));
    let pad = blur * 2.0 + spread.abs() + dx.abs() + dy.abs() + 4.0;
    let outer = vg::RRect::new_rect(r.with_outset((pad, pad)));
    let mut p = fill(color);
    if blur > 0.0 {
        p.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, blur * 0.5, false));
    }
    canvas.save();
    canvas.clip_rrect(rr, vg::ClipOp::Intersect, true);
    canvas.draw_drrect(outer, hole, &p);
    canvas.restore();
}

/// Draws `text` with CSS letter-spacing (`tracking` device px after every
/// character) and returns the advance including the trailing spacing, as
/// CSS lays it out.
pub fn draw_text_tracked(canvas: &vg::Canvas, text: &str, x: f32, base: f32, font: &vg::Font, tracking: f32, color: Rgba) -> f32 {
    let p = fill(color);
    let mut cx = x;
    let mut buf = [0u8; 4];
    for ch in text.chars() {
        let g = ch.encode_utf8(&mut buf);
        canvas.draw_str(&*g, (cx, base), font, &p);
        cx += text_width(font, g) + tracking;
    }
    cx - x
}

/// The advance of `text` with CSS letter-spacing (see [`draw_text_tracked`]).
pub fn tracked_width(text: &str, font: &vg::Font, tracking: f32) -> f32 {
    let mut buf = [0u8; 4];
    text.chars().map(|ch| text_width(font, ch.encode_utf8(&mut buf)) + tracking).sum()
}

/// The baseline CSS gives a line box of `line_h` starting at `top`:
/// half-leading around the font's ascent + descent.
pub fn css_baseline(font: &vg::Font, top: f32, line_h: f32) -> f32 {
    let (_, m) = font.metrics();
    let (asc, desc) = (-m.ascent, m.descent);
    top + (line_h - (asc + desc)) * 0.5 + asc
}

/// An SVG path (the mockup's icon data) placed at `origin` and scaled to
/// device px.
pub fn svg_path(d: &str, origin: (f32, f32), s: f32) -> vg::Path {
    let p = vg::Path::from_svg(d).unwrap_or_default();
    p.make_transform(&vg::Matrix::scale_translate((s, s), origin))
}
