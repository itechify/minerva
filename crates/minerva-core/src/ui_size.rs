//! The game's UI Size (ADR 0020): how large the game draws its UI next to the samples. A
//! reader measures it by a part of the UI the game draws at a known size, scales what it
//! found to the samples' size, and reads that as it stands.

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

/// A capture at the samples' UI Size, and the capture it was scaled from.
pub(crate) struct AtSamplesSize<'a> {
    pub img: Cow<'a, RgbImage>,
    captured: &'a RgbImage,
}

impl<'a> AtSamplesSize<'a> {
    /// `captured`, drawn at `ui_size`, at the samples' size: as it is, if it is theirs
    /// already. The filter is Catmull-Rom (ADR 0020): its weights add up to one, so the UI's
    /// flat colours stay flat, and its edges stay sharp enough for the readers' tolerances.
    pub fn new(captured: &'a RgbImage, ui_size: f32) -> Self {
        let img = if (ui_size - 1.0).abs() <= SAMPLES_SIZE_WITHIN {
            Cow::Borrowed(captured)
        } else {
            let (w, h) = captured.dimensions();
            let at = |px: u32| ((px as f32 / ui_size).round() as u32).max(1);
            Cow::Owned(image::imageops::resize(captured, at(w), at(h), FilterType::CatmullRom))
        };
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
    let (max, min) = (p.iter().max().copied().unwrap_or(0), p.iter().min().copied().unwrap_or(0));
    max <= POINTER_BLACK && max - min <= POINTER_TINT
}
