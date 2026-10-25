//! Item Lookup (ADR 0007): the player hovers an item in the game and presses the hotkey.
//! The shell captures the game's client area and the pointer, the Item Tooltip reader
//! reads the item's name, and MeowDB's search turns the name into the item's page.
//!
//! A search result counts only if its name is the name read, but for the letters the
//! recogniser confuses (I, l and 1; O and 0), case, spacing and punctuation, or if it is
//! clearly closer to the name read than any other result. Otherwise no page opens: the
//! owl says no MeowDB item has that name, rather than open a guess. Several results
//! with the name are listed for the player to pick from.
//!
//! What a lookup comes to, the owl says in the Owl Bubble at the top centre of the game
//! window, where the player is looking, for a few seconds (ADR 0025); the panel is left as
//! it is.

use crate::item_tooltip::{ItemTooltip, ItemTooltipReading};
use crate::settings::Hotkey;
use crate::map_index::{Fetched, MapDataSource, SourceError};
use crate::matcher::{key, levenshtein};
use crate::meowdb;
use crate::pace::Pace;
use crate::panel::ScreenRect;
use serde::{Deserialize, Serialize};
use std::task::Poll;

/// One item in MeowDB's answer to a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemHit {
    /// MeowDB's own ID for the item, not the game's item ID.
    pub id: String,
    pub name: String,
    /// MeowDB's category for it ("Ore & Mineral", "One-Handed Sword"); `None` when it
    /// gives none (chairs, for one).
    pub category: Option<String>,
}

/// What the owl says about the latest Item Lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ItemLookup {
    /// The Item Tooltip's name read as `read`, and MeowDB is being asked for it.
    LookingUp { read: String },
    /// The item's MeowDB page was opened.
    Opened { name: String, url: String },
    /// Several MeowDB items have the name read as `read`: the player picks one.
    Choose { read: String, items: Vec<ItemChoice> },
    /// No Item Tooltip near the pointer.
    NoTooltip,
    /// An Item Tooltip near the pointer, but its name couldn't be read.
    Unreadable,
    /// One of our windows covers the Item Tooltip's name, so it wasn't read: our windows
    /// show in the capture.
    Covered { by: OurWindow },
    /// No MeowDB item has the name read as `read`.
    NotFound { read: String },
    /// No item in OSMS's item files has the name read as `read`, so no Item Card shows.
    NoSuchItem { read: String },
    /// OSMS's item files haven't arrived yet (they are asked for at startup).
    ItemsLoading,
    /// OSMS couldn't be reached and no item files are saved; they are asked for again
    /// every minute.
    ItemsUnavailable,
    /// OSMS's item files don't read, and there are none earlier to fall back on:
    /// Minerva needs an update.
    ItemsFormatChanged,
    /// MeowDB couldn't be reached.
    Unreachable,
    /// MeowDB's search answered in a shape the reader doesn't know.
    FormatChanged,
}

/// One of several MeowDB items that share the name read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemChoice {
    /// MeowDB's own ID for the item.
    pub id: String,
    pub name: String,
    /// MeowDB's category for it; `None` when it gives none.
    pub category: Option<String>,
    pub url: String,
}

/// One of Minerva's own windows, which show in our captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OurWindow {
    Panel,
    GrindTracker,
    /// The Owl Bubble, still saying the last Item Lookup's line, or the update's.
    OwlBubble,
    PersonalBests,
    Settings,
}

/// The latest Item Lookup's line, for the owl to say.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemLookupLine {
    /// Counts the player's lookups, so the overlay can tell a new line from the last one
    /// when both say the same.
    pub lookup: u32,
    /// The hotkey the player pressed: Item Lookup's, or the Item Card's when it says what
    /// kept a card from showing.
    pub hotkey: Hotkey,
    #[serde(flatten)]
    pub outcome: ItemLookup,
}

/// How long a line stays, from when it was set (capture time). The Owl Bubble fades it out
/// over its last second ([`crate::OwlBubble::lasts_ms`]).
pub(crate) const LINE_MS: u64 = 6_000;
/// How long the list of items sharing a name stays: long enough to read and pick.
pub(crate) const CHOICES_MS: u64 = 30_000;
/// A result whose name isn't the one read counts if it is at least this similar to it
/// (1 minus edits over length)...
const CLOSE_ENOUGH: f32 = 0.85;
/// ...and every other name in the answer is at least this many more edits away.
const CLEAR_LEAD: usize = 2;

/// The latest Item Lookup: the capture asked for, the search in flight, and the line.
#[derive(Default)]
pub(crate) struct ItemLookups {
    /// A capture was asked for and its reading hasn't come back: another press waits.
    capturing: bool,
    /// The name being looked up, until MeowDB answers, and whether it was asked for yet.
    searching: Option<(String, bool)>,
    /// Of several MeowDB items with the name, open the one of this category if just one
    /// is: the kind of item the Item Card showed.
    prefer: Option<String>,
    line: Option<ItemLookupLine>,
    /// When the line goes (capture time); `None` while it stays (looking up).
    line_until: Option<u64>,
    lookups: u32,
    /// The latest time heard (capture time).
    now_ms: u64,
    /// MeowDB's pace, kept by the searches.
    pace: Pace,
}

impl ItemLookups {
    /// The player pressed the hotkey: whether to capture the game for a new lookup. Not
    /// while the last capture is still being read.
    pub fn start(&mut self) -> bool {
        !std::mem::replace(&mut self.capturing, true)
    }

    /// A capture for a lookup was asked for and its reading hasn't come back.
    pub fn capturing(&self) -> bool {
        self.capturing
    }

    /// Let the line go: an Item Card shows in its place.
    pub fn clear(&mut self) {
        self.line = None;
        self.line_until = None;
    }

    /// Say `outcome`: what kept an Item Card from showing, at `at_ms`.
    pub fn say(&mut self, outcome: ItemLookup, at_ms: u64) {
        self.lookups += 1;
        self.now_ms = self.now_ms.max(at_ms);
        self.show_for(outcome, Hotkey::ItemCard);
    }

    /// What the capture read as (`None`: it failed), with where our windows are on
    /// screen. Returns the page to open, if MeowDB answered at once.
    pub fn read(
        &mut self,
        reading: Option<ItemTooltipReading>,
        ours: &[(OurWindow, ScreenRect)],
        source: &mut dyn MapDataSource,
    ) -> Option<String> {
        self.capturing = false;
        self.searching = None;
        self.prefer = None;
        self.lookups += 1;
        let at_ms = reading.as_ref().map_or(self.now_ms, |r| r.at_ms);
        self.now_ms = self.now_ms.max(at_ms);
        let outcome = match reading.map(|r| r.tooltip) {
            Some(ItemTooltip::None) => ItemLookup::NoTooltip,
            Some(ItemTooltip::Found { area, name }) => {
                match ours.iter().find(|(_, window)| window.overlaps(area)) {
                    Some((by, _)) => ItemLookup::Covered { by: *by },
                    None => match name.as_deref().and_then(item_name) {
                        Some(read) => {
                            self.searching = Some((read.clone(), false));
                            ItemLookup::LookingUp { read }
                        }
                        None => ItemLookup::Unreadable,
                    },
                }
            }
            // The capture failed: nothing was read.
            None => ItemLookup::Unreadable,
        };
        self.show(outcome);
        self.poll(source, at_ms)
    }

    /// The player clicked the Item Card's name, `name`: look it up as if read from an Item
    /// Tooltip, preferring the MeowDB item of category `kind` among several with the name.
    /// Returns the page to open, if MeowDB answered at once.
    pub fn look_up(&mut self, name: String, kind: String, source: &mut dyn MapDataSource, at_ms: u64) -> Option<String> {
        self.lookups += 1;
        self.now_ms = self.now_ms.max(at_ms);
        self.searching = Some((name.clone(), false));
        self.prefer = Some(kind);
        self.show(ItemLookup::LookingUp { read: name });
        self.poll(source, at_ms)
    }

    /// Time passed: check on the search in flight, and let the line go when its time is
    /// up. Returns the page to open, once MeowDB has answered.
    pub fn poll(&mut self, source: &mut dyn MapDataSource, at_ms: u64) -> Option<String> {
        self.now_ms = self.now_ms.max(at_ms);
        if self.line_until.is_some_and(|until| self.now_ms >= until) {
            self.line = None;
            self.line_until = None;
        }
        let (read, asked) = self.searching.take()?;
        // A search not yet asked waits out MeowDB's pace from the last one that went over
        // the network.
        if !asked && !self.pace.ready(at_ms) {
            self.searching = Some((read, false));
            return None;
        }
        let answer = match source.item_search(&read) {
            Poll::Pending => {
                if !asked {
                    self.pace.sent(at_ms);
                }
                self.searching = Some((read, true));
                return None;
            }
            Poll::Ready(answer) => answer,
        };
        let (outcome, open) = match answer {
            Ok(Fetched::Fresh(hits) | Fetched::SavedCopy { data: hits, .. }) => match resolve(&read, &hits) {
                Resolved::One(hit) => {
                    let url = meowdb::item_url(&hit.id);
                    (ItemLookup::Opened { name: hit.name, url: url.clone() }, Some(url))
                }
                Resolved::Several(hits) if let Some(hit) = one_of_category(&hits, self.prefer.as_deref()) => {
                    let url = meowdb::item_url(&hit.id);
                    (ItemLookup::Opened { name: hit.name, url: url.clone() }, Some(url))
                }
                Resolved::Several(hits) => {
                    let items = hits
                        .into_iter()
                        .map(|hit| ItemChoice { url: meowdb::item_url(&hit.id), id: hit.id, name: hit.name, category: hit.category })
                        .collect();
                    (ItemLookup::Choose { read, items }, None)
                }
                Resolved::Neither => (ItemLookup::NotFound { read }, None),
            },
            Err(SourceError::Unreachable(_)) => (ItemLookup::Unreachable, None),
            Err(SourceError::FormatChanged(_)) => (ItemLookup::FormatChanged, None),
        };
        self.show(outcome);
        open
    }

    /// The player picked one of the items sharing the name read: its page, if it is one
    /// of them.
    pub fn pick(&mut self, id: &str) -> Option<String> {
        let Some(ItemLookupLine { outcome: ItemLookup::Choose { items, .. }, .. }) = &self.line else {
            return None;
        };
        let choice = items.iter().find(|choice| choice.id == id)?.clone();
        self.show(ItemLookup::Opened { name: choice.name, url: choice.url.clone() });
        Some(choice.url)
    }

    pub fn line(&self) -> Option<ItemLookupLine> {
        self.line.clone()
    }

    fn show(&mut self, outcome: ItemLookup) {
        self.show_for(outcome, Hotkey::ItemLookup);
    }

    fn show_for(&mut self, outcome: ItemLookup, hotkey: Hotkey) {
        self.line_until = match outcome {
            ItemLookup::LookingUp { .. } => None,
            ItemLookup::Choose { .. } => Some(self.now_ms + CHOICES_MS),
            _ => Some(self.now_ms + LINE_MS),
        };
        self.line = Some(ItemLookupLine { lookup: self.lookups, hotkey, outcome });
    }
}

/// The item's name in what the Item Tooltip's name line read as: trimmed, spaces
/// collapsed, without a scrolled item's "(+3)". `None` without two letters or digits.
pub(crate) fn item_name(read: &str) -> Option<String> {
    let mut name = read.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some(open) = name.rfind('(') {
        let upgrades = name[open + 1..].trim_end_matches(')');
        if upgrades.trim_start_matches('+').chars().all(|c| c.is_ascii_digit()) {
            name.truncate(open);
            name = name.trim_end().to_string();
        }
    }
    (name.chars().filter(|c| c.is_alphanumeric()).count() >= 2).then_some(name)
}

/// The one of `hits` whose MeowDB category is `category`, if just one is.
fn one_of_category(hits: &[ItemHit], category: Option<&str>) -> Option<ItemHit> {
    let category = category?;
    let mut of_category =
        hits.iter().filter(|hit| hit.category.as_deref().is_some_and(|c| c.trim().eq_ignore_ascii_case(category)));
    match (of_category.next(), of_category.next()) {
        (Some(hit), None) => Some(hit.clone()),
        _ => None,
    }
}

/// What a name read resolves to among MeowDB's answer.
enum Resolved {
    One(ItemHit),
    Several(Vec<ItemHit>),
    Neither,
}

/// A name for matching: the title matcher's (letters and digits only, lower case, the
/// upright strokes I, l, 1, |, ! as one), with O and 0 as one too.
pub(crate) fn item_key(name: &str) -> Vec<char> {
    key(name).into_iter().map(|c| if c == '0' { 'o' } else { c }).collect()
}

fn resolve(read: &str, hits: &[ItemHit]) -> Resolved {
    // The answer's names, each with its items.
    let mut names: Vec<(Vec<char>, Vec<ItemHit>)> = Vec::new();
    for hit in hits {
        let name = item_key(&hit.name);
        match names.iter_mut().find(|(known, _)| *known == name) {
            Some((_, items)) => items.push(hit.clone()),
            None => names.push((name, vec![hit.clone()])),
        }
    }
    let keys: Vec<&[char]> = names.iter().map(|(name, _)| &name[..]).collect();
    let Some(closest) = closest(&item_key(read), &keys) else { return Resolved::Neither };
    match &names[closest].1[..] {
        [one] => Resolved::One(one.clone()),
        items => Resolved::Several(items.to_vec()),
    }
}

/// Which of `names` (as [`item_key`] makes them, each once) the name `read` is: the one
/// it is, or one clearly closer to it than any other. `None` when none is.
pub(crate) fn closest(read: &[char], names: &[&[char]]) -> Option<usize> {
    if let Some(same) = names.iter().position(|name| *name == read) {
        return Some(same);
    }
    let mut edits: Vec<(usize, usize)> = names.iter().map(|name| levenshtein(read, name)).enumerate().collect();
    edits.sort_by_key(|&(_, edits)| edits);
    let &(closest, fewest) = edits.first()?;
    let len = read.len().max(names[closest].len());
    let similar = len > 0 && 1.0 - fewest as f32 / len as f32 >= CLOSE_ENOUGH;
    let clear = edits.get(1).is_none_or(|&(_, next)| next >= fewest + CLEAR_LEAD);
    (similar && clear).then_some(closest)
}
