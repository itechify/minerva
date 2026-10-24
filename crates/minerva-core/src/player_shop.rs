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
//!    An empty slot is a band with no text: one the game's tooltip covers part of has its
//!    edge on every scanline, which isn't text, as a listing's name starts at the band's
//!    left, [`NAME_LEFT_MAX`] px in at most. The listing the player selected is orange
//!    (251,167,46) from 7 px into its band, its text black as on the others, and is read
//!    as they are.
//! 4. A slot where a listing was read before, at the same spot, that now has none and holds
//!    pixels the shop never draws, is covered (by the pointer, or the game's tooltip by it):
//!    it is reported as such, so what it read before can stand, marked as covered.
//! 5. ocrs reads every line at once, black on white, enlarged twice over. A price counts
//!    only in the game's exact shape: "<quantity> for <mesos> mesos", the mesos with their
//!    commas where the game puts them, and only with as many digits and commas as its line's
//!    ink is wide for ([`PRICE_DIGIT_W`]): a digit ocrs drops leaves the shape whole.
//! 6. While the pointer is on the list, the Item Tooltip by it is read too, as the Item
//!    Card reads it (ADR 0007): the Deal Finder names a listing the game cut short by it.
//! 7. Each listing's picture is read as the scroll tier its colour shows, were it a scroll's
//!    ([`picture_tier`]): the Deal Finder names a scroll the game cut short by it (#140).

use crate::Frame;
use crate::item_tooltip::{ItemTooltip, ShopTooltipReader};
use crate::osms_items::ScrollTier;
use crate::panel::{ScreenPoint, ScreenRect};
use crate::prep::Rect;
use crate::ink::{at_brightness, black_on_white, luma};
use crate::recogniser::{Recogniser, RememberedLines, frame_to_rgb};
use crate::ui_size;
use image::imageops;
use image::{GrayImage, RgbImage};
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
    /// Where the mouse pointer was on screen when the frame was captured, if known.
    pub pointer: Option<ScreenPoint>,
}

/// What a Deal Finder capture reads as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopReading {
    /// The Player Shop's item list, if one shows.
    pub shop: Option<ShopList>,
    /// The pointer on the list, and the Item Tooltip by it; `None` while it is off the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover: Option<ShopHover>,
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
    /// With no shop found, what the reader came closest with (#133); `None` with a shop, or
    /// with nothing near.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub near_miss: Option<ShopNearMiss>,
}

/// What a Deal Finder capture with no Player Shop found came closest to one with: the stack of
/// band rows, of any width, with the most rows. Bands at a UI Size the reader doesn't take
/// (ADR 0020) show here, as another of the game's windows in the band's colour may.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopNearMiss {
    /// Where its rows are on screen, from the first to the last: as wide as they are.
    pub area: ScreenRect,
    /// How many band rows it has.
    pub rows: u32,
    /// The band's brightness next to the screenshots' colour, in hundredths (100: theirs).
    pub brightness_hundredths: u32,
}

/// The pointer on a Player Shop's list, and the Item Tooltip the game shows by it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopHover {
    /// Where the pointer is on screen.
    pub pointer: ScreenPoint,
    pub tooltip: ItemTooltip,
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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub covered: bool,
    /// The scroll tier its picture's colour shows, were it a scroll; `None`: none plainly, or
    /// the picture is covered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<ScrollTier>,
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

    /// Where it is on screen down to every listing: a covered or selected one may lie past
    /// the bands found, as the last one showing does under the pointer, which breaks its band.
    pub(crate) fn span(&self) -> ScreenRect {
        let top = self.rows.iter().map(|row| row.area.y).fold(self.area.y, i32::min);
        let end = self.rows.iter().map(|row| row.area.bottom()).fold(self.area.bottom(), i64::max);
        ScreenRect { y: top, height: (end - i64::from(top)) as u32, ..self.area }
    }
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
    /// Reads the Item Tooltip by the pointer on the list.
    tooltips: ShopTooltipReader,
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
/// A stack of band rows is what a capture with no list came closest with (#133) when its rows
/// are this wide at least, at most a quarter of the samples' band and narrower than the game
/// draws it at any UI Size...
const NEAR_MISS_W: u32 = 64;
/// ...and it has this many rows at least: fewer are any light blue-grey in the game, and would
/// make a near miss of nearly every capture.
const NEAR_MISS_ROWS: usize = MIN_BAND_ROWS / 2;
/// Kept around the list's part of the capture when it is scaled to the samples' UI Size, past
/// the scaling's reach (px at their size).
const VIEW_MARGIN: u32 = 4;
/// The price line's top is this many px below the name line's, give or take
/// [`PRICE_SLACK`] (the letters' tops differ).
const PRICE_BELOW: u32 = 27;
const PRICE_SLACK: u32 = 3;
/// A listing's name starts 5 to 8 px into its band, and its ink no further in than this, the
/// pointer's over it or not: ink that starts further in, on a slot's every scanline, is the
/// game's tooltip's edge over an empty slot (#134).
const NAME_LEFT_MAX: u32 = 12;
/// A price line's ink is this many px wide, and this many more for each digit and comma in it,
/// to a px or two at every sample's size and at others (the quantity's digits too: "1 for 25
/// mesos" is 127 px, "800 for 1,999,999 mesos" 214).
const PRICE_W: f32 = 94.0;
const PRICE_DIGIT_W: f32 = 11.0;
const PRICE_COMMA_W: f32 = 4.7;
/// A listing's band starts this far above its price line's top (the "1" of "1 for", the
/// same height in every listing), and is this tall.
const BAND_ABOVE_PRICE: u32 = 33;
const BAND_H: u32 = 54;
/// The item pictures' column starts this far left of the band.
const PICTURE_LEFT: u32 = 58;
/// A listing's picture lies between this far left of its band and this far: its column, clear
/// of the shop window's frame on its left and of the band on its right.
const PICTURE_FROM: u32 = PICTURE_LEFT - 4;
const PICTURE_TO: u32 = 4;
/// A picture's pixel is its cell's background within this much per channel, at the list's
/// brightness; one darker than this share of the background is an outline.
const PICTURE_BACKGROUND_TOL: f32 = 12.0;
const PICTURE_DARKEST: f32 = 0.35;
/// A tier is read from at least this many of a picture's pixels: about a third of a scroll's.
const PICTURE_MIN_PX: usize = 400;
/// Each scroll tier's picture's mean chromaticity, as red's share less green's and green's
/// less blue's, from the samples', live and not; a picture within [`TIER_REACH`] of one is
/// that tier's. The nearest two (Lesser's blue-grey and Chaos's grey) are 0.07 apart, and every
/// sample's within 0.02 of its own.
const TIER_COLOURS: [(ScrollTier, (f32, f32)); 4] = [
    (ScrollTier::Lesser, (-0.048, 0.0)),
    (ScrollTier::Intermediate, (0.142, 0.075)),
    (ScrollTier::Greater, (0.074, 0.157)),
    (ScrollTier::Chaos, (0.020, 0.005)),
];
const TIER_REACH: f32 = 0.025;
/// The pointer's hand reaches this far right of and below its tip, and a little up and left.
const POINTER_REACH: f32 = 34.0;
const POINTER_BEHIND: f32 = 2.0;
/// The game's Item Tooltip's box starts this far left of and above its name line, and reaches
/// this far below it (more than the tallest's).
const TOOLTIP_AROUND_NAME: f32 = 24.0;
const TOOLTIP_BELOW_NAME: f32 = 400.0;
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
/// A pixel is the band's colour at its brightness within this much per channel, at any
/// brightness ([`crate::ink::at_brightness`]).
const BAND_TOL: f32 = 6.0;
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
        let tooltips = ShopTooltipReader::bundled();
        Self { recogniser: Recogniser::bundled(), read: RememberedLines::default(), slots: None, tooltips }
    }

    pub fn read(&mut self, capture: &ShopCapture) -> ShopReading {
        let img = frame_to_rgb(&capture.frame);
        let (shop, near_miss) = match img.as_ref().map(|img| self.read_rgb(img, capture.origin)) {
            Some(Ok(shop)) => (Some(shop), None),
            Some(Err(near_miss)) => (None, near_miss),
            None => (None, None),
        };
        // Only on the list: off it, no listing is hovered, and the search for the tooltip
        // costs as much again as finding the list.
        let on_list = |pointer: &ScreenPoint| shop.as_ref().is_some_and(|shop| shop.span().holds(*pointer));
        let hover = img.as_ref().zip(capture.pointer.filter(on_list)).map(|(img, pointer)| ShopHover {
            pointer,
            tooltip: self.tooltips.read(img, capture.origin, pointer),
        });
        let mut shop = shop;
        if let Some((shop, hover)) = shop.as_mut().zip(hover.as_ref()) {
            hide_covered_pictures(shop, hover);
        }
        ShopReading { shop, hover, at_ms: capture.frame.at_ms, near_miss }
    }

    /// The shop's list in `capture`; or, with none, what came closest to one.
    fn read_rgb(&mut self, capture: &RgbImage, origin: ScreenPoint) -> Result<ShopList, Option<ShopNearMiss>> {
        // The list: the likeliest stack of bands that holds a listing, else the likeliest
        // (every listing sold out). Another of the game's windows may be drawn in the band's
        // colour too.
        let mut first = None;
        let mut chosen = None;
        let (found_lists, nearest) = find_lists(capture, BAND_WIDTHS);
        for found in found_lists {
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
        let Some((found, known, view, list, listings)) = chosen.or(first) else {
            return Err(nearest.map(|(list, rows)| ShopNearMiss {
                area: Rect { x: list.x0, y: list.y0, w: list.w, h: list.y1 - list.y0 }.on_screen(origin),
                rows: rows as u32,
                brightness_hundredths: (list.brightness * 100.0).round() as u32,
            }));
        };
        let img = view.img.as_ref();
        let on_screen = |r: Rect| view.to_capture(r).on_screen(origin);
        // Every clean line, read at once.
        let lines: Vec<&TextLine> = listings.iter().filter(|l| l.clean).flat_map(|l| [&l.name, &l.price]).collect();
        let texts = self.read_lines(img, &lines, list.brightness);
        let mut texts = texts.into_iter();
        let band_area = |top: u32| Rect { x: list.x0, y: top, w: BAND_W, h: BAND_H };
        let covered_row = |top: u32| ShopRow { area: on_screen(band_area(top)), name: None, price: None, covered: true, tier: None };
        // Each listing read clean, where it is. One that isn't is covered: its lines may be
        // pieces of others', so it counts only in its slot on the grid.
        let mut rows: Vec<(u32, ShopRow)> = Vec::new();
        let mut unclean = Vec::new();
        for listing in &listings {
            if listing.clean {
                let name = texts.next().flatten();
                let price = texts.next().flatten().filter(|read| fits(read, &listing.price));
                let price = price.as_deref().and_then(parse_price);
                let tier = picture_tier(img, &list, listing.top());
                rows.push((listing.top(), ShopRow { area: on_screen(band_area(listing.top())), name, price, covered: false, tier }));
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
        Ok(ShopList { area: on_screen(area), rows })
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
        let img = ui_size::to_samples_size(&part, ui_size);
        let list = List { scaled: true, ..find_lists(&img, BAND_W..=BAND_W).0.into_iter().next()? };
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
    at_brightness(p, BAND_RGB, BAND_TOL)
}

/// Where the item list may be, likeliest first: where band rows of one width in `widths` end
/// at one column, enough of them and held by no wider stack, the most rows first. And, from the
/// same scan, the stack of band rows at any width (at least [`NEAR_MISS_W`]) with the most
/// rows, if it has [`NEAR_MISS_ROWS`]: what a capture with no list came closest with (#133).
fn find_lists(img: &RgbImage, widths: RangeInclusive<u32>) -> (Vec<List>, Option<(List, usize)>) {
    let (w, h) = img.dimensions();
    let lengths = widths.start().saturating_sub(BAND_SLACK)..=widths.end() + BAND_SLACK;
    // Each band row at least as wide as a near miss's, by its end and its width: its scanline
    // and brightness.
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
            if x - start >= NEAR_MISS_W.min(*lengths.start()) {
                ends.entry((x, x - start)).or_default().push((y, k));
            }
        }
    }
    // A run a pixel short at one end ends a column over, or is narrower: count each end and
    // width with its neighbours.
    // A stack as wide as a band counts no rows narrower than the narrowest band: those are kept
    // for near misses alone.
    let near = |end: u32, width: u32| {
        let floor = if width >= *lengths.start() { *lengths.start() } else { 0 };
        (end.saturating_sub(1)..=end + 1)
            .flat_map(move |e| (width.saturating_sub(BAND_SLACK).max(floor)..=width + BAND_SLACK).map(move |w| (e, w)))
            .filter_map(|key| ends.get(&key))
    };
    let count = |end: u32, width: u32| near(end, width).map(Vec::len).sum::<usize>();
    let min_rows = |width: u32| (MIN_BAND_ROWS as f32 * width as f32 / BAND_W as f32).round() as usize;
    let stacks: Vec<(u32, u32)> = ends
        .keys()
        .copied()
        .filter(|&(end, width)| width >= *lengths.start() && count(end, width) >= min_rows(width))
        .collect();
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
    let likeliest = |&(end, width): &(u32, u32)| (std::cmp::Reverse((count(end, width), ends[&(end, width)].len(), width)), end);
    lists.sort_by_key(likeliest);
    let list = |(end, width): (u32, u32)| {
        let mut rows: Vec<(u32, f32)> = near(end, width).flatten().copied().collect();
        rows.sort_by_key(|row| row.0);
        let mut brightnesses: Vec<f32> = rows.iter().map(|&(_, k)| k).collect();
        brightnesses.sort_by(f32::total_cmp);
        let brightness = brightnesses[brightnesses.len() / 2];
        let (y0, y1) = (rows[0].0, rows[rows.len() - 1].0 + 1);
        List { x0: end - width, w: width, y0, y1, brightness, scaled: false }
    };
    let nearest = ends
        .keys()
        .copied()
        .filter(|&(end, width)| count(end, width) >= NEAR_MISS_ROWS)
        .min_by_key(likeliest)
        .map(|stack| (list(stack), count(stack.0, stack.1)));
    (lists.into_iter().map(list).collect(), nearest)
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
        let at_names = lines[i].x0 <= list.x0 + NAME_LEFT_MAX;
        match below {
            Some(below) if below.abs_diff(PRICE_BELOW) <= PRICE_SLACK && at_names => {
                let (name, price) = (lines[i], lines[i + 1]);
                let clean = clean(img, list, &name) && clean(img, list, &price);
                out.push(Listing { name, price, clean });
                i += 2;
            }
            // A line on its own: the other is covered, or it isn't a listing's (nor is one
            // that doesn't start where names do).
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
    let dark = (top..(top + BAND_H).min(h))
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|&(x, y)| luma(img.get_pixel(x, y).0) <= COVERING_DARK * list.brightness);
    dark.take(COVERED_MIN).count() == COVERED_MIN
}

/// The scroll tier the picture of the listing whose band starts at `top` shows by its colour,
/// were it a scroll's: every scroll of a tier has the same picture, and the tiers' differ in
/// colour alone (#140). Its colour is the mean chromaticity of the picture's pixels (each
/// pixel's share of red, green and blue), which a darker capture doesn't change: those off the
/// cell's background, neither lighter than it (the bundle count's white, the pointer's) nor
/// much darker (outlines). `None` when it isn't plainly one tier's, or too little of it shows.
fn picture_tier(img: &RgbImage, list: &List, top: u32) -> Option<ScrollTier> {
    let (x0, x1) = (list.x0.checked_sub(PICTURE_FROM)?, list.x0.checked_sub(PICTURE_TO)?);
    let (y0, y1) = (top, (top + BAND_H).min(img.height()));
    let cell: Vec<[u8; 3]> = (y0..y1).flat_map(|y| (x0..x1).map(move |x| img.get_pixel(x, y).0)).collect();
    let mut counts: HashMap<[u8; 3], usize> = HashMap::new();
    for &p in &cell {
        *counts.entry(p).or_default() += 1;
    }
    let (background, _) = counts.into_iter().max_by_key(|&(p, n)| (n, p))?;
    let total = |p: [u8; 3]| p.iter().map(|&c| f32::from(c)).sum::<f32>();
    let tol = PICTURE_BACKGROUND_TOL * list.brightness;
    let (lightest, darkest) = (total(background), total(background) * PICTURE_DARKEST);
    let mut sum = [0.0f32; 3];
    let mut n = 0;
    for p in cell {
        let off_background = (0..3).any(|i| (f32::from(p[i]) - f32::from(background[i])).abs() > tol);
        let total = total(p);
        if !off_background || total > lightest || total < darkest {
            continue;
        }
        for i in 0..3 {
            sum[i] += f32::from(p[i]) / total;
        }
        n += 1;
    }
    if n < PICTURE_MIN_PX {
        return None;
    }
    let (rg, gb) = ((sum[0] - sum[1]) / n as f32, (sum[1] - sum[2]) / n as f32);
    let distance = |(r, g): (f32, f32)| (rg - r).hypot(gb - g);
    let (tier, nearest) = TIER_COLOURS.iter().map(|&(tier, colour)| (tier, distance(colour))).min_by(|a, b| a.1.total_cmp(&b.1))?;
    (nearest <= TIER_REACH).then_some(tier)
}

/// Read no tier for a listing whose picture the pointer covers, or the Item Tooltip by it: a
/// picture partly covered may read as another tier's (the tooltip's slate edge over a Greater's
/// gold reads grey, as Chaos's). The tier read there before stands for it in the Deal Finder.
fn hide_covered_pictures(shop: &mut ShopList, hover: &ShopHover) {
    let size = shop.ui_size();
    let px = |v: f32| (v * size).round() as i32;
    let reach = px(POINTER_REACH + POINTER_BEHIND) as u32;
    let pointer = ScreenRect { x: hover.pointer.x - px(POINTER_BEHIND), y: hover.pointer.y - px(POINTER_BEHIND), width: reach, height: reach };
    let tooltip = match &hover.tooltip {
        ItemTooltip::Found { area, .. } => {
            let (x, y) = (area.x - px(TOOLTIP_AROUND_NAME), area.y - px(TOOLTIP_AROUND_NAME));
            let below = px(TOOLTIP_AROUND_NAME + TOOLTIP_BELOW_NAME) as u32;
            Some(ScreenRect { x, y, width: area.width + 2 * px(TOOLTIP_AROUND_NAME) as u32, height: below })
        }
        ItemTooltip::None => None,
    };
    let (from, to) = (px(PICTURE_FROM as f32), px(PICTURE_TO as f32));
    for row in &mut shop.rows {
        let picture = ScreenRect { x: row.area.x - from, width: (from - to) as u32, ..row.area };
        if picture.overlaps(pointer) || tooltip.is_some_and(|tooltip| picture.overlaps(tooltip)) {
            row.tier = None;
        }
    }
}

/// `line`'s ink, black on white.
fn line_ink(img: &RgbImage, line: &TextLine, brightness: f32) -> GrayImage {
    let (w, h) = (line.x1 - line.x0, line.y1 - line.y0);
    let rgb = line.band.rgb(brightness);
    black_on_white(w, h, 0, |x, y| ink_of(img.get_pixel(line.x0 + x, line.y0 + y).0, rgb).0)
}

/// A price line as the game writes it: "<quantity> for <mesos> mesos", the mesos grouped in
/// threes with commas ("1 for 12,999 mesos"). `None` for anything else.
fn parse_price(read: &str) -> Option<ListedPrice> {
    let words: Vec<&str> = read.split_whitespace().collect();
    let [quantity, "for", mesos, "mesos"] = words[..] else { return None };
    let quantity: u64 = digits(quantity)?;
    let mesos = grouped(mesos)?;
    (quantity > 0).then_some(ListedPrice { quantity, mesos })
}

/// Whether a price line read as `read` is as wide as its ink, `line`: ocrs may drop a digit
/// from a line it otherwise reads whole ("1 for 250,000 mesos" as "1 for 50,000 mesos", #134),
/// which still has the game's shape. The game's digits are all as wide.
fn fits(read: &str, line: &TextLine) -> bool {
    let count = |c: fn(&char) -> bool| read.chars().filter(c).count() as f32;
    let width = PRICE_W + PRICE_DIGIT_W * count(char::is_ascii_digit) + PRICE_COMMA_W * count(|&c| c == ',');
    (width - (line.x1 - line.x0) as f32).abs() <= PRICE_DIGIT_W / 2.0
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
    fn a_picture_under_the_pointer_or_its_tooltip_reads_no_tier() {
        // Three listings at the samples' UI Size, their pictures from 54 px left of their bands.
        let row = |y: i32| ShopRow {
            area: ScreenRect { x: 158, y, width: 253, height: 54 },
            name: None,
            price: None,
            covered: false,
            tier: Some(ScrollTier::Greater),
        };
        let shop = || ShopList { area: ScreenRect { x: 100, y: 0, width: 311, height: 180 }, rows: vec![row(0), row(63), row(125)] };
        let tiers = |shop: &ShopList| shop.rows.iter().map(|row| row.tier.is_some()).collect::<Vec<_>>();
        let hover = |x: i32, y: i32, tooltip: ItemTooltip| ShopHover { pointer: ScreenPoint { x, y }, tooltip };

        // The pointer on the middle one's picture, no tooltip by it.
        let mut on_picture = shop();
        hide_covered_pictures(&mut on_picture, &hover(116, 80, ItemTooltip::None));
        assert_eq!(tiers(&on_picture), [true, false, true]);

        // On its name, the tooltip by it from below it over the last one's picture.
        let mut on_name = shop();
        let tooltip = ItemTooltip::Found { area: ScreenRect { x: 120, y: 150, width: 300, height: 18 }, name: None };
        hide_covered_pictures(&mut on_name, &hover(250, 80, tooltip));
        assert_eq!(tiers(&on_name), [true, true, false]);
    }

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
