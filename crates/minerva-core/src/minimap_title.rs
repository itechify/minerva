//! Finds the Minimap Title (ADR 0021) in a capture of the game's client area, or of the part of
//! it where the title was last found, wherever the player put the minimap and at any UI Size
//! (ADR 0020).
//!
//! 1. The minimap window has an orange WORLD button at its top right, open or collapsed, and
//!    on a map without a minimap: [`WORLD_W`] by [`WORLD_H`] px of (240,167,21) at the samples'
//!    UI Size. It is where rows of that orange, as wide as one another, stack as tall as the
//!    button is for its width; or less tall at the capture's top, where a capture of the part
//!    of the client area the title was in may cut it off. Its width is the UI Size.
//! 2. The title is read around it, scaled to the samples' size. The open minimap shows it on
//!    two lines, on its light-blue panel below the button (`prep::prepare`); a collapsed one, or
//!    a map without one, on one line, "Street : Map", on the grey bar left of the button and
//!    its two small ones (`prep::prepare_one_line`). Where there is no panel, the bar is read.
//! 3. Where the title was found is the minimap window's top, from its left edge, as wide and
//!    as tall as either layout is for the longest names: what the shell captures next, while
//!    the title is there.

use crate::prep::{Prepared, prepare, prepare_one_line};
use crate::ui_size::AtSamplesSize;
use image::{GenericImageView, RgbImage};
use std::ops::RangeInclusive;

/// The WORLD button's orange is this wide and tall at the samples' UI Size...
const WORLD_W: f32 = 52.5;
const WORLD_H: f32 = 15.0;
/// ...and this wide at any (×0.4 to ×2.4 of the samples', past the largest the game draws)...
const ANY_WORLD_W: RangeInclusive<u32> = 21..=126;
/// ...and as tall as its width makes it, give or take this share.
const WORLD_H_WITHIN: f32 = 0.25;
/// A row of the button has at least this share of its columns orange, or the cream of its
/// letters "WORLD" ([`warm`]).
const WORLD_ROW_ORANGE: f32 = 0.5;
/// What is read around the button, at the samples' UI Size: from this far left of its right
/// edge to this far right of it, and from this far above its bottom to this far below. The
/// widest minimap (the longest names, on one line) is about 650 px; the open minimap's panel
/// ends 72 rows below the button.
const READ_LEFT: f32 = 700.0;
const READ_RIGHT: f32 = 12.0;
const READ_ABOVE: f32 = 26.0;
const READ_BELOW: f32 = 84.0;
/// The one-line layout, from the button's bottom: the text's rows, and a clear row of the bar
/// above them and one below (the descenders reach 3 rows below the button)...
const BAR_TEXT_ROWS: (i32, i32) = (-17, 4);
const BAR_CLEAR_ROWS: [i32; 2] = [-18, 4];
/// ...and the text ends this far left of the button, where the two small buttons are.
const BAR_BUTTONS: u32 = 47;
/// Where the title was found reaches from this far left of the minimap window's left edge, as
/// wide as the widest minimap, with room to spare.
const FOUND_LEFT: f32 = 8.0;
const FOUND_W: f32 = 720.0;

/// The WORLD button's orange, at any brightness: (240,167,21), lighter at its top (255,190,51).
/// Not the Status Bar's level tiles (255,148,21).
fn orange(p: [u8; 3]) -> bool {
    // Whole numbers: every pixel of a capture is tested.
    let [r, g, b] = p.map(u32::from);
    r >= 150 && (60 * r..=84 * r).contains(&(100 * g)) && 100 * b <= 36 * r
}

/// The button's orange, or the cream of its letters (241,217,184) and the light orange they
/// blend into it with, at any brightness.
fn warm(p: [u8; 3]) -> bool {
    let [r, g, b] = p;
    r >= 150 && r >= g && g >= b && r - b >= 30
}

/// A WORLD button found: its orange's columns and rows, ends exclusive.
#[derive(Debug, Clone, Copy)]
struct Button {
    x0: u32,
    x1: u32,
    top: u32,
    bottom: u32,
}

impl Button {
    /// How large the game draws the minimap next to the samples.
    fn ui_size(&self) -> f32 {
        (self.x1 - self.x0) as f32 / WORLD_W
    }
}

/// The WORLD buttons in `img`, top to bottom: stacks of rows of orange over the same columns,
/// as tall as their width makes them, or less tall at the image's top.
fn buttons(img: &RgbImage) -> Vec<Button> {
    let (w, h) = img.dimensions();
    let raw = img.as_raw();
    let at = |x: u32, y: u32| {
        let i = 3 * (y as usize * w as usize + x as usize);
        [raw[i], raw[i + 1], raw[i + 2]]
    };
    let is = |x: u32, y: u32| orange(at(x, y));
    let share = |y: u32, x0: u32, x1: u32| (x0..x1).filter(|&x| warm(at(x, y))).count() as f32 / (x1 - x0) as f32;
    let (mut found, mut seen) = (Vec::new(), Vec::<Button>::new());
    for y in 0..h {
        let mut x = 0;
        while x < w {
            if !is(x, y) {
                x += 1;
                continue;
            }
            let x0 = x;
            while x < w && is(x, y) {
                x += 1;
            }
            if !ANY_WORLD_W.contains(&(x - x0)) || seen.iter().any(|b| (b.top..b.bottom).contains(&y) && x0 < b.x1 && b.x0 < x) {
                continue;
            }
            let mut top = y;
            while top > 0 && share(top - 1, x0, x) >= WORLD_ROW_ORANGE {
                top -= 1;
            }
            let mut bottom = y + 1;
            while bottom < h && share(bottom, x0, x) >= WORLD_ROW_ORANGE {
                bottom += 1;
            }
            let button = Button { x0, x1: x, top, bottom };
            let (tall, want) = ((bottom - top) as f32, button.ui_size() * WORLD_H);
            let fits = (tall - want).abs() <= want * WORLD_H_WITHIN || top == 0 && tall < want;
            if fits {
                found.push(button);
            }
            seen.push(button);
        }
    }
    found
}

/// A Minimap Title found in a capture, ready for recognition.
pub(crate) struct Found {
    pub prepared: Prepared,
    /// Where the title is in the capture: the minimap window's top, from its left edge, as wide
    /// and as tall as either layout is for the longest names. It may reach past the capture.
    pub area: (i32, i32, u32, u32),
}

/// The Minimap Title in `img`, if a WORLD button there has either layout's title beside it.
pub(crate) fn find(img: &RgbImage) -> Option<Found> {
    buttons(img).into_iter().find_map(|button| around(img, button))
}

/// The title by `button`, read at the samples' UI Size.
fn around(img: &RgbImage, button: Button) -> Option<Found> {
    let size = button.ui_size();
    let (w, h) = img.dimensions();
    let px = |samples: f32| (samples * size).round() as i64;
    let (x0, x1) = ((button.x1 as i64 - px(READ_LEFT)).max(0), (button.x1 as i64 + px(READ_RIGHT)).min(w as i64));
    let (y0, y1) = ((button.bottom as i64 - px(READ_ABOVE)).max(0), (button.bottom as i64 + px(READ_BELOW)).min(h as i64));
    let crop = img.view(x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32).to_image();
    let at = AtSamplesSize::new(&crop, size);
    // The button, in the crop at the samples' size.
    let scaled = |v: u32, from: i64| ((v as i64 - from) as f32 / size).round() as i32;
    let (left, bottom) = (scaled(button.x0, x0), scaled(button.bottom, y0));
    let row = |dy: i32| (bottom + dy).max(0) as u32;
    let prepared = prepare(&at.img).or_else(|| {
        let right = (left - BAR_BUTTONS as i32).max(0) as u32;
        prepare_one_line(&at.img, (row(BAR_TEXT_ROWS.0), row(BAR_TEXT_ROWS.1)), right, BAR_CLEAR_ROWS.map(row))
    })?;
    let (window_left, _) = at.back(prepared.left as f32, 0.0);
    let area = (
        x0 as i32 + window_left.round() as i32 - px(FOUND_LEFT) as i32,
        button.bottom as i32 - px(READ_ABOVE) as i32,
        px(FOUND_W) as u32,
        px(READ_ABOVE + READ_BELOW) as u32,
    );
    Some(Found { prepared, area })
}
