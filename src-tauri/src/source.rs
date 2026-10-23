//! The shell's data source: what the core asks for, fetched over HTTPS with an honest user
//! agent ([`crate::http`]) and kept in a disk cache. That is OSMS Data Explorer's file set
//! and pictures (ADR 0008, in [`crate::osms`] and [`crate::pictures`]), MeowDB's search
//! (ADR 0007, in [`crate::meowdb`]), and the Item Card's OSMS item files (ADR 0010, in
//! [`crate::osms`] too). Reading them is left to the core.
//!
//! Fetches run on the async runtime and never block the caller: the first call starts
//! one and answers `Pending`, later calls collect it.

use crate::meowdb::ItemSearch;
use crate::osms::CachedFileSet;
use crate::pictures::Pictures;
use minerva_core::osms::{OsmsFiles, SavedFiles};
use minerva_core::osms_items::{ItemFiles, SavedItemFiles};
use minerva_core::{Fetched, ItemHit, MapDataSource, SourceError};
use std::path::PathBuf;
use std::task::Poll;

/// The data source the app's core is given, fetching from OSMS Data Explorer and MeowDB.
pub struct WebSource {
    /// One HTTP client for all of it, and for the Map Leaderboards' uploads.
    client: reqwest::Client,
    /// OSMS's file set.
    osms: CachedFileSet<OsmsFiles>,
    /// OSMS's item files.
    items: CachedFileSet<ItemFiles>,
    /// OSMS's pictures.
    pictures: Pictures,
    /// MeowDB's search.
    item_search: ItemSearch,
}

impl WebSource {
    /// A source caching under `cache_dir` (created when first written).
    pub fn new(cache_dir: PathBuf) -> reqwest::Result<Self> {
        let client = crate::http::client()?;
        Ok(Self {
            client: client.clone(),
            osms: CachedFileSet::new(client.clone(), &cache_dir),
            items: CachedFileSet::new(client.clone(), &cache_dir),
            pictures: Pictures::new(client.clone(), &cache_dir),
            item_search: ItemSearch::new(client, &cache_dir),
        })
    }
}

impl WebSource {
    /// Its HTTP client, which says who is asking, for what else Minerva sends.
    pub fn client(&self) -> reqwest::Client {
        self.client.clone()
    }
}

impl MapDataSource for WebSource {
    fn saved_osms_files(&mut self) -> Option<SavedFiles> {
        self.osms.saved()
    }

    fn fetch_osms_files(&mut self) -> Poll<Result<OsmsFiles, SourceError>> {
        self.osms.fetch()
    }

    fn picture(&mut self, path: &str) -> Poll<Result<String, SourceError>> {
        self.pictures.poll(path)
    }

    fn item_search(&mut self, name: &str) -> Poll<Result<Fetched<Vec<ItemHit>>, SourceError>> {
        self.item_search.poll(name)
    }

    fn saved_item_files(&mut self) -> Option<SavedItemFiles> {
        self.items.saved()
    }

    fn fetch_item_files(&mut self) -> Poll<Result<ItemFiles, SourceError>> {
        self.items.fetch()
    }
}
