//! The market prices the player keeps (ADR 0010, ADR 0011): what they found an item sells
//! for between players, typed into its Item Card, for reference, on Market Lines they name
//! ("Your price", "Lowest ask"). Every card lists every line, under maplestory.market's
//! (ADR 0030), and they are saved between launches as the Personal Bests are. And the Deal
//! Line: the line, or maplestory.market's figure, the Deal Finder judges listings against.

use crate::maplestory_market::SourceLine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The shape of [`SavedMarketPrices`] this core writes. Bumped whenever it changes: 2 when
/// the one price became Market Lines (ADR 0011), 3 when a player started with none and no Deal
/// Line chosen became maplestory.market's Going rate (ADR 0030).
pub const MARKET_PRICES_VERSION: u32 = 3;

/// The line a player started with before shape 3, and the one 0.8.0's single price is read
/// into.
const FIRST_LINE_ID: &str = "1";
const FIRST_LINE_NAME: &str = "Your price";

/// A market price the player set for an item, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPrice {
    pub mesos: u64,
    /// When they set it (ms since the Unix epoch); `None` if the wall clock wasn't known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_at_unix_ms: Option<u64>,
}

/// One Market Line: its name, and the price the player keeps on it for each item, by the
/// game's item ID (8 digits).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedMarketLine {
    /// What tells the line from the others, whatever it is named: the core numbers the
    /// lines the player adds, and keeps any other ID as it was written (a tool of the
    /// player's own may fill a line of its own).
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub prices: BTreeMap<String, MarketPrice>,
}

/// The market prices the player keeps, on their Market Lines in the order the card lists
/// them: saved between launches. Missing fields (an empty file) take their defaults: the
/// one line a player starts with. Prices saved in shape 1, one per item (0.8.0), are read
/// as that line's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", from = "SavedShape")]
pub struct SavedMarketPrices {
    pub version: u32,
    pub lines: Vec<SavedMarketLine>,
    /// The Deal Line: what the Deal Finder compares listings with (ADR 0014), by its ID: a
    /// line's, or one of maplestory.market's figures' ([`SourceLine::id`]); `None` until the
    /// player chooses one, for the Going rate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deal_line: Option<String>,
}

impl Default for SavedMarketPrices {
    /// A player starts with no lines of their own: maplestory.market's figures are on every
    /// card.
    fn default() -> Self {
        Self { version: MARKET_PRICES_VERSION, lines: Vec::new(), deal_line: None }
    }
}

fn first_line(prices: BTreeMap<String, MarketPrice>) -> SavedMarketLine {
    SavedMarketLine { id: FIRST_LINE_ID.into(), name: FIRST_LINE_NAME.into(), prices }
}

/// The market prices as saved in any shape: shape 1's `items`, or a later one's `lines`.
#[derive(Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SavedShape {
    version: u32,
    items: BTreeMap<String, MarketPrice>,
    lines: Option<Vec<SavedMarketLine>>,
    deal_line: Option<String>,
}

impl Default for SavedShape {
    fn default() -> Self {
        Self { version: MARKET_PRICES_VERSION, items: BTreeMap::new(), lines: None, deal_line: None }
    }
}

impl From<SavedShape> for SavedMarketPrices {
    /// Shape 1 becomes the one line it had. Shapes 1 and 2 become shape 3: the line every
    /// player started with goes if it holds no prices, and with no Deal Line chosen, the
    /// first line left becomes it, as it was then, so no player's badges change. A later
    /// shape keeps its version, so the shell can tell one it doesn't know.
    fn from(saved: SavedShape) -> Self {
        let (lines, deal_line) = match saved.version {
            ..=1 => (vec![first_line(saved.items)], None),
            2 => (saved.lines.unwrap_or_else(|| vec![first_line(BTreeMap::new())]), saved.deal_line),
            version => return Self { version, lines: saved.lines.unwrap_or_default(), deal_line: saved.deal_line },
        };
        let lines: Vec<SavedMarketLine> = lines
            .into_iter()
            .filter(|line| !(line.id == FIRST_LINE_ID && line.name == FIRST_LINE_NAME && line.prices.is_empty()))
            .collect();
        let deal_line = deal_line.or_else(|| lines.first().map(|line| line.id.clone()));
        Self { version: MARKET_PRICES_VERSION, lines, deal_line }
    }
}

impl SavedMarketPrices {
    /// Whether they were saved in a shape this version reads: this one (one saved before
    /// there were Market Lines reads as this one). The shell sets a file in any other aside.
    pub fn in_a_known_shape(&self) -> bool {
        self.version == MARKET_PRICES_VERSION
    }
}

/// Prices a tool of the player's own leaves for their Market Lines (ADR 0013): a file in
/// `market-prices.json`'s shape in the `market-imports` folder beside it, which Minerva
/// takes in as it comes, onto the lines of the same IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketImport {
    pub version: u32,
    pub lines: Vec<SavedMarketLine>,
}

impl MarketImport {
    /// Whether it is in a shape this version reads: the market prices' own, or shape 2, whose
    /// lines read the same. The shell sets one in any other aside.
    pub fn in_a_known_shape(&self) -> bool {
        (2..=MARKET_PRICES_VERSION).contains(&self.version)
    }
}

/// What the Deal Finder judges listings against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DealLine<'a> {
    /// One of maplestory.market's figures (ADR 0030).
    Source(SourceLine),
    /// One of the player's Market Lines.
    Line(&'a SavedMarketLine),
}

impl DealLine<'_> {
    pub fn id(&self) -> &str {
        match self {
            Self::Source(source) => source.id(),
            Self::Line(line) => &line.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Source(source) => source.name(),
            Self::Line(line) => &line.name,
        }
    }
}

/// The market prices held, and a count of their changes, so the shell saves them only then.
#[derive(Default)]
pub(crate) struct MarketPrices {
    saved: SavedMarketPrices,
    revision: u64,
}

impl MarketPrices {
    pub fn restore(&mut self, saved: SavedMarketPrices) {
        self.saved = saved;
    }

    pub fn saved(&self) -> &SavedMarketPrices {
        &self.saved
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The Market Lines, in the order the card lists them.
    pub fn lines(&self) -> &[SavedMarketLine] {
        &self.saved.lines
    }

    pub fn line(&self, line: &str) -> Option<&SavedMarketLine> {
        self.saved.lines.iter().find(|known| known.id == line)
    }

    /// The Deal Line, with maplestory.market's figures `from_market` or not: what the player
    /// chose, or the Going rate while they chose none or removed the line they chose. Without
    /// maplestory.market's figures, the first line in place of one of them. `None` with
    /// nothing to judge against.
    pub fn deal_line(&self, from_market: bool) -> Option<DealLine<'_>> {
        let chosen = self.saved.deal_line.as_deref();
        if let Some(line) = chosen.and_then(|id| self.line(id)) {
            return Some(DealLine::Line(line));
        }
        let source = chosen.and_then(SourceLine::with_id).unwrap_or(SourceLine::GoingRate);
        if from_market { Some(DealLine::Source(source)) } else { self.saved.lines.first().map(DealLine::Line) }
    }

    /// The player chose `line` as the Deal Line: a line's ID, or one of maplestory.market's
    /// figures'. Nothing changes for a line not held.
    pub fn choose_deal_line(&mut self, line: &str) {
        if self.line(line).is_none() && SourceLine::with_id(line).is_none() {
            return;
        }
        let changed = self.saved.deal_line.as_deref() != Some(line);
        self.saved.deal_line = Some(line.to_string());
        self.changed(changed);
    }

    /// Keep `price` for item `id` (8 digits) on `line`, or forget its price there (`None`).
    pub fn set(&mut self, line: &str, id: &str, price: Option<MarketPrice>) {
        let Some(line) = self.saved.lines.iter_mut().find(|known| known.id == line) else { return };
        let changed = match price {
            Some(price) => line.prices.insert(id.to_string(), price) != Some(price),
            None => line.prices.remove(id).is_some(),
        };
        self.changed(changed);
    }

    /// Whether another line than `except` is named `name`, whatever its case.
    pub fn name_taken(&self, name: &str, except: Option<&str>) -> bool {
        let name = name.to_lowercase();
        self.saved.lines.iter().any(|line| Some(line.id.as_str()) != except && line.name.to_lowercase() == name)
    }

    /// A new line named `name`, at the end, with no prices: numbered one past the highest
    /// number among the lines' IDs, so it takes none of theirs.
    pub fn add(&mut self, name: &str) {
        let highest = self.saved.lines.iter().filter_map(|line| line.id.parse::<u64>().ok()).max().unwrap_or(0);
        let id = (highest + 1).to_string();
        self.saved.lines.push(SavedMarketLine { id, name: name.to_string(), prices: BTreeMap::new() });
        self.changed(true);
    }

    pub fn rename(&mut self, line: &str, name: &str) {
        let Some(line) = self.saved.lines.iter_mut().find(|known| known.id == line) else { return };
        let changed = line.name != name;
        line.name = name.to_string();
        self.changed(changed);
    }

    /// Remove `line`, and its prices on every item. The Deal Line, if it was, goes back to
    /// the Going rate: a line added later may be numbered as this one was.
    pub fn remove(&mut self, line: &str) {
        let before = self.saved.lines.len();
        self.saved.lines.retain(|known| known.id != line);
        if self.saved.deal_line.as_deref() == Some(line) {
            self.saved.deal_line = None;
        }
        self.changed(self.saved.lines.len() != before);
    }

    /// Take in `import`'s prices, each onto the line of its ID, the newest winning: a price
    /// held stays if it was set after the one imported (the player typed it since). A line
    /// not held is added at the end, named as the import names it, with a number after
    /// the name if another line has it. Nothing is removed, and a line held keeps its name.
    pub fn import(&mut self, import: MarketImport) {
        let mut changed = false;
        for imported in import.lines {
            let at = match self.saved.lines.iter().position(|line| line.id == imported.id) {
                Some(at) => at,
                None => {
                    let name = self.free_name(&imported.name, &imported.id);
                    self.saved.lines.push(SavedMarketLine { id: imported.id, name, prices: BTreeMap::new() });
                    changed = true;
                    self.saved.lines.len() - 1
                }
            };
            let prices = &mut self.saved.lines[at].prices;
            for (id, price) in imported.prices {
                let newer = prices.get(&id).is_none_or(|held| held.set_at_unix_ms.unwrap_or(0) <= price.set_at_unix_ms.unwrap_or(0));
                if newer && prices.insert(id, price) != Some(price) {
                    changed = true;
                }
            }
        }
        self.changed(changed);
    }

    /// `name` (the line's ID `id` if it is only spaces), or, if another line has it in any
    /// case, the name with the first number from 2 after it that no line has.
    fn free_name(&self, name: &str, id: &str) -> String {
        let name = if name.trim().is_empty() { id.trim() } else { name.trim() };
        if !self.name_taken(name, None) {
            return name.to_string();
        }
        (2..).map(|n| format!("{name} {n}")).find(|numbered| !self.name_taken(numbered, None)).expect("a number no line has")
    }

    fn changed(&mut self, changed: bool) {
        if changed {
            self.revision += 1;
        }
    }
}

/// The most a price may be: more mesos than the game lets a player hold, many times over.
const MOST_MESOS: f64 = 1e13;

/// A price as the player typed it: digits, with commas, spaces or "mesos" as they like, and
/// "k", "m" or "b" for thousands, millions and billions ("450k", "1.2m"). `Ok(None)` for
/// nothing typed: the price is cleared. `Err` for anything else.
pub(crate) fn parse(typed: &str) -> Result<Option<u64>, ()> {
    let mut text: String = typed.to_ascii_lowercase().chars().filter(|c| !c.is_whitespace() && *c != ',' && *c != '_').collect();
    for unit in ["mesos", "meso"] {
        if let Some(number) = text.strip_suffix(unit) {
            text = number.to_string();
            break;
        }
    }
    if text.is_empty() {
        return Ok(None);
    }
    let (number, times) = match text.chars().last() {
        Some('k') => (&text[..text.len() - 1], 1e3),
        Some('m') => (&text[..text.len() - 1], 1e6),
        Some('b') => (&text[..text.len() - 1], 1e9),
        _ => (&text[..], 1.0),
    };
    let well_formed = !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit() || c == '.')
        && number.chars().filter(|&c| c == '.').count() <= 1
        && number.chars().any(|c| c.is_ascii_digit());
    if !well_formed {
        return Err(());
    }
    let mesos = number.parse::<f64>().map_err(|_| ())? * times;
    if !mesos.is_finite() || mesos > MOST_MESOS {
        return Err(());
    }
    Ok(Some(mesos.round() as u64))
}
