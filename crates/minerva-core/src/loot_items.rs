//! The Items the Pickup Log says the player picked up (ADR 0026, #171): each Item line's name
//! is matched against OSMS's item files, as Item Lookup matches a name (ADR 0010), and the
//! Session tallies the quantities by name.
//!
//! Never a guess, never a drop. A name that is an Item's, or close enough to one, is tallied
//! as OSMS spells it. A name the game cut short (`Arrows for Cros...`) is matched by what it
//! starts with, its tab narrowing the choice; if it still starts more than one Item's name it
//! is tallied as the game shows it. A name that matches no Item (a misread, or an Item newer
//! than OSMS's files) is tallied as read, but only once the same text has been read on two
//! lines, so a single misread never makes an Item of its own. A name the whole set can't place
//! is matched within its tab by the recogniser's usual slips on this font ([`crate::pickup_slips`]),
//! so a single Item's count may now and then be a little high, the total never.

use crate::bug_report::Note;
use crate::osms_items::{InventoryTab, ItemSet};
use serde::Serialize;

/// One Item the Session has picked up, and how many.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LootItem {
    pub name: String,
    pub count: u64,
}

/// What an Item line's name came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Named {
    /// An Item's name, as OSMS spells it.
    Item(String),
    /// A name the game cut short that more than one Item's name starts with: as it shows it.
    CutShort(String),
    /// A name that matches no Item, as read (with the "..." of one cut short).
    Unmatched(String),
}

/// Name an Item line `name` (without the "..." if `cut_short`), picked up into `tab`, from
/// `items`; with none loaded, no name matches.
pub(crate) fn name(items: Option<&ItemSet>, name: &str, cut_short: bool, tab: &str) -> Named {
    // The game's "..." read as ".." or ".:" (the Bug Report of 0.15.0, 2026-10-27 18:29): still a
    // name cut short. No Item's name ends in two such marks.
    let marks = name.trim_end().chars().rev().take_while(|c| matches!(c, '.' | ':' | ',' | ';' | '…')).count();
    let (name, cut_short) = match marks {
        2.. => (name.trim_end().trim_end_matches(['.', ':', ',', ';', '…']).trim_end(), true),
        _ => (name, cut_short),
    };
    let shown = if cut_short { format!("{}...", name.trim_end()) } else { name.trim().to_string() };
    let Some(items) = items else { return Named::Unmatched(shown) };
    if !cut_short {
        let tab = InventoryTab::named_like(tab);
        if let Some(item) = in_its_tab(items, name, tab) {
            return Named::Item(item.to_string());
        }
        // A name the whole set can't place may still be the one of its tab's names it is clearly
        // closest to, never one of two.
        return match tab.and_then(|tab| items.named_in_tab(name, tab)) {
            Some(name) => Named::Item(name.to_string()),
            None => Named::Unmatched(shown),
        };
    }
    let mut names = items.names_starting_with(name);
    // The game's tabs are Equip, Use, Etc, Setup and Cash. A tab it isn't can't narrow the
    // choice (a misread), and narrows it to nothing if no name starting so is filed under it.
    if let Some(tab) = InventoryTab::named(tab) {
        names.retain(|name| name.iter().any(|item| item.inventory_tab() == tab));
    }
    match names.len() {
        // Read a few glyphs wrong at the cut, it may still clearly start one name of its tab.
        0 => match InventoryTab::named_like(tab).and_then(|tab| items.starting_like_in_tab(name, tab)) {
            Some(name) => Named::Item(name.to_string()),
            None => Named::Unmatched(shown),
        },
        1 => Named::Item(names[0][0].name.clone()),
        _ => Named::CutShort(shown),
    }
}

/// What an Item line's name came to by plain edits alone, as it was before the recogniser's slips
/// were known ([`ItemSet::named_in_tab_plain`]): the Item it names, if any. The Bug Report probe
/// compares it with [`name`].
pub(crate) fn name_plain(items: &ItemSet, name: &str, tab: &str) -> Option<String> {
    let tab = InventoryTab::named_like(tab);
    if let Some(item) = in_its_tab(items, name, tab) {
        return Some(item.to_string());
    }
    items.named_in_tab_plain(name, tab?).map(String::from)
}

/// The Item the whole set names `read` as ([`ItemSet::named`]), if it can be the one picked up into
/// `tab`: a name read exactly is its Item whatever tab was read (the tab may be the misread), but a
/// name only close to one is that Item only if it is filed under the tab the line says.
fn in_its_tab<'a>(items: &'a ItemSet, read: &str, tab: Option<InventoryTab>) -> Option<&'a str> {
    let named = items.named(read);
    let item = named.first()?;
    let exact = crate::item_lookup::item_key(read) == crate::item_lookup::item_key(&item.name);
    let filed = tab.is_none_or(|tab| named.iter().any(|item| item.inventory_tab() == tab));
    (exact || filed).then_some(item.name.as_str())
}

/// A name that matched no Item, with the lines it was read on.
#[derive(Debug, Clone)]
struct Unmatched {
    text: String,
    /// On how many lines it has been read.
    lines: u32,
    /// The quantities of those lines not yet tallied: all of them until the second line.
    waiting: u64,
}

/// The Items picked up in a Session.
#[derive(Debug, Clone, Default)]
pub(crate) struct ItemTally {
    counts: Vec<LootItem>,
    unmatched: Vec<Unmatched>,
}

impl ItemTally {
    /// Tally `quantity` of an Item line that `named`, with a note for the Bug Report of what
    /// the matching couldn't settle.
    pub fn add(&mut self, named: Named, quantity: u32, notes: &mut Vec<Note>) {
        let quantity = u64::from(quantity);
        match named {
            Named::Item(name) => self.count(name, quantity),
            Named::CutShort(name) => {
                notes.push(Note::ItemCutShort { name: name.clone() });
                self.count(name, quantity);
            }
            Named::Unmatched(text) => {
                let at = match self.unmatched.iter().position(|seen| seen.text == text) {
                    Some(at) => at,
                    None => {
                        self.unmatched.push(Unmatched { text: text.clone(), lines: 0, waiting: 0 });
                        self.unmatched.len() - 1
                    }
                };
                let seen = &mut self.unmatched[at];
                seen.lines += 1;
                seen.waiting += quantity;
                notes.push(Note::ItemUnmatched { name: text.clone(), lines: seen.lines });
                if seen.lines >= 2 {
                    let waiting = std::mem::take(&mut seen.waiting);
                    self.count(text, waiting);
                }
            }
        }
    }

    fn count(&mut self, name: String, quantity: u64) {
        match self.counts.iter_mut().find(|item| item.name == name) {
            Some(item) => item.count = item.count.saturating_add(quantity),
            None => self.counts.push(LootItem { name, count: quantity }),
        }
    }

    /// The Items so far, the most picked up first, then by name.
    pub fn items(&self) -> Vec<LootItem> {
        let mut items = self.counts.clone();
        items.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        items
    }

    /// How many Items in all, whichever they are.
    pub fn total(&self) -> u64 {
        self.counts.iter().fold(0, |sum, item| sum.saturating_add(item.count))
    }
}
