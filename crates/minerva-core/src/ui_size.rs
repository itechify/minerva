//! The game's UI Size (ADR 0020): how large the game draws its UI next to the samples. A
//! reader measures it by a part of the UI the game draws at a known size, scales what it
//! found to the samples' size, and reads that as it stands.

use image::RgbImage;
use image::imageops::FilterType;

/// A UI Size this close to the samples' is theirs: read as it is, with no resampling to blur
/// it. The game's other sizes are further off (×0.9 is the nearest seen, Default Ratio at
/// 1080 rows).
const SAMPLES_SIZE_WITHIN: f32 = 0.04;

/// `img`, drawn at `ui_size`, at the samples' size instead: `None` if it is theirs already.
/// The filter is Catmull-Rom (ADR 0020): its weights add up to one, so the UI's flat colours
/// stay flat, and its edges stay sharp enough for the readers' tolerances.
pub(crate) fn to_samples_size(img: &RgbImage, ui_size: f32) -> Option<RgbImage> {
    if (ui_size - 1.0).abs() <= SAMPLES_SIZE_WITHIN {
        return None;
    }
    let (w, h) = img.dimensions();
    let at = |px: u32| ((px as f32 / ui_size).round() as u32).max(1);
    Some(image::imageops::resize(img, at(w), at(h), FilterType::CatmullRom))
}
