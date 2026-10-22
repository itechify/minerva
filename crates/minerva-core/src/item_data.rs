//! The Item Card's OSMS item files (ADR 0010), held apart from the map set: fetched on the
//! map set's schedule, but read, saved and swapped on their own, so a change in them takes
//! down only the Item Card.

use crate::game_data::Refresh;
use crate::item_lookup::ItemLookup;
use crate::map_index::{MapDataSource, SourceError};
use crate::osms_items::{self, ItemSet};
use crate::{MINERVA_VERSION, SAVED_COPY_AFTER_MS, SET_RECHECK_MS, SET_RETRY_MS};
use std::task::Poll;

/// OSMS's item files as read, and when they were saved (ms since the Unix epoch): by an
/// earlier launch, or as they were fetched in this one (the wall clock then); `None` if the
/// wall clock wasn't known.
struct Held {
    items: ItemSet,
    saved_at_unix_ms: Option<u64>,
}

/// OSMS's item files: the ones held, and when to ask for them next.
#[derive(Default)]
pub(crate) struct ItemFilesLoader {
    held: Option<Held>,
    /// When to ask for them next (capture time); 0 at startup.
    ask_at: u64,
    /// The source is fetching them.
    in_flight: bool,
    /// The files an earlier launch saved have been looked at (on the first tick).
    saved_seen: bool,
    /// The files an earlier launch saved, read, while fresh ones are fetched at startup
    /// because they are too old to use as they are: what the card falls back on if none can
    /// be had.
    fallback: Option<Held>,
    /// Why none are held, once asking for them failed.
    failed: Option<SourceError>,
    /// What the last fetch came to, until it is taken: for the player's Refresh.
    fetched: Option<Refresh>,
}

impl ItemFilesLoader {
    /// At startup (the first tick), the files this version saved at an earlier launch, if
    /// they were saved under a day ago; otherwise, and every day after, or when asked now
    /// ([`ItemFilesLoader::ask_now`]), ones the source fetches afresh, and with none at all,
    /// a minute after OSMS couldn't be reached. Fetched files that read replace the ones held
    /// whole. Ones that can't be had or don't read leave the ones held as they are, and at
    /// startup fall back on the saved ones, however old. Whether the items held, or why there
    /// are none, may have changed.
    pub fn load(&mut self, source: &mut dyn MapDataSource, at_ms: u64, wall_clock: Option<u64>) -> bool {
        if !self.in_flight {
            if at_ms < self.ask_at {
                return false;
            }
            if !self.saved_seen {
                self.saved_seen = true;
                if self.take_saved(source, wall_clock) {
                    self.ask_at = at_ms + SET_RECHECK_MS;
                    return true;
                }
            }
        }
        let fetched = match source.fetch_item_files() {
            Poll::Pending => {
                self.in_flight = true;
                return false;
            }
            Poll::Ready(fetched) => fetched,
        };
        self.in_flight = false;
        self.ask_at = at_ms + SET_RECHECK_MS;
        let read = fetched.and_then(|files| osms_items::read(&files));
        self.fetched = Some(Refresh::of(&read));
        match (read, self.fallback.take()) {
            // The source saved them as it fetched them: now.
            (Ok(items), _) => self.held = Some(Held { items, saved_at_unix_ms: wall_clock }),
            (Err(_), _) if self.held.is_some() => return false,
            (Err(_), Some(saved)) => self.held = Some(saved),
            (Err(failed), None) => {
                if matches!(failed, SourceError::Unreachable(_)) {
                    self.ask_at = at_ms + SET_RETRY_MS;
                }
                self.failed = Some(failed);
            }
        }
        true
    }

    /// Ask for the files afresh at the next [`ItemFilesLoader::load`], unless a fetch is
    /// under way: that one counts.
    pub fn ask_now(&mut self) {
        self.ask_at = 0;
    }

    /// What the last fetch came to, once, since it was last taken.
    pub fn take_fetched(&mut self) -> Option<Refresh> {
        self.fetched.take()
    }

    /// Take the files an earlier launch saved if this version saved them under a day ago by
    /// the wall clock, or else keep them to fall back on; whether they were taken. Without
    /// the wall clock their age is unknown, so they aren't taken as they are, and files that
    /// don't read are as good as none.
    fn take_saved(&mut self, source: &mut dyn MapDataSource, wall_clock: Option<u64>) -> bool {
        let Some(saved) = source.saved_item_files() else { return false };
        let Ok(items) = osms_items::read(&saved.files) else { return false };
        let held = Held { items, saved_at_unix_ms: Some(saved.saved_at_unix_ms) };
        let age = wall_clock.and_then(|now| now.checked_sub(saved.saved_at_unix_ms));
        let ours = saved.saved_by.as_deref() == Some(MINERVA_VERSION);
        if ours && age.is_some_and(|age| age < SET_RECHECK_MS) {
            self.held = Some(held);
            return true;
        }
        self.fallback = Some(held);
        false
    }

    /// The items held, or what the alert says for want of them.
    pub fn items(&self) -> Result<&ItemSet, ItemLookup> {
        match (&self.held, &self.failed) {
            (Some(held), _) => Ok(&held.items),
            (None, None) => Err(ItemLookup::ItemsLoading),
            (None, Some(SourceError::Unreachable(_))) => Err(ItemLookup::ItemsUnavailable),
            (None, Some(SourceError::FormatChanged(_))) => Err(ItemLookup::ItemsFormatChanged),
        }
    }

    /// When the files held were saved, if that was over 7 days ago by the wall clock `now`,
    /// so they are a Saved Copy: none could be had since.
    pub fn saved_copy_from(&self, now: Option<u64>) -> Option<u64> {
        let saved_at = self.held.as_ref()?.saved_at_unix_ms?;
        (now?.saturating_sub(saved_at) > SAVED_COPY_AFTER_MS).then_some(saved_at)
    }
}
