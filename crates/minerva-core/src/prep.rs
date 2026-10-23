//! Title preprocessing (from the recogniser spike, issue #2), for the Minimap Title's two
//! layouts (ADR 0021). [`crate::minimap_title`] finds the title and hands it here at the
//! samples' UI Size.
//!
//! The two-line layout, the open minimap's:
//!
//! 1. Locate the title band: the rows at the top of the minimap where the flat
//!    light-blue panel colour (164,200,218) dominates. No band = minimap hidden or
//!    covered = unreadable.
//! 2. Trim horizontally where the panel ends (world graphics right of a narrow minimap
//!    are never read), and skip past the map icon.
//! 3. Colour mask: the title text is neutral light grey (~223,223,223, B-R ~ 0); the
//!    panel is B-R = 54 and the outline a darker blue (B-R 60..80). So "ink" = how
//!    neutral the pixel is, gated on luma. Rendered as black text on white.
//! 4. Split the two lines (street, map) with a horizontal projection profile.
//!
//! The one-line layout ("Street : Map", a collapsed minimap's or a map's without one) is dark
//! grey text on the bar's light grey: "ink" is how dark a neutral pixel is ([`prepare_one_line`]).

use image::{GrayImage, Luma, RgbImage};

/// Panel background colour (as in the sample screenshots) and per-channel tolerance.
/// Live GDI captures show the same colour uniformly darker, (153,187,204) ≈ 0.935×, so
/// the panel test matches the colour at any brightness within `PANEL_SCALE`.
const PANEL_RGB: [f32; 3] = [164.0, 200.0, 218.0];
const PANEL_TOL: f32 = 10.0;
const PANEL_SCALE: std::ops::RangeInclusive<f32> = 0.8..=1.1;
/// Search only this top-left area of a frame for the minimap.
const SEARCH_W: u32 = 900;
const SEARCH_H: u32 = 400;
/// The widest panel-colour row of the band has at least this many pixels.
const BAND_MIN_PANEL_PX: u32 = 120;
/// Every row of the title band has at least this many panel-colour pixels (rows
/// through a long name keep only ~80).
const BAND_ROW_MIN_PX: u32 = 30;
/// Rows ignored at the top and bottom of the band before masking.
const BAND_EDGE: u32 = 2;
/// Minimum height of the title band (rejects the 1-px rule above the header).
const BAND_MIN_H: u32 = 25;
/// "Very dark" (icon frame) luma, and how many such pixels make a column part of the icon.
const ICON_DARK_LUMA: f32 = 70.0;
const ICON_MIN_DARK_PX: u32 = 4;
/// The icon is within this many px of the panel's left edge.
const ICON_MAX_RIGHT: u32 = 120;
/// A column right of the icon with less total ink than this is the icon-to-text gap...
const ICON_GAP_MAX_INK: f32 = 0.05;
/// ...and the gap is at least this many consecutive such columns.
const ICON_GAP_COLS: u32 = 3;
/// Ink mask thresholds.
const INK_BR_HI: f32 = 48.0;
const INK_BR_SPAN: f32 = 40.0;
const INK_MIN_LUMA: f32 = 170.0;
/// A row counts as text if its summed ink exceeds this.
const LINE_MIN_INK: f32 = 1.5;
/// A column of a line counts as text if its summed ink exceeds this.
const COL_MIN_INK: f32 = 0.3;
/// Words are at least this many text-free columns apart (letters 1-2, words 4-6).
const WORD_GAP: u32 = 4;
/// A column is part of an upright stroke (the game font's "I": 2-3 px wide, 14 rows
/// tall on a 15-18-row line) when at least this share of the line's rows have at
/// least `STROKE_INK` ink.
const STROKE_ROWS: f32 = 0.65;
const STROKE_INK: f32 = 0.5;
/// The widest upright stroke, in columns.
const STROKE_MAX_W: u32 = 4;
/// "III": the most strokes a trailing numeral is read from.
const NUMERAL_MAX_STROKES: usize = 3;
/// The one-line layout: a neutral pixel has its channels at most this far apart...
const BAR_TINT: u8 = 16;
/// ...is ink from this luma down, wholly so this far below it...
const BAR_INK_FROM: f32 = 190.0;
const BAR_INK_SPAN: f32 = 110.0;
/// ...and is the bar's light grey from this luma up (215 on the samples).
const BAR_MIN_LUMA: f32 = 170.0;
/// The bar is at least this wide left of its buttons.
const BAR_MIN_W: u32 = 40;
/// White padding around the masked text canvas.
pub const PAD: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// A title ready for recognition: black-on-white ink canvas plus its line boxes in it
/// (canvas coordinates, tight ink bounds): the street line and the map line, or the one line
/// "Street : Map".
pub struct Prepared {
    pub canvas: GrayImage,
    pub lines: Vec<Rect>,
    /// Where the minimap window's left edge is in the image prepared: the panel's, or the
    /// bar's.
    pub left: u32,
    /// The map line ends in a Roman numeral I, II or III, counted from the mask.
    pub numeral: Option<Numeral>,
}

/// A map line's last word made of nothing but upright strokes: "I", "II" or "III".
/// ocrs can't read the game font's "II" and "III" (it gives an unknown glyph, or one
/// "I"), so they are counted instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Numeral {
    /// Where the word starts (canvas x).
    pub x: u32,
    pub strokes: usize,
}

fn luma(p: [u8; 3]) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

fn is_panel(p: [u8; 3]) -> bool {
    // Brightness relative to the reference panel colour, taken from the blue channel.
    let k = p[2] as f32 / PANEL_RGB[2];
    PANEL_SCALE.contains(&k) && (0..2).all(|i| (p[i] as f32 - k * PANEL_RGB[i]).abs() <= PANEL_TOL)
}

/// 0.0 (background) ..= 1.0 (title text).
fn ink(p: [u8; 3]) -> f32 {
    if luma(p) < INK_MIN_LUMA {
        return 0.0;
    }
    let br = p[2] as f32 - p[0] as f32;
    ((INK_BR_HI - br) / INK_BR_SPAN).clamp(0.0, 1.0)
}

/// Find the title band (panel rows, trimmed to the panel's x extent).
fn locate_title(img: &RgbImage) -> Option<Rect> {
    let sw = img.width().min(SEARCH_W);
    let sh = img.height().min(SEARCH_H);
    let counts: Vec<u32> = (0..sh)
        .map(|y| (0..sw).filter(|&x| is_panel(img.get_pixel(x, y).0)).count() as u32)
        .collect();
    let on: Vec<bool> = counts.iter().map(|&c| c >= BAND_ROW_MIN_PX).collect();
    let (y0, y1) = runs(&on).into_iter().find(|&(a, b)| {
        b - a >= BAND_MIN_H && counts[a as usize..b as usize].iter().any(|&c| c >= BAND_MIN_PANEL_PX)
    })?;
    let h = y1 - y0;
    // Panel x extent: columns where the panel colour fills at least a third of the band.
    let cols: Vec<u32> = (0..sw)
        .filter(|&x| (y0..y1).filter(|&y| is_panel(img.get_pixel(x, y).0)).count() as u32 * 3 >= h)
        .collect();
    let (x0, x1) = (*cols.first()?, *cols.last()? + 1);
    Some(Rect { x: x0, y: y0, w: x1 - x0, h })
}

/// Group consecutive indices where `on[i]` into (start, end_exclusive) runs.
fn runs(on: &[bool]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < on.len() {
        if on[i] {
            let s = i;
            while i < on.len() && on[i] {
                i += 1;
            }
            out.push((s as u32, i as u32));
        } else {
            i += 1;
        }
    }
    out
}

/// Locate, trim, mask and split a frame. `None` means unreadable.
pub fn prepare(img: &RgbImage) -> Option<Prepared> {
    let region = locate_title(img)?;
    let Rect { x: rx, w: rw, .. } = region;
    // Drop the band's outermost rows: after resampling they blend with the light
    // border next to the panel and read as a full-width line of ink.
    let (ry, rh) = (region.y + BAND_EDGE, region.h.checked_sub(2 * BAND_EDGE)?);
    // Skip the map icon: rightmost column near the left with several very dark pixels.
    let icon_right = (rx..(rx + ICON_MAX_RIGHT).min(rx + rw))
        .filter(|&x| {
            (ry..ry + rh).filter(|&y| luma(img.get_pixel(x, y).0) < ICON_DARK_LUMA).count() as u32
                >= ICON_MIN_DARK_PX
        })
        .max()
        .unwrap_or(rx);
    // Step past the icon's light anti-aliased rim: text starts after the first run of
    // ink-free columns (the icon-to-text gap).
    let col_ink = |x: u32| (ry..ry + rh).map(|y| ink(img.get_pixel(x, y).0)).sum::<f32>();
    let tx1 = rx + rw;
    let tx0 = (icon_right + 1..tx1.saturating_sub(ICON_GAP_COLS))
        .find(|&x| (x..x + ICON_GAP_COLS).all(|c| col_ink(c) < ICON_GAP_MAX_INK))?;
    if tx0 >= tx1 {
        return None;
    }
    let (tw, th) = (tx1 - tx0, rh);
    let inks: Vec<f32> = (0..th)
        .flat_map(|y| (0..tw).map(move |x| (x, y)))
        .map(|(x, y)| ink(img.get_pixel(tx0 + x, ry + y).0))
        .collect();
    let at = |x: u32, y: u32| inks[(y * tw + x) as usize];

    // Lines: horizontal projection; keep the two tallest runs, top to bottom.
    let row_on: Vec<bool> =
        (0..th).map(|y| (0..tw).map(|x| at(x, y)).sum::<f32>() > LINE_MIN_INK).collect();
    let mut row_runs = runs(&row_on);
    if row_runs.len() < 2 {
        return None; // not a two-line title
    }
    row_runs.sort_by_key(|r| std::cmp::Reverse(r.1 - r.0));
    row_runs.truncate(2);
    row_runs.sort();
    let line = |(ly0, ly1): (u32, u32)| -> Option<Rect> {
        let col_on: Vec<bool> =
            (0..tw).map(|x| (ly0..ly1).map(|y| at(x, y)).sum::<f32>() > COL_MIN_INK).collect();
        let first = col_on.iter().position(|&b| b)? as u32;
        let last = col_on.iter().rposition(|&b| b)? as u32 + 1;
        Some(Rect { x: first + PAD, y: ly0 + PAD, w: last - first, h: ly1 - ly0 })
    };
    let lines = vec![line(row_runs[0])?, line(row_runs[1])?];
    let numeral = trailing_numeral(&at, lines[1]);

    let mut canvas = GrayImage::from_pixel(tw + 2 * PAD, th + 2 * PAD, Luma([255]));
    for y in 0..th {
        for x in 0..tw {
            canvas.put_pixel(x + PAD, y + PAD, Luma([(255.0 * (1.0 - at(x, y))).round() as u8]));
        }
    }
    Some(Prepared { canvas, lines, left: rx, numeral })
}

/// The one-line layout's ink (0.0 background ..= 1.0 text): how dark a neutral pixel is. The
/// bar is (215,215,215), the text (45,45,45), anti-aliased between them.
fn bar_ink(p: [u8; 3]) -> f32 {
    let (max, min) = (p.iter().max().copied().unwrap_or(0), p.iter().min().copied().unwrap_or(0));
    if max - min > BAR_TINT {
        return 0.0;
    }
    ((BAR_INK_FROM - luma(p)) / BAR_INK_SPAN).clamp(0.0, 1.0)
}

/// The bar's light grey, at any brightness.
fn is_bar(p: [u8; 3]) -> bool {
    let (max, min) = (p.iter().max().copied().unwrap_or(0), p.iter().min().copied().unwrap_or(0));
    max - min <= BAR_TINT && luma(p) >= BAR_MIN_LUMA
}

/// The one-line layout's title in `img`, at the samples' UI Size: dark grey text on the bar,
/// on `rows`, ending at column `right` (where the bar's buttons start). The bar's left edge is
/// where its light grey ends, left along the clear rows `clear` above and below the text.
pub fn prepare_one_line(img: &RgbImage, rows: (u32, u32), right: u32, clear: [u32; 2]) -> Option<Prepared> {
    let (y0, y1) = (rows.0, rows.1.min(img.height()));
    let right = right.min(img.width());
    if y0 >= y1 || clear.iter().any(|&y| y >= img.height()) {
        return None;
    }
    // Left along both clear rows, as far as both are the bar's.
    let left = (0..right).rev().find(|&x| clear.iter().any(|&y| !is_bar(img.get_pixel(x, y).0))).map_or(0, |x| x + 1);
    if right < left + BAR_MIN_W {
        return None;
    }
    let (tw, th) = (right - left, y1 - y0);
    let inks: Vec<f32> = (0..th)
        .flat_map(|y| (0..tw).map(move |x| (x, y)))
        .map(|(x, y)| bar_ink(img.get_pixel(left + x, y0 + y).0))
        .collect();
    let at = |x: u32, y: u32| inks[(y * tw + x) as usize];
    let col_on: Vec<bool> = (0..tw).map(|x| (0..th).map(|y| at(x, y)).sum::<f32>() > COL_MIN_INK).collect();
    let row_on: Vec<bool> = (0..th).map(|y| (0..tw).map(|x| at(x, y)).sum::<f32>() > LINE_MIN_INK).collect();
    let (first, last) = (col_on.iter().position(|&b| b)? as u32, col_on.iter().rposition(|&b| b)? as u32 + 1);
    let (top, bottom) = (row_on.iter().position(|&b| b)? as u32, row_on.iter().rposition(|&b| b)? as u32 + 1);
    let line = Rect { x: first + PAD, y: top + PAD, w: last - first, h: bottom - top };
    let mut canvas = GrayImage::from_pixel(tw + 2 * PAD, th + 2 * PAD, Luma([255]));
    for y in 0..th {
        for x in 0..tw {
            canvas.put_pixel(x + PAD, y + PAD, Luma([(255.0 * (1.0 - at(x, y))).round() as u8]));
        }
    }
    Some(Prepared { canvas, lines: vec![line], left, numeral: None })
}

/// The Roman numeral I, II or III ending a line (`line` in canvas coordinates; `at`
/// gives ink by text coordinates): its last word, if every inked column of it is an
/// upright stroke or right beside one.
fn trailing_numeral(at: &impl Fn(u32, u32) -> f32, line: Rect) -> Option<Numeral> {
    let (x0, x1) = (line.x - PAD, line.x - PAD + line.w);
    let (y0, y1) = (line.y - PAD, line.y - PAD + line.h);
    let inked = |x: u32| (y0..y1).map(|y| at(x, y)).sum::<f32>() > COL_MIN_INK;
    let stroke = |x: u32| {
        (y0..y1).filter(|&y| at(x, y) >= STROKE_INK).count() as f32 >= STROKE_ROWS * line.h as f32
    };
    // The last word starts after the last gap between words.
    let mut gap = 0;
    let word_start = (x0..x1).rev().find(|&x| {
        gap = if inked(x) { 0 } else { gap + 1 };
        gap == WORD_GAP
    })? + WORD_GAP;
    let word = word_start..x1;
    let near_stroke = |x: u32| (x.saturating_sub(1)..=x + 1).any(|c| word.contains(&c) && stroke(c));
    if !word.clone().all(|x| !inked(x) || near_stroke(x)) {
        return None;
    }
    let on: Vec<bool> = word.clone().map(stroke).collect();
    let strokes = runs(&on);
    let fits = (1..=NUMERAL_MAX_STROKES).contains(&strokes.len())
        && strokes.iter().all(|(a, b)| b - a <= STROKE_MAX_W);
    fits.then_some(Numeral { x: word_start + PAD, strokes: strokes.len() })
}
