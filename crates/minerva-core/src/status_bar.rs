//! The Status Bar reader (ADR 0016): finds the game's Status Bar at the bottom of the client
//! area and reads the Character's level, job and name from it.
//!
//! The game draws its UI at a fixed pixel size, and pins the Status Bar to the bottom of the
//! client area. The shell captures a strip there; how far up the bar sits in it isn't
//! assumed, it is found:
//!
//! 1. The level is written in orange tiles (255,148,21), one per digit, [`TILE_PITCH`] px
//!    apart, in a dark grey box (the "LV." box). The tiles are where orange components of a
//!    tile's size line up at that pitch, with nothing but the box around them. Live captures
//!    show the game's colours a little darker than screenshots do (`prep.rs`), so the orange
//!    and the box are matched at any brightness.
//! 2. The job ("Magician") and the name ("Egbert") are neutral light grey on the bar's slate,
//!    two lines right of the box, at set heights from the tiles. Every pixel of the slate
//!    between them is bluish grey, the text's anti-aliasing included: a pixel that isn't (the
//!    game's pointer, a window of ours) means the line is covered, and it is unreadable,
//!    never misread.
//! 3. ocrs reads the level's glyphs (white on the tiles), the job and the name at once, black
//!    on white, enlarged twice over, and the reader remembers what each line's ink read as,
//!    so an unchanged bar costs a scan of the strip and no recognition.
//! 4. A reading counts only if it is what the bar could show: the level as many digits as
//!    there are tiles, a 1 on each tile whose glyph is narrow and on no other, from 1 to
//!    [`MAX_LEVEL`]; the job one of [`JOBS`], as the game spells it; the name letters and
//!    digits, as the game allows. Anything else is unreadable.
//! 5. The EXP figures are at a set distance from the box, as the job and name are (ADR
//!    0017): [`StatusBar::exp_figures`] says where, for the EXP bar reader.

use crate::Frame;
use crate::panel::{ScreenPoint, ScreenRect};
use crate::recogniser::{Recogniser, RememberedLines, frame_to_rgb};
use image::{GrayImage, Luma, RgbImage};
use serde::{Deserialize, Serialize};

/// A capture of the bottom of the game's client area, for the Status Bar.
#[derive(Debug, Clone)]
pub struct StatusBarCapture {
    pub frame: Frame,
    /// Where the frame's top-left pixel is on screen.
    pub origin: ScreenPoint,
}

/// What a Status Bar capture reads as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusBarReading {
    /// The Status Bar, if it shows (not on the character select screen, say).
    pub bar: Option<StatusBar>,
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
}

/// The Status Bar's Character part, as read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusBar {
    /// Where the level, the job and the name are on screen: what a Bug Report keeps of the
    /// capture, and what our windows keep clear of.
    pub area: ScreenRect,
    /// The level after "LV."; `None`: unreadable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    /// The job, as the game writes it ("Magician"); `None`: unreadable or covered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    /// The Character's name; `None`: unreadable or covered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Where the EXP figures ("EXP. 160 [3.81%]") are on screen (ADR 0017), found from the LV
    /// box; `None` if they would be outside the capture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp_figures: Option<ScreenRect>,
}

/// The jobs as the game names them, which a job read must be one of, exactly. Only
/// "Magician" has been seen in a sample: the rest are the game's job names as players know
/// them, and one spelt otherwise on the bar reads as unreadable until a sample shows it.
const JOBS: &[&str] = &[
    "Beginner",
    "Warrior",
    "Fighter",
    "Page",
    "Spearman",
    "Crusader",
    "White Knight",
    "Dragon Knight",
    "Hero",
    "Paladin",
    "Dark Knight",
    "Magician",
    "Wizard (Fire, Poison)",
    "Wizard (Ice, Lightning)",
    "Wizard (F/P)",
    "Wizard (I/L)",
    "F/P Wizard",
    "I/L Wizard",
    "Cleric",
    "Mage (Fire, Poison)",
    "Mage (Ice, Lightning)",
    "Mage (F/P)",
    "Mage (I/L)",
    "F/P Mage",
    "I/L Mage",
    "Priest",
    "Arch Mage (Fire, Poison)",
    "Arch Mage (Ice, Lightning)",
    "F/P Arch Mage",
    "I/L Arch Mage",
    "Bishop",
    "Bowman",
    "Archer",
    "Hunter",
    "Crossbowman",
    "Ranger",
    "Sniper",
    "Bowmaster",
    "Marksman",
    "Thief",
    "Rogue",
    "Assassin",
    "Bandit",
    "Hermit",
    "Chief Bandit",
    "Night Lord",
    "Shadower",
    "Pirate",
    "Brawler",
    "Gunslinger",
    "Marauder",
    "Outlaw",
    "Buccaneer",
    "Corsair",
];

/// The highest level a Character can have.
pub const MAX_LEVEL: u32 = 200;

/// How tall a strip of the client area's bottom the shell captures: the bar is 55 rows,
/// pinned to the bottom, with room to spare.
pub const STATUS_BAR_STRIP: u32 = 96;

/// A level tile's orange component is about 17 px wide and 20 tall...
const TILE_W: std::ops::RangeInclusive<u32> = 13..=21;
const TILE_H: std::ops::RangeInclusive<u32> = 16..=24;
/// ...and the tiles are this many px apart, give or take two.
const TILE_PITCH: u32 = 18;
/// At most this many digits.
const MAX_TILES: usize = 3;
/// The box around the tiles is this far from them at least: what is checked for its grey.
const BOX_MARGIN: u32 = 2;
/// The box ends this far right of the last tile at most (about 18 px for two digits).
const BOX_REACH: u32 = 48;
/// The text starts this far right of the box, and the bar's next part (HP) this far.
const TEXT_FROM: u32 = 4;
const TEXT_TO: u32 = 200;
/// The job's rows, and the name's, from the tiles' top (the name's descenders included).
const JOB_ROWS: (i32, i32) = (-12, 10);
const NAME_ROWS: (i32, i32) = (10, 32);
/// The area kept of the bar: from this far left of the box's right edge to the bar's HP
/// part, and from the bar's top border to its bottom (rows from the tiles' top).
const AREA_LEFT: u32 = 125;
const AREA_RIGHT: u32 = 206;
const AREA_ROWS: (i32, i32) = (-18, 37);
/// The EXP figures (ADR 0017): from a little left of the "EXP." label, 563 px right of the
/// box's right edge, to just short of the Cash Shop button, 213 px right of the label; and
/// from 12 rows above the brackets' top, 7 above the tiles', to 20 below it. The same on
/// every sample, and where the EXP Bar Region players marked before ADR 0017 had them.
const EXP_FIGURES_COLUMNS: (u32, u32) = (549, 773);
const EXP_FIGURES_ROWS: (i32, i32) = (-19, 13);
/// A glyph on a tile at most this many columns wide is a 1's (5, with its flag): every other
/// digit is wider (the 3 is 10).
const NARROW: usize = 6;
/// The lines are enlarged this many times before ocrs reads them.
const UPSCALE: u32 = 2;
/// A name is this many letters or digits.
const NAME_LEN: std::ops::RangeInclusive<usize> = 2..=12;

/// Reads Status Bar captures with the bundled recogniser (ADR 0002), which it shares with
/// the other readers. It remembers what each line it read said.
pub struct StatusBarReader {
    recogniser: Recogniser,
    read: RememberedLines,
}

impl StatusBarReader {
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled(), read: RememberedLines::default() }
    }

    pub fn read(&mut self, capture: &StatusBarCapture) -> StatusBarReading {
        let bar = frame_to_rgb(&capture.frame).and_then(|img| self.read_rgb(&img, capture.origin));
        StatusBarReading { bar, at_ms: capture.frame.at_ms }
    }

    fn read_rgb(&mut self, img: &RgbImage, origin: ScreenPoint) -> Option<StatusBar> {
        let found = find(img)?;
        let (w, h) = img.dimensions();
        let rows = |(from, to): (i32, i32)| {
            let at = |dy: i32| (found.top as i32 + dy).clamp(0, h as i32) as u32;
            (at(from), at(to))
        };
        let (x0, x1) = (found.box_right + TEXT_FROM, (found.box_right + TEXT_TO).min(w));
        let job = text_ink(img, x0, x1, rows(JOB_ROWS), found.scale);
        let name = text_ink(img, x0, x1, rows(NAME_ROWS), found.scale);
        let level = level_ink(img, &found);
        // Every line there is, read at once: the level's, the job's, the name's.
        let inks: Vec<GrayImage> = [&level.as_ref().map(|(ink, _)| ink.clone()), &job, &name]
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        let mut texts = self.read.read(&self.recogniser, &inks, UPSCALE).into_iter();
        let level = level.and_then(|(_, narrow)| level_of(texts.next().flatten()?.as_str(), &narrow));
        let job = job.and_then(|_| job_of(texts.next().flatten()?.as_str()));
        let name = name.and_then(|_| name_of(texts.next().flatten()?.as_str()));
        let (top, bottom) = rows(AREA_ROWS);
        let left = found.box_right.saturating_sub(AREA_LEFT);
        let right = (found.box_right + AREA_RIGHT).min(w);
        let area = ScreenRect {
            x: origin.x + left as i32,
            y: origin.y + top as i32,
            width: right - left,
            height: bottom - top,
        };
        let (fx0, fx1) = (found.box_right + EXP_FIGURES_COLUMNS.0, found.box_right + EXP_FIGURES_COLUMNS.1);
        let (fy0, fy1) = (found.top as i32 + EXP_FIGURES_ROWS.0, found.top as i32 + EXP_FIGURES_ROWS.1);
        let exp_figures = (fx1 <= w && fy0 >= 0 && fy1 <= h as i32).then(|| ScreenRect {
            x: origin.x + fx0 as i32,
            y: origin.y + fy0,
            width: fx1 - fx0,
            height: (fy1 - fy0) as u32,
        });
        Some(StatusBar { area, level, job, name, exp_figures })
    }
}

/// Where the bar was found: its level tiles (left to right, each its columns, ends
/// exclusive), their top row, the LV box's right edge, and the bar's brightness next to the
/// screenshots' colours.
struct Found {
    tiles: Vec<(u32, u32)>,
    top: u32,
    bottom: u32,
    box_right: u32,
    scale: f32,
}

/// A tile's orange at any brightness: its brightness next to the screenshots' (255,148,21),
/// if `p` is it. The tiles shade to (255,130,21) at their foot and (203,113,24) below.
fn orange(p: [u8; 3]) -> Option<f32> {
    let [r, g, b] = p.map(f32::from);
    let is = r >= 120.0 && (0.46..=0.64).contains(&(g / r)) && b / r <= 0.16;
    is.then_some(r / 255.0)
}

/// The LV box's dark grey (47 to 50, and a shade darker at its foot) at any brightness.
fn box_grey(p: [u8; 3]) -> bool {
    let max = p.iter().copied().max().unwrap_or(0);
    let min = p.iter().copied().min().unwrap_or(0);
    max - min <= 8 && (24..=72).contains(&max)
}

/// A connected run of orange pixels: its bounding box (ends exclusive), and the brightest
/// red in it.
#[derive(Debug, Clone, Copy)]
struct Blob {
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
    scale: f32,
}

/// The orange components (4-connected) of a tile's size.
fn tiles(img: &RgbImage) -> Vec<Blob> {
    let (w, h) = img.dimensions();
    let mut seen = vec![false; (w * h) as usize];
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if seen[(y * w + x) as usize] || orange(img.get_pixel(x, y).0).is_none() {
                continue;
            }
            let mut blob = Blob { x0: x, x1: x + 1, y0: y, y1: y + 1, scale: 0.0 };
            let mut stack = vec![(x, y)];
            seen[(y * w + x) as usize] = true;
            while let Some((cx, cy)) = stack.pop() {
                let k = orange(img.get_pixel(cx, cy).0).unwrap_or(0.0);
                blob = Blob {
                    x0: blob.x0.min(cx),
                    x1: blob.x1.max(cx + 1),
                    y0: blob.y0.min(cy),
                    y1: blob.y1.max(cy + 1),
                    scale: blob.scale.max(k),
                };
                let next = [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)];
                for (nx, ny) in next {
                    if nx < w && ny < h && !seen[(ny * w + nx) as usize] && orange(img.get_pixel(nx, ny).0).is_some() {
                        seen[(ny * w + nx) as usize] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            if TILE_W.contains(&(blob.x1 - blob.x0)) && TILE_H.contains(&(blob.y1 - blob.y0)) {
                out.push(blob);
            }
        }
    }
    out
}

/// The level's tiles in the LV box: tiles in a row at their pitch, with the box's grey all
/// round them, and the box's right edge right of them.
fn find(img: &RgbImage) -> Option<Found> {
    let (w, h) = img.dimensions();
    let mut blobs = tiles(img);
    blobs.sort_by_key(|b| (b.y0, b.x0));
    let in_line = |a: &Blob, b: &Blob| {
        a.y0.abs_diff(b.y0) <= 1 && a.y1.abs_diff(b.y1) <= 1 && (b.x0 as i64 - a.x0 as i64 - TILE_PITCH as i64).abs() <= 2
    };
    let mut i = 0;
    while i < blobs.len() {
        let mut group = vec![blobs[i]];
        while group.len() <= MAX_TILES {
            let last = *group.last().unwrap_or(&blobs[i]);
            match blobs.iter().find(|b| in_line(&last, b)) {
                Some(next) => group.push(*next),
                None => break,
            }
        }
        i += 1;
        // A tile already in a group found from its left isn't a group's first.
        if group.len() > MAX_TILES || blobs.iter().any(|b| in_line(b, &group[0])) {
            continue;
        }
        let (x0, x1) = (group[0].x0, group[group.len() - 1].x1);
        let (top, bottom) = (group.iter().map(|b| b.y0).min()?, group.iter().map(|b| b.y1).max()?);
        if x0 < BOX_MARGIN || top < BOX_MARGIN || bottom + BOX_MARGIN + 1 >= h || x1 + BOX_MARGIN >= w {
            continue;
        }
        // The box all round: above, below (a row of shade between), left, and right.
        let grey = |x: u32, y: u32| box_grey(img.get_pixel(x, y).0);
        let boxed = (x0..x1).all(|x| grey(x, top - BOX_MARGIN) && grey(x, bottom + BOX_MARGIN))
            && (top..bottom).all(|y| grey(x0 - BOX_MARGIN, y) && grey(x1 + BOX_MARGIN - 1, y));
        if !boxed {
            continue;
        }
        // The box's right edge, along a row of the tiles' clear of their glyphs.
        let row = top + 1;
        let Some(box_right) = (x1 + BOX_MARGIN..w.min(x1 + BOX_REACH + 1)).find(|&x| !grey(x, row)) else { continue };
        if box_right == x1 + BOX_MARGIN {
            continue;
        }
        let scale = group.iter().map(|b| b.scale).fold(0.0, f32::max);
        return Some(Found { tiles: group.iter().map(|b| (b.x0, b.x1)).collect(), top, bottom, box_right, scale });
    }
    None
}

/// The tiles' glyphs, black on white, side by side, and which are narrow (a 1's). A glyph is
/// near-white on the orange: its blue is what sets it apart.
fn level_ink(img: &RgbImage, found: &Found) -> Option<(GrayImage, Vec<bool>)> {
    let (y0, y1) = (found.top + 2, found.bottom - 2);
    let ink = |p: [u8; 3]| {
        let (lo, hi) = (21.0 * found.scale, 223.0 * found.scale);
        ((f32::from(p[2]) - lo) / (hi - lo)).clamp(0.0, 1.0)
    };
    let mut glyphs: Vec<(u32, u32)> = Vec::new();
    let mut narrow = Vec::new();
    for &(x0, x1) in &found.tiles {
        let (x0, x1) = (x0 + 2, x1 - 2);
        let inked: Vec<u32> = (x0..x1).filter(|&x| (y0..y1).any(|y| ink(img.get_pixel(x, y).0) >= 0.5)).collect();
        // A tile with no glyph isn't a digit's.
        let (&first, &last) = (inked.first()?, inked.last()?);
        narrow.push((last - first + 1) as usize <= NARROW);
        glyphs.push((x0, x1));
    }
    let width: u32 = glyphs.iter().map(|(x0, x1)| x1 - x0).sum();
    let mut canvas = GrayImage::from_pixel(width, y1 - y0, Luma([255]));
    let mut at = 0;
    for (x0, x1) in glyphs {
        for x in x0..x1 {
            for y in y0..y1 {
                canvas.put_pixel(at + x - x0, y - y0, Luma([(255.0 * (1.0 - ink(img.get_pixel(x, y).0))).round() as u8]));
            }
        }
        at += x1 - x0;
    }
    Some((canvas, narrow))
}

/// Whether `p` is the bar's slate, the text on it, or the anti-aliasing between them, at the
/// bar's brightness `scale`: bluish grey, never darker than the slate. The text can be near
/// white (236 on a live capture whose tiles were the screenshots' orange), so nothing grey is
/// too bright: the pointer and the game's tooltip give themselves away by their black.
fn slate_or_text(p: [u8; 3], scale: f32) -> bool {
    let [r, g, b] = p.map(i32::from);
    let luma = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    (40.0 * scale..=255.0).contains(&luma) && (-8..=24).contains(&(b - r)) && g >= r - 8 && g <= b + 8
}

/// How much `p` is the text's light grey, 0.0 (the slate) ..= 1.0.
fn text(p: [u8; 3], scale: f32) -> f32 {
    let [r, g, b] = p.map(i32::from);
    if (r - g).abs() > 24 || (g - b).abs() > 24 {
        return 0.0;
    }
    let luma = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    ((luma - 120.0 * scale) / (80.0 * scale)).clamp(0.0, 1.0)
}

/// The line of text in columns `x0..x1` and `rows`, black on white, cut to its ink; `None`
/// if nothing is written there or something covers it.
fn text_ink(img: &RgbImage, x0: u32, x1: u32, (y0, y1): (u32, u32), scale: f32) -> Option<GrayImage> {
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let clean = (y0..y1).all(|y| (x0..x1).all(|x| slate_or_text(img.get_pixel(x, y).0, scale)));
    if !clean {
        return None;
    }
    let inked = |x: u32, y: u32| text(img.get_pixel(x, y).0, scale) >= 0.5;
    let cols: Vec<u32> = (x0..x1).filter(|&x| (y0..y1).any(|y| inked(x, y))).collect();
    let rows: Vec<u32> = (y0..y1).filter(|&y| (x0..x1).any(|x| inked(x, y))).collect();
    let (cx0, cx1) = ((*cols.first()?).saturating_sub(1).max(x0), (*cols.last()? + 2).min(x1));
    let (cy0, cy1) = ((*rows.first()?).saturating_sub(1).max(y0), (*rows.last()? + 2).min(y1));
    Some(GrayImage::from_fn(cx1 - cx0, cy1 - cy0, |x, y| {
        Luma([(255.0 * (1.0 - text(img.get_pixel(cx0 + x, cy0 + y).0, scale))).round() as u8])
    }))
}

/// The level from what ocrs read of the tiles' glyphs, if it is one the bar could show: a
/// digit for each tile, a 1 on each narrow glyph and on no other.
fn level_of(read: &str, narrow: &[bool]) -> Option<u32> {
    let digits: Vec<char> = read
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            'O' | 'o' | 'D' => '0',
            'l' | 'I' | '|' | 'i' => '1',
            c => c,
        })
        .collect();
    let shaped = digits.len() == narrow.len()
        && digits.iter().all(char::is_ascii_digit)
        && digits[0] != '0'
        && digits.iter().zip(narrow).all(|(&d, &narrow)| (d == '1') == narrow);
    if !shaped {
        return None;
    }
    let level: u32 = digits.iter().collect::<String>().parse().ok()?;
    (1..=MAX_LEVEL).contains(&level).then_some(level)
}

/// The job read, as the game names it, if it is one of [`JOBS`] (spaces aside).
fn job_of(read: &str) -> Option<String> {
    let bare = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let read = bare(read);
    JOBS.iter().find(|job| bare(job) == read).map(|job| job.to_string())
}

/// The name read, if it is one the game allows: letters and digits only.
fn name_of(read: &str) -> Option<String> {
    let read = read.trim();
    let shaped = NAME_LEN.contains(&read.chars().count()) && read.chars().all(|c| c.is_ascii_alphanumeric());
    shaped.then(|| read.to_string())
}

// What ocrs reads is the reader's own: no public input carries it, and a frame that ocrs
// misreads in just this way can't be made to order. So, as `player_shop.rs`'s prices, the text
// a misread could be is checked here. What the samples read (13, "Magician", "Egbert") is
// checked through the public inputs, in `tests/status_bar_samples.rs`.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_reads_only_as_a_digit_per_tile_with_its_ones_on_narrow_glyphs() {
        assert_eq!(level_of("l3", &[true, false]), Some(13));
        assert_eq!(level_of("1 3", &[true, false]), Some(13));
        assert_eq!(level_of("113", &[true, true, false]), Some(113));
        for (read, narrow) in [
            ("13", &[false, false][..]),
            ("13", &[true, true][..]),
            ("3", &[true, false][..]),
            ("133", &[true, false][..]),
            ("1?", &[true, false][..]),
            ("03", &[false, false][..]),
            ("201", &[false, false, true][..]),
        ] {
            assert_eq!(level_of(read, narrow), None, "{read} {narrow:?}");
        }
    }

    #[test]
    fn a_job_reads_only_as_one_the_game_names() {
        assert_eq!(job_of("White  Knight"), Some("White Knight".to_string()));
        assert_eq!(job_of("Wizard (Fire,Poison)"), Some("Wizard (Fire, Poison)".to_string()));
        for wrong in ["Maqician", "magician", "Magic1an", ""] {
            assert_eq!(job_of(wrong), None, "{wrong}");
        }
    }

    #[test]
    fn a_name_reads_only_as_letters_and_digits() {
        assert_eq!(name_of("Xx2Cool4U"), Some("Xx2Cool4U".to_string()));
        for wrong in ["Eg bert", "Egbert.", "E", "Abcdefghijklm", "Ëgbert"] {
            assert_eq!(name_of(wrong), None, "{wrong}");
        }
    }
}
