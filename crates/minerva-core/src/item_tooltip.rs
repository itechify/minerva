//! The Item Tooltip reader (ADR 0007): finds the Item Tooltip nearest the pointer in a
//! capture of the game's client area and reads the item's name from it.
//!
//! 1. Find the name's bullet: a round cyan dot (86,198,255 at its core, 6-8 px across)
//!    with the tooltip's dark slate fill to its left. The stat lines' bullets are orange,
//!    and a crafted item's maker line above the name has none, so the bullet marks the
//!    name line wherever the game puts the tooltip (below the pointer, or moved up or
//!    beside it near the window's edges). Of the bullets near the pointer, the nearest
//!    whose name could be found counts.
//! 2. The name is bold near-white ink (218,218,223) starting just right of the bullet,
//!    taller than it, on the slate fill. The game world shows through the fill, dimmed
//!    and tinted blue, so only bright, near-neutral pixels count as ink. The name ends
//!    at a gap wider than a space, or at the tooltip's light border, which runs on above
//!    and below the line where no letter does. Over a light window (the Player Shop's
//!    chat) the border isn't solid, its text showing through, and the window past it reads
//!    as ink too: there the name ends where the column is light above the line.
//! 3. ocrs reads the name as a black-on-white line, enlarged twice over.

use crate::Frame;
use crate::panel::{ScreenPoint, ScreenRect};
use crate::prep::{PAD, Rect};
use crate::recogniser::{Recogniser, frame_to_rgb};
use image::{GrayImage, Luma, RgbImage};
use std::collections::HashMap;

/// A capture of the game's whole client area for Item Lookup, and where the pointer was.
#[derive(Debug, Clone)]
pub struct ItemCapture {
    pub frame: Frame,
    /// Where the frame's top-left pixel is on screen: the client area's corner.
    pub origin: ScreenPoint,
    /// Where the mouse pointer was on screen when the frame was captured.
    pub pointer: ScreenPoint,
}

/// What an Item Lookup capture reads as.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemTooltipReading {
    pub tooltip: ItemTooltip,
    /// Where the mouse pointer was on screen, as in [`ItemCapture::pointer`]: where the
    /// Item Card goes.
    pub pointer: ScreenPoint,
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
}

/// The Item Tooltip nearest the pointer, if any.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemTooltip {
    /// No Item Tooltip near the pointer.
    None,
    /// An Item Tooltip: where its name line is on screen (from its bullet to a little
    /// past the name, so a window of ours that cuts the name short overlaps it), and
    /// the name as read (`None`: unreadable).
    Found { area: ScreenRect, name: Option<String> },
}

/// Reads Item Lookup captures with the bundled recogniser (ADR 0002), which it shares
/// with the other readers.
pub struct ItemTooltipReader {
    recogniser: Recogniser,
}

impl ItemTooltipReader {
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled() }
    }

    pub fn read(&self, capture: &ItemCapture) -> ItemTooltipReading {
        let read = |img: &RgbImage| find(img, capture.origin, capture.pointer, |canvas| read_canvas(&self.recogniser, canvas));
        let tooltip = frame_to_rgb(&capture.frame).map_or(ItemTooltip::None, |img| read(&img));
        ItemTooltipReading { tooltip, pointer: capture.pointer, at_ms: capture.frame.at_ms }
    }
}

/// Reads the Item Tooltip by the pointer over a Player Shop for the Deal Finder, as
/// [`ItemTooltipReader`] does, remembering what each name's ink read as: a tooltip left
/// showing while the player hovers a listing costs no recognition.
pub(crate) struct ShopTooltipReader {
    recogniser: Recogniser,
    read: HashMap<(u32, Vec<u8>), Option<String>>,
}

/// It remembers at most this many names; past them it starts over.
const REMEMBERED: usize = 64;

impl ShopTooltipReader {
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled(), read: HashMap::new() }
    }

    /// The Item Tooltip nearest `pointer` (on screen) in `img`, the client area whose
    /// top-left pixel is at `origin`.
    pub fn read(&mut self, img: &RgbImage, origin: ScreenPoint, pointer: ScreenPoint) -> ItemTooltip {
        let (recogniser, read) = (&self.recogniser, &mut self.read);
        find(img, origin, pointer, |canvas| {
            let key = (canvas.width(), canvas.as_raw().clone());
            if let Some(name) = read.get(&key) {
                return name.clone();
            }
            let name = read_canvas(recogniser, canvas);
            if read.len() >= REMEMBERED {
                read.clear();
            }
            read.insert(key, name.clone());
            name
        })
    }
}

/// The Item Tooltip nearest `pointer` in `img`, its name read by `read` from its canvas.
fn find(img: &RgbImage, origin: ScreenPoint, pointer: ScreenPoint, mut read: impl FnMut(&GrayImage) -> Option<String>) -> ItemTooltip {
    let pointer = (pointer.x - origin.x, pointer.y - origin.y);
    let mut bullets = bullets(img, pointer);
    bullets.sort_by_key(|b| b.distance_sq(pointer));
    let on_screen = |r: Rect| ScreenRect { x: origin.x + r.x as i32, y: origin.y + r.y as i32, width: r.w, height: r.h };
    if let Some((bullet, line)) = bullets.iter().find_map(|b| Some((b, name_line(img, b)?))) {
        let slack = line.y1 - line.y0;
        let area = Rect { x: bullet.x0, y: line.y0, w: line.x1 + slack - bullet.x0, h: line.y1 - line.y0 };
        return ItemTooltip::Found { area: on_screen(area), name: read(&name_canvas(img, &line)) };
    }
    // A bullet with nothing readable beside it: the name may be covered.
    match bullets.first() {
        Some(b) => {
            let area = Rect { x: b.x0, y: b.y0.saturating_sub(b.h()), w: b.w() + NAMELESS_SLACK, h: 3 * b.h() };
            ItemTooltip::Found { area: on_screen(area), name: None }
        }
        None => ItemTooltip::None,
    }
}

/// The name's ink, black on white, padded as ocrs reads it.
fn name_canvas(img: &RgbImage, line: &NameLine) -> GrayImage {
    let (w, h) = (line.x1 - line.x0, line.y1 - line.y0);
    let mut canvas = GrayImage::from_pixel(w + 2 * PAD, h + 2 * PAD, Luma([255]));
    for y in 0..h {
        for x in 0..w {
            let v = ink(img.get_pixel(line.x0 + x, line.y0 + y).0);
            canvas.put_pixel(x + PAD, y + PAD, Luma([(255.0 * (1.0 - v)).round() as u8]));
        }
    }
    canvas
}

/// What the name on `canvas` reads as, enlarged `UPSCALE` times.
fn read_canvas(recogniser: &Recogniser, canvas: &GrayImage) -> Option<String> {
    let (w, h) = (canvas.width() - 2 * PAD, canvas.height() - 2 * PAD);
    let (cw, ch) = (canvas.width() * UPSCALE, canvas.height() * UPSCALE);
    let canvas = image::imageops::resize(canvas, cw, ch, image::imageops::FilterType::Triangle);
    let rect = Rect { x: PAD * UPSCALE, y: PAD * UPSCALE, w: w * UPSCALE, h: h * UPSCALE };
    let lines = recogniser.read_lines(&canvas, &[rect])?;
    let text = lines.first()?.as_ref()?.to_string();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// How far from the pointer (px, either way) the name's bullet is looked for: the
/// tooltip is up to ~450 px wide and moves to the pointer's left or above it near the
/// window's edges.
const SEARCH: i32 = 640;
/// A bullet is this many px across, give or take.
const BULLET_MIN: u32 = 4;
const BULLET_MAX: u32 = 11;
/// Columns checked left of a bullet for the tooltip's fill.
const FILL_COLS: u32 = 3;
/// The name starts within this many bullet widths right of the bullet.
const NAME_START: u32 = 3;
/// The name's letters are at least this many rows taller than the bullet.
const TALLER: u32 = 3;
/// ...and at most this many times as tall as it.
const MAX_TALL: u32 = 4;
/// Per row of the name's height: a gap of more columns than this ends the name (a space
/// is ~0.6 of the height, the gap to whatever else is on the line far more).
const NAME_GAP_PER_H: f32 = 1.2;
/// The tooltip's border runs on at least this many rows above and below the line.
const BORDER_RISE: u32 = 4;
/// The tooltip's edge is light on this many rows above the line, down to the one just above
/// it: the name's letters never reach there, and a crafted item's maker line above the name
/// doesn't come down to it.
const EDGE_ROWS: u32 = 4;
/// A column holds ink when its ink in the line's rows sums past this.
const COL_MIN_INK: f32 = 0.5;
/// A row holds ink when its ink near the name's start sums past this.
const ROW_MIN_INK: f32 = 0.5;
/// How far past a bullet with no name beside it the name's area reaches.
const NAMELESS_SLACK: u32 = 200;
/// The name is enlarged this many times before ocrs reads it.
const UPSCALE: u32 = 2;

/// The name's bullet colour: (86,198,255) at its core, cyan.
fn bullet_blue(p: [u8; 3]) -> bool {
    let [r, g, b] = p.map(i32::from);
    b >= 200 && g >= 140 && r <= 140 && g - r >= 70
}

/// The tooltip's slate fill, with whatever is behind it showing through: dark, and no
/// warmer than neutral. How blue it looks depends on what is behind it: over the Item
/// Inventory window's edge or a purple icon it is barely bluer than red (Stirge Wing's and
/// Horny Mushroom Cap's samples), over the game's sky clearly so.
fn fill(p: [u8; 3]) -> bool {
    let [r, _, b] = p.map(i32::from);
    luma(p) < 130.0 && b >= r
}

fn luma(p: [u8; 3]) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

/// Name ink, 0.0 (the fill, or the world dimmed behind it) ..= 1.0 (the near-white
/// letters, luma ~218). Only near-neutral pixels count: the fill and the edges of
/// letters on it are blue.
fn ink(p: [u8; 3]) -> f32 {
    let [r, g, b] = p.map(i32::from);
    if b - r > INK_MAX_TINT || (g - r).abs() > INK_MAX_TINT {
        return 0.0;
    }
    ((luma(p) - INK_LUMA_LO) / (INK_LUMA_HI - INK_LUMA_LO)).clamp(0.0, 1.0)
}

const INK_MAX_TINT: i32 = 45;
const INK_LUMA_LO: f32 = 130.0;
const INK_LUMA_HI: f32 = 200.0;

/// A bullet-coloured component's bounding box (frame px, ends exclusive).
#[derive(Debug, Clone, Copy)]
struct Blob {
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
}

impl Blob {
    fn w(&self) -> u32 {
        self.x1 - self.x0
    }

    fn h(&self) -> u32 {
        self.y1 - self.y0
    }

    fn distance_sq(&self, (px, py): (i32, i32)) -> i64 {
        let (cx, cy) = (((self.x0 + self.x1) / 2) as i64, ((self.y0 + self.y1) / 2) as i64);
        (cx - px as i64).pow(2) + (cy - py as i64).pow(2)
    }
}

/// The bullets near the pointer: round cyan dots with the tooltip's fill left of them.
fn bullets(img: &RgbImage, (px, py): (i32, i32)) -> Vec<Blob> {
    let (w, h) = img.dimensions();
    let clamp = |v: i32, max: u32| v.clamp(0, max as i32) as u32;
    let (x0, x1) = (clamp(px - SEARCH, w), clamp(px + SEARCH, w));
    let (y0, y1) = (clamp(py - SEARCH, h), clamp(py + SEARCH, h));
    let (sw, sh) = (x1 - x0, y1 - y0);
    let mut seen = vec![false; (sw * sh) as usize];
    let at = |x: u32, y: u32| ((y - y0) * sw + (x - x0)) as usize;
    let mut out = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if seen[at(x, y)] || !bullet_blue(img.get_pixel(x, y).0) {
                continue;
            }
            let mut blob = Blob { x0: x, x1: x + 1, y0: y, y1: y + 1 };
            let mut count = 0;
            let mut stack = vec![(x, y)];
            seen[at(x, y)] = true;
            while let Some((cx, cy)) = stack.pop() {
                count += 1;
                blob = Blob { x0: blob.x0.min(cx), x1: blob.x1.max(cx + 1), y0: blob.y0.min(cy), y1: blob.y1.max(cy + 1) };
                let next = [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)];
                for (nx, ny) in next {
                    if (x0..x1).contains(&nx) && (y0..y1).contains(&ny) && !seen[at(nx, ny)] && bullet_blue(img.get_pixel(nx, ny).0) {
                        seen[at(nx, ny)] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            let round = (BULLET_MIN..=BULLET_MAX).contains(&blob.w())
                && (BULLET_MIN..=BULLET_MAX).contains(&blob.h())
                && blob.w().abs_diff(blob.h()) <= 3
                && 2 * count >= blob.w() * blob.h();
            if round && filled_left_of(img, &blob) {
                out.push(blob);
            }
        }
    }
    out
}

/// Whether the columns just left of the blob, on its middle rows, are tooltip fill.
fn filled_left_of(img: &RgbImage, blob: &Blob) -> bool {
    let gap = 2;
    if blob.x0 < gap + FILL_COLS {
        return false;
    }
    let mid = (blob.y0 + blob.y1) / 2;
    let xs = blob.x0 - gap - FILL_COLS..blob.x0 - gap;
    xs.clone().flat_map(|x| (mid - 1..=mid + 1).map(move |y| (x, y))).all(|(x, y)| fill(img.get_pixel(x, y).0))
}

/// The name's ink beside a bullet (frame px, ends exclusive).
#[derive(Debug, Clone, Copy)]
struct NameLine {
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
}

/// The name beside `bullet`: the ink starting just right of it, on rows taller than it
/// and around it, up to a gap wider than a space or the tooltip's border. `None` when
/// there is no such ink: then it isn't the name's bullet, or the name is covered.
fn name_line(img: &RgbImage, bullet: &Blob) -> Option<NameLine> {
    let (w, h) = img.dimensions();
    let bh = bullet.h();
    let band = (bullet.y0.saturating_sub(MAX_TALL * bh), (bullet.y1 + MAX_TALL * bh).min(h));
    let col_ink = |x: u32, (y0, y1): (u32, u32)| (y0..y1).map(|y| ink(img.get_pixel(x, y).0)).sum::<f32>();

    // Where the name starts: the first inked column right of the bullet, on its rows.
    let reach = (bullet.x1 + NAME_START * bullet.w()).min(w);
    let x0 = (bullet.x1..reach).find(|&x| col_ink(x, (bullet.y0, bullet.y1)) > COL_MIN_INK)?;

    // Its rows: the run of inked rows (over its first letters) that holds the bullet's
    // middle row.
    let probe = (x0, (x0 + 6 * bh).min(w));
    let row_ink = |y: u32| (probe.0..probe.1).map(|x| ink(img.get_pixel(x, y).0)).sum::<f32>();
    let mid = (bullet.y0 + bullet.y1) / 2;
    let inked = |y: u32| row_ink(y) > ROW_MIN_INK;
    if !inked(mid) {
        return None;
    }
    let mut y0 = mid;
    while y0 > band.0 && inked(y0 - 1) {
        y0 -= 1;
    }
    let mut y1 = mid + 1;
    while y1 < band.1 && inked(y1) {
        y1 += 1;
    }
    let lh = y1 - y0;
    if lh < bh + TALLER || lh > MAX_TALL * bh {
        return None;
    }

    // Where it ends: a gap wider than a space, or the border, which runs on above and
    // below the line.
    let rows = (y0.saturating_sub(1), (y1 + 1).min(h));
    let border = |x: u32| {
        let above = (y0.saturating_sub(BORDER_RISE + 2), y0.saturating_sub(2));
        let below = ((y1 + 2).min(h), (y1 + BORDER_RISE + 2).min(h));
        let solid = |(a, b): (u32, u32)| b > a && (a..b).all(|y| ink(img.get_pixel(x, y).0) >= 0.5);
        solid(above) && solid(below)
    };
    // Or the tooltip's edge: light above the line, where no letter reaches. Past it a light
    // window (the Player Shop's chat) reads as ink too, and its text shows through the edge.
    let light = |x: u32, y: u32| ink(img.get_pixel(x, y).0) >= 0.5;
    let edge = |x: u32| y0 >= EDGE_ROWS && (y0 - EDGE_ROWS..y0).all(|y| light(x, y));
    let max_gap = (lh as f32 * NAME_GAP_PER_H).ceil() as u32;
    let mut x1 = x0 + 1;
    let mut x = x0 + 1;
    while x < w && x - x1 <= max_gap {
        if edge(x) {
            // The edge's inner side, lit above the line too, isn't the name's.
            while x1 > x0 + 1 && light(x1 - 1, y0 - 2) {
                x1 -= 1;
            }
            break;
        }
        if col_ink(x, rows) > COL_MIN_INK {
            if border(x) {
                break;
            }
            x1 = x + 1;
        }
        x += 1;
    }
    Some(NameLine { x0, x1, y0: rows.0, y1: rows.1 })
}
