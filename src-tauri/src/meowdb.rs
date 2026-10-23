//! MeowDB's search (ADR 0007), Minerva's only request to MeowDB: fetched over HTTPS
//! with an honest user agent and kept in a disk cache. Reading it is left to the core. No
//! MeowDB page is fetched.
//!
//! Item Lookup: the core asks MeowDB's search for an item's name once per lookup the
//! player makes. An answer is cached for [`ITEM_SEARCH_FRESH_FOR`] (a week), so looking
//! the same item up again asks MeowDB nothing. Every answer MeowDB gives is cached, even
//! one the core can't read.

use crate::errors::error;
use crate::http::{self, InFlight};
use minerva_core::{Fetched, ItemHit, SourceError, meowdb};
use std::path::{Path, PathBuf};
use std::task::Poll;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long a cached answer of MeowDB's search is used without asking again: the items a
/// name finds hardly change.
const ITEM_SEARCH_FRESH_FOR: Duration = Duration::from_secs(7 * 24 * 60 * 60);

type Answer = Result<Fetched<Vec<ItemHit>>, SourceError>;

/// MeowDB's search: cached, and fetched when an answer isn't.
pub struct ItemSearch {
    client: reqwest::Client,
    cache_dir: PathBuf,
    /// The item name being searched for.
    in_flight: InFlight<Fetched<Vec<ItemHit>>>,
}

impl ItemSearch {
    /// A search caching under `cache_dir` (created when first written), fetched with
    /// `client`.
    pub fn new(client: reqwest::Client, cache_dir: &Path) -> Self {
        Self { client, cache_dir: cache_dir.to_path_buf(), in_flight: InFlight::default() }
    }

    /// The items MeowDB's search gives for `name`: at once from the cache, otherwise once
    /// fetched, in the background.
    pub fn poll(&mut self, name: &str) -> Poll<Answer> {
        if let Some(items) = self.in_flight.collect(name, format_args!("item search {name:?}")) {
            return items;
        }
        // Read here, so a cached answer is given at once and the page opens right away.
        let path = self.file(name);
        if let Some(answer) = cached(&path, ITEM_SEARCH_FRESH_FOR) {
            return Poll::Ready(meowdb::read_item_search(&answer).map(Fetched::Fresh));
        }
        let url = match reqwest::Url::parse_with_params(meowdb::ITEM_SEARCH_URL, [("q", name), ("locale", "en")]) {
            Ok(url) => url.to_string(),
            Err(e) => return Poll::Ready(Err(SourceError::FormatChanged(format!("search URL: {e}")))),
        };
        // In place of another name's search on its way: the latest lookup's is the one asked for.
        let client = self.client.clone();
        self.in_flight.start(name, async move { settle(&path, http::text(&client, &url).await) })
    }

    fn file(&self, name: &str) -> PathBuf {
        // The name as hex: any name makes a safe file name, and no two share one.
        let key: String = name.to_lowercase().bytes().map(|b| format!("{b:02x}")).collect();
        self.cache_dir.join(format!("item-search-{key}.json"))
    }
}

/// The page cached at `path`, if it was saved less than `fresh_for` ago. Even one that
/// can't be read: refetching won't fix a format change until MeowDB changes again, so
/// it waits out its freshness like any page.
fn cached(path: &Path, fresh_for: Duration) -> Option<String> {
    let saved_at = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let age = SystemTime::now().duration_since(saved_at).ok()?;
    if age < fresh_for { std::fs::read_to_string(path).ok() } else { None }
}

/// The page cached at `path`, however old, and when it was saved (ms since the Unix
/// epoch): what to fall back on when MeowDB can't be reached.
fn saved_copy(path: &Path) -> Option<(String, u64)> {
    let page = std::fs::read_to_string(path).ok()?;
    let saved_at = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let since_epoch = saved_at.and_then(|t| t.duration_since(UNIX_EPOCH).ok()).unwrap_or_default();
    Some((page, since_epoch.as_millis() as u64))
}

/// After a fetch: cache and read what MeowDB served, or if it couldn't be reached,
/// read the copy saved at `path` if there is one.
fn settle(path: &Path, fetched: Result<String, String>) -> Answer {
    let read = meowdb::read_item_search;
    match fetched {
        Ok(page) => {
            if let Err(e) = crate::files::write_atomically(path, page.as_bytes()) {
                error!("MeowDB cache: couldn't write {}: {e}", path.display());
            }
            read(&page).map(Fetched::Fresh)
        }
        Err(e) => {
            error!("MeowDB: {e}");
            match saved_copy(path) {
                Some((page, saved_at_unix_ms)) => read(&page).map(|data| Fetched::SavedCopy { data, saved_at_unix_ms }),
                None => Err(SourceError::Unreachable(e)),
            }
        }
    }
}
