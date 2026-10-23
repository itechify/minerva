//! OSMS's files, held and kept fresh (ADR 0008, ADR 0010): the map set and the Item Card's
//! item files are each a [`Loader`] of their own, fetched on the same daily schedule but
//! read, saved and swapped apart, so a change in one takes down only what reads it.

use crate::game_data::Refresh;
use crate::map_index::{MapDataSource, SourceError};
use crate::osms::OsmsFiles;
use crate::osms_items::ItemFiles;
use crate::{MINERVA_VERSION, SAVED_COPY_AFTER_MS, SET_RECHECK_MS, SET_RETRY_MS};
use std::task::Poll;

/// A set of OSMS's files as the source saved them, at an earlier launch or earlier in this
/// one, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved<F> {
    pub files: F,
    /// When they were saved (ms since the Unix epoch).
    pub saved_at_unix_ms: u64,
    /// The Minerva version that fetched and saved them ([`crate::MINERVA_VERSION`]); `None`
    /// for files saved before they said so.
    pub saved_by: Option<String>,
}

/// A set of OSMS's files the source saves and fetches.
pub(crate) trait Files: Sized {
    /// The files as the source last saved them, if it did.
    fn saved(source: &mut dyn MapDataSource) -> Option<Saved<Self>>;
    /// The files fetched afresh.
    fn fetch(source: &mut dyn MapDataSource) -> Poll<Result<Self, SourceError>>;
}

impl Files for OsmsFiles {
    fn saved(source: &mut dyn MapDataSource) -> Option<Saved<Self>> {
        source.saved_osms_files()
    }

    fn fetch(source: &mut dyn MapDataSource) -> Poll<Result<Self, SourceError>> {
        source.fetch_osms_files()
    }
}

impl Files for ItemFiles {
    fn saved(source: &mut dyn MapDataSource) -> Option<Saved<Self>> {
        source.saved_item_files()
    }

    fn fetch(source: &mut dyn MapDataSource) -> Poll<Result<Self, SourceError>> {
        source.fetch_item_files()
    }
}

/// OSMS's files as read, and when they were saved (ms since the Unix epoch): by an earlier
/// launch, or as they were fetched in this one (the wall clock then); `None` if the wall
/// clock wasn't known.
pub(crate) struct Held<T> {
    pub data: T,
    pub saved_at_unix_ms: Option<u64>,
}

/// What a [`Loader::load`] came to.
pub(crate) struct Loaded<T> {
    /// What a fetch that finished came to, for the player's Refresh.
    pub refresh: Option<Refresh>,
    pub load: Load<T>,
}

/// What to do with the files held after a [`Loader::load`].
pub(crate) enum Load<T> {
    /// Nothing: it isn't time to ask, a fetch is under way, or one failed while files are
    /// held.
    Nothing,
    /// Hold these in place of those held ([`Loader::hold`]): fetched and read, saved
    /// recently enough by this version, or the saved ones fallen back on.
    Hold(Held<T>),
    /// Nothing is held, and none could be had: [`Loader::failed`] says why.
    Failed,
}

/// A set of OSMS's files: the ones held, and when to ask for them next.
pub(crate) struct Loader<T> {
    held: Option<Held<T>>,
    /// When to ask for them next (capture time); 0 at startup.
    ask_at: u64,
    /// The source is fetching them.
    in_flight: bool,
    /// The files an earlier launch saved have been looked at (on the first tick or frame).
    saved_seen: bool,
    /// The files an earlier launch saved, read, while fresh ones are fetched at startup
    /// because they are too old to use as they are: what is fallen back on if none can be
    /// had.
    fallback: Option<Held<T>>,
    /// Why none are held, once asking for them failed.
    failed: Option<SourceError>,
}

impl<T> Default for Loader<T> {
    fn default() -> Self {
        Self { held: None, ask_at: 0, in_flight: false, saved_seen: false, fallback: None, failed: None }
    }
}

impl<T> Loader<T> {
    /// At startup (the first tick or frame), the files this version saved at an earlier
    /// launch, if they were saved under [`SET_RECHECK_MS`] ago; otherwise, and every
    /// [`SET_RECHECK_MS`] after, or when asked now ([`Loader::ask_now`]), ones the source
    /// fetches afresh, and with none at all, a minute after OSMS couldn't be reached.
    /// Fetched files that `read` replace the ones held whole. Ones that can't be had or don't
    /// read leave the ones held as they are, and at startup fall back on the saved ones,
    /// however old. What is to be held comes back for the caller to [`Loader::hold`].
    pub fn load<F: Files>(
        &mut self,
        source: &mut dyn MapDataSource,
        at_ms: u64,
        wall_clock: Option<u64>,
        read: impl Fn(&F) -> Result<T, SourceError>,
    ) -> Loaded<T> {
        let nothing = Loaded { refresh: None, load: Load::Nothing };
        if !self.in_flight {
            if at_ms < self.ask_at {
                return nothing;
            }
            if !self.saved_seen {
                self.saved_seen = true;
                if let Some(held) = self.take_saved(source, wall_clock, &read) {
                    self.ask_at = at_ms + SET_RECHECK_MS;
                    return Loaded { refresh: None, load: Load::Hold(held) };
                }
            }
        }
        let fetched = match F::fetch(source) {
            Poll::Pending => {
                self.in_flight = true;
                return nothing;
            }
            Poll::Ready(fetched) => fetched,
        };
        self.in_flight = false;
        self.ask_at = at_ms + SET_RECHECK_MS;
        let read = fetched.and_then(|files| read(&files));
        let refresh = Some(Refresh::of(&read));
        let load = match (read, self.fallback.take()) {
            // The source saved them as it fetched them: now.
            (Ok(data), _) => Load::Hold(Held { data, saved_at_unix_ms: wall_clock }),
            (Err(_), _) if self.held.is_some() => Load::Nothing,
            (Err(_), Some(saved)) => Load::Hold(saved),
            (Err(failed), None) => {
                if matches!(failed, SourceError::Unreachable(_)) {
                    self.ask_at = at_ms + SET_RETRY_MS;
                }
                self.failed = Some(failed);
                Load::Failed
            }
        };
        Loaded { refresh, load }
    }

    /// The files an earlier launch saved, to hold as they are if this version saved them
    /// under [`SET_RECHECK_MS`] ago by the wall clock; otherwise they are kept to fall back
    /// on. Without the wall clock their age is unknown, so they aren't held as they are, and
    /// files that don't read are as good as none.
    fn take_saved<F: Files>(
        &mut self,
        source: &mut dyn MapDataSource,
        wall_clock: Option<u64>,
        read: impl Fn(&F) -> Result<T, SourceError>,
    ) -> Option<Held<T>> {
        let saved = F::saved(source)?;
        let data = read(&saved.files).ok()?;
        let held = Held { data, saved_at_unix_ms: Some(saved.saved_at_unix_ms) };
        let age = wall_clock.and_then(|now| now.checked_sub(saved.saved_at_unix_ms));
        let ours = saved.saved_by.as_deref() == Some(MINERVA_VERSION);
        if ours && age.is_some_and(|age| age < SET_RECHECK_MS) {
            return Some(held);
        }
        self.fallback = Some(held);
        None
    }

    /// Hold `held` in place of the files held.
    pub fn hold(&mut self, held: Held<T>) {
        self.held = Some(held);
    }

    /// Ask for the files afresh at the next [`Loader::load`], unless a fetch is under way:
    /// that one counts.
    pub fn ask_now(&mut self) {
        self.ask_at = 0;
    }

    /// The files held, as read.
    pub fn held(&self) -> Option<&T> {
        self.held.as_ref().map(|held| &held.data)
    }

    /// Why none are held, once asking for them failed.
    pub fn failed(&self) -> Option<&SourceError> {
        self.failed.as_ref()
    }

    /// When the files held were saved (ms since the Unix epoch), if that is known.
    pub fn saved_at_unix_ms(&self) -> Option<u64> {
        self.held.as_ref()?.saved_at_unix_ms
    }

    /// When the files held were saved, if that was over [`SAVED_COPY_AFTER_MS`] ago by the
    /// wall clock `now`, so they are a Saved Copy: none could be had since.
    pub fn saved_copy_from(&self, now: Option<u64>) -> Option<u64> {
        let saved_at = self.saved_at_unix_ms()?;
        (now?.saturating_sub(saved_at) > SAVED_COPY_AFTER_MS).then_some(saved_at)
    }
}
