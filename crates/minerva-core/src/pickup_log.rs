//! The Pickup Log reader (ADR 0026): the fading lines in the bottom-right corner of the
//! client area that announce gained EXP, mesos and picked-up Items. It finds the log itself,
//! counts the lines that arrived by how far the stack moved since the last clean capture, and
//! reads only those. It never counts a line it isn't sure of.
//!
//! The reader keeps the last clean capture's lines, so one reader reads one stream of
//! captures, in order. What a capture reads as carries the lines that arrived, so a Bug
//! Report's replay needs no pixels.

use crate::ink::{black_on_white, components, luma, runs};
use crate::recogniser::{Recogniser, RememberedLines, frame_to_rgb};
use crate::ui_size::AtSamplesSize;
use crate::{Frame, ScreenPoint, ScreenRect};
use image::{GrayImage, RgbImage};
use serde::{Deserialize, Serialize};

/// How often the Pickup Log is captured while it is read (ms): 10 times a second, on a thread
/// of its own (ADR 0026).
pub const PICKUP_LOG_CAPTURE_MS: u64 = 100;

/// A capture of the band of the client area the Pickup Log is looked for in.
#[derive(Debug, Clone)]
pub struct PickupLogCapture {
    pub frame: Frame,
    /// Where the frame's top-left pixel is on screen.
    pub origin: ScreenPoint,
}

/// What a Pickup Log capture reads as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PickupLogReading {
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
    /// Whether the log was seen, and where.
    pub seen: PickupLogSeen,
    /// Visible line slots, top (0) to bottom (5); faded slots leave gaps in this list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<u8>,
    /// The stack's accepted movement, in rows; zero for a skipped capture or baseline.
    #[serde(default)]
    pub shift: u8,
    /// The lines that arrived since the last clean capture, oldest first. Empty unless
    /// [`PickupLogSeen::Clear`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arrived: Vec<PickupLine>,
    /// More than one shape shift fitted, before colour and brightness broke ties.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub chose: bool,
    /// Every row was new: lines may have come and gone unseen.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub overflowed: bool,
}

/// Whether a capture showed the Pickup Log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PickupLogSeen {
    /// Lines read clean: `area` is where they are on screen.
    Clear { area: ScreenRect },
    /// Lines found, but some of their rows can't be read (a Mob, a skill effect or one of our
    /// windows over them): the capture is skipped, and the next clean one is matched to the
    /// last clean one.
    Covered { area: ScreenRect },
    /// No lines: the log is empty, or isn't where it was looked for.
    Nothing,
}

/// One line of the Pickup Log, by its kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PickupLine {
    /// "You received EXP (+n)": white for the player's own kill (`party: false`), yellow for
    /// a share of a partymate's (`party: true`).
    Exp { amount: u64, party: bool },
    /// "Bonus EXP for PARTY (+n)", on every kill while partied.
    PartyBonus { amount: u64 },
    /// "You have gained mesos (+n)".
    Mesos { amount: u64 },
    /// "<name> x<quantity> earned. (<tab>)". `name` is as read, without the "..." of a name
    /// the game cut short (`cut_short`).
    Item { name: String, quantity: u32, tab: String, cut_short: bool },
    /// A line that arrived but didn't read as any of these: counted as arrived, never as a
    /// pickup.
    Unreadable,
}

/// Reads a stream of Pickup Log captures, in order.
pub struct PickupLogReader {
    recogniser: Recogniser,
    read: RememberedLines,
    previous: Option<Stack>,
    layout: Option<Layout>,
    dimensions: (u32, u32),
}

#[derive(Clone, Copy)]
struct Layout {
    right: u32,
    bottom: u32,
    size: f32,
}

struct Stack {
    rows: Vec<Option<Row>>,
}

struct Row {
    ink: GrayImage,
    contrast: Vec<f32>,
    shape: Vec<usize>,
    yellow: bool,
    fresh: bool,
    readable: bool,
}

impl Default for PickupLogReader {
    fn default() -> Self {
        Self::bundled()
    }
}

impl std::fmt::Debug for PickupLogReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PickupLogReader").field("dimensions", &self.dimensions).finish_non_exhaustive()
    }
}

impl PickupLogReader {
    /// A reader with the bundled recogniser.
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled(), read: RememberedLines::default(), previous: None, layout: None, dimensions: (0, 0) }
    }

    /// Read one capture, the next after the last one read.
    pub fn read(&mut self, capture: &PickupLogCapture) -> PickupLogReading {
        self.read_avoiding(capture, &[])
    }

    /// Read screen pixels, leaving out our windows, as for the Minimap Title. A window
    /// touching the last known log skips the capture, preserving its last clean stack.
    pub fn read_avoiding(&mut self, capture: &PickupLogCapture, ours: &[ScreenRect]) -> PickupLogReading {
        let mut reading = PickupLogReading {
            at_ms: capture.frame.at_ms,
            seen: PickupLogSeen::Nothing,
            rows: Vec::new(),
            shift: 0,
            arrived: Vec::new(),
            chose: false,
            overflowed: false,
        };
        let Some(mut img) = frame_to_rgb(&capture.frame) else { return reading };
        if self.dimensions != img.dimensions() {
            self.previous = None;
            self.layout = None;
            self.dimensions = img.dimensions();
        }
        let overlaps = |a: ScreenRect, b: ScreenRect| {
            (a.x as i64) < b.x as i64 + b.width as i64
                && (b.x as i64) < a.x as i64 + a.width as i64
                && (a.y as i64) < b.y as i64 + b.height as i64
                && (b.y as i64) < a.y as i64 + a.height as i64
        };
        if let Some(layout) = self.layout {
            let area = ScreenRect {
                x: capture.origin.x + layout.right as i32 - (310.0 * layout.size).round() as i32,
                y: capture.origin.y + layout.bottom as i32 - (126.0 * layout.size).round() as i32,
                width: (313.0 * layout.size).ceil() as u32,
                height: (130.0 * layout.size).ceil() as u32,
            };
            if ours.iter().any(|&r| overlaps(r, area)) {
                reading.seen = PickupLogSeen::Covered { area };
                return reading;
            }
        }
        crate::recogniser::blank(&mut img, capture.origin, ours);
        if self.layout.is_none() {
            self.layout = locate(&img);
        }
        let Some(layout) = self.layout else {
            let area = ScreenRect { x: capture.origin.x, y: capture.origin.y, width: img.width(), height: img.height() };
            if ours.iter().any(|&r| overlaps(r, area)) {
                reading.seen = PickupLogSeen::Covered { area };
                return reading;
            }
            self.previous = Some(Stack { rows: (0..6).map(|_| None).collect() });
            return reading;
        };
        // Search the generous capture band once, then scale only the six line slots.
        // Scaling the empty space below them at every tick dominates small UI Sizes.
        let crop_x = layout.right.saturating_sub((314.0 * layout.size).ceil() as u32);
        let crop_y = layout.bottom.saturating_sub((132.0 * layout.size).ceil() as u32);
        let crop_right = (layout.right + (6.0 * layout.size).ceil() as u32).min(img.width());
        let crop_bottom = (layout.bottom + (6.0 * layout.size).ceil() as u32).min(img.height());
        let cropped = image::imageops::crop_imm(&img, crop_x, crop_y, crop_right - crop_x, crop_bottom - crop_y).to_image();
        let at = AtSamplesSize::new(&cropped, layout.size);
        let right = ((layout.right - crop_x) as f32 / layout.size).round() as u32;
        let bottom = ((layout.bottom - crop_y) as f32 / layout.size).round() as u32;
        let rows = (0..6)
            .map(|slot| {
                let top = bottom as i32 - 22 * (5 - slot) - 16;
                row(&at.img, right, top)
            })
            .collect::<Vec<_>>();
        if rows.iter().all(Option::is_none) {
            return reading;
        }
        reading.rows = rows.iter().enumerate().filter_map(|(i, row)| row.as_ref().map(|_| i as u8)).collect();
        let (x0, y0) = at.back(right.saturating_sub(310) as f32, bottom.saturating_sub(126) as f32);
        let (x1, y1) = at.back((right + 3) as f32, (bottom + 4) as f32);
        let area = ScreenRect {
            x: capture.origin.x + crop_x as i32 + x0.round() as i32,
            y: capture.origin.y + crop_y as i32 + y0.round() as i32,
            width: (x1 - x0).round() as u32,
            height: (y1 - y0).round() as u32,
        };
        reading.seen = PickupLogSeen::Clear { area };
        let mut current = Stack { rows };
        if let Some(previous) = &self.previous {
            reading.chose = (0..=6).filter(|&s| fits(previous, &current, s, false)).count() > 1;
            let shifts = (0..=6).filter(|&s| fits(previous, &current, s, true)).collect::<Vec<_>>();
            let Some(&shift) = shifts.first() else {
                reading.seen = PickupLogSeen::Covered { area };
                return reading;
            };
            reading.shift = shift as u8;
            reading.overflowed = shift == 6;
            let new: Vec<_> = current.rows[6 - shift..].iter().flatten().collect();
            let inks = new.iter().map(|r| r.ink.clone()).collect::<Vec<_>>();
            let texts = self.read.read(&self.recogniser, &inks, 3);
            reading.arrived =
                new.iter().zip(texts).map(|(r, t)| t.and_then(|t| parse(&t, r.yellow)).unwrap_or(PickupLine::Unreadable)).collect();
            let missed =
                reading.arrived.iter().enumerate().filter_map(|(i, r)| (*r == PickupLine::Unreadable).then_some(i)).collect::<Vec<_>>();
            let alternatives = missed
                .iter()
                .map(|&i| {
                    let mut ink = inks[i].clone();
                    for p in ink.pixels_mut() {
                        let weight = (255 - p[0]) as f32 / 255.0;
                        p[0] = (255.0 * (1.0 - ((weight * 170.0 - 40.0) / 130.0).clamp(0.0, 1.0))).round() as u8;
                    }
                    ink
                })
                .collect::<Vec<_>>();
            let alternatives = self.read.read(&self.recogniser, &alternatives, 2);
            for (&i, text) in missed.iter().zip(alternatives) {
                if let Some(line) = text.and_then(|t| parse(&t, new[i].yellow)) {
                    reading.arrived[i] = line;
                }
            }
            for i in 0..6 - shift {
                if let (Some(old), Some(new)) = (&previous.rows[i + shift], &mut current.rows[i]) {
                    new.shape = old.shape.clone();
                }
            }
        }
        self.previous = Some(current);
        reading
    }

    /// Start a new stream after capture stops, or the Character changes. Keep the OCR
    /// cache, but establish a new baseline before counting lines already on screen.
    pub fn reset(&mut self) {
        self.previous = None;
        self.layout = None;
    }
}

/// Find the closing parenthesis shared by every right-aligned line. Its 17-row height
/// measures UI Size even when there is only one line; the other parentheses confirm its
/// 22-row pitch. No screen coordinate or Status Bar scale is used by the reader.
fn locate(img: &RgbImage) -> Option<Layout> {
    let mut mask = RgbImage::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let [r, g, b] = p.0;
        let neutral = r.abs_diff(g) < 28 && r.abs_diff(b) < 35;
        let yellow = r.abs_diff(g) < 28 && (b as f32) < r as f32 * 0.6;
        if r.min(g) > 110 && (neutral || yellow) {
            mask.put_pixel(x, y, image::Rgb([255; 3]));
        }
    }
    let candidates = components(&mask, 0..img.width(), 0..img.height(), |p| (p[0] != 0).then_some(1.0));
    let mut ends = Vec::new();
    for c in candidates {
        if !(8..=42).contains(&c.h()) || !(0.16..=0.45).contains(&(c.w() as f32 / c.h() as f32)) {
            continue;
        }
        let size = c.h() as f32 / 17.0;
        let mid = c.y0 + c.h() / 2;
        let left_at = |y| (c.x0..c.x1).find(|&x| mask.get_pixel(x, y)[0] != 0).unwrap_or(c.x1);
        if left_at(mid) <= left_at(c.y0) || left_at(mid) <= left_at(c.y1 - 1) {
            continue;
        }
        let x = c.x1 - 1;
        let shadow = (c.y0..c.y1).filter(|&y| edge(img, x, y, 2).0 > 35.0).count();
        if shadow < (c.h() / 4) as usize {
            continue;
        }
        ends.push(Layout { right: x, bottom: c.y1 - 1, size });
    }
    let aligned_with = |e: &Layout, other: &Layout| {
        let pitch = 22.0 * e.size;
        let dy = other.bottom.abs_diff(e.bottom) as f32;
        other.right.abs_diff(e.right) <= 2 && (other.size - e.size).abs() < 0.1 && (dy / pitch - (dy / pitch).round()).abs() < 0.1
    };
    let mut ranked = ends.iter().map(|e| (ends.iter().filter(|other| aligned_with(e, other)).count(), e)).collect::<Vec<_>>();
    ranked.sort_by_key(|&(count, e)| std::cmp::Reverse((count, e.right)));
    // Only the strongest alignment hypotheses need scaling. A textured world can
    // contain many narrow components; resampling once for every one is not a 10 Hz reader.
    ranked
        .into_iter()
        .take(4)
        .filter_map(|(_, e)| {
            let aligned = ends.iter().filter(|other| aligned_with(e, other)).collect::<Vec<_>>();
            let bottom = aligned.iter().map(|e| e.bottom).max()?;
            let top = aligned.iter().map(|e| e.bottom).min()?;
            let distance = (bottom - top) as f32;
            let steps = (distance / (22.0 * e.size)).round();
            let mut size = if steps >= 1.0 { distance / (22.0 * steps) } else { e.size };
            let mut right = e.right;
            let mut bottom = bottom;
            // A parent's antialiasing may join the background at one end. Its straight
            // right stem still repeats at the line pitch, independently of that join.
            let use_stems = steps < 1.0 && (size - 1.0).abs() > 0.04;
            for x in e.right.saturating_sub(2)..=e.right {
                if !use_stems {
                    break;
                }
                let column = (0..mask.height()).map(|y| mask.get_pixel(x, y)[0] != 0).collect::<Vec<_>>();
                let stems = runs(&column, 1)
                    .into_iter()
                    .filter(|&(a, b)| (b - a) as f32 >= 3.0 * size && (b - a) as f32 <= 10.0 * size)
                    .collect::<Vec<_>>();
                let pairs = stems
                    .windows(2)
                    .filter(|p| {
                        let pitch = (p[1].0 - p[0].0) as f32;
                        (pitch / (22.0 * size) - 1.0).abs() < 0.12
                    })
                    .collect::<Vec<_>>();
                if let Some(pair) = pairs.last() {
                    size = (pair[1].0 - pair[0].0) as f32 / 22.0;
                    right = x;
                    bottom = pair[1].0 + (11.0 * size).round() as u32;
                }
            }
            if !(0..6).any(|slot| {
                bottom
                    .checked_sub((22.0 * slot as f32 * size).round() as u32)
                    .is_some_and(|bottom| parenthesis_scaled(img, right, bottom, size))
            }) {
                return None;
            }
            let crop_x = right.saturating_sub((314.0 * size).ceil() as u32);
            let crop_y = bottom.saturating_sub((132.0 * size).ceil() as u32);
            let crop_right = (right + (6.0 * size).ceil() as u32).min(img.width());
            let crop_bottom = (bottom + (116.0 * size).ceil() as u32).min(img.height());
            let cropped = image::imageops::crop_imm(img, crop_x, crop_y, crop_right - crop_x, crop_bottom - crop_y).to_image();
            let at = AtSamplesSize::new(&cropped, size);
            let scaled_right = ((right - crop_x) as f32 / size).round() as u32;
            let mut scaled_bottom = ((bottom - crop_y) as f32 / size).round() as u32;
            row(&at.img, scaled_right, scaled_bottom as i32 - 16)?;
            for _ in 0..5 {
                if row(&at.img, scaled_right, scaled_bottom as i32 + 22 - 16).is_some_and(|r| r.fresh) {
                    scaled_bottom += 22;
                } else {
                    break;
                }
            }
            let bottom = crop_y + (scaled_bottom as f32 * size).round() as u32;
            if !(0..6).any(|slot| scaled_bottom.checked_sub(22 * slot).is_some_and(|bottom| parenthesis(&at.img, scaled_right, bottom))) {
                return None;
            }
            Some((aligned.len(), right, Layout { bottom, right, size }))
        })
        .max_by_key(|(n, x, _)| (*n, *x))
        .map(|(_, _, l)| l)
}

fn parenthesis(img: &RgbImage, right: u32, bottom: u32) -> bool {
    parenthesis_scaled(img, right, bottom, 1.0)
}

fn parenthesis_scaled(img: &RgbImage, right: u32, bottom: u32, size: f32) -> bool {
    let at = |n: f32| (n * size).round() as u32;
    if right < at(3.0) || bottom < at(16.0) {
        return false;
    }
    let top = bottom - at(16.0);
    let near = |x: u32, y: u32| {
        (x.saturating_sub(1)..=x + 1)
            .flat_map(|x| (y.saturating_sub(1)..=y + 1).map(move |y| edge(img, x, y, at(2.0).max(1)).0))
            .fold(0.0, f32::max)
    };
    let stem = (5..=11).filter(|&y| near(right, top + at(y as f32)) > 35.0).count();
    let corners = [(right - at(3.0), top), (right - at(2.0), top + at(2.0)), (right - at(2.0), top + at(13.0)), (right - at(3.0), bottom)];
    stem >= 5 && corners.iter().filter(|&&(x, y)| near(x, y) > 25.0).count() >= 3
}

fn edge(img: &RgbImage, x: u32, y: u32, distance: u32) -> (f32, bool) {
    if x + distance >= img.width() || y + distance >= img.height() {
        return (0.0, false);
    }
    let p = img.get_pixel(x, y).0;
    let q = img.get_pixel(x + distance, y + distance).0;
    let d = [p[0] as f32 - q[0] as f32, p[1] as f32 - q[1] as f32, p[2] as f32 - q[2] as f32];
    let yellow = d[0].min(d[1]) > d[2] * 1.7 && d[0].min(d[1]) > 30.0;
    (if yellow { (d[0] + d[1]) / 2.0 } else { luma(p) - luma(q) }, yellow)
}

fn row(img: &RgbImage, right: u32, top: i32) -> Option<Row> {
    if top < 1 || top as u32 + 18 >= img.height() || right < 100 || right + 3 >= img.width() {
        return None;
    }
    let left = right.saturating_sub(310);
    let width = right - left + 1;
    let mut contrast = Vec::with_capacity(width as usize * 17);
    let mut strong = 0;
    let mut yellow = 0;
    let mut end = 0;
    let mut bright = 0;
    let mut shape = Vec::new();
    for y in top as u32..top as u32 + 17 {
        for x in left..=right {
            let (e, is_yellow) = edge(img, x, y, 2);
            contrast.push(e);
            if e > 110.0 {
                strong += 1;
                yellow += usize::from(is_yellow);
                if x + 4 >= right {
                    end += 1;
                }
            }
            let [r, g, b] = img.get_pixel(x, y).0;
            if r.min(g) > 160 && (r.abs_diff(b) < 35 || b < 90) && e > 80.0 {
                shape.push(contrast.len() - 1);
            }
            if r.min(g) > 220 && (r.abs_diff(b) < 35 || b < 90) {
                bright += 1;
            }
        }
    }
    let visible = contrast.iter().filter(|&&v| v > 40.0).count();
    if visible < 60 {
        return None;
    }
    let readable = strong > 80 && end >= 4;
    let fresh = readable && bright >= 50;
    let columns = (left..=right)
        .map(|x| {
            (top as u32..top as u32 + 17).any(|y| {
                let [r, g, b] = img.get_pixel(x, y).0;
                r.min(g) > 180 && (r.abs_diff(b) < 35 || b < 90) && edge(img, x, y, 2).0 > 65.0
            })
        })
        .collect::<Vec<_>>();
    let start = runs(&columns, 20).last().map_or(0, |&(x, _)| x.saturating_sub(1));
    let ink = black_on_white(width - start, 19, 2, |x, y| {
        let [r, g, b] = img.get_pixel(left + start + x, top as u32 + y - 1).0;
        let white = r.abs_diff(g) < 30 && r.abs_diff(b) < 35;
        let yellow = r.abs_diff(g) < 30 && (b as f32) < r as f32 * 0.6;
        if white || yellow { ((r.min(g) as f32 - 60.0) / 170.0).clamp(0.0, 1.0) } else { 0.0 }
    });
    Some(Row { ink, contrast, shape, yellow: yellow * 2 > strong, readable, fresh })
}

fn same(old: &Row, new: &Row, tie_break: bool) -> bool {
    if tie_break && old.yellow != new.yellow && old.fresh && new.fresh {
        return false;
    }
    let points =
        old.shape.iter().filter_map(|&i| (old.contrast[i] > 20.0).then_some((&old.contrast[i], &new.contrast[i]))).collect::<Vec<_>>();
    if points.len() < 40 {
        return false;
    }
    let mut ratios = points.iter().map(|(a, b)| **b / **a).collect::<Vec<_>>();
    ratios.sort_by(f32::total_cmp);
    let alpha = ratios[ratios.len() / 2];
    let matched = points.iter().filter(|(a, b)| (**b - alpha * **a).abs() < 35.0).count();
    let width = old.contrast.len() / 17;
    let ending = old.shape.iter().filter(|&&i| i % width >= width - 4).collect::<Vec<_>>();
    let end_matches = ending.iter().filter(|&&&i| new.contrast[i] > old.contrast[i] * alpha * 0.5).count();
    let tail = old.shape.iter().filter(|&&i| i % width >= width - 45 && old.contrast[i] > 20.0).collect::<Vec<_>>();
    let tail_matches = tail.iter().filter(|&&&i| (new.contrast[i] - alpha * old.contrast[i]).abs() < 35.0).count();
    alpha > 0.15
        && (!tie_break || alpha <= 1.08)
        && matched * 100 >= points.len() * 65
        && end_matches * 2 >= ending.len()
        && tail_matches * 100 >= tail.len() * 80
}

fn fits(old: &Stack, new: &Stack, shift: usize, tie_break: bool) -> bool {
    let anchored = (0..6 - shift).any(|i| match (&old.rows[i + shift], &new.rows[i]) {
        (Some(a), Some(b)) => same(a, b, tie_break),
        _ => false,
    });
    if new.rows[6 - shift..].iter().any(|r| r.as_ref().is_none_or(|r| !r.fresh && !(anchored && r.readable))) {
        return false;
    }
    for i in 0..6 - shift {
        match (&old.rows[i + shift], &new.rows[i]) {
            (Some(a), Some(b)) if b.fresh && !same(a, b, tie_break) || a.fresh && !b.readable => return false,
            (Some(a), None) if a.fresh => return false,
            (None, Some(b)) if b.fresh => return false,
            _ => {}
        }
    }
    true
}

fn parse(text: &str, yellow: bool) -> Option<PickupLine> {
    let text = text.trim();
    let letters = |s: &str| s.chars().filter(char::is_ascii_alphabetic).flat_map(char::to_lowercase).collect::<Vec<_>>();
    for (prefix, kind) in [("You received EXP", 0), ("Bonus EXP for PARTY", 1), ("You have gained mesos", 2)] {
        if let Some((label, tail)) = text.rsplit_once('(')
            && crate::matcher::levenshtein(&letters(label), &letters(prefix)) <= 2
        {
            let number = tail.trim().strip_prefix('+')?.strip_suffix(')')?;
            let amount = number.replace(',', "").parse().ok()?;
            return Some(match kind {
                0 => PickupLine::Exp { amount, party: yellow },
                1 => PickupLine::PartyBonus { amount },
                _ => PickupLine::Mesos { amount },
            });
        }
    }
    let (before, tab) = text.rsplit_once('.')?;
    let tab = tab.trim().strip_prefix('(')?.strip_suffix(')')?;
    if !["Equip", "Use", "Setup", "Etc", "Cash"].contains(&tab) {
        return None;
    }
    let before = before.strip_suffix(" earned")?;
    let (name, quantity) = before.rsplit_once(' ')?;
    let quantity = quantity.strip_prefix('x').or_else(|| quantity.strip_prefix('?'))?;
    let quantity = quantity.parse().ok().filter(|&n| n > 0)?;
    let cut_short = name.ends_with("...");
    let name = name.trim_end_matches("...").to_string();
    (!name.is_empty()).then_some(PickupLine::Item { name, quantity, tab: tab.into(), cut_short })
}
