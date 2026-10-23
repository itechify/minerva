//! The EXP bar (ADR 0005): the figures on the in-game EXP bar, "EXP. 160 [3.81%]", read
//! from the Status Bar (ADR 0017): where the Status Bar reader found them in its capture, or
//! where they were last found while it can't find the bar.
//!
//! 1. Find the brackets: the only yellow-green glyphs in the figures, a pair of narrow
//!    upright components with the same top and bottom. The bar's fill below is the same
//!    colour but never pairs up like that. If the mark takes in another pair (the HP
//!    figures to the left), the rightmost pair is the EXP's. Their height measures the figures' UI
//!    Size (ADR 0020): figures at another are scaled to the samples' size, and read as below.
//! 2. The digits are neutral light grey on the dark slate bar, as tall as the brackets.
//!    The EXP is the cluster of ink just left of "[", back to the "EXP." label (taller than
//!    the brackets); with no label just left of it (a gap wider than between digits, or the
//!    edge of what was captured), it is unreadable. The percentage is the ink between the
//!    brackets.
//! 3. ocrs reads both as black-on-white lines, enlarged twice over. Only their digits
//!    count: the percentage always has two decimals, so its digits are hundredths of a
//!    percent.
//! 4. ocrs misreads this font now and then, the same way on every frame alike, so a
//!    reading must be one the bar could show, or the frame is unreadable: each figure in
//!    the game's shape ("68,063", "71.12%": a glyph merged into its neighbour or lost
//!    under the pointer breaks it), and a 2 read wherever the mask has a 2's flat foot,
//!    and nowhere else (ocrs has read the 7 of "71.07%" as a 2).

use crate::bug_report::rect_crop;
use crate::panel::ScreenRect;
use crate::prep::{PAD, Rect};
use crate::recogniser::{Recogniser, frame_to_rgb};
use crate::ui_size::{self, AtSamplesSize};
use crate::{Frame, StatusBar, StatusBarCapture};
use image::{GrayImage, Luma, RgbImage};
use serde::{Deserialize, Serialize};

/// What the EXP bar shows: the EXP gained in the current level, and how much of the
/// level that is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpBar {
    pub exp: u64,
    /// The percentage in hundredths: 381 for "3.81%".
    pub percent_hundredths: u32,
}

/// What the EXP figures read as in one capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpBarReading {
    /// `None` when the figures can't be read whole (covered, or not where they were looked
    /// for).
    pub bar: Option<ExpBar>,
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
    /// Where on screen the figures were read; `None` for a frame of the figures alone, and
    /// when there was nowhere to read them (no Status Bar found, none remembered).
    pub place: Option<ExpBarPlace>,
}

/// Where the EXP figures were read in a Status Bar capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpBarPlace {
    /// On screen, physical px.
    pub area: ScreenRect,
    /// Where they were last found, as the Status Bar couldn't be found in this capture.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub remembered: bool,
}

/// What the shell reads the EXP figures with in each Status Bar capture, while the Grind
/// Tracker is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpFigures {
    /// Where the figures were last found on screen, if the client area hasn't changed size
    /// since: where to read them while the Status Bar can't be found.
    pub last_found: Option<ScreenRect>,
}

/// Reads the EXP figures with the bundled recogniser (ADR 0002), which it shares with the
/// [`crate::TitleReader`]s.
pub struct ExpBarReader {
    recogniser: Recogniser,
}

impl ExpBarReader {
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled() }
    }

    /// Read a frame of the figures alone, as if they had been found there.
    pub fn read(&self, frame: &Frame) -> ExpBarReading {
        let bar = frame_to_rgb(frame).and_then(|img| self.read_rgb(&img));
        ExpBarReading { bar, at_ms: frame.at_ms, place: None }
    }

    /// Read the figures in a Status Bar capture: where the Status Bar read in it (`bar`)
    /// found them, or else where `figures` says they were last found.
    pub fn read_status_bar(&self, capture: &StatusBarCapture, bar: Option<&StatusBar>, figures: ExpFigures) -> ExpBarReading {
        let found = bar.and_then(|bar| bar.exp_figures).map(|area| ExpBarPlace { area, remembered: false });
        let place = found.or(figures.last_found.map(|area| ExpBarPlace { area, remembered: true }));
        let at_ms = capture.frame.at_ms;
        let bar = place
            .and_then(|place| figures_frame(capture, place))
            .and_then(|frame| frame_to_rgb(&frame))
            .and_then(|img| self.read_rgb(&img));
        ExpBarReading { bar, at_ms, place }
    }

    fn read_rgb(&self, img: &RgbImage) -> Option<ExpBar> {
        let at = AtSamplesSize::new(img, measure(img)?);
        let prepared = prepare(&at.img)?;
        // Read at the samples' size, the pointer over the figures can look like their ink.
        if at.pointer_over(prepared.columns, prepared.rows) {
            return None;
        }
        let lines = self.recogniser.read_lines(&prepared.canvas, &[prepared.exp, prepared.percent])?;
        let text = |i: usize| lines.get(i)?.as_ref().map(|l| l.to_string());
        let (exp_text, percent_text) = (text(0)?, text(1)?);
        let twos = |text: &str| text.chars().filter(|&c| c == '2').count();
        if twos(&exp_text) != prepared.exp_feet || twos(&percent_text) != prepared.percent_feet {
            return None;
        }
        let bar = ExpBar { exp: thousands(&exp_text)?, percent_hundredths: percent(&percent_text)? };
        consistent(bar).then_some(bar)
    }
}

/// The figures at `place` in a Status Bar capture; `None` if the capture doesn't hold them.
pub(crate) fn figures_frame(capture: &StatusBarCapture, place: ExpBarPlace) -> Option<Frame> {
    rect_crop(&capture.frame, capture.origin, place.area, 0)
}

/// A reading that could be on an EXP bar: under 100%, and no EXP at 0.00% unless the
/// level needs more than 10,000 EXP per hundredth (it never comes close).
fn consistent(bar: ExpBar) -> bool {
    bar.percent_hundredths < 10_000 && (bar.percent_hundredths > 0 || bar.exp < 10_000)
}

/// The digits of a run of them as ocrs read it, with the letters and signs it mistakes for
/// digits in this font put right (it has read the 5 of "53.05%" as a "$"). `None` if
/// anything else is in it, or nothing is.
fn digits(text: &str) -> Option<String> {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '0'..='9' => out.push(c),
            'O' | 'o' | 'D' => out.push('0'),
            'l' | 'I' | '|' | 'i' => out.push('1'),
            '$' => out.push('5'),
            ' ' => {}
            _ => return None,
        }
    }
    (!out.is_empty() && out.len() <= 12).then_some(out)
}

/// The EXP from "68,063": one to three digits, then threes after each thousands comma
/// (which ocrs reads as a point at this size, "8.435"). ocrs has merged a glyph into the
/// one beside it ("26,777" read "26,77"), and the pointer over the "EXP." label has cut
/// a figure short at its comma (".909"); neither has this shape. At another UI Size it has
/// read a comma as both (".,": "100.,300"), which is one between digits.
fn thousands(text: &str) -> Option<u64> {
    let text = text.trim();
    let separators = [',', '.'];
    if text.starts_with(separators) || text.ends_with(separators) {
        return None;
    }
    let groups: Vec<String> = text.split(separators).filter(|group| !group.is_empty()).map(digits).collect::<Option<_>>()?;
    let (first, rest) = groups.split_first()?;
    let shaped = first.len() <= 3 && rest.iter().all(|group| group.len() == 3);
    shaped.then(|| groups.concat().parse().ok()).flatten()
}

/// Hundredths of a percent from "3.81%": one or two digits, the point (or a comma, as
/// ocrs reads it at times) and two decimals. ocrs has merged two glyphs into one
/// ("30.05%" read "30.0%"), which doesn't have this shape.
fn percent(text: &str) -> Option<u32> {
    let (whole, decimals) = text.trim().trim_end_matches(['%', ')', ']']).split_once(['.', ','])?;
    let (whole, decimals) = (digits(whole)?, digits(decimals)?);
    let shaped = whole.len() <= 2 && decimals.len() == 2;
    shaped.then(|| format!("{whole}{decimals}").parse().ok()).flatten()
}

/// The figures, masked and boxed for ocrs, and how many 2s each shows.
struct Prepared {
    canvas: GrayImage,
    /// Where the figures are, from the EXP's first column to the "]", and the brackets' rows.
    columns: (u32, u32),
    rows: (u32, u32),
    exp: Rect,
    percent: Rect,
    exp_feet: usize,
    percent_feet: usize,
}

/// Bracket colour: yellow-green, (150,198,61) at its core.
fn bracket(p: [u8; 3]) -> bool {
    let [r, g, b] = p.map(i32::from);
    g >= 120 && g - b >= 60 && g - r >= 25
}

fn luma(p: [u8; 3]) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

/// Digit ink, 0.0 (the dark slate bar, luma ~80) ..= 1.0 (the light grey digits, ~215).
/// Only near-neutral pixels count: the bar's edges are bluish, the brackets green.
fn ink(p: [u8; 3]) -> f32 {
    let [r, g, b] = p.map(i32::from);
    if (b - r).abs() > INK_MAX_TINT || (g - r).abs() > INK_MAX_TINT {
        return 0.0;
    }
    ((luma(p) - INK_LUMA_LO) / (INK_LUMA_HI - INK_LUMA_LO)).clamp(0.0, 1.0)
}

const INK_MAX_TINT: i32 = 24;
const INK_LUMA_LO: f32 = 100.0;
const INK_LUMA_HI: f32 = 190.0;
/// A column holds a glyph when its ink in the band sums past this.
const COL_MIN_INK: f32 = 0.5;
/// A bracket is at most this wide, and at least this tall and twice as tall as wide, at the
/// samples' UI Size (ADR 0020)...
const BRACKET_MAX_W: u32 = 4;
const BRACKET_MIN_H: u32 = 6;
/// ...where it is this tall: what the figures' UI Size is measured by. At any UI Size, a
/// bracket is at least this tall, and twice as tall as wide.
const BRACKET_H: u32 = 11;
const ANY_BRACKET_MIN_H: u32 = 4;
/// Per row of bracket height: glyphs of one figure are fewer ink-free columns apart than
/// this (digits touch or nearly; the "EXP." label ends 4 columns before them on the
/// samples)...
const GLYPH_GAP_PER_H: f32 = 0.2;
/// ...and a figure never has a gap wider than this.
const SPACE_PER_H: f32 = 0.6;
/// The "EXP." label rises this many rows above the brackets; digits never do.
const LABEL_RISE: u32 = 3;
/// Above that, this many rows are the slate, over the label.
const LABEL_CLEAR: u32 = 3;
/// Per row of bracket height, the label is this wide (35 columns by 11 rows on the samples).
/// The pointer, taken for it or run into it, isn't: the EXP left of it may be cut short.
const LABEL_W_PER_H: std::ops::RangeInclusive<f32> = 2.7..=3.5;
/// Per row of bracket height: a 2's flat foot, along the brackets' bottom row, is at
/// least this long. No other glyph's is: the rest end in a curve or a stroke, half as
/// long or less.
const FOOT_PER_H: f32 = 0.6;
/// The masked figures are enlarged this many times before ocrs reads them. At their
/// own 11-13 rows, ocrs read the 7 of "1.74%" as a 2 on the 2026-10-07 recording. No
/// size stops its misreads for good: on the 2026-10-11 Bug Report's frames, 1.5x to 4x
/// each misread some, as did spacing the glyphs apart; step 4 catches them.
const UPSCALE: u32 = 2;

/// A connected run of bracket-coloured pixels: its bounding box.
#[derive(Debug, Clone, Copy)]
struct Blob {
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
}

/// Bounding boxes of the bracket-coloured components (4-connected).
fn blobs(img: &RgbImage) -> Vec<Blob> {
    let (w, h) = img.dimensions();
    let mut seen = vec![false; (w * h) as usize];
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if seen[(y * w + x) as usize] || !bracket(img.get_pixel(x, y).0) {
                continue;
            }
            let mut blob = Blob { x0: x, x1: x, y0: y, y1: y };
            let mut stack = vec![(x, y)];
            seen[(y * w + x) as usize] = true;
            while let Some((cx, cy)) = stack.pop() {
                blob = Blob { x0: blob.x0.min(cx), x1: blob.x1.max(cx), y0: blob.y0.min(cy), y1: blob.y1.max(cy) };
                let next = [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)];
                for (nx, ny) in next {
                    if nx < w && ny < h && !seen[(ny * w + nx) as usize] && bracket(img.get_pixel(nx, ny).0) {
                        seen[(ny * w + nx) as usize] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            out.push(blob);
        }
    }
    out
}

/// The "[" and "]" of the figures: the rightmost pair of bracket-shaped components with
/// the same top and bottom and room for a digit between them (a stray component on
/// other rows between them doesn't break the pair).
/// Brackets are at most `max_w` wide and at least `min_h` tall.
fn brackets(img: &RgbImage, (max_w, min_h): (u32, u32)) -> Option<(Blob, Blob)> {
    let mut shaped: Vec<Blob> = blobs(img)
        .into_iter()
        .filter(|b| {
            let (w, h) = (b.x1 - b.x0 + 1, b.y1 - b.y0 + 1);
            w <= max_w && h >= min_h && h >= 2 * w
        })
        .collect();
    shaped.sort_by_key(|b| b.x0);
    let pairs = |open: &Blob, close: &Blob| {
        open.y0.abs_diff(close.y0) <= 1 && open.y1.abs_diff(close.y1) <= 1 && close.x0 > open.x1 + 3
    };
    shaped.iter().rev().find_map(|close| {
        let open = shaped.iter().rev().find(|open| pairs(open, close))?;
        Some((*open, *close))
    })
}

/// The figures' UI Size: their brackets' height next to the samples'.
fn measure(img: &RgbImage) -> Option<f32> {
    let (open, close) = brackets(img, (u32::MAX, ANY_BRACKET_MIN_H))?;
    let h = (open.y1 - open.y0).max(close.y1 - close.y0) + 1;
    Some(h as f32 / BRACKET_H as f32)
}

fn prepare(img: &RgbImage) -> Option<Prepared> {
    let (open, close) = brackets(img, (BRACKET_MAX_W, BRACKET_MIN_H))?;
    // The digits' rows: the brackets', with a row either side for anti-aliasing.
    let y0 = open.y0.min(close.y0).saturating_sub(1);
    let y1 = (open.y1.max(close.y1) + 2).min(img.height());
    let col_ink = |x: u32| (y0..y1).map(|y| ink(img.get_pixel(x, y).0)).sum::<f32>();
    let inked = |x: u32| col_ink(x) > COL_MIN_INK;
    let rises = |x: u32| {
        (y0.saturating_sub(LABEL_RISE)..y0).any(|y| ink(img.get_pixel(x, y).0) >= 0.5)
    };
    // ...and no higher: the pointer's tip, taken for the label, may, its white or its black.
    let towers = |x: u32| {
        (1..=LABEL_CLEAR).filter(|&k| y0 >= LABEL_RISE + k).any(|k| {
            let p = img.get_pixel(x, y0 - LABEL_RISE - k).0;
            ink(p) >= 0.5 || ui_size::pointer_black(p)
        })
    };

    // The percentage: the ink between the brackets.
    let inside: Vec<u32> = (open.x1 + 1..close.x0).filter(|&x| inked(x)).collect();
    let (px0, px1) = (*inside.first()?, *inside.last()? + 1);

    // The EXP: the clusters of ink left of "[", back to the label (any cluster of it rises
    // above the brackets, and it is the label's width). Without the label just left of it,
    // the EXP may go on past what was captured ("100,300" cut to "00,300", where the figures
    // were looked for at another UI Size), or the pointer is over it: unreadable.
    let h = (open.y1 - open.y0 + 1) as f32;
    let split = (h * GLYPH_GAP_PER_H).ceil() as usize;
    let max_gap = (h * SPACE_PER_H).ceil() as u32;
    let foot = (h * FOOT_PER_H).ceil() as u32;
    let on: Vec<bool> = (0..open.x0).map(inked).collect();
    let mut number: Option<(u32, u32)> = None;
    let mut labelled = false;
    for (x0, x1) in clusters(&on, split).into_iter().rev() {
        if number.is_some_and(|(n0, _)| n0 - x1 > max_gap) {
            break;
        }
        if (x0..x1).any(rises) {
            labelled = LABEL_W_PER_H.contains(&((x1 - x0) as f32 / h)) && !(x0..x1).any(towers);
            break;
        }
        number = Some((x0, number.map_or(x1, |(_, n1)| n1)));
    }
    let (ex0, ex1) = number.filter(|_| labelled)?;

    // Black on white, from the EXP's first column to the percentage's last.
    let (cx0, cw, ch) = (ex0, px1 - ex0, y1 - y0);
    let mut canvas = GrayImage::from_pixel(cw + 2 * PAD, ch + 2 * PAD, Luma([255]));
    for y in 0..ch {
        for x in 0..cw {
            let skip = (open.x0..=open.x1).contains(&(cx0 + x));
            let v = if skip { 0.0 } else { ink(img.get_pixel(cx0 + x, y0 + y).0) };
            canvas.put_pixel(x + PAD, y + PAD, Luma([(255.0 * (1.0 - v)).round() as u8]));
        }
    }
    let (w, h) = (canvas.width() * UPSCALE, canvas.height() * UPSCALE);
    let canvas = image::imageops::resize(&canvas, w, h, image::imageops::FilterType::Triangle);
    let line = |x0: u32, x1: u32| Rect {
        x: (x0 - cx0 + PAD) * UPSCALE,
        y: PAD * UPSCALE,
        w: (x1 - x0) * UPSCALE,
        h: ch * UPSCALE,
    };
    // The 2s: their flat feet, on the brackets' bottom row.
    let feet = |x0: u32, x1: u32| flat_feet(img, open.y1, x0..x1, foot);
    Some(Prepared {
        canvas,
        columns: (ex0, close.x1 + 1),
        rows: (y0, y1),
        exp: line(ex0, ex1),
        percent: line(px0, px1),
        exp_feet: feet(ex0, ex1),
        percent_feet: feet(px0, px1),
    })
}

/// How many runs of ink, at least `foot` columns long, row `y` has in `columns`.
fn flat_feet(img: &RgbImage, y: u32, columns: std::ops::Range<u32>, foot: u32) -> usize {
    let mut feet = 0;
    let mut run = 0;
    for x in columns {
        if ink(img.get_pixel(x, y).0) >= 0.5 {
            run += 1;
        } else {
            feet += usize::from(run >= foot);
            run = 0;
        }
    }
    feet + usize::from(run >= foot)
}

/// Runs of `on` columns as (start, end_exclusive), joining runs fewer than `split`
/// columns apart.
fn clusters(on: &[bool], split: usize) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (x, _) in on.iter().enumerate().filter(|(_, on)| **on) {
        let x = x as u32;
        match out.last_mut() {
            Some((_, end)) if (x - *end) < split as u32 => *end = x + 1,
            _ => out.push((x, x + 1)),
        }
    }
    out
}
