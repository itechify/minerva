//! The Player Shop reader (ADR 0014): finds an open Player Shop's item list in a capture of
//! the game's client area and reads each row's name and price, for the Deal Finder.
//!
//! The game draws the shop's UI at the UI Size its setting and the window's size make, wherever
//! on the client area the window is (ADR 0020). The sizes below are the samples':
//!
//! 1. Each listing is a light blue-grey band (202,213,223) exactly [`BAND_W`] px wide, with
//!    its item's name on one line and "1 for 999 mesos" on the line [`PRICE_BELOW`] px
//!    below it. A band row is a scanline run of that colour; the list is where many such
//!    runs of one width end at the same column, at any width in [`BAND_WIDTHS`]: the UI Size
//!    is that width next to [`BAND_W`]. At another UI Size than the samples', the list's part
//!    of the capture is scaled to theirs, smoothly, and read there. Live captures show the
//!    game's colours a little darker than screenshots do (`prep.rs`), so the band is matched
//!    at any brightness, as the minimap's panel is.
//! 2. The text is black, blended into the band where it doesn't cover a pixel fully, so
//!    every pixel of a listing's lines lies between the band's colour and black. A pixel
//!    that doesn't (the game's pointer, a window of ours) means the line is covered: the
//!    row is unreadable, never misread.
//! 3. A sold-out listing is greyed: its band isn't the listing colour, so it isn't read.
//!    An empty slot is a band with no text. The listing the player selected is orange
//!    (251,167,46) from 7 px into its band, its text black as on the others, and is read
//!    as they are.
//! 4. A slot where a listing was read before, at the same spot, that now has none and holds
//!    pixels the shop never draws, is covered (by the pointer, or the game's tooltip by it):
//!    it is reported as such, so what it read before can stand, marked as covered.
//! 5. ocrs reads every line at once, black on white, enlarged twice over. A price counts
//!    only in the game's exact shape: "<quantity> for <mesos> mesos", the mesos with their
//!    commas where the game puts them.

use crate::Frame;
use crate::panel::{ScreenPoint, ScreenRect};
use crate::prep::Rect;
use crate::recogniser::{Recogniser, RememberedLines, frame_to_rgb};
use image::imageops::{self, FilterType};
use image::{GrayImage, Luma, RgbImage};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::RangeInclusive;

/// A capture of the game's whole client area for the Deal Finder.
#[derive(Debug, Clone)]
pub struct ShopCapture {
    pub frame: Frame,
    /// Where the frame's top-left pixel is on screen: the client area's corner.
    pub origin: ScreenPoint,
}

/// What a Deal Finder capture reads as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopReading {
    /// The Player Shop's item list, if one shows.
    pub shop: Option<ShopList>,
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
}

/// A Player Shop's item list, as it shows on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopList {
    /// Where the list's listings are on screen, from the first one's top to the last one's
    /// bottom, the item pictures' column included: what a Bug Report keeps of the capture.
    pub area: ScreenRect,
    /// Its listings that aren't sold out, top to bottom.
    pub rows: Vec<ShopRow>,
}

/// One listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopRow {
    /// Where the listing is on screen: its band, both lines.
    pub area: ScreenRect,
    /// The item's name as read; `None`: unreadable or covered.
    pub name: Option<String>,
    /// Its price as read; `None`: unreadable, covered, or not in the game's shape.
    pub price: Option<ListedPrice>,
    /// Something covers it, so neither was read: the pointer, or the game's tooltip by it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub covered: bool,
}

impl ShopList {
    /// The UI Size it shows at, next to the samples' (ADR 0020): its listings' band width next
    /// to theirs, or with no listing, its own width next to theirs.
    pub(crate) fn ui_size(&self) -> f32 {
        match self.rows.first() {
            Some(row) => row.area.width as f32 / BAND_W as f32,
            None => self.area.width as f32 / (PICTURE_LEFT + BAND_W) as f32,
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// A listing's price: `quantity` of the item for `mesos`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedPrice {
    pub quantity: u64,
    pub mesos: u64,
}

/// Reads Deal Finder captures with the bundled recogniser (ADR 0002), which it shares with
/// the other readers. It remembers what each line it read said, so a shop left open is
/// read again without ocrs.
pub struct PlayerShopReader {
    recogniser: Recogniser,
    /// What each line read as, by its ink.
    read: RememberedLines,
    /// The slots a listing was read in, while the list stays where it is: the column its
    /// bands start at, and each slot's top (frame px).
    slots: Option<(u32, Vec<u32>)>,
}

/// A listing's band is this many px wide at the samples' UI Size...
const BAND_W: u32 = 253;
/// ...give or take this many.
const BAND_SLACK: u32 = 2;
/// The widths it is found at, at any UI Size (ADR 0020): from #130's (162 px) and a little
/// smaller, to past Default Ratio's at 3840x2160 (about 455 px).
const BAND_WIDTHS: RangeInclusive<u32> = 140..=480;
/// The list is found where at least this many band rows end at one column, at the samples' UI
/// Size, fewer at a smaller one: two listings' worth of the rows above, between and below their
/// text.
const MIN_BAND_ROWS: usize = 12;
/// Kept around the list's part of the capture when it is scaled to the samples' UI Size, past
/// the scaling's reach (px at their size).
const VIEW_MARGIN: u32 = 4;
/// The price line's top is this many px below the name line's, give or take
/// [`PRICE_SLACK`] (the letters' tops differ).
const PRICE_BELOW: u32 = 27;
const PRICE_SLACK: u32 = 3;
/// A listing's band starts this far above its price line's top (the "1" of "1 for", the
/// same height in every listing), and is this tall.
const BAND_ABOVE_PRICE: u32 = 33;
const BAND_H: u32 = 54;
/// The item pictures' column starts this far left of the band.
const PICTURE_LEFT: u32 = 58;
/// The band's colour, as screenshots show it.
const BAND_RGB: [f32; 3] = [202.0, 213.0, 223.0];
/// The selected listing's colour, as screenshots show it, and how far into the band it
/// starts (its edge is a blend with what is around it).
const SELECTED_RGB: [f32; 3] = [251.0, 167.0, 46.0];
const SELECTED_TOL: f32 = 10.0;
const SELECTED_INSET: u32 = 8;
/// Its text's anti-aliasing strays further from the orange-to-black line than the blue's.
const SELECTED_FOREIGN: f32 = 28.0;
/// The light rule between a listing's name and price, as screenshots show it, which a
/// descender may reach: on the band, and on the selected listing.
const RULE_RGB: [f32; 3] = [222.0, 223.0, 223.0];
const SELECTED_RULE_RGB: [f32; 3] = [255.0, 195.0, 96.0];
/// A pixel of the rule, or of a descender's ink blended into it, lies this close to the
/// rule-to-black line, and is no brighter than the rule (the pointer's white is).
const RULE_TOL: f32 = 16.0;
const RULE_BRIGHTEST: f32 = 1.04;
/// A slot with no listing read is covered when at least this many of its pixels are dark:
/// darker than anything the shop draws there but text, which a sold-out listing's (light grey)
/// and an empty slot (none) don't have.
const COVERED_MIN: usize = 15;
const COVERING_DARK: f32 = 90.0;
/// Two slots' tops this close are one slot's.
const SAME_SLOT: u32 = 3;
/// The slots' tops are this far apart on average: 63 and 62 px in turn.
const SLOT_PITCH: f32 = 62.6;
/// A pixel is the band's colour at its brightness within this much per channel...
const BAND_TOL: f32 = 6.0;
/// ...at any brightness in this range of the screenshots'.
const BAND_BRIGHTNESS: RangeInclusive<f32> = 0.8..=1.1;
/// A scanline is part of a listing's text when at least this share of it is the band.
const TEXT_ROW_BAND: f32 = 0.3;
/// A pixel is ink when it is at least this much of the way from the band to black.
const INK_MIN: f32 = 0.3;
/// A pixel further than this from the band-to-black line (per channel, roughly) isn't the
/// band or its text: something covers the listing.
const FOREIGN: f32 = 14.0;
/// Columns kept clear of the band's ends when looking at its text.
const EDGE: u32 = 2;
/// The lines are enlarged this many times before ocrs reads them.
const UPSCALE: u32 = 2;

impl PlayerShopReader {
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled(), read: RememberedLines::default(), slots: None }
    }

    pub fn read(&mut self, capture: &ShopCapture) -> ShopReading {
        let shop = frame_to_rgb(&capture.frame).and_then(|img| self.read_rgb(&img, capture.origin));
        ShopReading { shop, at_ms: capture.frame.at_ms }
    }

    fn read_rgb(&mut self, capture: &RgbImage, origin: ScreenPoint) -> Option<ShopList> {
        // The list: the likeliest stack of bands that holds a listing, else the likeliest
        // (every listing sold out). Another of the game's windows may be drawn in the band's
        // colour too.
        let mut first = None;
        let mut chosen = None;
        for found in find_lists(capture, BAND_WIDTHS) {
            // The slots listings were read clean in at this spot (capture px).
            let known = match &self.slots {
                Some((x0, slots)) if *x0 == found.x0 => slots.clone(),
                _ => Vec::new(),
            };
            let Some((view, list)) = View::of(capture, found, &known) else { continue };
            let listings = listings(view.img.as_ref(), &list);
            if !listings.is_empty() {
                chosen = Some((found, known, view, list, listings));
                break;
            }
            first = first.or(Some((found, known, view, list, listings)));
        }
        let (found, known, view, list, listings) = chosen.or(first)?;
        let img = view.img.as_ref();
        let on_screen = |r: Rect| {
            let r = view.to_capture(r);
            ScreenRect { x: origin.x + r.x as i32, y: origin.y + r.y as i32, width: r.w, height: r.h }
        };
        // Every clean line, read at once.
        let lines: Vec<&TextLine> = listings.iter().filter(|l| l.clean).flat_map(|l| [&l.name, &l.price]).collect();
        let texts = self.read_lines(img, &lines, list.brightness);
        let mut texts = texts.into_iter();
        let band_area = |top: u32| Rect { x: list.x0, y: top, w: BAND_W, h: BAND_H };
        let covered_row = |top: u32| ShopRow { area: on_screen(band_area(top)), name: None, price: None, covered: true };
        // Each listing read clean, where it is. One that isn't is covered: its lines may be
        // pieces of others', so it counts only in its slot on the grid.
        let mut rows: Vec<(u32, ShopRow)> = Vec::new();
        let mut unclean = Vec::new();
        for listing in &listings {
            if listing.clean {
                let name = texts.next().flatten();
                let price = texts.next().flatten().as_deref().and_then(parse_price);
                rows.push((listing.top(), ShopRow { area: on_screen(band_area(listing.top())), name, price, covered: false }));
            } else {
                unclean.push(listing.top());
            }
        }
        // The slots listings were read clean in at this spot: the list's slots lie on a grid
        // every one of them is on.
        let mut known: Vec<u32> = known.into_iter().filter_map(|top| view.top_from_capture(top)).collect();
        // Another shop's window may open a few px higher or lower where this one was: slots
        // off the grid of those read clean now are another list's.
        if let (Some(&slot), Some(&(top, _))) = (known.first(), rows.first())
            && on_grid(slot, top).abs_diff(top) > SAME_SLOT
        {
            known.clear();
        }
        let anchor = rows.first().map(|&(top, _)| top).or(known.first().copied());
        for &(top, _) in &rows {
            if !known.iter().any(|&slot| slot.abs_diff(top) <= SAME_SLOT) {
                known.push(top);
            }
        }
        let free = |rows: &[(u32, ShopRow)], slot: u32| !rows.iter().any(|&(top, _)| top.abs_diff(slot) <= SAME_SLOT);
        for slot in unclean.into_iter().filter_map(|top| anchor.map(|anchor| on_grid(anchor, top))) {
            if free(&rows, slot) {
                rows.push((slot, covered_row(slot)));
            }
        }
        // A slot read before with no listing now, holding what the shop never draws.
        for &slot in &known {
            if free(&rows, slot) && covered(img, &list, slot) {
                rows.push((slot, covered_row(slot)));
            }
        }
        known.sort_unstable();
        self.slots = Some((found.x0, known.into_iter().map(|top| view.top_to_capture(top)).collect()));
        rows.sort_by_key(|&(top, _)| top);
        let rows = rows.into_iter().map(|(_, row)| row).collect();
        let x = list.x0.saturating_sub(PICTURE_LEFT);
        let area = Rect { x, y: list.y0, w: list.x0 + BAND_W - x, h: list.y1 - list.y0 };
        Some(ShopList { area: on_screen(area), rows })
    }

    /// Read `lines` of `img` (each its own ink, black on white), remembering what each
    /// one's ink read as: one entry per line, `None` where nothing was read.
    fn read_lines(&mut self, img: &RgbImage, lines: &[&TextLine], brightness: f32) -> Vec<Option<String>> {
        let inks: Vec<GrayImage> = lines.iter().map(|line| line_ink(img, line, brightness)).collect();
        self.read.read(&self.recogniser, &inks, UPSCALE)
    }
}

/// Where the item list is: the column its bands start at, how wide they are, the rows from
/// the first band row to the last, how bright the band is next to the screenshots' colour, and
/// whether it was scaled.
#[derive(Debug, Clone, Copy)]
struct List {
    x0: u32,
    w: u32,
    y0: u32,
    y1: u32,
    brightness: f32,
    /// It is in a view scaled to the samples' UI Size, which blurs what the game draws.
    scaled: bool,
}

/// What the list is read in: the capture itself at the samples' UI Size, or else the list's
/// part of it scaled to that size (ADR 0020), with where that part's corner is in the capture
/// and how many capture px one of its px is.
struct View<'a> {
    img: Cow<'a, RgbImage>,
    x: u32,
    y: u32,
    ui_size: f32,
}

impl<'a> View<'a> {
    /// The view of `capture` to read the list `found` there in, down to the slots `known`
    /// (their tops), and the list in it.
    fn of(capture: &'a RgbImage, found: List, known: &[u32]) -> Option<(Self, List)> {
        if found.w.abs_diff(BAND_W) <= BAND_SLACK {
            return Some((View { img: Cow::Borrowed(capture), x: 0, y: 0, ui_size: 1.0 }, found));
        }
        let ui_size = found.w as f32 / BAND_W as f32;
        let out = |px: u32| (px as f32 * ui_size).ceil() as u32;
        // The pictures' column, and a selected listing above or below the bands found.
        let reach = BAND_H + PRICE_BELOW + VIEW_MARGIN;
        let top = known.iter().fold(found.y0, |top, &slot| top.min(slot));
        let bottom = known.iter().fold(found.y1, |bottom, &slot| bottom.max(slot + out(BAND_H)));
        let (x0, y0) = (found.x0.saturating_sub(out(PICTURE_LEFT + VIEW_MARGIN)), top.saturating_sub(out(reach)));
        let x1 = (found.x0 + found.w + out(VIEW_MARGIN)).min(capture.width());
        let y1 = (bottom + out(reach)).min(capture.height());
        let part = imageops::crop_imm(capture, x0, y0, x1 - x0, y1 - y0).to_image();
        let at_samples = |px: u32| ((px as f32 / ui_size).round() as u32).max(1);
        let mut img = imageops::resize(&part, at_samples(x1 - x0), at_samples(y1 - y0), FilterType::CatmullRom);
        within_neighbours(&mut img, &part);
        let list = List { scaled: true, ..find_lists(&img, BAND_W..=BAND_W).into_iter().next()? };
        Some((View { img: Cow::Owned(img), x: x0, y: y0, ui_size }, list))
    }

    /// `r` (view px) in capture px.
    fn to_capture(&self, r: Rect) -> Rect {
        let px = |v: u32| (v as f32 * self.ui_size).round() as u32;
        Rect { x: self.x + px(r.x), y: self.y + px(r.y), w: px(r.w), h: px(r.h) }
    }

    /// A slot's top (view px) in capture px.
    fn top_to_capture(&self, top: u32) -> u32 {
        self.y + (top as f32 * self.ui_size).round() as u32
    }

    /// A slot's top (capture px) in view px; `None` above the view.
    fn top_from_capture(&self, top: u32) -> Option<u32> {
        let top = ((top as f32 - self.y as f32) / self.ui_size).round();
        (top >= 0.0).then_some(top as u32)
    }
}

/// Keeps each pixel of `scaled`, a scaling of `source`, between the darkest and the brightest of
/// the four pixels of `source` it lies between, channel by channel. Catmull-Rom overshoots
/// beside a sharp edge, as the band's beside black text, brighter than either side: a pixel
/// near white there would read as the pointer's white over the listing.
fn within_neighbours(scaled: &mut RgbImage, source: &RgbImage) {
    let (sw, sh) = source.dimensions();
    let (rx, ry) = (sw as f32 / scaled.width() as f32, sh as f32 / scaled.height() as f32);
    let around = |at: u32, ratio: f32, size: u32| {
        let from = ((at as f32 + 0.5) * ratio - 0.5).floor().clamp(0.0, (size - 1) as f32) as u32;
        (from, (from + 1).min(size - 1))
    };
    for (x, y, p) in scaled.enumerate_pixels_mut() {
        let ((xa, xb), (ya, yb)) = (around(x, rx, sw), around(y, ry, sh));
        let near = [source.get_pixel(xa, ya), source.get_pixel(xb, ya), source.get_pixel(xa, yb), source.get_pixel(xb, yb)];
        for c in 0..3 {
            let (lo, hi) = near.iter().fold((u8::MAX, u8::MIN), |(lo, hi), q| (lo.min(q[c]), hi.max(q[c])));
            p[c] = p[c].clamp(lo, hi);
        }
    }
}

/// A listing's band: the listing colour, or the selected listing's orange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Band {
    Listing,
    Selected,
}

impl Band {
    /// Its colour at the list's brightness.
    fn rgb(self, brightness: f32) -> [f32; 3] {
        match self {
            Band::Listing => BAND_RGB,
            Band::Selected => SELECTED_RGB,
        }
        .map(|c| c * brightness)
    }

    /// How far off its band-to-black line a pixel of its text may stray.
    fn foreign(self) -> f32 {
        match self {
            Band::Listing => FOREIGN,
            Band::Selected => SELECTED_FOREIGN,
        }
    }

    /// Whether `p` is the light rule between its listing's lines, at the list's brightness.
    fn rule(self, p: [u8; 3], brightness: f32) -> bool {
        let (along, off) = along_and_off(p, self.rule_rgb(brightness));
        off <= RULE_TOL && along <= RULE_BRIGHTEST
    }

    /// The rule's colour at the list's brightness.
    fn rule_rgb(self, brightness: f32) -> [f32; 3] {
        match self {
            Band::Listing => RULE_RGB,
            Band::Selected => SELECTED_RULE_RGB,
        }
        .map(|c| c * brightness)
    }

    /// Whether `p` is a blend of the rule, the band and the text's black, as scaling to the
    /// samples' UI Size runs the rule into the rows beside it (ADR 0020): as close to them as
    /// the text lies to the band's line to black, and no brighter than the rule.
    fn rule_blend(self, p: [u8; 3], brightness: f32) -> bool {
        let (band, rule, p) = (self.rgb(brightness), self.rule_rgb(brightness), p.map(f32::from));
        let dot = |a: [f32; 3], b: [f32; 3]| a.iter().zip(&b).map(|(x, y)| x * y).sum::<f32>();
        // p as so much of the band and so much of the rule, in a blend with black.
        let (bb, rr, br, pb, pr) = (dot(band, band), dot(rule, rule), dot(band, rule), dot(p, band), dot(p, rule));
        let det = bb * rr - br * br;
        let (of_band, of_rule) = ((pb * rr - pr * br) / det, (pr * bb - pb * br) / det);
        if of_band < 0.0 || of_rule < 0.0 || of_band + of_rule > RULE_BRIGHTEST {
            return false;
        }
        let off: [f32; 3] = std::array::from_fn(|i| p[i] - of_band * band[i] - of_rule * rule[i]);
        dot(off, off).sqrt() <= self.foreign()
    }

    /// The columns of the list that hold its band, clear of its edges.
    fn columns(self, list: &List, width: u32) -> (u32, u32) {
        let inset = match self {
            Band::Listing => EDGE,
            Band::Selected => SELECTED_INSET,
        };
        (list.x0 + inset, (list.x0 + BAND_W - EDGE).min(width))
    }
}

/// Which band `p` is the colour of, at the list's `brightness`.
fn band_of(p: [u8; 3], brightness: f32) -> Option<Band> {
    if band_brightness(p).is_some() {
        return Some(Band::Listing);
    }
    let orange = SELECTED_RGB.map(|c| c * brightness);
    (0..3).all(|i| (p[i] as f32 - orange[i]).abs() <= SELECTED_TOL).then_some(Band::Selected)
}

/// The band's brightness next to the screenshots' colour, if `p` is its colour.
fn band_brightness(p: [u8; 3]) -> Option<f32> {
    let k = p[2] as f32 / BAND_RGB[2];
    let close = BAND_BRIGHTNESS.contains(&k) && (0..2).all(|i| (p[i] as f32 - k * BAND_RGB[i]).abs() <= BAND_TOL);
    close.then_some(k)
}

/// Where the item list may be, likeliest first: where band rows of one width in `widths` end
/// at one column, enough of them and held by no wider stack, the most rows first.
fn find_lists(img: &RgbImage, widths: RangeInclusive<u32>) -> Vec<List> {
    let (w, h) = img.dimensions();
    let lengths = widths.start().saturating_sub(BAND_SLACK)..=widths.end() + BAND_SLACK;
    // Each band row at least as wide as the narrowest band, by its end and its width: its
    // scanline and brightness.
    let mut ends: HashMap<(u32, u32), Vec<(u32, f32)>> = HashMap::new();
    for y in 0..h {
        let mut x = 0;
        while x < w {
            let Some(k) = band_brightness(img.get_pixel(x, y).0) else {
                x += 1;
                continue;
            };
            let start = x;
            while x < w && band_brightness(img.get_pixel(x, y).0).is_some() {
                x += 1;
            }
            if x - start >= *lengths.start() {
                ends.entry((x, x - start)).or_default().push((y, k));
            }
        }
    }
    // A run a pixel short at one end ends a column over, or is narrower: count each end and
    // width with its neighbours.
    let near = |end: u32, width: u32| {
        (end.saturating_sub(1)..=end + 1)
            .flat_map(move |e| (width.saturating_sub(BAND_SLACK)..=width + BAND_SLACK).map(move |w| (e, w)))
            .filter_map(|key| ends.get(&key))
    };
    let count = |end: u32, width: u32| near(end, width).map(Vec::len).sum::<usize>();
    let min_rows = |width: u32| (MIN_BAND_ROWS as f32 * width as f32 / BAND_W as f32).round() as usize;
    let stacks: Vec<(u32, u32)> = ends.keys().copied().filter(|&(end, width)| count(end, width) >= min_rows(width)).collect();
    // A stack is a part of a wider one's bands, of any width, where it starts as they do and
    // something covers their right ends (the game's tooltip), or where it ends as they do and
    // has fewer rows, beside their text. The other way about, a wider stack with fewer rows
    // that ends as the bands do is the bands run into the pictures' column, where scaling has
    // blurred the line between them away.
    let inside = |(end, width): (u32, u32), (wider_end, wider): (u32, u32)| {
        let (start, wider_start) = (end - width, wider_end - wider);
        let cut_short = wider_start.abs_diff(start) <= 1;
        let beside_text = wider_end.abs_diff(end) <= 1 && count(wider_end, wider) > count(end, width);
        wider > width + BAND_SLACK && (cut_short || beside_text)
    };
    let mut lists: Vec<(u32, u32)> = stacks
        .iter()
        .copied()
        .filter(|&(end, width)| lengths.contains(&width) && !stacks.iter().any(|&other| inside((end, width), other)))
        .collect();
    lists.sort_by_key(|&(end, width)| (std::cmp::Reverse((count(end, width), ends[&(end, width)].len(), width)), end));
    lists
        .into_iter()
        .map(|(end, width)| {
            let mut rows: Vec<(u32, f32)> = near(end, width).flatten().copied().collect();
            rows.sort_by_key(|row| row.0);
            let mut brightnesses: Vec<f32> = rows.iter().map(|&(_, k)| k).collect();
            brightnesses.sort_by(f32::total_cmp);
            let brightness = brightnesses[brightnesses.len() / 2];
            let (y0, y1) = (rows[0].0, rows[rows.len() - 1].0 + 1);
            List { x0: end - width, w: width, y0, y1, brightness, scaled: false }
        })
        .collect()
}

/// A line of text in a listing: its inked columns and rows (ends exclusive), and the band
/// it is on.
#[derive(Debug, Clone, Copy)]
struct TextLine {
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
    band: Band,
}

/// A listing: its name line and price line, and whether nothing covers them.
struct Listing {
    name: TextLine,
    price: TextLine,
    clean: bool,
}

impl Listing {
    /// Its band's top: a set distance above its price line's.
    fn top(&self) -> u32 {
        self.price.y0.saturating_sub(BAND_ABOVE_PRICE)
    }
}

/// How much of the way from `band` (its colour at the list's brightness) to black `p` is,
/// and how far off that line it lies.
fn ink_of(p: [u8; 3], band: [f32; 3]) -> (f32, f32) {
    let (along, off) = along_and_off(p, band);
    ((1.0 - along).clamp(0.0, 1.0), off)
}

/// Where `p` lies along the line from black (0) to `colour` (1), and how far off it.
fn along_and_off(p: [u8; 3], colour: [f32; 3]) -> (f32, f32) {
    let norm = colour.iter().map(|c| c * c).sum::<f32>();
    let p = p.map(f32::from);
    let along = p.iter().zip(&colour).map(|(a, b)| a * b).sum::<f32>() / norm;
    let off = p.iter().zip(&colour).map(|(a, b)| (a - along * b).powi(2)).sum::<f32>().sqrt();
    (along, off)
}

/// The listings in `list`, top to bottom: pairs of text lines, the name and the price
/// [`PRICE_BELOW`] px under it, on scanlines that are mostly band. Sold-out listings and
/// empty slots have none.
fn listings(img: &RgbImage, list: &List) -> Vec<Listing> {
    let (w, h) = img.dimensions();
    let (x0, x1) = (list.x0 + EDGE, (list.x0 + BAND_W - EDGE).min(w));
    // A selected listing above or below the bands found is read too.
    let reach = BAND_H + PRICE_BELOW;
    let (y0, y1) = (list.y0.saturating_sub(reach), (list.y1 + reach).min(h));
    // The band a scanline is text on: inked, and mostly band of one colour.
    let text_row = |y: u32| {
        let (mut listing, mut selected) = (0, 0);
        for x in x0..x1 {
            match band_of(img.get_pixel(x, y).0, list.brightness) {
                Some(Band::Listing) => listing += 1,
                Some(Band::Selected) => selected += 1,
                None => {}
            }
        }
        let band = if listing >= selected { Band::Listing } else { Band::Selected };
        let (bx0, bx1) = band.columns(list, w);
        if listing.max(selected) as f32 <= TEXT_ROW_BAND * (bx1 - bx0) as f32 {
            return None;
        }
        let rgb = band.rgb(list.brightness);
        let inked = (bx0..bx1).any(|x| {
            let (ink, off) = ink_of(img.get_pixel(x, y).0, rgb);
            ink >= INK_MIN && off <= band.foreign()
        });
        inked.then_some(band)
    };
    let rows: Vec<Option<Band>> = (y0..y1).map(text_row).collect();
    let mut lines = Vec::new();
    let mut y = 0;
    while y < rows.len() {
        let Some(band) = rows[y] else {
            y += 1;
            continue;
        };
        let top = y;
        while y < rows.len() && rows[y] == Some(band) {
            y += 1;
        }
        lines.push(line_extent(img, list, band, y0 + top as u32, y0 + y as u32));
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let below = lines.get(i + 1).filter(|next| next.band == lines[i].band).map(|next| next.y0 - lines[i].y0);
        match below {
            Some(below) if below.abs_diff(PRICE_BELOW) <= PRICE_SLACK => {
                let (name, price) = (lines[i], lines[i + 1]);
                let clean = clean(img, list, &name) && clean(img, list, &price);
                out.push(Listing { name, price, clean });
                i += 2;
            }
            // A line on its own: the other is covered, or it isn't a listing's.
            _ => i += 1,
        }
    }
    out
}

/// The text line on scanlines `y0..y1` of the list, on `band`: its inked columns.
fn line_extent(img: &RgbImage, list: &List, band: Band, y0: u32, y1: u32) -> TextLine {
    let (x0, x1) = band.columns(list, img.width());
    let rgb = band.rgb(list.brightness);
    let inked = |x: u32| (y0..y1).any(|y| ink_of(img.get_pixel(x, y).0, rgb).0 >= INK_MIN);
    let first = (x0..x1).find(|&x| inked(x)).unwrap_or(x0);
    let last = (x0..x1).rev().find(|&x| inked(x)).map_or(x1, |x| x + 1);
    TextLine { x0: first, x1: last.max(first + 1), y0, y1, band }
}

/// Whether every pixel of the band across `line`'s rows, and a row above and below, is the
/// band, its text or the rule under the name (in a scaled view, a blend of them): nothing
/// covers it.
fn clean(img: &RgbImage, list: &List, line: &TextLine) -> bool {
    let band = line.band;
    let (x0, x1) = band.columns(list, img.width());
    let (y0, y1) = (line.y0.saturating_sub(1), (line.y1 + 1).min(img.height()));
    let rgb = band.rgb(list.brightness);
    let shops = |p: [u8; 3]| {
        ink_of(p, rgb).1 <= band.foreign() || band.rule(p, list.brightness) || (list.scaled && band.rule_blend(p, list.brightness))
    };
    (y0..y1).all(|y| (x0..x1).all(|x| shops(img.get_pixel(x, y).0)))
}

/// The top of the slot on the list's grid nearest `top`, the grid through `anchor` (a slot's
/// top).
fn on_grid(anchor: u32, top: u32) -> u32 {
    let slots = ((top as f32 - anchor as f32) / SLOT_PITCH).round();
    (anchor as f32 + (slots * SLOT_PITCH).round()).max(0.0) as u32
}

/// Whether the slot whose band starts at `top` is covered: something dark is on it, the
/// game's pointer's outline or the game's tooltip's slate fill, while no listing is read
/// there. A sold-out listing holds nothing dark (its band and its text are light greys, near
/// white in live captures) and an empty slot is band alone, so neither is ever covered.
fn covered(img: &RgbImage, list: &List, top: u32) -> bool {
    let (w, h) = img.dimensions();
    let (x0, x1) = (list.x0 + EDGE, (list.x0 + BAND_W - EDGE).min(w));
    let dark = (top..(top + BAND_H).min(h)).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| {
        let [r, g, b] = img.get_pixel(x, y).0.map(f32::from);
        0.299 * r + 0.587 * g + 0.114 * b <= COVERING_DARK * list.brightness
    });
    dark.take(COVERED_MIN).count() == COVERED_MIN
}

/// `line`'s ink, black on white.
fn line_ink(img: &RgbImage, line: &TextLine, brightness: f32) -> GrayImage {
    let (w, h) = (line.x1 - line.x0, line.y1 - line.y0);
    let rgb = line.band.rgb(brightness);
    GrayImage::from_fn(w, h, |x, y| {
        let (ink, _) = ink_of(img.get_pixel(line.x0 + x, line.y0 + y).0, rgb);
        Luma([(255.0 * (1.0 - ink)).round() as u8])
    })
}

/// A price line as the game writes it: "<quantity> for <mesos> mesos", the mesos grouped in
/// threes with commas ("1 for 12,999 mesos"). `None` for anything else.
pub(crate) fn parse_price(read: &str) -> Option<ListedPrice> {
    let words: Vec<&str> = read.split_whitespace().collect();
    let [quantity, "for", mesos, "mesos"] = words[..] else { return None };
    let quantity: u64 = digits(quantity)?;
    let mesos = grouped(mesos)?;
    (quantity > 0).then_some(ListedPrice { quantity, mesos })
}

fn digits(text: &str) -> Option<u64> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) && !text.starts_with('0')).then(|| text.parse().ok())?
}

/// A number with commas where the game puts them: 1 to 3 digits, then groups of 3.
fn grouped(text: &str) -> Option<u64> {
    let mut groups = text.split(',');
    let first = groups.next()?;
    if first.is_empty() || first.len() > 3 {
        return None;
    }
    let rest: Vec<&str> = groups.collect();
    if rest.iter().any(|group| group.len() != 3 || !group.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let all: String = std::iter::once(first).chain(rest).collect();
    digits(&all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_read_only_in_the_games_shape() {
        assert_eq!(parse_price("1 for 999 mesos"), Some(ListedPrice { quantity: 1, mesos: 999 }));
        assert_eq!(parse_price("10 for 12,999 mesos"), Some(ListedPrice { quantity: 10, mesos: 12_999 }));
        assert_eq!(parse_price("1 for 1,234,567 mesos"), Some(ListedPrice { quantity: 1, mesos: 1_234_567 }));
        for wrong in ["1 for 12999 mesos", "1 for 1,29 mesos", "1 for 12,9999 mesos", "1for 999 mesos", "0 for 9 mesos", "1 for 999", "1 tor 999 mesos"] {
            assert_eq!(parse_price(wrong), None, "{wrong}");
        }
    }
}
