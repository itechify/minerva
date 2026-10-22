//! The settings view's Game Data: when OSMS's data held was fetched, and the player's
//! Refresh, which has OSMS's file set and item files fetched afresh at once, whatever
//! their schedule.

use crate::map_index::SourceError;
use serde::Serialize;

/// OSMS's data as the settings view shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameData {
    /// When the map set held was fetched (ms since the Unix epoch), by this launch or an
    /// earlier one; `None` while none is held, or the wall clock wasn't known then.
    pub fetched_at_unix_ms: Option<u64>,
    /// The player's last Refresh, under way or what it came to, until the settings view
    /// closes; `None` if there is none.
    pub refresh: Option<Refresh>,
}

/// A Refresh, under way or what it came to: its file set's and item files' outcome, the
/// worse of the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Refresh {
    /// Both were fetched and read: the data held is OSMS's latest.
    UpToDate,
    /// OSMS couldn't be reached: the data held stays as it is.
    Unreachable,
    /// What OSMS serves doesn't read: Minerva needs an update for it. The data held stays.
    NeedsUpdate,
    /// Under way.
    Refreshing,
}

impl Refresh {
    /// What a fetch that read as `read` comes to.
    pub(crate) fn of<T>(read: &Result<T, SourceError>) -> Self {
        match read {
            Ok(_) => Self::UpToDate,
            Err(SourceError::Unreachable(_)) => Self::Unreachable,
            Err(SourceError::FormatChanged(_)) => Self::NeedsUpdate,
        }
    }
}

/// The player's Refresh: under way until both the file set and the item files have been
/// fetched once since it was pressed, then what it came to.
#[derive(Default)]
pub(crate) struct Refreshes {
    under_way: bool,
    /// While under way, how the file set's fetch came to, once it has.
    set: Option<Refresh>,
    /// While under way, how the item files' fetch came to, once it has.
    items: Option<Refresh>,
    /// What the last one came to, until it is forgotten.
    came_to: Option<Refresh>,
}

impl Refreshes {
    /// The player pressed Refresh: whether a new one starts, as none is under way.
    pub fn start(&mut self) -> bool {
        if self.under_way {
            return false;
        }
        *self = Self { under_way: true, ..Self::default() };
        true
    }

    /// The file set was fetched, and came to `refresh`.
    pub fn set_fetched(&mut self, refresh: Refresh) {
        if self.under_way {
            self.set = Some(refresh);
            self.settle();
        }
    }

    /// The item files were fetched, and came to `refresh`.
    pub fn items_fetched(&mut self, refresh: Refresh) {
        if self.under_way {
            self.items = Some(refresh);
            self.settle();
        }
    }

    fn settle(&mut self) {
        if let (Some(set), Some(items)) = (self.set, self.items) {
            *self = Self { came_to: Some(set.max(items)), ..Self::default() };
        }
    }

    /// The settings view closed: what the last Refresh came to goes. One under way goes on.
    pub fn forget(&mut self) {
        self.came_to = None;
    }

    pub fn state(&self) -> Option<Refresh> {
        if self.under_way { Some(Refresh::Refreshing) } else { self.came_to }
    }
}
