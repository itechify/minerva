//! The Deal Finder (ADR 0014): while the player has it on (Ctrl+Shift+D by default), the
//! shell captures the game's client area a few times a second, the Player Shop reader reads
//! the open Player Shop's listings, and a Deal Badge beside each one says whether its price
//! is a deal against the Deal Line: the Market Line (ADR 0011), or maplestory.market's figure
//! (ADR 0030), the player chose in the Settings window. A listing is a deal when the price of one item is at least the Deal Margin
//! (a percentage, 0 by default) below the item's price on the Deal Line. A throwing star or
//! a bullet is one set however many it holds (the listing's number), so its listing's whole
//! price is compared.
//!
//! A listing covered for now (by the pointer, or the game's tooltip by it) keeps the badge
//! of what was read there before, marked covered, while the listings read since say the
//! list is as it was: none read otherwise, none gone.
//!
//! Only what the reader is sure of is judged: a name that isn't one item's (unreadable, or
//! cut short and shared by several), or a price not in the game's shape, gets a "?" badge,
//! never another item's verdict. Equipment is judged by its name alone: Market Lines keep
//! one price per item, whatever its stats, and maplestory.market's are of every copy.
//!
//! A name cut short and shared by several items is named by the Item Tooltip the game shows
//! while the player hovers its listing: when the pointer is on its slot, and the tooltip's
//! name starts as the listing's does and is one item's. The name stands while what was read
//! there stands. The tooltip by the pointer usually hides the listing's price, so its badge
//! keeps the price read there before, marked covered.
//!
//! A scroll's name cut short is named by its picture, unhovered (#140): when every item whose
//! name starts so is a scroll, the one of the tier its picture's colour shows. The tier read
//! stands while what was read there stands, as the pointer passes over the picture. A tooltip
//! that names another item than the picture leaves it "?": one of them was misread.

use crate::item_card::item_id;
use crate::item_lookup::item_key;
use crate::item_tooltip::ItemTooltip;
use crate::maplestory_market::Market;
use crate::market_prices::DealLine;
use crate::osms_items::{ItemFacts, ItemSet, ScrollTier};
use crate::panel::{GameWindow, ScreenRect, Size, clear_of};
use crate::player_shop::{ListedPrice, ShopHover, ShopList, ShopReading, ShopRow};
use serde::Serialize;

/// The Deal Finder, as the panel shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealFinderView {
    /// The player has it on.
    pub on: bool,
    /// The Deal Line's name; `None` with nothing to judge against.
    pub deal_line: Option<String>,
    /// The Deal Line is one of maplestory.market's figures: the panel credits it.
    pub from_market: bool,
    /// What the Settings window offers as the Deal Line: maplestory.market's figures while
    /// they are on, then the player's Market Lines.
    pub choices: Vec<DealLineChoice>,
    /// "Market prices from maplestory.market" is on.
    pub market_prices: bool,
    /// How far below the Deal Line a price must be to be a deal, in percent.
    pub margin_percent: u32,
    /// A Player Shop's listings were found in the latest capture.
    pub shop_found: bool,
}

/// Something the Deal Line may be, as the Settings window offers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealLineChoice {
    /// What [`crate::Minerva::choose_deal_line`] takes for it.
    pub id: String,
    pub name: String,
    /// It is the Deal Line.
    pub chosen: bool,
}

/// One listing's Deal Badge, level with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealBadge {
    /// Which listing it is, counted from the top of those not sold out: what a click on it
    /// names ([`crate::Minerva::open_deal_item`]).
    pub row: u32,
    /// Its listing's top, from the top of the badges' window, and its height (physical px).
    pub top: u32,
    pub height: u32,
    /// Its listing is covered now, so its verdict is what was read there before (or "?"
    /// with nothing read there before).
    pub covered: bool,
    /// Its item's card shows, opened from it: the badge stays pressed.
    pub open: bool,
    #[serde(flatten)]
    pub verdict: Verdict,
}

/// What a Deal Badge says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Verdict {
    /// One item costs at least the Deal Margin less than the Deal Line's price for it:
    /// `percent` off it (negative: below), rounded, the line's price, and how good a deal
    /// that is.
    Deal { item: DealItem, listed: ListedPrice, percent: i32, line_mesos: u64, grade: DealGrade },
    /// It doesn't: at or above the Deal Line's price, or below it by less than the Deal
    /// Margin.
    NoDeal { item: DealItem, listed: ListedPrice, percent: i32, line_mesos: u64 },
    /// The Deal Line has no price for the item.
    NoPrice { item: DealItem, listed: ListedPrice },
    /// The listing's name or price couldn't be read, or the name isn't one item's.
    Unknown,
}

/// How good a deal is, by how far below the Deal Line it is, graded as the game grades its
/// items: a good one under [`GREAT_BELOW`]%, a great one under [`STEAL_BELOW`]%, a steal past it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DealGrade {
    Good,
    Great,
    Steal,
}

/// A deal this many percent below the Deal Line, or more, is great...
pub const GREAT_BELOW: i32 = 20;
/// ...and this many, or more, a steal.
pub const STEAL_BELOW: i32 = 40;

/// The item a listing sells, as OSMS's item files have it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealItem {
    /// The game's item ID, 8 digits.
    pub id: String,
    pub name: String,
    /// A throwing star or a bullet: its listing's number is how many the set holds, and its
    /// price is the set's.
    pub rechargeable: bool,
}

/// The Deal Badges' window, while it shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealBadges {
    pub placement: ScreenRect,
    pub badges: Vec<DealBadge>,
}

/// The Deal Finder: whether it is on, and the latest shop read.
#[derive(Default)]
pub(crate) struct DealFinder {
    pub on: bool,
    /// The listings in the latest capture; `None` while no shop was found, or it is off.
    shop: Option<ShopList>,
    /// The items each listing in `shop` is, as they were last judged: what a click on its
    /// badge opens.
    items: Vec<Option<Vec<u32>>>,
    /// The listings last read whole, each where it is, while the list stays at `seen_at`
    /// (its left edge on screen) and nothing read since says it changed: what a covered
    /// listing's badge says.
    seen: Vec<ShopRow>,
    seen_at: Option<i32>,
    /// The listings an Item Tooltip named, while `seen` stands.
    named: Vec<Named>,
    /// The pointer on the list in the latest capture, and the tooltip by it.
    hover: Option<ShopHover>,
    /// The Deal Badges' window's size, in physical px for the current display: its width
    /// counts, as its height follows the shop's list.
    pub size: Size,
}

/// A listing whose name the game cut short, named by the Item Tooltip by the pointer on it.
struct Named {
    /// Its slot's top on screen.
    top: i32,
    /// Its name as read there, cut short.
    cut: String,
    /// The tooltip's name as read.
    tooltip: String,
}

/// Two listings whose tops are this close (physical px) are in one slot.
const SAME_SLOT: i32 = 3;

fn same_slot(a: &ShopRow, b: &ShopRow) -> bool {
    (a.area.y - b.area.y).abs() <= SAME_SLOT
}

/// Whether `row` was read whole: its name and its price.
fn whole(row: &ShopRow) -> bool {
    !row.covered && row.name.is_some() && row.price.is_some()
}

/// The badges sit this far left of the shop's window (physical px).
const SHOP_GAP: i32 = 6;
/// The shop's window reaches this far left of its item list's picture column, and this far
/// right of it (physical px), at the samples' UI Size: as much farther at a larger one (ADR
/// 0020).
const SHOP_LEFT_OF_LIST: f32 = 8.0;
const SHOP_RIGHT_OF_LIST: f32 = 809.0;

/// How far the shop's window reaches left and right of `shop`'s list (physical px).
fn shop_reach(shop: &ShopList) -> (i32, i32) {
    let size = shop.ui_size();
    ((SHOP_LEFT_OF_LIST * size).round() as i32, (SHOP_RIGHT_OF_LIST * size).round() as i32)
}

impl DealFinder {
    /// Turn it on or off. Off, it forgets the shop it read.
    pub fn toggle(&mut self) {
        self.on = !self.on;
        if !self.on {
            self.shop = None;
            self.items.clear();
            self.forget();
            self.seen_at = None;
            self.hover = None;
        }
    }

    /// Forget what was read before, and the names tooltips gave it.
    fn forget(&mut self) {
        self.seen.clear();
        self.named.clear();
    }

    /// What a capture read as; ignored while it is off (a capture taken before it was
    /// turned off). The listings read whole are remembered where they are. The list has
    /// changed (scrolled, or a listing bought or sold out) when one of them reads otherwise
    /// than what was read there before, or a listing read before is gone and not covered:
    /// then nothing read before is kept, nor the names tooltips gave it. A listing whose name
    /// is cut short is named by the Item Tooltip by the pointer on its slot, if the tooltip's
    /// name starts as its does. A listing read again with no tier (its picture under the
    /// pointer) keeps the tier read there before.
    pub fn read(&mut self, reading: ShopReading) {
        if !self.on {
            return;
        }
        if let Some(list) = &reading.shop {
            if self.seen_at != Some(list.area.x) {
                self.forget();
                self.seen_at = Some(list.area.x);
            }
            let otherwise = list.rows.iter().filter(|row| whole(row)).any(|row| {
                self.seen.iter().any(|seen| same_slot(seen, row) && (seen.name != row.name || seen.price != row.price))
            });
            let gone = self.seen.iter().any(|seen| !list.rows.iter().any(|row| same_slot(row, seen)));
            if otherwise || gone {
                self.forget();
            }
            for row in list.rows.iter().filter(|row| whole(row)) {
                match self.seen.iter_mut().find(|seen| same_slot(seen, row)) {
                    // The tier read before stands while the pointer covers the picture.
                    Some(seen) => *seen = ShopRow { tier: row.tier.or(seen.tier), ..row.clone() },
                    None => self.seen.push(row.clone()),
                }
            }
            if let Some(hover) = &reading.hover {
                self.name_hovered(list, hover);
            }
        }
        self.shop = reading.shop;
        self.hover = reading.hover;
    }

    /// Name the listing whose slot the pointer is on by the tooltip by it, if the name read
    /// there (now, or before while it is covered) is cut short and the tooltip's starts so.
    fn name_hovered(&mut self, list: &ShopList, hover: &ShopHover) {
        let ItemTooltip::Found { name: Some(tooltip), .. } = &hover.tooltip else { return };
        let Some(row) = list.rows.iter().find(|row| on_slot(list, row, hover)) else { return };
        let seen = self.seen.iter().find(|seen| same_slot(seen, row));
        let read = if row.covered { seen.and_then(|seen| seen.name.as_ref()) } else { row.name.as_ref() };
        let Some(cut) = read.filter(|read| cut_start(read).is_some()).cloned() else { return };
        if !starts_as(tooltip, &cut) {
            return;
        }
        self.named.retain(|named| (named.top - row.area.y).abs() > SAME_SLOT);
        self.named.push(Named { top: row.area.y, cut, tooltip: tooltip.clone() });
    }

    pub fn shop(&self) -> Option<&ShopList> {
        self.shop.as_ref()
    }

    /// Where the shop's window is on screen, across, and its list's listings, down; `None`
    /// while no shop was found.
    pub fn shop_window(&self) -> Option<ScreenRect> {
        let shop = self.shop.as_ref()?;
        let ((left, right), list) = (shop_reach(shop), shop.area);
        Some(ScreenRect { x: list.x - left, width: (right + left) as u32, ..list })
    }

    /// The items the listing `row`'s badge is about, if it names one.
    pub fn row_items(&self, row: u32) -> Option<Vec<u32>> {
        self.items.get(row as usize).cloned().flatten()
    }

    /// Each listing's badge, judged against `line` (maplestory.market's figures from `market`)
    /// with the Deal Margin `margin_percent`, its items from `items` (none yet: every badge is
    /// "?"); tops from `top`, the badges' window's top on screen.
    /// `opened` is the badge (on screen) the Item Card showing opened from, with its items.
    pub fn badges(
        &mut self,
        items: Option<&ItemSet>,
        line: Option<DealLine>,
        market: &Market,
        margin_percent: u32,
        top: i32,
        opened: Option<(ScreenRect, &[u32])>,
    ) -> Vec<DealBadge> {
        let Some(shop) = &self.shop else {
            self.items.clear();
            return Vec::new();
        };
        let mut judged = Vec::with_capacity(shop.rows.len());
        let badges = shop
            .rows
            .iter()
            .enumerate()
            .map(|(row, shown)| {
                // Covered, what was read there before; or so under the pointer, where its name
                // reads as before and the tooltip by the pointer hides its price.
                let in_slot = self.seen.iter().find(|seen| same_slot(seen, shown));
                let hidden = shown.price.is_none()
                    && shown.name.is_some()
                    && in_slot.is_some_and(|seen| seen.name == shown.name)
                    && self.hover.as_ref().is_some_and(|hover| on_slot(shop, shown, hover));
                let seen = in_slot.filter(|_| shown.covered || hidden);
                let listing = seen.unwrap_or(shown);
                // Its picture's tier; with the pointer on the picture, the one read there before.
                let tier = listing.tier.or_else(|| in_slot.filter(|seen| seen.name == listing.name).and_then(|seen| seen.tier));
                let named = items.zip(listing.name.as_deref()).and_then(|(items, name)| {
                    item_named(items, name).or_else(|| {
                        let top = shown.area.y;
                        let by_picture = tier.and_then(|tier| picture_named(items, name, tier));
                        let by_tooltip = self
                            .named
                            .iter()
                            .find(|named| (named.top - top).abs() <= SAME_SLOT && named.cut == name)
                            .and_then(|named| tooltip_named(items, &named.tooltip, name));
                        // Naming another item than the picture does, or a scroll of another tier,
                        // one of them was misread.
                        let other_tier = by_tooltip.as_ref().is_some_and(|(item, _)| tier.zip(item.tier).is_some_and(|(a, b)| a != b));
                        match (by_picture, by_tooltip) {
                            _ if other_tier => None,
                            (Some((item, ids)), Some((_, tooltip_ids))) => (ids == tooltip_ids).then_some((item, ids)),
                            (picture, tooltip) => picture.or(tooltip),
                        }
                    })
                });
                let ids = named.as_ref().map(|(_, ids)| ids.clone());
                let open = opened.is_some_and(|(badge, opened)| badge.y == shown.area.y && ids.as_deref() == Some(opened));
                judged.push(ids);
                let verdict = match (named, listing.price) {
                    (Some((item, ids)), Some(listed)) => {
                        let item = DealItem { id: item_id(ids[0]), name: item.name.clone(), rechargeable: item.rechargeable };
                        let price = match line {
                            Some(DealLine::Line(line)) => line.prices.get(&item.id).map(|price| price.mesos),
                            Some(DealLine::Source(source)) => market.price(source, &item.id),
                            None => None,
                        };
                        match price {
                            Some(mesos) => judge(item, listed, mesos, margin_percent),
                            None => Verdict::NoPrice { item, listed },
                        }
                    }
                    _ => Verdict::Unknown,
                };
                DealBadge {
                    row: row as u32,
                    top: (shown.area.y - top).max(0) as u32,
                    height: shown.area.height,
                    covered: shown.covered || seen.is_some(),
                    open,
                    verdict,
                }
            })
            .collect();
        self.items = judged;
        badges
    }

    /// Where the badges' window goes over `game`: `width` wide, as tall as the shop's list,
    /// just left of the shop's window, or right of it with no room on the left; kept inside
    /// the client area and off `avoid` (the captured regions). `None` while no shop was
    /// found.
    pub fn placement(&self, game: &GameWindow, avoid: &[ScreenRect]) -> Option<ScreenRect> {
        let shop = self.shop.as_ref()?;
        let (list, span) = (shop.area, shop.span());
        let client = game.client;
        let width = self.size.width.min(client.width);
        let (shop_left, shop_right) = shop_reach(shop);
        let left = list.x - shop_left - SHOP_GAP - width as i32;
        let x = if left >= client.x { left } else { list.x + shop_right + SHOP_GAP };
        let strip = ScreenRect { x, y: span.y, width, height: span.height.min(client.height) };
        Some(clear_of(strip, avoid, client))
    }
}

/// The verdict on `listed` for `item`, against `line_mesos` on the Deal Line.
fn judge(item: DealItem, listed: ListedPrice, line_mesos: u64, margin_percent: u32) -> Verdict {
    // The price of one; of a throwing star's or a bullet's set, the listing's.
    let each = if item.rechargeable { listed.mesos as f64 } else { listed.mesos as f64 / listed.quantity as f64 };
    let line = line_mesos as f64;
    let percent = if line > 0.0 { ((each - line) / line * 100.0).round() as i32 } else { 0 };
    let deal = each < line && each <= line * (1.0 - f64::from(margin_percent) / 100.0);
    if deal {
        let grade = match -percent {
            below if below >= STEAL_BELOW => DealGrade::Steal,
            below if below >= GREAT_BELOW => DealGrade::Great,
            _ => DealGrade::Good,
        };
        Verdict::Deal { item, listed, percent, line_mesos, grade }
    } else {
        Verdict::NoDeal { item, listed, percent, line_mesos }
    }
}

/// The item a listing's name read as `read` is, with the items that look alike (its name,
/// kind and shop price; the first keeps the market prices): the item of that name, but for
/// the letters the recogniser confuses; or, for a name cut short ("Pet Equip Speed Scroll:
/// L..."), the one item whose name starts so. `None` when no item, or several that don't
/// look alike, have it.
fn item_named<'a>(items: &'a ItemSet, read: &str) -> Option<(&'a ItemFacts, Vec<u32>)> {
    let found = match cut_start(read) {
        Some(start) => items.starting_with(start),
        None => items.named(read),
    };
    alike(found)
}

/// The item an Item Tooltip's name read as `tooltip` is, for the listing whose name the game
/// cut short as `cut`: as [`item_named`] has a whole name, when its name starts as `cut`
/// does.
fn tooltip_named<'a>(items: &'a ItemSet, tooltip: &str, cut: &str) -> Option<(&'a ItemFacts, Vec<u32>)> {
    let (item, ids) = alike(items.named(tooltip))?;
    starts_as(&item.name, cut).then_some((item, ids))
}

/// The scroll a listing whose name the game cut short as `cut` is, by the tier its picture
/// shows: when every item whose name starts so is a scroll, the one name of that tier among
/// them, as [`item_named`] has a whole name. `None` when an item that isn't a scroll starts
/// so too, or no name or several are of that tier.
fn picture_named<'a>(items: &'a ItemSet, cut: &str, tier: ScrollTier) -> Option<(&'a ItemFacts, Vec<u32>)> {
    let names = items.names_starting_with(cut_start(cut)?);
    if names.iter().flatten().any(|item| item.tier.is_none()) {
        return None;
    }
    let mut of_tier = names.into_iter().filter(|name| name.iter().all(|item| item.tier == Some(tier)));
    match (of_tier.next(), of_tier.next()) {
        (Some(name), None) => alike(name),
        _ => None,
    }
}

/// The first of `found`, with them all, when they look alike (their kinds and shop prices).
fn alike(found: Vec<&ItemFacts>) -> Option<(&ItemFacts, Vec<u32>)> {
    let first = *found.first()?;
    let alike = found.iter().all(|item| item.kind == first.kind && item.shop == first.shop);
    alike.then(|| (first, found.iter().map(|item| item.id).collect()))
}

/// Where a name the game cut short ("Pet Equip Speed Scroll: L...") starts; `None` for a
/// whole one.
fn cut_start(read: &str) -> Option<&str> {
    let trimmed = read.trim_end_matches(['.', '…', ' ']);
    (trimmed.len() < read.trim_end().len()).then_some(trimmed)
}

/// Whether `name` starts as `cut`, a name the game cut short, does: compared as item names
/// are.
fn starts_as(name: &str, cut: &str) -> bool {
    let start = item_key(cut_start(cut).unwrap_or(cut));
    !start.is_empty() && item_key(name).starts_with(&start)
}

/// Whether `hover`'s pointer is on `row`'s slot in `list`: its band, across the list's
/// width, the pictures' column included.
fn on_slot(list: &ShopList, row: &ShopRow, hover: &ShopHover) -> bool {
    let slot = ScreenRect { x: list.area.x, width: list.area.width, ..row.area };
    slot.holds(hover.pointer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> DealItem {
        DealItem { id: "04020000".into(), name: "Garnet Ore".into(), rechargeable: false }
    }

    fn verdict(quantity: u64, mesos: u64, line: u64, margin: u32) -> (bool, i32) {
        match judge(item(), ListedPrice { quantity, mesos }, line, margin) {
            Verdict::Deal { percent, .. } => (true, percent),
            Verdict::NoDeal { percent, .. } => (false, percent),
            _ => unreachable!(),
        }
    }

    #[test]
    fn a_deal_is_one_item_below_the_line_by_the_margin() {
        assert_eq!(verdict(1, 999, 1_000, 0), (true, 0));
        assert_eq!(verdict(1, 1_000, 1_000, 0), (false, 0));
        assert_eq!(verdict(10, 6_800, 1_000, 0), (true, -32));
        assert_eq!(verdict(1, 950, 1_000, 10), (false, -5));
        assert_eq!(verdict(1, 900, 1_000, 10), (true, -10));
        assert_eq!(verdict(1, 1_120, 1_000, 0), (false, 12));
    }
}
