//! OSMS Data Explorer's item files (ADR 0010): the raw texts the shell fetches from
//! [`crate::osms::DATA_URL`], and the reader that turns them into every item's Item Card
//! figures. A set of its own, apart from the map set ([`crate::osms`]): fetched on the
//! same daily schedule, but read, saved and swapped on its own, so a change in these files
//! takes down only the Item Card. Credit "OSMS Data Explorer (osmsdataexplorer.com)".

use crate::item_card::ShopPrice;
use crate::item_lookup::{closest, item_key};
use crate::loader::Saved;
use crate::map_index::SourceError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// OSMS's item files as fetched: each file's text, unread. Saved and swapped whole.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFiles {
    /// `items.json`: every item but the Cash Shop's, with its category, figures and price,
    /// and every scroll.
    pub items: String,
    /// `cash_shop.json`: every Cash Shop item.
    pub cash_shop: String,
}

impl ItemFiles {
    /// The files' names under [`crate::osms::DATA_URL`], in the order of the fields.
    pub const NAMES: [&str; 2] = ["items.json", "cash_shop.json"];
}

/// OSMS's item files as the source saved them, at an earlier launch or earlier in this
/// one, and when: their `files`, `saved_at_unix_ms` and the Minerva version they were
/// `saved_by` ([`crate::MINERVA_VERSION`]; `None` for files saved before they said so).
pub type SavedItemFiles = Saved<ItemFiles>;

/// One item, as its Item Card shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemFacts {
    /// The game's item ID.
    pub id: u32,
    pub name: String,
    /// What kind of item it is, to tell it from others sharing its name: a weapon's kind
    /// ("One-Handed Sword"), other equipment's slot ("Overall"), or its inventory tab
    /// ("Use", "Etc", "Setup"); "Scroll" or "Cash".
    pub kind: String,
    pub shop: ShopPrice,
    /// A throwing star or a bullet, recharged at a shop (`unitPrice`): a listing of it is one
    /// set, its number how many the set holds.
    pub rechargeable: bool,
    /// A scroll's tier; `None` for anything else.
    pub tier: Option<ScrollTier>,
}

/// An inventory tab, shared by the Pickup Log parser and Item matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InventoryTab { Equip, Use, Etc, Setup, Cash }

impl InventoryTab {
    pub fn as_str(self) -> &'static str {
        match self { Self::Equip => "Equip", Self::Use => "Use", Self::Etc => "Etc", Self::Setup => "Setup", Self::Cash => "Cash" }
    }

    pub fn named(read: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.as_str().eq_ignore_ascii_case(read.trim()))
    }

    /// The tab `read` is, or reads as with a glyph misread (`Ett` for `Etc`): the one tab within
    /// an edit of it, never one of two.
    pub fn named_like(read: &str) -> Option<Self> {
        if let Some(tab) = Self::named(read) {
            return Some(tab);
        }
        let read = read.trim().to_ascii_lowercase().chars().collect::<Vec<_>>();
        let mut near = Self::ALL.into_iter().filter(|tab| {
            let name = tab.as_str().to_ascii_lowercase().chars().collect::<Vec<_>>();
            crate::matcher::levenshtein(&read, &name) <= 1
        });
        near.next().filter(|_| near.next().is_none())
    }

    const ALL: [Self; 5] = [Self::Equip, Self::Use, Self::Etc, Self::Setup, Self::Cash];
}

impl ItemFacts {
    /// OSMS's kind is an equipment slot or weapon kind, and Scrolls belong in Use.
    pub fn inventory_tab(&self) -> InventoryTab {
        match self.kind.as_str() {
            "Use" | "Scroll" => InventoryTab::Use,
            "Etc" => InventoryTab::Etc,
            "Setup" => InventoryTab::Setup,
            "Cash" => InventoryTab::Cash,
            _ => InventoryTab::Equip,
        }
    }
}

/// A scroll's tier, which its picture shows by its colour alone (#140): every scroll of a tier
/// has the same picture, whatever it improves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScrollTier {
    /// 100%: blue-grey.
    Lesser,
    /// 60%: red.
    Intermediate,
    /// 10%: gold.
    Greater,
    /// Grey.
    Chaos,
}

impl ScrollTier {
    /// The tier OSMS's item files write as `tier`; `None` for one they don't.
    fn from_osms(tier: &str) -> Option<Self> {
        match tier.trim() {
            "Lesser" => Some(ScrollTier::Lesser),
            "Intermediate" => Some(ScrollTier::Intermediate),
            "Greater" => Some(ScrollTier::Greater),
            "Chaos" => Some(ScrollTier::Chaos),
            _ => None,
        }
    }
}

/// Every item in OSMS's item files: the game's items, its scrolls and the Cash Shop's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemSet {
    items: Vec<ItemFacts>,
    /// Where each item is in `items`, by its ID: the first, should two share one.
    by_id: HashMap<u32, usize>,
    /// Every name, as [`item_key`] makes it for matching, with the items that have it.
    names: Vec<(Vec<char>, Vec<usize>)>,
}

/// What an item set holds, for the probe's report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemCounts {
    pub items: usize,
    /// Those a shop pays a known price for.
    pub shop_priced: usize,
    /// Those a shop won't buy.
    pub not_sold: usize,
    pub cash: usize,
    pub scrolls: usize,
    /// Scrolls whose tier isn't one the Deal Finder knows (ADR 0014): their pictures name none.
    pub scrolls_without_tier: usize,
}

impl ItemSet {
    /// How many items it holds, and of which kinds.
    pub fn counts(&self) -> ItemCounts {
        let count = |keep: fn(&ItemFacts) -> bool| self.items.iter().filter(|item| keep(item)).count();
        ItemCounts {
            items: self.items.len(),
            shop_priced: count(|item| matches!(item.shop, ShopPrice::Sells { .. })),
            not_sold: count(|item| item.shop == ShopPrice::NotSold),
            cash: count(|item| item.shop == ShopPrice::Cash),
            scrolls: count(|item| item.kind == "Scroll"),
            scrolls_without_tier: count(|item| item.kind == "Scroll" && item.tier.is_none()),
        }
    }

    /// Every item, in the files' order.
    pub(crate) fn all(&self) -> impl Iterator<Item = &ItemFacts> {
        self.items.iter()
    }

    /// Item `id`, if the set has it.
    pub(crate) fn item(&self, id: u32) -> Option<&ItemFacts> {
        self.by_id.get(&id).map(|&at| &self.items[at])
    }

    /// The items with the name read from an Item Tooltip as `read`: the name, but for the
    /// letters the recogniser confuses (I, l and 1; O and 0), case, spacing and
    /// punctuation; or one clearly closer to it than any other. Empty when no name is.
    pub(crate) fn named(&self, read: &str) -> Vec<&ItemFacts> {
        let keys: Vec<&[char]> = self.names.iter().map(|(name, _)| &name[..]).collect();
        closest(&item_key(read), &keys)
            .map(|name| self.names[name].1.iter().map(|&item| &self.items[item]).collect())
            .unwrap_or_default()
    }

    /// The name `read` is among the names of the items filed under `tab`: one of them, or the one
    /// clearly closest to it by the recogniser's usual slips on the Pickup Log's font
    /// ([`crate::pickup_slips`]), though further from it than [`ItemSet::named`] asks for, because
    /// the tab says which names it can be. `None` when none is.
    pub(crate) fn named_in_tab(&self, read: &str, tab: InventoryTab) -> Option<&str> {
        self.in_tab(read, tab, crate::pickup_slips::closest_by_slips)
    }

    /// [`ItemSet::named_in_tab`] by plain edits alone, as it was before the recogniser's slips were
    /// known: what the Bug Report probe compares the slips with.
    pub(crate) fn named_in_tab_plain(&self, read: &str, tab: InventoryTab) -> Option<&str> {
        self.in_tab(read, tab, crate::pickup_slips::closest_plain)
    }

    fn in_tab(&self, read: &str, tab: InventoryTab, closest: fn(&[char], &[&[char]]) -> Option<usize>) -> Option<&str> {
        let names: Vec<_> = self.names.iter().filter(|(_, items)| items.iter().any(|&item| self.items[item].inventory_tab() == tab)).collect();
        let keys: Vec<&[char]> = names.iter().map(|(name, _)| &name[..]).collect();
        let name = closest(&item_key(read), &keys)?;
        Some(&self.items[names[name].1[0]].name)
    }

    /// The name among those filed under `tab` whose start `read`, a name the game cut short read a
    /// few glyphs wrong, is clearly closest to by the recogniser's usual slips
    /// ([`crate::pickup_slips::closest_start_by_slips`]). `None` when none is.
    pub(crate) fn starting_like_in_tab(&self, read: &str, tab: InventoryTab) -> Option<&str> {
        self.in_tab(read, tab, crate::pickup_slips::closest_start_by_slips)
    }

    /// The items of each name that starts as `start` does (a name the game cut short), name by
    /// name, as [`ItemSet::named`] compares them. Empty when none does.
    pub(crate) fn names_starting_with(&self, start: &str) -> Vec<Vec<&ItemFacts>> {
        let start = item_key(start);
        if start.is_empty() {
            return Vec::new();
        }
        let starting = self.names.iter().filter(|(name, _)| name.starts_with(&start));
        starting.map(|(_, items)| items.iter().map(|&item| &self.items[item]).collect()).collect()
    }

    /// The items with the one name that starts as `start` does (a name the game cut short),
    /// as [`ItemSet::named`] compares them. Empty when no name does, or several do.
    pub(crate) fn starting_with(&self, start: &str) -> Vec<&ItemFacts> {
        let mut names = self.names_starting_with(start);
        match names.len() {
            1 => names.remove(0),
            _ => Vec::new(),
        }
    }
}

/// Read OSMS's item files. All or nothing: a file that isn't JSON, or isn't shaped as the
/// reader expects, is [`SourceError::FormatChanged`], never partial figures.
pub fn read(files: &ItemFiles) -> Result<ItemSet, SourceError> {
    read_set(files).map_err(|why| SourceError::FormatChanged(format!("OSMS items: {why}")))
}

fn read_set(files: &ItemFiles) -> Result<ItemSet, String> {
    let file: ItemsFile = parse(&files.items, "items.json")?;
    let cash_shop: CashShopFile = parse(&files.cash_shop, "cash_shop.json")?;
    let mut items = Vec::new();
    let facts = |id, name: &str, kind: String, shop| ItemFacts { id, name: name.trim().to_string(), kind, shop, rechargeable: false, tier: None };
    for item in &file.items {
        let rechargeable = item.stats.contains_key("unitPrice");
        items.push(ItemFacts { rechargeable, ..facts(item.id, &item.name, kind(item), shop_price(item)?) });
    }
    for scroll in &file.scrolls {
        let tier = scroll.tier.as_deref().and_then(ScrollTier::from_osms);
        items.push(ItemFacts { tier, ..facts(scroll.id, &scroll.name, "Scroll".into(), ShopPrice::Unknown) });
    }
    for item in cash_shop.categories.iter().flat_map(|category| &category.items) {
        items.push(facts(item.id, &item.name, "Cash".into(), ShopPrice::Cash));
    }
    let mut by_id = HashMap::new();
    let mut names: Vec<(Vec<char>, Vec<usize>)> = Vec::new();
    let mut by_name: HashMap<Vec<char>, usize> = HashMap::new();
    for (at, item) in items.iter().enumerate() {
        by_id.entry(item.id).or_insert(at);
        let name = item_key(&item.name);
        match by_name.get(&name) {
            Some(&known) => names[known].1.push(at),
            None => {
                by_name.insert(name.clone(), names.len());
                names.push((name, vec![at]));
            }
        }
    }
    Ok(ItemSet { items, by_id, names })
}

/// What kind of item `item` is: a weapon's kind, spelt out as the game's tooltips do ("1H
/// Sword" is a "One-Handed Sword"), other equipment's slot, or the inventory tab the rest
/// go in.
fn kind(item: &OsmsItem) -> String {
    if let Some(weapon) = &item.weapon_type {
        let weapon = weapon.trim();
        return match (weapon.strip_prefix("1H "), weapon.strip_prefix("2H ")) {
            (Some(kind), _) => format!("One-Handed {kind}"),
            (_, Some(kind)) => format!("Two-Handed {kind}"),
            _ => weapon.to_string(),
        };
    }
    match item.category.as_str() {
        "Equipment" => item.sub_category.trim().to_string(),
        "Consumable" => "Use".into(),
        category => category.to_string(),
    }
}

/// What a shop pays for `item`: equipment's price is top-level, the rest's in its stats.
/// One a shop won't buy carries a placeholder price of 1; throwing stars' and bullets'
/// price isn't known to be what a shop pays for them (they carry a recharge price).
fn shop_price(item: &OsmsItem) -> Result<ShopPrice, String> {
    // `notSale: 1`; a 0 or a false would say a shop buys it.
    if item.stats.get("notSale").is_some_and(|flag| !matches!(flag, Value::Bool(false)) && flag.as_u64() != Some(0)) {
        return Ok(ShopPrice::NotSold);
    }
    if item.stats.contains_key("unitPrice") {
        return Ok(ShopPrice::Unknown);
    }
    let price = if item.category == "Equipment" { item.price.as_ref() } else { item.stats.get("price") };
    match price.map(mesos).transpose().map_err(|why| format!("item {}: {why}", item.id))? {
        Some(mesos) => Ok(ShopPrice::Sells { mesos }),
        None => Ok(ShopPrice::Unknown),
    }
}

/// A price as OSMS writes it: a number, or now and then a number in a string.
fn mesos(price: &Value) -> Result<u64, String> {
    let mesos = match price {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    };
    mesos.ok_or_else(|| format!("price {price} isn't mesos"))
}

fn parse<T: serde::de::DeserializeOwned>(text: &str, file: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|e| format!("{file}: {e}"))
}

/// `items.json`.
#[derive(Deserialize)]
struct ItemsFile {
    items: Vec<OsmsItem>,
    scrolls: Vec<OsmsScroll>,
}

/// A scroll in `items.json`'s `scrolls`.
#[derive(Deserialize)]
struct OsmsScroll {
    id: u32,
    name: String,
    /// "Lesser", "Intermediate", "Greater" or "Chaos".
    #[serde(default)]
    tier: Option<String>,
}

/// `cash_shop.json`.
#[derive(Deserialize)]
struct CashShopFile {
    categories: Vec<CashCategory>,
}

/// One of the Cash Shop's categories, with its items.
#[derive(Deserialize)]
struct CashCategory {
    items: Vec<CashItem>,
}

/// A Cash Shop item.
#[derive(Deserialize)]
struct CashItem {
    id: u32,
    name: String,
}

/// An item in `items.json`'s `items`.
#[derive(Deserialize)]
struct OsmsItem {
    id: u32,
    name: String,
    category: String,
    /// Equipment's slot ("Hat", "Weapon"), or "Consume", "Etc" or "Install".
    sub_category: String,
    /// A weapon's kind ("1H Sword", "Claw").
    #[serde(default)]
    weapon_type: Option<String>,
    /// Its figures, among them `price` on all but equipment.
    #[serde(default)]
    stats: serde_json::Map<String, Value>,
    /// Equipment's price.
    #[serde(default)]
    price: Option<Value>,
}
