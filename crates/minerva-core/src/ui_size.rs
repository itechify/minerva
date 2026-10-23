//! The game's UI Size (ADR 0020): how large the game draws its UI next to the samples. A
//! reader measures it by a part of the UI the game draws at a known size, scales what it
//! found to the samples' size, and reads that as it stands.

use crate::ink::tint;
use image::RgbImage;
use image::imageops::FilterType;
use std::borrow::Cow;

/// A UI Size this close to the samples' is theirs: read as it is, with no resampling to blur
/// it. The game's other sizes are further off (×0.9 is the nearest seen, Default Ratio at
/// 1080 rows).
const SAMPLES_SIZE_WITHIN: f32 = 0.04;

/// The pointer's outline: black, with no tint, no channel above this, two pixels a row of it.
/// The game draws no such black on its slate, but for its filter (Default Ratio (Filter
/// Applied)), which sharpens white text into a pixel or two of it, and the black outline of
/// the "EXP." label. The LV box's grey is 28 at its darkest.
const POINTER_BLACK: u8 = 16;
const POINTER_TINT: u8 = 6;
/// The pointer is over a line with at least this many pixels of its black in it.
const POINTER_OUTLINE: usize = 4;

/// `img`, drawn at `ui_size`, scaled to the samples' size with Catmull-Rom (ADR 0020): its
/// weights add up to one, so the UI's flat colours stay flat, and its edges stay sharp enough
/// for the readers' tolerances. Each pixel is kept within its neighbours in `img`.
pub(crate) fn to_samples_size(img: &RgbImage, ui_size: f32) -> RgbImage {
    let at = |px: u32| ((px as f32 / ui_size).round() as u32).max(1);
    let mut scaled = image::imageops::resize(img, at(img.width()), at(img.height()), FilterType::CatmullRom);
    within_neighbours(&mut scaled, img);
    scaled
}

/// Keeps each pixel of `scaled`, a scaling of `source`, between the darkest and the brightest of
/// the four pixels of `source` it lies between, channel by channel. Catmull-Rom overshoots
/// beside a sharp edge: brighter than either side (by a Player Shop band, beside black text, a
/// pixel near white would read as the pointer's white over the listing), or darker (a near-
/// black fringe by the Status Bar's tiles and text).
fn within_neighbours(scaled: &mut RgbImage, source: &RgbImage) {
    let (sw, sh) = source.dimensions();
    let (rx, ry) = (sw as f32 / scaled.width() as f32, sh as f32 / scaled.height() as f32);
    let around = |at: u32, ratio: f32, size: u32| {
        let from = ((at as f32 + 0.5) * ratio - 0.5).floor().clamp(0.0, (size - 1) as f32) as u32;
        (from, (from + 1).min(size - 1))
    };
    for (x, y, p) in scaled.enumerate_pixels_mut() {
        let ((xa, xb), (ya, yb)) = (around(x, rx, sw), around(y, ry, sh));
        let near = [source.get_pixel(xa, ya), source.get_pixel(xb, ya), source.get_pixel(xa, yb), source.get_pixel(xb, yb)];
        for c in 0..3 {
            let (lo, hi) = near.iter().fold((u8::MAX, u8::MIN), |(lo, hi), q| (lo.min(q[c]), hi.max(q[c])));
            p[c] = p[c].clamp(lo, hi);
        }
    }
}

/// A capture at the samples' UI Size, and the capture it was scaled from.
pub(crate) struct AtSamplesSize<'a> {
    pub img: Cow<'a, RgbImage>,
    captured: &'a RgbImage,
}

impl<'a> AtSamplesSize<'a> {
    /// `captured`, drawn at `ui_size`, at the samples' size ([`to_samples_size`]): as it is, if
    /// it is theirs already.
    pub fn new(captured: &'a RgbImage, ui_size: f32) -> Self {
        let near = (ui_size - 1.0).abs() <= SAMPLES_SIZE_WITHIN;
        let img = if near { Cow::Borrowed(captured) } else { Cow::Owned(to_samples_size(captured, ui_size)) };
        Self { img, captured }
    }

    /// Where column `x` and row `y` of the scaled capture are in the capture.
    pub fn back(&self, x: f32, y: f32) -> (f32, f32) {
        let ((w, h), (sw, sh)) = (self.captured.dimensions(), self.img.dimensions());
        (x * w as f32 / sw as f32, y * h as f32 / sh as f32)
    }

    /// Whether the pointer is over columns `x0..x1` and rows `y0..y1` of the scaled capture:
    /// its black outline, in the capture as it was captured. Scaled down, the outline blurs
    /// into the white inside it, and looks like text.
    pub fn pointer_over(&self, (x0, x1): (u32, u32), (y0, y1): (u32, u32)) -> bool {
        let (w, h) = self.captured.dimensions();
        let (cx0, cy0) = self.back(x0 as f32, y0 as f32);
        let (cx1, cy1) = self.back(x1 as f32, y1 as f32);
        let (cx0, cy0) = (cx0.floor() as u32, cy0.floor() as u32);
        let (cx1, cy1) = ((cx1.ceil() as u32).min(w), (cy1.ceil() as u32).min(h));
        let pixels = (cy0..cy1).flat_map(|y| (cx0..cx1).map(move |x| (x, y)));
        pixels.filter(|&(x, y)| pointer_black(self.captured.get_pixel(x, y).0)).count() >= POINTER_OUTLINE
    }
}

/// Whether `p` is the pointer's outline: black, with no tint.
pub(crate) fn pointer_black(p: [u8; 3]) -> bool {
    p.into_iter().max().unwrap_or(0) <= POINTER_BLACK && tint(p) <= POINTER_TINT
}
