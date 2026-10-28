//! maplestory.market's prices (ADR 0030): what an item goes for between players, from its
//! scans of the Free Market's stores, on every Item Card above the player's own Market Lines,
//! and for the Deal Finder to judge listings against. Two figures: the **Going rate**, its
//! established price (the 3-day average sale, or with too few sales the median asking
//! price), and **Cheapest now**, the cheapest listing for one.
//!
//! Minerva never asks maplestory.market itself: the site's Worker does, with its key, and the
//! core asks the Worker for a few items at a time ([`MarketAsk`]), which the shell makes and
//! answers ([`crate::Minerva::on_market_answer`]). A card's item is asked for as the card
//! shows, a shop's items as the Deal Finder reads it while one of these is the Deal Line.
//! What comes back is kept in memory, never saved: a saved market price would look like the
//! player's own. The player may turn it off in the Settings window: then nothing is asked,
//! and nothing of it shows.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// maplestory.market, which the card's links open and credit.
pub const MARKET_SITE: &str = "https://maplestory.market";

/// Minerva's link code at maplestory.market (its API key's): its terms ask for it on every
/// link to the site, so the visits Minerva brings count for the key. Not a secret.
pub const MARKET_REF: &str = "76rbdbd7";

/// The Going rate's and Cheapest now's IDs, as the Deal Line: no line the player adds is
/// numbered so, and a tool of theirs has no reason to name one so.
pub const GOING_RATE_LINE: &str = "maplestory.market:going-rate";
pub const CHEAPEST_LINE: &str = "maplestory.market:cheapest-now";

/// What the card and the Settings window call them.
pub const GOING_RATE_NAME: &str = "Going rate";
pub const CHEAPEST_NAME: &str = "Cheapest now";

/// The most items one ask holds: the Worker's most, a shop's listings.
const MOST_PER_ASK: usize = 20;

/// The Worker pulls each table again once it is an hour old: an item held from a table that
/// old is asked for again when wanted...
const TABLE_STANDS_MS: u64 = 60 * 60_000;

/// ...but never within a minute of the last ask for it, whatever came of that.
const ASK_AGAIN_MS: u64 = 60_000;

/// Which of maplestory.market's figures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLine {
    GoingRate,
    Cheapest,
}

impl SourceLine {
    /// It, by its ID as the Deal Line.
    pub fn with_id(id: &str) -> Option<Self> {
        match id {
            GOING_RATE_LINE => Some(Self::GoingRate),
            CHEAPEST_LINE => Some(Self::Cheapest),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::GoingRate => GOING_RATE_LINE,
            Self::Cheapest => CHEAPEST_LINE,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::GoingRate => GOING_RATE_NAME,
            Self::Cheapest => CHEAPEST_NAME,
        }
    }
}

/// An ask of the Worker for maplestory.market's figures on a few items, from when the core
/// makes it until the shell tells it the answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketAsk {
    /// Which ask it is: one more with each the core makes. The answer names it.
    pub id: u64,
    /// The items, by the game's item ID (8 digits).
    pub items: Vec<String>,
}

impl MarketAsk {
    /// Where on the site it goes, from [`crate::SITE_URL`]: a `GET`.
    pub fn path(&self) -> String {
        let ids: Vec<&str> = self.items.iter().map(|id| number(id)).collect();
        format!("api/market?ids={}", ids.join(","))
    }
}

/// The game's item ID as maplestory.market writes it: without the leading zeros.
fn number(id: &str) -> &str {
    let trimmed = id.trim_start_matches('0');
    if trimmed.is_empty() { "0" } else { trimmed }
}

/// What an ask came to, as the shell tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MarketAnswer {
    /// The Worker's figures: when it pulled each table (ms since the Unix epoch; `None` for one
    /// it never has, or with no key), and each item's from them, by the game's item ID (8
    /// digits). An item in neither table is left out.
    Answered { prices_at_unix_ms: Option<u64>, listed_at_unix_ms: Option<u64>, items: BTreeMap<String, ItemFigures> },
    /// The Worker couldn't be reached, or answered otherwise: why, in words.
    Failed { why: String },
}

/// One item's figures in maplestory.market's tables.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFigures {
    /// Its row in the established prices; `None` when nothing of it was listed or sold in
    /// the last 3 days.
    pub priced: Option<Priced>,
    /// Its row in what is listed now; `None` when none is listed.
    pub listed: Option<Listed>,
}

/// An item's established price, and what is behind it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Priced {
    /// The Going rate; `None` when maplestory.market has none: too few recent sales, and the
    /// asking prices missing or too far apart.
    pub going_rate: Option<GoingRate>,
    /// Its sales in the last 3 days, and in the last day; `None` when none were seen.
    pub sales: Option<Sales>,
    /// The middle half of its asking prices in the last 3 days; `None` with none.
    pub asking: Option<Asking>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoingRate {
    /// For one, in mesos.
    pub mesos: u64,
    pub basis: Basis,
    /// How many sales, or listings, it is from.
    pub samples: u64,
}

/// What the Going rate is: the 3-day average sale price, or with too few sales (or sales too
/// far from what is asked) the 3-day median asking price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Basis {
    Sold,
    Listed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sales {
    pub sales: u64,
    pub today: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asking {
    /// The 25th and 75th percentiles of the asking price for one, in mesos.
    pub low: u64,
    pub high: u64,
    pub listings: u64,
}

/// What is listed now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Listed {
    /// The cheapest listing's price for one, in mesos: a throwing star's or a bullet's for the
    /// set, as its listing is one.
    pub cheapest: u64,
    pub listings: u64,
}

/// What the Worker answered `status` with `body` comes to: its figures, or why not.
pub fn read_market_answer(status: u16, body: &str) -> MarketAnswer {
    #[derive(Deserialize)]
    struct Answer {
        prices: Option<Table<PricesRow>>,
        listed: Option<Table<ListedRow>>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Table<Row> {
        pulled_at: u64,
        items: BTreeMap<String, Row>,
    }
    #[derive(Deserialize)]
    struct PricesRow {
        reference: Option<Reference>,
        sold: Option<Sold>,
        seen: Option<Seen>,
    }
    #[derive(Deserialize)]
    struct Reference {
        unit_price: f64,
        basis: String,
        samples: u64,
    }
    #[derive(Deserialize)]
    struct Sold {
        sales: u64,
        day_sales: u64,
    }
    #[derive(Deserialize)]
    struct Seen {
        listings: u64,
        p25_unit_price: f64,
        p75_unit_price: f64,
    }
    #[derive(Deserialize)]
    struct ListedRow {
        min_unit_price: f64,
        listings: u64,
    }
    match status {
        200 => {}
        429 => return MarketAnswer::Failed { why: "asked too often".into() },
        status => return MarketAnswer::Failed { why: format!("the site answered {status}") },
    }
    let answer: Answer = match serde_json::from_str(body) {
        Ok(answer) => answer,
        Err(e) => return MarketAnswer::Failed { why: format!("an answer in another shape: {e}") },
    };
    // The game's item ID as the game writes it, 8 digits, from maplestory.market's number.
    let key = |id: &str| id.parse::<u32>().ok().map(|id| format!("{id:08}"));
    let mut items: BTreeMap<String, ItemFigures> = BTreeMap::new();
    for (id, row) in answer.prices.iter().flat_map(|table| &table.items) {
        let Some(item) = key(id).map(|id| items.entry(id).or_default()) else { continue };
        let going_rate = row.reference.as_ref().and_then(|r| {
            let basis = match r.basis.as_str() {
                "sold" => Basis::Sold,
                "listed" => Basis::Listed,
                _ => return None,
            };
            Some(GoingRate { mesos: mesos(r.unit_price)?, basis, samples: r.samples })
        });
        let sales = row.sold.as_ref().map(|s| Sales { sales: s.sales, today: s.day_sales });
        let asking = row
            .seen
            .as_ref()
            .and_then(|s| Some(Asking { low: mesos(s.p25_unit_price)?, high: mesos(s.p75_unit_price)?, listings: s.listings }));
        item.priced = Some(Priced { going_rate, sales, asking });
    }
    for (id, row) in answer.listed.iter().flat_map(|table| &table.items) {
        let Some(item) = key(id).map(|id| items.entry(id).or_default()) else { continue };
        item.listed = mesos(row.min_unit_price).map(|cheapest| Listed { cheapest, listings: row.listings });
    }
    MarketAnswer::Answered {
        prices_at_unix_ms: answer.prices.map(|table| table.pulled_at),
        listed_at_unix_ms: answer.listed.map(|table| table.pulled_at),
        items,
    }
}

/// A price for one as maplestory.market gives it (an average may have a fraction), in whole
/// mesos; `None` for no price at all.
fn mesos(price: f64) -> Option<u64> {
    (price.is_finite() && price >= 0.0).then(|| price.round() as u64)
}

/// What one of maplestory.market's rows on a card says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SourceRow {
    /// Asked for, not answered yet.
    Loading,
    /// The Worker couldn't be reached, or has nothing from maplestory.market.
    NotAvailable,
    /// Nothing of the item listed (or for the Going rate, sold) on maplestory.market lately.
    NoneListed,
    /// maplestory.market has no Going rate for it: too few recent sales, and the asking prices
    /// missing or too far apart.
    NoPrice,
    GoingRate {
        going_rate: GoingRate,
        sales: Option<Sales>,
        asking: Option<Asking>,
        /// When the Worker pulled the table (ms since the Unix epoch).
        as_of_unix_ms: u64,
    },
    Cheapest {
        listed: Listed,
        as_of_unix_ms: u64,
    },
}

/// maplestory.market's rows on a card, above the player's own lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRows {
    pub going_rate: SourceRow,
    pub cheapest: SourceRow,
    /// It is equipment: maplestory.market's figures are of every copy, whatever its stats.
    pub any_roll: bool,
}

/// One item's figures as held, and from when.
#[derive(Debug, Clone)]
struct Held {
    figures: ItemFigures,
    prices_at: Option<u64>,
    listed_at: Option<u64>,
    /// When the ask that brought them was made (the core's clock).
    asked_at_ms: u64,
}

/// maplestory.market's figures held, and the asks for more under way.
#[derive(Default)]
pub(crate) struct Market {
    held: HashMap<String, Held>,
    /// Items the last ask for failed, and when it was made (the core's clock).
    failed: HashMap<String, u64>,
    /// The asks under way, each with when it was made (the core's clock).
    under_way: Vec<(MarketAsk, u64)>,
    /// The last ask's id: ids only grow.
    last_id: u64,
    /// The items wanted now: the card's, then the shop's.
    wanted: Vec<String>,
}

impl Market {
    /// The items wanted now, as the card and the Deal Finder show: asked for at the next tick
    /// if not held, or held from a table an hour old.
    pub fn want(&mut self, items: Vec<String>) {
        self.wanted = items;
    }

    /// Ask for the items wanted that are due, at most [`MOST_PER_ASK`] an ask; while `on`, or
    /// else nothing is asked, and the asks under way are dropped.
    pub fn tick(&mut self, at_ms: u64, wall_clock: Option<u64>, on: bool) {
        if !on {
            self.stop();
            return;
        }
        let mut due: Vec<String> = Vec::new();
        for item in &self.wanted {
            let asking = self.under_way.iter().any(|(ask, _)| ask.items.contains(item));
            let since = |asked_at: u64| at_ms.saturating_sub(asked_at) >= ASK_AGAIN_MS;
            let is_due = match (self.held.get(item), self.failed.get(item)) {
                _ if asking || due.contains(item) => false,
                (_, Some(&failed_at)) => since(failed_at),
                (None, None) => true,
                (Some(held), None) => since(held.asked_at_ms) && stale(held, at_ms, wall_clock),
            };
            if is_due {
                due.push(item.clone());
            }
        }
        for items in due.chunks(MOST_PER_ASK) {
            self.last_id += 1;
            self.under_way.push((MarketAsk { id: self.last_id, items: items.to_vec() }, at_ms));
        }
    }

    /// Drop the asks under way: their answers change nothing.
    pub fn stop(&mut self) {
        self.under_way.clear();
    }

    pub fn asks(&self) -> Vec<MarketAsk> {
        self.under_way.iter().map(|(ask, _)| ask.clone()).collect()
    }

    /// Ask `id` came to `answer`. One no longer under way changes nothing.
    pub fn answer(&mut self, id: u64, answer: MarketAnswer) {
        let Some(at) = self.under_way.iter().position(|(ask, _)| ask.id == id) else { return };
        let (ask, asked_at_ms) = self.under_way.remove(at);
        match answer {
            MarketAnswer::Answered { prices_at_unix_ms, listed_at_unix_ms, mut items } => {
                for item in ask.items {
                    let figures = items.remove(&item).unwrap_or_default();
                    self.failed.remove(&item);
                    self.held.insert(item, Held { figures, prices_at: prices_at_unix_ms, listed_at: listed_at_unix_ms, asked_at_ms });
                }
            }
            MarketAnswer::Failed { .. } => {
                for item in ask.items {
                    self.failed.insert(item, asked_at_ms);
                }
            }
        }
    }

    /// maplestory.market's rows for item `id` (8 digits): what is held, else whether it is
    /// being asked for or couldn't be had.
    pub fn rows(&self, id: &str) -> MarketRows {
        let any_roll = id.parse::<u32>().is_ok_and(|id| id / 1_000_000 == 1);
        let Some(held) = self.held.get(id) else {
            let row = if self.failed.contains_key(id) { SourceRow::NotAvailable } else { SourceRow::Loading };
            return MarketRows { going_rate: row.clone(), cheapest: row, any_roll };
        };
        let going_rate = match (held.prices_at, &held.figures.priced) {
            (None, _) => SourceRow::NotAvailable,
            (Some(_), None) => SourceRow::NoneListed,
            (Some(_), Some(Priced { going_rate: None, .. })) => SourceRow::NoPrice,
            (Some(at), Some(Priced { going_rate: Some(going_rate), sales, asking })) => {
                SourceRow::GoingRate { going_rate: *going_rate, sales: *sales, asking: *asking, as_of_unix_ms: at }
            }
        };
        let cheapest = match (held.listed_at, held.figures.listed) {
            (None, _) => SourceRow::NotAvailable,
            (Some(_), None) => SourceRow::NoneListed,
            (Some(at), Some(listed)) => SourceRow::Cheapest { listed, as_of_unix_ms: at },
        };
        MarketRows { going_rate, cheapest, any_roll }
    }

    /// `line`'s price for item `id` (8 digits), for the Deal Finder; `None` while none is held.
    pub fn price(&self, line: SourceLine, id: &str) -> Option<u64> {
        let figures = &self.held.get(id)?.figures;
        match line {
            SourceLine::GoingRate => figures.priced.as_ref()?.going_rate.map(|rate| rate.mesos),
            SourceLine::Cheapest => figures.listed.map(|listed| listed.cheapest),
        }
    }
}

/// Whether `held` is from a table the Worker will have pulled again since: an hour old by the
/// wall clock, or with none known, asked for an hour ago.
fn stale(held: &Held, at_ms: u64, wall_clock: Option<u64>) -> bool {
    let pulled = [held.prices_at, held.listed_at].into_iter().flatten().min();
    match (wall_clock, pulled) {
        (Some(now), Some(pulled)) => now.saturating_sub(pulled) >= TABLE_STANDS_MS,
        _ => at_ms.saturating_sub(held.asked_at_ms) >= TABLE_STANDS_MS,
    }
}
