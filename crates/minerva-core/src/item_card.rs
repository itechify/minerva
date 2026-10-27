//! The Item Card (ADR 0010): the player hovers an item in the game and presses the hotkey
//! (Ctrl+Shift+I by default). The shell captures the game's client area and the pointer,
//! the Item Tooltip reader reads the item's name, and the card shows at the pointer what
//! the game's tooltip doesn't: what a shop pays for the item, and the market prices the
//! player keeps for it on their Market Lines (ADR 0011), with links to its MeowDB page and
//! to its page on maplestory.market, the market's listings and prices. The name is matched against OSMS's items on the player's
//! machine: nothing is asked at the press.
//!
//! The player can also find an item in Search: its card then shows beside the panel.
//!
//! One card shows at a time, until the player closes it or presses the hotkey again. What
//! keeps a card from showing, the owl says in the Owl Bubble (ADR 0025).

use crate::grind_tracker::ClientOffset;
use crate::item_lookup::{ItemLookup, OurWindow, item_name};
use crate::item_tooltip::{ItemTooltip, ItemTooltipReading};
use crate::market_prices::{MarketPrice, MarketPrices};
use crate::osms_items::{ItemFacts, ItemSet};
use crate::panel::{GameWindow, ScreenPoint, ScreenRect, Size, in_view};
use serde::Serialize;

/// What a shop pays for an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ShopPrice {
    /// A shop pays this for one.
    Sells { mesos: u64 },
    /// A shop won't buy it.
    NotSold,
    /// A Cash Shop item.
    Cash,
    /// OSMS has no price for it (scrolls, starter gear, arrows), or it is a throwing star or
    /// a bullet: OSMS gives what recharging them costs, not what a shop pays for them.
    Unknown,
}

/// The Item Card's credit (ADR 0010): its data is OSMS Data Explorer's, its name opens
/// NiaMeowDB's page and its market section the item's page on maplestory.market.
pub const ITEM_CARD_CREDIT: &str = "Data: OSMS Data Explorer · Pages: NiaMeowDB, maplestory.market";

/// The item's page on maplestory.market, the listings in the Free Market's stores and the
/// prices they sell at: the market section's buttons open it in the player's browser. Its
/// address takes the game's item ID as a number, without the leading zeros the game writes.
pub fn market_page(id: &str) -> String {
    format!("https://maplestory.market/items/{}", id.trim_start_matches('0'))
}

/// One of the card's Market Lines, with the price the player keeps on it for the item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketLine {
    /// What tells the line from the others, whatever it is named.
    pub id: String,
    pub name: String,
    /// The price the player keeps on it for the item; `None` until they set one.
    pub price: Option<MarketPrice>,
    /// It is the Deal Line, which the Deal Finder compares listings with (ADR 0014).
    pub deal_line: bool,
}

/// What the card's market section has open: a field for the player to type in, or the
/// question before a line goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "field", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MarketEdit {
    /// A line's price field.
    Price {
        line: String,
        /// What the player last entered that isn't a price, to say so; `None` until then.
        not_a_price: Option<String>,
    },
    /// A line's name field, to rename it (or remove it, from there); `line` is `None` for
    /// a line the player is adding.
    Name {
        line: Option<String>,
        /// Why what the player last entered can't be the line's name; `None` until then.
        problem: Option<NameProblem>,
    },
    /// Whether to remove a line: it goes with its prices on `prices` items.
    Remove { line: String, prices: usize },
}

/// Why what the player entered can't be a Market Line's name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum NameProblem {
    /// Nothing but spaces.
    Empty,
    /// Another line has the name, whatever its case.
    Taken { name: String },
}

/// One item's Item Card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemCard {
    /// The game's item ID, 8 digits as the game writes them (`"02000000"`).
    pub id: String,
    pub name: String,
    /// Where the overlay loads its picture from (OSMS's, as the game draws it), once it is
    /// in; `None` until then.
    pub icon: Option<String>,
    pub shop: ShopPrice,
    /// Every Market Line, in the player's order, each with the item's price on it.
    pub market: Vec<MarketLine>,
    /// What the market section has open, while it has something.
    pub market_edit: Option<MarketEdit>,
    /// When OSMS's item files were saved (ms since the Unix epoch), if that was over 7 days
    /// ago: the card shows a Saved Copy, as OSMS couldn't be reached since. `None` for
    /// files fresher than that.
    pub saved_copy_from_unix_ms: Option<u64>,
    /// [`ITEM_CARD_CREDIT`].
    pub credit: &'static str,
}

/// One of several items sharing the name read, for the player to pick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemCardChoice {
    /// The game's item ID, 8 digits.
    pub id: String,
    /// What kind of item it is: a weapon's kind ("One-Handed Sword"), other equipment's
    /// slot ("Overall"), or its inventory tab ("Use", "Etc", "Setup"); "Scroll" or "Cash".
    pub kind: String,
    /// Where the overlay loads its picture from (OSMS's, as the game draws it), once it is
    /// in; `None` until then.
    pub icon: Option<String>,
}

/// What the Item Card shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ItemCardView {
    /// One item's card.
    Card(ItemCard),
    /// Several items have the name read, and don't look alike (their kinds or shop prices
    /// differ): the player picks theirs, by the picture the game's tooltip shows.
    Choose { name: String, items: Vec<ItemCardChoice> },
}

/// A side of the Item Card's window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Edge {
    Left,
    Right,
}

/// The Item Card showing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShownItemCard {
    /// Counts the cards the player opened, so the window can tell a new card from the last
    /// one when both show the same item.
    pub card: u32,
    /// Opened from a Deal Badge, the edge of the card that faces it: the card unfolds out of
    /// the badge from there. `None` for a card opened otherwise, or dragged since.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unfold_from: Option<Edge>,
    #[serde(flatten)]
    pub view: ItemCardView,
}

/// What a press of the Item Card's hotkey came to.
pub(crate) enum Pressed {
    /// The card of the item read shows.
    Shown,
    /// No Item Tooltip by the pointer: the card showing closed, and nothing is said.
    Closed,
    /// The owl says why no card shows.
    Say(ItemLookup),
}

/// The Item Card showing, if any.
#[derive(Default)]
pub(crate) struct ItemCards {
    shown: Option<Shown>,
    cards: u32,
    /// The card's size, in physical px for the current display.
    pub size: Size,
}

/// The card showing: which items, where it opened, and where the player dragged it.
struct Shown {
    card: u32,
    opened: Opened,
    /// Where the player dragged the card, relative to the game's client area; `None` while
    /// it is at the pointer.
    moved: Option<ClientOffset>,
    showing: Showing,
    /// What its market section has open.
    editing: Option<Editing>,
}

/// What the card's market section has open, as [`MarketEdit`] says it without what the
/// prices tell (how many a line has).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Editing {
    Price { line: String, not_a_price: Option<String> },
    Name { line: Option<String>, problem: Option<NameProblem> },
    Remove { line: String },
}

/// Where a card opened: at the pointer, where the hotkey was pressed over the item's
/// tooltip; beside the panel, picked from Search; or beside a Deal Badge (`badge`, on
/// screen) the player clicked, off the Player Shop (`shop`).
#[derive(Clone, Copy)]
enum Opened {
    AtPointer(ScreenPoint),
    BesidePanel,
    BesideBadge { badge: ScreenRect, shop: ScreenRect },
}

/// What the card showing shows.
enum Showing {
    /// The card of these items, which share their name, kind and shop price: they look
    /// alike, so they show one card.
    Card(Vec<u32>),
    /// These items share their name, but not their kind or shop price: the player picks
    /// one, by its picture and kind. Each is listed once for the items that look alike.
    Choose(Vec<Vec<u32>>),
}

impl ItemCards {
    /// What the capture read as (`None`: it failed), with where our windows are on screen,
    /// matched against `items`.
    pub fn read(
        &mut self,
        reading: Option<ItemTooltipReading>,
        ours: &[(OurWindow, ScreenRect)],
        items: Result<&ItemSet, ItemLookup>,
    ) -> Pressed {
        // One card at a time: the press replaces the card showing, or closes it.
        let had_card = self.shown.take().is_some();
        let Some(reading) = reading else { return Pressed::Say(ItemLookup::Unreadable) };
        let (area, name) = match &reading.tooltip {
            ItemTooltip::None if had_card => return Pressed::Closed,
            ItemTooltip::None => return Pressed::Say(ItemLookup::NoTooltip),
            ItemTooltip::Found { area, name } => (*area, name),
        };
        if let Some((by, _)) = ours.iter().find(|(_, window)| window.overlaps(area)) {
            return Pressed::Say(ItemLookup::Covered { by: *by });
        }
        let Some(read) = name.as_deref().and_then(item_name) else { return Pressed::Say(ItemLookup::Unreadable) };
        let items = match items {
            Ok(items) => items,
            Err(missing) => return Pressed::Say(missing),
        };
        let mut alike: Vec<(&ItemFacts, Vec<u32>)> = Vec::new();
        for item in items.named(&read) {
            match alike.iter_mut().find(|(first, _)| first.kind == item.kind && first.shop == item.shop) {
                Some((_, ids)) => ids.push(item.id),
                None => alike.push((item, vec![item.id])),
            }
        }
        let mut groups: Vec<Vec<u32>> = alike.into_iter().map(|(_, ids)| ids).collect();
        let showing = match groups.len() {
            0 => return Pressed::Say(ItemLookup::NoSuchItem { read }),
            1 => Showing::Card(groups.remove(0)),
            _ => Showing::Choose(groups),
        };
        self.open(Opened::AtPointer(reading.pointer), showing);
        Pressed::Shown
    }

    /// The player picked the items `ids`, which look alike, from Search: their card shows
    /// beside the panel, in place of any card showing.
    pub fn show(&mut self, ids: Vec<u32>) {
        self.open(Opened::BesidePanel, Showing::Card(ids));
    }

    /// The player clicked the Deal Badge at `badge` (on screen) of a listing of the items
    /// `ids`, which look alike, in the Player Shop at `shop`: their card shows beside the
    /// badge, off the shop, in place of any card showing.
    pub fn show_beside_badge(&mut self, ids: Vec<u32>, badge: ScreenRect, shop: ScreenRect) {
        self.open(Opened::BesideBadge { badge, shop }, Showing::Card(ids));
    }

    /// The Deal Badge (on screen) the card showing opened beside, with its items, while it
    /// stays there (not dragged).
    pub fn badge(&self) -> Option<(ScreenRect, &[u32])> {
        match self.shown.as_ref()? {
            Shown { opened: Opened::BesideBadge { badge, .. }, moved: None, showing: Showing::Card(ids), .. } => {
                Some((*badge, ids.as_slice()))
            }
            _ => None,
        }
    }

    fn open(&mut self, opened: Opened, showing: Showing) {
        self.cards += 1;
        self.shown = Some(Shown { card: self.cards, opened, moved: None, showing, editing: None });
    }

    /// The player picked item `id` from the items listed: its card shows in the list's
    /// place. Nothing changes for an item not listed.
    pub fn pick(&mut self, id: &str) {
        let Some(Shown { showing, .. }) = &mut self.shown else { return };
        let Showing::Choose(groups) = showing else { return };
        if let Some(picked) = groups.iter().find(|ids| ids.first().is_some_and(|&first| item_id(first) == id)) {
            *showing = Showing::Card(picked.clone());
        }
    }

    /// The pictures the card showing wants, below [`crate::osms::PICTURES_URL`]: its item's,
    /// or those of the items it lists to pick from.
    pub fn pictures(&self) -> Vec<String> {
        match &self.shown {
            Some(Shown { showing: Showing::Card(ids), .. }) => ids.first().map(|&id| icon_path(&item_id(id))).into_iter().collect(),
            Some(Shown { showing: Showing::Choose(groups), .. }) => {
                groups.iter().filter_map(|ids| ids.first()).map(|&id| icon_path(&item_id(id))).collect()
            }
            None => Vec::new(),
        }
    }

    /// The player closed the card.
    pub fn close(&mut self) {
        self.shown = None;
    }

    /// The market section opens `editing`, in place of what it had open, on a card (not a
    /// choice).
    pub fn edit(&mut self, editing: Editing) {
        if let Some(shown @ Shown { showing: Showing::Card(_), .. }) = &mut self.shown {
            shown.editing = Some(editing);
        }
    }

    /// What the market section has open, if anything.
    pub fn editing(&self) -> Option<&Editing> {
        self.shown.as_ref()?.editing.as_ref()
    }

    /// The market section closes what it had open, the prices as they were.
    pub fn close_market(&mut self) {
        if let Some(shown) = &mut self.shown {
            shown.editing = None;
        }
    }

    /// The item whose card shows (8 digits), not a choice: the first of the items that
    /// look alike, under which they keep one price on each line.
    pub fn card_item(&self) -> Option<String> {
        match &self.shown {
            Some(Shown { showing: Showing::Card(ids), .. }) => ids.first().map(|&id| item_id(id)),
            _ => None,
        }
    }

    /// The name and kind of the item whose card shows, as `items` has it; `None` while no
    /// card shows, or a choice does.
    pub fn item<'a>(&self, items: &'a ItemSet) -> Option<&'a ItemFacts> {
        match &self.shown {
            Some(Shown { showing: Showing::Card(ids), .. }) => items.item(*ids.first()?),
            _ => None,
        }
    }

    /// The player dragged the card showing so its top-left corner is at (x, y) on screen,
    /// over `game`: it stays there, with the game window, until it closes.
    pub fn moved_to(&mut self, x: i32, y: i32, game: &GameWindow) {
        if let Some(shown) = &mut self.shown {
            shown.moved = Some(ClientOffset { x: x - game.client.x, y: y - game.client.y });
        }
    }

    /// Where the card showing goes over `game`: where the player dragged it, wholly on a
    /// monitor and moved off any of `avoid` (the captured regions); or else at the pointer
    /// ([`card_placement`]), or beside the `panel` and off our `others` windows as well
    /// ([`beside_panel`]). `None` while no card shows, or while one opened beside the panel
    /// has no panel to go beside.
    pub fn placement(&self, game: &GameWindow, avoid: &[ScreenRect], panel: Option<ScreenRect>, others: &[ScreenRect]) -> Option<ScreenRect> {
        let shown = self.shown.as_ref()?;
        let size = self.size;
        Some(match (shown.moved, shown.opened) {
            (Some(at), _) => {
                let (x, y) = (game.client.x + at.x, game.client.y + at.y);
                in_view(game, ScreenRect { x, y, width: size.width, height: size.height.min(game.client.height) }, avoid)
            }
            (None, Opened::AtPointer(pointer)) => card_placement(game, pointer, size, avoid),
            (None, Opened::BesidePanel) => {
                let panel = panel?;
                let avoid: Vec<ScreenRect> = avoid.iter().chain(others).chain([&panel]).copied().collect();
                beside_panel(game, panel, size, &avoid)
            }
            (None, Opened::BesideBadge { badge, shop }) => {
                let avoid: Vec<ScreenRect> = avoid.iter().chain([&badge, &shop]).copied().collect();
                beside_badge(game, badge, shop, size, &avoid)
            }
        })
    }

    /// The card showing, as `items` has its items (a Saved Copy from `saved_copy_from`, if
    /// they are one), with the market prices the player keeps for its item on each line
    /// of `market`, and the pictures in so far (by `picture`, from a path below
    /// [`crate::osms::PICTURES_URL`]).
    pub fn view(
        &self,
        items: Option<&ItemSet>,
        saved_copy_from: Option<u64>,
        market: &MarketPrices,
        picture: impl Fn(&str) -> Option<String>,
    ) -> Option<ShownItemCard> {
        let shown = self.shown.as_ref()?;
        let items = items?;
        let view = match &shown.showing {
            Showing::Card(ids) => {
                let card = card(ids, items, &picture)?;
                let deal_line = market.deal_line().map(|line| line.id.as_str());
                let lines = market
                    .lines()
                    .iter()
                    .map(|line| MarketLine {
                        id: line.id.clone(),
                        name: line.name.clone(),
                        price: line.prices.get(&card.id).copied(),
                        deal_line: deal_line == Some(line.id.as_str()),
                    })
                    .collect();
                ItemCardView::Card(ItemCard {
                    market: lines,
                    market_edit: shown.editing.as_ref().and_then(|editing| market_edit(editing, market)),
                    saved_copy_from_unix_ms: saved_copy_from,
                    ..card
                })
            }
            Showing::Choose(groups) => {
                let listed: Vec<&ItemFacts> = groups.iter().filter_map(|ids| items.item(*ids.first()?)).collect();
                ItemCardView::Choose {
                    name: listed.first()?.name.clone(),
                    items: listed
                        .iter()
                        .map(|item| ItemCardChoice {
                            id: item_id(item.id),
                            kind: item.kind.clone(),
                            icon: picture(&icon_path(&item_id(item.id))),
                        })
                        .collect(),
                }
            }
        };
        Some(ShownItemCard { card: shown.card, unfold_from: None, view })
    }
}

/// What the market section has open, as the card shows it; nothing for a line that has
/// gone since it opened.
fn market_edit(editing: &Editing, market: &MarketPrices) -> Option<MarketEdit> {
    let known = |line: &str| market.line(line).is_some();
    Some(match editing.clone() {
        Editing::Price { line, not_a_price } if known(&line) => MarketEdit::Price { line, not_a_price },
        Editing::Name { line, problem } if line.as_deref().is_none_or(known) => MarketEdit::Name { line, problem },
        Editing::Remove { line } => {
            let prices = market.line(&line)?.prices.len();
            MarketEdit::Remove { line, prices }
        }
        _ => return None,
    })
}

/// The card of the items `ids`, which look alike: the first's, with its picture once
/// `picture` has it.
fn card(ids: &[u32], items: &ItemSet, picture: impl Fn(&str) -> Option<String>) -> Option<ItemCard> {
    let first = items.item(*ids.first()?)?;
    Some(ItemCard {
        id: item_id(first.id),
        name: first.name.clone(),
        icon: picture(&icon_path(&item_id(first.id))),
        shop: first.shop,
        market: Vec::new(),
        market_edit: None,
        saved_copy_from_unix_ms: None,
        credit: ITEM_CARD_CREDIT,
    })
}

/// The game's item ID as the game writes it: 8 digits.
pub(crate) fn item_id(id: u32) -> String {
    format!("{id:08}")
}

/// Where OSMS keeps the picture of item `id` (8 digits), below
/// [`crate::osms::PICTURES_URL`].
pub(crate) fn icon_path(id: &str) -> String {
    format!("current/images/items/{id}.png")
}

/// The card's corner sits this far right of and below the pointer's tip (physical px), like
/// the game's own tooltip: a click without moving the mouse still lands on the item.
const POINTER_GAP: i32 = 8;
/// Gap kept between the card and a captured region it moves clear of (physical px).
const REGION_GAP: i32 = 8;

/// Where the card of `size` goes for the pointer at `pointer`: its corner just below and
/// right of the pointer, or left of it, or above it, whichever first stays inside the
/// game's client area and off `avoid` (the captured regions, since our windows show in our
/// captures). With none of them clear, below and right of the pointer, moved below (or
/// above) the regions it would cover.
fn card_placement(game: &GameWindow, pointer: ScreenPoint, size: Size, avoid: &[ScreenRect]) -> ScreenRect {
    let client = game.client;
    let (width, height) = (size.width.min(client.width), size.height.min(client.height));
    let (right, left) = (pointer.x + POINTER_GAP, pointer.x - POINTER_GAP - width as i32);
    let (below, above) = (pointer.y + POINTER_GAP, pointer.y - POINTER_GAP - height as i32);
    let at = |x, y| ScreenRect { x, y, width, height };
    first_clear(&[at(right, below), at(left, below), at(right, above), at(left, above)], client, avoid)
}

/// The card's side sits this far from the panel's (physical px), as the Grind Tracker's.
const PANEL_GAP: i32 = 8;

/// Where a card of `size` picked from Search goes, and the Settings window until the player
/// drags it: beside the `panel`, the tops level, on the side towards the middle of the game,
/// or else the other side; whichever first stays inside the game's client area and off
/// `avoid` (the captured regions and our other windows). With neither clear, towards the middle, moved inside the client area and below
/// (or above) what it would cover.
pub(crate) fn beside_panel(game: &GameWindow, panel: ScreenRect, size: Size, avoid: &[ScreenRect]) -> ScreenRect {
    let client = game.client;
    let (width, height) = (size.width.min(client.width), size.height.min(client.height));
    let left = ScreenRect { x: panel.x - PANEL_GAP - width as i32, y: panel.y, width, height };
    let right = ScreenRect { x: panel.x + panel.width as i32 + PANEL_GAP, ..left };
    let middle = |r: ScreenRect| 2 * i64::from(r.x) + i64::from(r.width);
    let spots = if middle(panel) >= middle(client) { [left, right] } else { [right, left] };
    first_clear(&spots, client, avoid)
}

/// Where a card of `size` opened from the Deal Badge at `badge` goes: left of it, the tops
/// level, or else right of the Player Shop at `shop`; whichever first stays inside the game's
/// client area and off `avoid` (the captured regions, the badge and the shop, whose
/// listings the Deal Finder reads). With neither clear, left of the badge, moved inside the
/// client area and below (or above) what it would cover.
fn beside_badge(game: &GameWindow, badge: ScreenRect, shop: ScreenRect, size: Size, avoid: &[ScreenRect]) -> ScreenRect {
    let client = game.client;
    let (width, height) = (size.width.min(client.width), size.height.min(client.height));
    let left = ScreenRect { x: badge.x - PANEL_GAP - width as i32, y: badge.y, width, height };
    let right = ScreenRect { x: shop.x + shop.width as i32 + PANEL_GAP, ..left };
    first_clear(&[left, right], client, avoid)
}

/// The first of `spots` (all the same size) wholly inside `client` and off `avoid`. With
/// none of them clear, the first, moved inside `client` and below (or above) any of `avoid`
/// it would cover.
fn first_clear(spots: &[ScreenRect], client: ScreenRect, avoid: &[ScreenRect]) -> ScreenRect {
    let clear = |card: &ScreenRect| !avoid.iter().any(|region| card.overlaps(*region));
    if let Some(card) = spots.iter().find(|card| client.contains(**card) && clear(card)) {
        return *card;
    }
    let mut card = clamp(spots[0], client);
    // Each region is gone over twice, as a move off one may cover another.
    for _ in 0..2 {
        for region in avoid {
            if card.overlaps(*region) {
                let under = region.y + region.height as i32 + REGION_GAP;
                card.y = if client.contains(ScreenRect { y: under, ..card }) { under } else { region.y - REGION_GAP - card.height as i32 };
            }
        }
    }
    card
}

/// `card` moved the least to lie inside `client`, which is at least as big.
fn clamp(card: ScreenRect, client: ScreenRect) -> ScreenRect {
    let x = card.x.clamp(client.x, client.x + (client.width - card.width) as i32);
    let y = card.y.clamp(client.y, client.y + (client.height - card.height) as i32);
    ScreenRect { x, y, ..card }
}
