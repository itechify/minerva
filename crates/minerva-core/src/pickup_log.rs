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

// Geometry at the samples' UI Size, relative to the newest line's closing parenthesis.
const ROWS: usize = 6;
const ROW_PITCH: u32 = 22;
const TEXT_HEIGHT: u32 = 17;
const TEXT_LEFT: u32 = 310;
const STACK_TOP: u32 = (ROWS as u32 - 1) * ROW_PITCH + TEXT_HEIGHT - 1;
const AREA_RIGHT: u32 = 3;
const AREA_BOTTOM: u32 = 4;
const CROP_MARGIN: u32 = 6;
const CROP_LEFT: u32 = TEXT_LEFT + 4;
const CROP_TOP: u32 = STACK_TOP + CROP_MARGIN;
const SEARCH_WIDTH: f32 = 400.0;
const SEARCH_HEIGHT: f32 = 480.0;
const STATUS_BAR_HEIGHT: f32 = 55.0;
/// How far the lines' last pixel is from the client area's right edge, at the samples' size:
/// 11 to 12 px in both recordings and the Bug Report that found textures instead (ADR 0026),
/// and 6 px in the committed crops, which were cut a little inside it. The log is looked for
/// only this close to the band's right edge, whatever is further left.
const RIGHT_MARGIN: std::ops::RangeInclusive<f32> = 2.0..=16.0;
/// How far a located layout's size may differ from the UI Size the Status Bar measured, as a
/// share of it.
const SIZE_TOLERANCE: f32 = 0.15;
/// How long a located layout may go on without one of its rows reading as a known line kind
/// before it is given up and searched for again (ms).
const UNCONFIRMED_DROP_MS: u64 = 3000;

/// The generous discovery band, anchored to the client's bottom-right and sized by the
/// Status Bar. The reader measures the lines' own UI Size within it.
pub(crate) fn capture_band(client: ScreenRect, status_bar: ScreenRect) -> ScreenRect {
    let size = status_bar.height as f32 / STATUS_BAR_HEIGHT;
    let width = ((SEARCH_WIDTH * size).ceil() as u32).min(client.width);
    let height = ((SEARCH_HEIGHT * size).ceil() as u32).min(client.height);
    ScreenRect { x: client.x + (client.width - width) as i32, y: client.y + (client.height - height) as i32, width, height }
}

/// The UI Size the Status Bar measured, in thousandths: what the band is sized by.
pub(crate) fn ui_size_permille(status_bar: ScreenRect) -> u32 {
    (status_bar.height as f32 * 1000.0 / STATUS_BAR_HEIGHT).round() as u32
}

/// A capture of the band of the client area the Pickup Log is looked for in.
#[derive(Debug, Clone)]
pub struct PickupLogCapture {
    pub frame: Frame,
    /// Where the frame's top-left pixel is on screen.
    pub origin: ScreenPoint,
    /// The UI Size the Status Bar measured, as the band was made (ADR 0020): the log's lines
    /// are drawn at it, and end a fixed small distance from the frame's right edge, which is
    /// the client area's. It is [`PanelState::pickup_log_ui_size_permille`](crate::PanelState)
    /// over 1000.
    pub ui_size: f32,
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
    /// Whether one of the layout's rows has read as a known line kind: until one has, the layout
    /// may be a lookalike, and nothing it shows is counted.
    confirmed: bool,
    /// When the layout was located, while it is unconfirmed.
    located_at_ms: Option<u64>,
    dimensions: (u32, u32),
    ui_size: f32,
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
        Self { recogniser: Recogniser::bundled(), read: RememberedLines::default(), previous: None, layout: None, confirmed: false, located_at_ms: None, dimensions: (0, 0), ui_size: 0.0 }
    }

    /// Read one capture, the next after the last one read.
    pub fn read(&mut self, capture: &PickupLogCapture) -> PickupLogReading {
        self.read_avoiding(capture, &[])
    }

    /// Read screen pixels, leaving out our windows, as for the Minimap Title. A window
    /// touching the last known log skips the capture, preserving its last clean stack.
    pub fn read_avoiding(&mut self, capture: &PickupLogCapture, ours: &[ScreenRect]) -> PickupLogReading {
        let reading = self.read_located(capture, ours);
        // A line that arrived and read as a known kind confirms the layout too: a fresh line is
        // the easiest to read, where the rows already there, faded, often aren't.
        self.confirmed |= self.layout.is_some() && reading.arrived.iter().any(|line| *line != PickupLine::Unreadable);
        // A layout none of whose rows has read as a known line kind may be a lookalike: it
        // is followed, so its first arrivals are counted once it is confirmed, but it reports
        // nothing until then, and a miss is dropped ([`Self::confirm`]).
        if !self.confirmed && reading.seen != PickupLogSeen::Nothing {
            return PickupLogReading { at_ms: reading.at_ms, seen: PickupLogSeen::Nothing, rows: Vec::new(), shift: 0, arrived: Vec::new(), chose: false, overflowed: false };
        }
        reading
    }

    fn read_located(&mut self, capture: &PickupLogCapture, ours: &[ScreenRect]) -> PickupLogReading {
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
        if self.dimensions != img.dimensions() || self.ui_size != capture.ui_size {
            self.reset();
            self.dimensions = img.dimensions();
            self.ui_size = capture.ui_size;
        }
        if let Some(layout) = self.layout {
            let area = ScreenRect {
                x: capture.origin.x + layout.right as i32 - (TEXT_LEFT as f32 * layout.size).round() as i32,
                y: capture.origin.y + layout.bottom as i32 - (STACK_TOP as f32 * layout.size).round() as i32,
                width: ((TEXT_LEFT + AREA_RIGHT) as f32 * layout.size).ceil() as u32,
                height: ((STACK_TOP + AREA_BOTTOM) as f32 * layout.size).ceil() as u32,
            };
            if ours.iter().any(|&r| r.overlaps(area)) {
                reading.seen = PickupLogSeen::Covered { area };
                return reading;
            }
        }
        crate::recogniser::blank(&mut img, capture.origin, ours);
        if self.layout.is_none() {
            self.layout = locate_log(&img, capture.ui_size);
            self.located_at_ms = self.layout.map(|_| capture.frame.at_ms);
        }
        let Some(layout) = self.layout else {
            // Until the log has been located, no lines could equally mean a Mob
            // covers it. The first clean sighting must establish the baseline.
            return reading;
        };
        // Search the generous capture band once, then scale only the six line slots.
        // Scaling the empty space below them at every tick dominates small UI Sizes.
        let crop_x = layout.right.saturating_sub((CROP_LEFT as f32 * layout.size).ceil() as u32);
        let crop_y = layout.bottom.saturating_sub((CROP_TOP as f32 * layout.size).ceil() as u32);
        let crop_right = (layout.right + (CROP_MARGIN as f32 * layout.size).ceil() as u32).min(img.width());
        let crop_bottom = (layout.bottom + (CROP_MARGIN as f32 * layout.size).ceil() as u32).min(img.height());
        let cropped = image::imageops::crop_imm(&img, crop_x, crop_y, crop_right - crop_x, crop_bottom - crop_y).to_image();
        let at = AtSamplesSize::new(&cropped, layout.size);
        let right = ((layout.right - crop_x) as f32 / layout.size).round() as u32;
        let bottom = ((layout.bottom - crop_y) as f32 / layout.size).round() as u32;
        let rows = (0..ROWS)
            .map(|slot| {
                let top = bottom as i32 - ROW_PITCH as i32 * (ROWS - 1 - slot) as i32 - (TEXT_HEIGHT - 1) as i32;
                read_row(&at.img, right, top)
            })
            .collect::<Vec<_>>();
        if !self.confirmed {
            self.confirm(&rows, capture.frame.at_ms);
        }
        if rows.iter().all(Option::is_none) {
            return reading;
        }
        reading.rows = rows.iter().enumerate().filter_map(|(i, row)| row.as_ref().map(|_| i as u8)).collect();
        let (x0, y0) = at.back(right.saturating_sub(TEXT_LEFT) as f32, bottom.saturating_sub(STACK_TOP) as f32);
        let (x1, y1) = at.back((right + AREA_RIGHT) as f32, (bottom + AREA_BOTTOM) as f32);
        let area = ScreenRect {
            x: capture.origin.x + crop_x as i32 + x0.round() as i32,
            y: capture.origin.y + crop_y as i32 + y0.round() as i32,
            width: (x1 - x0).round() as u32,
            height: (y1 - y0).round() as u32,
        };
        reading.seen = PickupLogSeen::Clear { area };
        let mut current = Stack { rows };
        if let Some(previous) = &self.previous {
            reading.chose = (0..=ROWS).filter(|&s| stack_matches_shift(previous, &current, s, false)).count() > 1;
            let shifts = (0..=ROWS).filter(|&s| stack_matches_shift(previous, &current, s, true)).collect::<Vec<_>>();
            let Some(&shift) = shifts.first() else {
                reading.seen = PickupLogSeen::Covered { area };
                return reading;
            };
            reading.shift = shift as u8;
            reading.overflowed = shift == ROWS;
            for i in 0..ROWS - shift {
                if let (Some(old), Some(new)) = (&previous.rows[i + shift], &mut current.rows[i]) {
                    new.shape = old.shape.clone();
                }
            }
            reading.arrived = self.read_arrivals(&current.rows[ROWS - shift..]);
        }
        self.previous = Some(current);
        reading
    }

    /// Whether a row of a located layout that isn't yet trusted reads as a known line kind. If
    /// none has for [`UNCONFIRMED_DROP_MS`], the layout was a miss: it is dropped, to be
    /// searched for again, so a lookalike is never kept.
    fn confirm(&mut self, rows: &[Option<Row>], at_ms: u64) {
        let readable = rows.iter().flatten().filter(|r| r.readable).collect::<Vec<_>>();
        let inks = readable.iter().map(|r| r.ink.clone()).collect::<Vec<_>>();
        let known = self.read.read(&self.recogniser, &inks, 3).into_iter().zip(&readable).any(|(text, r)| text.is_some_and(|t| parse(&t, r.yellow).is_some()));
        if known {
            self.confirmed = true;
        } else if self.located_at_ms.is_some_and(|at| at_ms.saturating_sub(at) >= UNCONFIRMED_DROP_MS) {
            self.previous = None;
            self.layout = None;
            self.located_at_ms = None;
        }
    }

    fn read_arrivals(&mut self, rows: &[Option<Row>]) -> Vec<PickupLine> {
        let new: Vec<_> = rows.iter().flatten().collect();
        let inks = new.iter().map(|r| r.ink.clone()).collect::<Vec<_>>();
        let texts = self.read.read(&self.recogniser, &inks, 3);
        let mut arrived: Vec<PickupLine> =
            new.iter().zip(texts).map(|(r, t)| t.and_then(|t| parse(&t, r.yellow)).unwrap_or(PickupLine::Unreadable)).collect();
        let missed =
            arrived.iter().enumerate().filter_map(|(i, r)| (*r == PickupLine::Unreadable).then_some(i)).collect::<Vec<_>>();
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
                arrived[i] = line;
            }
        }
        arrived
    }

    /// Start a new stream after capture stops, or the Character changes. Keep the OCR
    /// cache, but establish a new baseline before counting lines already on screen.
    pub fn reset(&mut self) {
        self.previous = None;
        self.layout = None;
        self.confirmed = false;
        self.located_at_ms = None;
    }
}

/// Find the closing parenthesis shared by every right-aligned line, only within
/// [`RIGHT_MARGIN`] of the frame's right edge and at `ui_size` (the Status Bar's): the log is
/// drawn there, and a stack of lookalike shapes in the world behind it is not it (ADR 0026).
/// The parenthesis's 17-row height is checked against that size even with only one line; the
/// other parentheses confirm its 22-row pitch.
fn locate_log(img: &RgbImage, ui_size: f32) -> Option<Layout> {
    let (mask, ends) = closing_parentheses(img, ui_size);
    let mut ranked = ends.iter().map(|e| (ends.iter().filter(|other| aligned_with(e, other)).count(), e)).collect::<Vec<_>>();
    ranked.sort_by_key(|&(count, e)| std::cmp::Reverse((count, e.right)));
    // Only the strongest alignment hypotheses need scaling. A textured world can
    // contain many narrow components; resampling once for every one is not a 10 Hz reader.
    ranked
        .into_iter()
        .take(4)
        .filter_map(|(_, e)| refine_layout(img, &mask, &ends, e))
        .filter(|(_, _, l)| fits(img, l, ui_size))
        .max_by_key(|(n, x, _)| (*n, *x))
        .map(|(_, _, l)| l)
}

/// Whether a layout's last pixel is within [`RIGHT_MARGIN`] of the frame's right edge and its
/// size is near `ui_size`.
fn fits(img: &RgbImage, layout: &Layout, ui_size: f32) -> bool {
    let from_edge = (img.width() - 1).saturating_sub(layout.right) as f32 / ui_size;
    RIGHT_MARGIN.contains(&from_edge) && (layout.size - ui_size).abs() <= SIZE_TOLERANCE * ui_size
}

fn closing_parentheses(img: &RgbImage, ui_size: f32) -> (RgbImage, Vec<Layout>) {
    let mut mask = RgbImage::new(img.width(), img.height());
    // Only the columns the log's last glyphs can be in: the rest is the world.
    let from = img.width().saturating_sub(((RIGHT_MARGIN.end() + 12.0) * ui_size).ceil() as u32);
    for (x, y, p) in img.enumerate_pixels() {
        if x < from {
            continue;
        }
        let [r, g, b] = p.0;
        let neutral = r.abs_diff(g) < 28 && r.abs_diff(b) < 35;
        let yellow = r.abs_diff(g) < 28 && (b as f32) < r as f32 * 0.6;
        if r.min(g) > 110 && (neutral || yellow) {
            mask.put_pixel(x, y, image::Rgb([255; 3]));
        }
    }
    let candidates = components(&mask, from..img.width(), 0..img.height(), |p| (p[0] != 0).then_some(1.0));
    let mut ends = Vec::new();
    for c in candidates {
        if !(8..=42).contains(&c.h()) || !(0.16..=0.45).contains(&(c.w() as f32 / c.h() as f32)) {
            continue;
        }
        let size = c.h() as f32 / TEXT_HEIGHT as f32;
        let mid = c.y0 + c.h() / 2;
        let left_at = |y| (c.x0..c.x1).find(|&x| mask.get_pixel(x, y)[0] != 0).unwrap_or(c.x1);
        if left_at(mid) <= left_at(c.y0) || left_at(mid) <= left_at(c.y1 - 1) {
            continue;
        }
        let x = c.x1 - 1;
        let shadow = (c.y0..c.y1).filter(|&y| shadow_contrast(img, x, y, 2).0 > 35.0).count();
        if shadow < (c.h() / 4) as usize {
            continue;
        }
        let layout = Layout { right: x, bottom: c.y1 - 1, size };
        if fits(img, &layout, ui_size) {
            ends.push(layout);
        }
    }
    (mask, ends)
}

fn aligned_with(e: &Layout, other: &Layout) -> bool {
    let pitch = ROW_PITCH as f32 * e.size;
    let dy = other.bottom.abs_diff(e.bottom) as f32;
    other.right.abs_diff(e.right) <= 2 && (other.size - e.size).abs() < 0.1 && (dy / pitch - (dy / pitch).round()).abs() < 0.1
}

fn refine_layout(img: &RgbImage, mask: &RgbImage, ends: &[Layout], e: &Layout) -> Option<(usize, u32, Layout)> {
    let aligned = ends.iter().filter(|other| aligned_with(e, other)).collect::<Vec<_>>();
    let bottom = aligned.iter().map(|e| e.bottom).max()?;
    let top = aligned.iter().map(|e| e.bottom).min()?;
    let distance = (bottom - top) as f32;
    let steps = (distance / (ROW_PITCH as f32 * e.size)).round();
    let mut size = if steps >= 1.0 { distance / (ROW_PITCH as f32 * steps) } else { e.size };
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
                (pitch / (ROW_PITCH as f32 * size) - 1.0).abs() < 0.12
            })
            .collect::<Vec<_>>();
        if let Some(pair) = pairs.last() {
            size = (pair[1].0 - pair[0].0) as f32 / ROW_PITCH as f32;
            right = x;
            bottom = pair[1].0 + (11.0 * size).round() as u32;
        }
    }
    if !(0..ROWS).any(|slot| {
        bottom
            .checked_sub((ROW_PITCH as f32 * slot as f32 * size).round() as u32)
            .is_some_and(|bottom| parenthesis_scaled(img, right, bottom, size))
    }) {
        return None;
    }
    let crop_x = right.saturating_sub((CROP_LEFT as f32 * size).ceil() as u32);
    let crop_y = bottom.saturating_sub((CROP_TOP as f32 * size).ceil() as u32);
    let crop_right = (right + (CROP_MARGIN as f32 * size).ceil() as u32).min(img.width());
    let crop_bottom = (bottom + (((ROWS as u32 - 1) * ROW_PITCH + CROP_MARGIN) as f32 * size).ceil() as u32).min(img.height());
    let cropped = image::imageops::crop_imm(img, crop_x, crop_y, crop_right - crop_x, crop_bottom - crop_y).to_image();
    let at = AtSamplesSize::new(&cropped, size);
    let scaled_right = ((right - crop_x) as f32 / size).round() as u32;
    let mut scaled_bottom = ((bottom - crop_y) as f32 / size).round() as u32;
    read_row(&at.img, scaled_right, scaled_bottom as i32 - (TEXT_HEIGHT - 1) as i32)?;
    for _ in 0..ROWS - 1 {
        if read_row(&at.img, scaled_right, scaled_bottom as i32 + ROW_PITCH as i32 - (TEXT_HEIGHT - 1) as i32).is_some_and(|r| r.fresh) {
            scaled_bottom += ROW_PITCH;
        } else {
            break;
        }
    }
    let bottom = crop_y + (scaled_bottom as f32 * size).round() as u32;
    if !(0..ROWS).any(|slot| scaled_bottom.checked_sub(ROW_PITCH * slot as u32).is_some_and(|bottom| parenthesis(&at.img, scaled_right, bottom))) {
        return None;
    }
    Some((aligned.len(), right, Layout { bottom, right, size }))
}


fn parenthesis(img: &RgbImage, right: u32, bottom: u32) -> bool {
    parenthesis_scaled(img, right, bottom, 1.0)
}

fn parenthesis_scaled(img: &RgbImage, right: u32, bottom: u32, size: f32) -> bool {
    let at = |n: f32| (n * size).round() as u32;
    if right < at(3.0) || bottom < at((TEXT_HEIGHT - 1) as f32) {
        return false;
    }
    let top = bottom - at((TEXT_HEIGHT - 1) as f32);
    let near = |x: u32, y: u32| {
        (x.saturating_sub(1)..=x + 1)
            .flat_map(|x| (y.saturating_sub(1)..=y + 1).map(move |y| shadow_contrast(img, x, y, at(2.0).max(1)).0))
            .fold(0.0, f32::max)
    };
    let stem = (5..=11).filter(|&y| near(right, top + at(y as f32)) > 35.0).count();
    let corners = [(right - at(3.0), top), (right - at(2.0), top + at(2.0)), (right - at(2.0), top + at(13.0)), (right - at(3.0), bottom)];
    stem >= 5 && corners.iter().filter(|&&(x, y)| near(x, y) > 25.0).count() >= 3
}

fn shadow_contrast(img: &RgbImage, x: u32, y: u32, distance: u32) -> (f32, bool) {
    if x + distance >= img.width() || y + distance >= img.height() {
        return (0.0, false);
    }
    let p = img.get_pixel(x, y).0;
    let q = img.get_pixel(x + distance, y + distance).0;
    let d = [p[0] as f32 - q[0] as f32, p[1] as f32 - q[1] as f32, p[2] as f32 - q[2] as f32];
    let yellow = d[0].min(d[1]) > d[2] * 1.7 && d[0].min(d[1]) > 30.0;
    (if yellow { (d[0] + d[1]) / 2.0 } else { luma(p) - luma(q) }, yellow)
}

fn read_row(img: &RgbImage, right: u32, top: i32) -> Option<Row> {
    if top < 1 || top as u32 + TEXT_HEIGHT + 1 >= img.height() || right < 100 || right + AREA_RIGHT >= img.width() {
        return None;
    }
    let left = right.saturating_sub(TEXT_LEFT);
    let width = right - left + 1;
    let mut contrast = Vec::with_capacity(width as usize * TEXT_HEIGHT as usize);
    let mut strong = 0;
    let mut yellow = 0;
    let mut end = 0;
    let mut bright = 0;
    let mut shape = Vec::new();
    for y in top as u32..top as u32 + TEXT_HEIGHT {
        for x in left..=right {
            let (e, is_yellow) = shadow_contrast(img, x, y, 2);
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
            (top as u32..top as u32 + TEXT_HEIGHT).any(|y| {
                let [r, g, b] = img.get_pixel(x, y).0;
                r.min(g) > 180 && (r.abs_diff(b) < 35 || b < 90) && shadow_contrast(img, x, y, 2).0 > 65.0
            })
        })
        .collect::<Vec<_>>();
    let start = runs(&columns, 20).last().map_or(0, |&(x, _)| x.saturating_sub(1));
    let ink = black_on_white(width - start, TEXT_HEIGHT + 2, 2, |x, y| {
        let [r, g, b] = img.get_pixel(left + start + x, top as u32 + y - 1).0;
        let white = r.abs_diff(g) < 30 && r.abs_diff(b) < 35;
        let yellow = r.abs_diff(g) < 30 && (b as f32) < r as f32 * 0.6;
        if white || yellow { ((r.min(g) as f32 - 60.0) / 170.0).clamp(0.0, 1.0) } else { 0.0 }
    });
    Some(Row { ink, contrast, shape, yellow: yellow * 2 > strong, readable, fresh })
}

fn rows_match(old: &Row, new: &Row, tie_break: bool) -> bool {
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
    let width = old.contrast.len() / TEXT_HEIGHT as usize;
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

fn stack_matches_shift(old: &Stack, new: &Stack, shift: usize, tie_break: bool) -> bool {
    let anchored = (0..ROWS - shift).any(|i| match (&old.rows[i + shift], &new.rows[i]) {
        (Some(a), Some(b)) => rows_match(a, b, tie_break),
        _ => false,
    });
    if new.rows[ROWS - shift..].iter().any(|r| r.as_ref().is_none_or(|r| !r.fresh && !(anchored && r.readable))) {
        return false;
    }
    for i in 0..ROWS - shift {
        match (&old.rows[i + shift], &new.rows[i]) {
            (Some(a), Some(b)) if b.fresh && !rows_match(a, b, tie_break) || a.fresh && !b.readable => return false,
            (Some(a), None) if a.fresh => return false,
            (None, Some(b)) if b.fresh => return false,
            _ => {}
        }
    }
    true
}

enum GainKind { OwnOrPartyExp, PartyBonus, Mesos }

impl GainKind {
    fn prefix(&self) -> &'static str {
        match self { Self::OwnOrPartyExp => "You received EXP", Self::PartyBonus => "Bonus EXP for PARTY", Self::Mesos => "You have gained mesos" }
    }
    fn line(self, amount: u64, yellow: bool) -> PickupLine {
        match self { Self::OwnOrPartyExp => PickupLine::Exp { amount, party: yellow }, Self::PartyBonus => PickupLine::PartyBonus { amount }, Self::Mesos => PickupLine::Mesos { amount } }
    }
}

fn parse(text: &str, yellow: bool) -> Option<PickupLine> {
    let text = text.trim();
    let letters = |s: &str| s.chars().filter(char::is_ascii_alphabetic).flat_map(char::to_lowercase).collect::<Vec<_>>();
    for kind in [GainKind::OwnOrPartyExp, GainKind::PartyBonus, GainKind::Mesos] {
        if let Some((label, tail)) = text.rsplit_once('(')
            && crate::matcher::levenshtein(&letters(label), &letters(kind.prefix())) <= 2
        {
            let number = tail.trim().strip_prefix('+')?.strip_suffix(')')?;
            let amount = number.replace(',', "").parse().ok()?;
            return Some(kind.line(amount, yellow));
        }
    }
    let (before, tab) = text.rsplit_once('.')?;
    let tab = tab.trim().strip_prefix('(')?.strip_suffix(')')?;
    let tab = crate::osms_items::InventoryTab::named(tab)?.as_str();
    let before = before.strip_suffix(" earned")?;
    let (name, quantity) = before.rsplit_once(' ')?;
    let quantity = quantity.strip_prefix('x').or_else(|| quantity.strip_prefix('?'))?;
    let quantity = quantity.parse().ok().filter(|&n| n > 0)?;
    let cut_short = name.ends_with("...");
    let name = name.trim_end_matches("...").to_string();
    (!name.is_empty()).then_some(PickupLine::Item { name, quantity, tab: tab.into(), cut_short })
}
