//! Pixel arithmetic the readers share: how light or how neutral a pixel is, whether it is a
//! known colour at any brightness, the black-on-white canvases ocrs reads, and the runs and
//! connected components glyphs are found by.

use image::{GrayImage, Luma, RgbImage};
use std::ops::{Range, RangeInclusive};

/// How light `p` is, its channels weighed as Rec. 601 does.
pub(crate) fn luma(p: [u8; 3]) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

/// How far apart `p`'s channels are: 0 for a neutral grey.
pub(crate) fn tint(p: [u8; 3]) -> u8 {
    let (min, max) = p.into_iter().fold((u8::MAX, u8::MIN), |(lo, hi), c| (lo.min(c), hi.max(c)));
    max - min
}

/// Live GDI captures show the game's colours uniformly darker than screenshots do (the
/// minimap panel's (164,200,218) as (153,187,204), ≈ 0.935×), so a colour is matched at any
/// brightness in this range of the screenshots'.
pub(crate) const BRIGHTNESS: RangeInclusive<f32> = 0.8..=1.1;

/// `p`'s brightness next to `rgb` (a colour as screenshots show it), if `p` is that colour at
/// a brightness in [`BRIGHTNESS`]: its red and green within `tol` of the colour's at the
/// brightness its blue says.
pub(crate) fn at_brightness(p: [u8; 3], rgb: [f32; 3], tol: f32) -> Option<f32> {
    let k = p[2] as f32 / rgb[2];
    let close = BRIGHTNESS.contains(&k) && (0..2).all(|i| (p[i] as f32 - k * rgb[i]).abs() <= tol);
    close.then_some(k)
}

/// The digit ocrs read as `c`, where it mistakes the game's digits for letters and signs: O,
/// o and D for a 0, and l, I, | and i for a 1. Any other glyph as it is.
pub(crate) fn ocr_digit(c: char) -> char {
    match c {
        'O' | 'o' | 'D' => '0',
        'l' | 'I' | '|' | 'i' => '1',
        c => c,
    }
}

/// `w` by `h` of ink (0.0 background ..= 1.0 text), by `ink(x, y)`, as black text on white
/// with `pad` white px around it: what ocrs reads.
pub(crate) fn black_on_white(w: u32, h: u32, pad: u32, ink: impl Fn(u32, u32) -> f32) -> GrayImage {
    let mut canvas = GrayImage::from_pixel(w + 2 * pad, h + 2 * pad, Luma([255]));
    for y in 0..h {
        for x in 0..w {
            canvas.put_pixel(x + pad, y + pad, Luma([(255.0 * (1.0 - ink(x, y))).round() as u8]));
        }
    }
    canvas
}

/// The runs of `on` as (start, end_exclusive), joining runs fewer than `join_below` apart:
/// with 1, only consecutive ones.
pub(crate) fn runs(on: &[bool], join_below: usize) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (x, _) in on.iter().enumerate().filter(|(_, on)| **on) {
        let x = x as u32;
        match out.last_mut() {
            Some((_, end)) if (x - *end) < join_below as u32 => *end = x + 1,
            _ => out.push((x, x + 1)),
        }
    }
    out
}

/// A connected run of pixels of one colour: its bounding box (ends exclusive), how many
/// pixels it has, and the greatest weight among them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Component {
    pub x0: u32,
    pub x1: u32,
    pub y0: u32,
    pub y1: u32,
    pub pixels: u32,
    pub max: f32,
}

impl Component {
    pub fn w(&self) -> u32 {
        self.x1 - self.x0
    }

    pub fn h(&self) -> u32 {
        self.y1 - self.y0
    }
}

/// The 4-connected components of the pixels `weigh` gives a weight, in columns `xs` and rows
/// `ys` of `img` (cut to it), in the order their first pixels come, row by row.
pub(crate) fn components(
    img: &RgbImage,
    xs: Range<u32>,
    ys: Range<u32>,
    weigh: impl Fn([u8; 3]) -> Option<f32>,
) -> Vec<Component> {
    let (x0, x1) = (xs.start, xs.end.min(img.width()));
    let (y0, y1) = (ys.start, ys.end.min(img.height()));
    if x1 <= x0 || y1 <= y0 {
        return Vec::new();
    }
    let (sw, sh) = (x1 - x0, y1 - y0);
    let mut seen = vec![false; (sw * sh) as usize];
    let at = |x: u32, y: u32| ((y - y0) * sw + (x - x0)) as usize;
    let weight = |x: u32, y: u32| weigh(img.get_pixel(x, y).0);
    let mut out = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if seen[at(x, y)] {
                continue;
            }
            let Some(k) = weight(x, y) else { continue };
            let mut found = Component { x0: x, x1: x + 1, y0: y, y1: y + 1, pixels: 0, max: k };
            let mut stack = vec![(x, y, k)];
            seen[at(x, y)] = true;
            while let Some((cx, cy, k)) = stack.pop() {
                found = Component {
                    x0: found.x0.min(cx),
                    x1: found.x1.max(cx + 1),
                    y0: found.y0.min(cy),
                    y1: found.y1.max(cy + 1),
                    pixels: found.pixels + 1,
                    max: found.max.max(k),
                };
                let next = [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)];
                for (nx, ny) in next {
                    if (x0..x1).contains(&nx)
                        && (y0..y1).contains(&ny)
                        && !seen[at(nx, ny)]
                        && let Some(k) = weight(nx, ny)
                    {
                        seen[at(nx, ny)] = true;
                        stack.push((nx, ny, k));
                    }
                }
            }
            out.push(found);
        }
    }
    out
}
