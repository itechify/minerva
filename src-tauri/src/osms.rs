//! OSMS Data Explorer's files (ADR 0008, ADR 0010): the map set's five files and the Item
//! Card's two item files under [`minerva_core::osms::DATA_URL`], fetched with Minerva's own
//! user agent, one request at a time and with no other pace. Nothing from OSMS is bundled.
//!
//! Each set is saved on disk as one file of its own, so it is swapped whole and a change in
//! one takes down only what reads it, and the file's modification time says when it was
//! saved; the file says which Minerva version saved it. The core decides when to fetch each
//! afresh: it looks at the saved one at startup, and asks for a fresh one when that is too
//! old or another version's, every day after, and when the player presses Refresh. A fetched
//! set is saved only once the core's reader reads it. If any file fails, or the set doesn't
//! read, nothing is saved, and the core keeps the set it has.

use crate::errors::error;
use crate::files::Stamped;
use crate::http::{self, InFlight};
use minerva_core::osms::{DATA_URL, OsmsFiles};
use minerva_core::osms_items::{self, ItemFiles};
use minerva_core::{MINERVA_VERSION, Saved, SourceError};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::task::Poll;
use std::time::UNIX_EPOCH;

/// One of OSMS's sets of files, as the shell fetches and saves it.
pub trait FileSet: Serialize + DeserializeOwned + Send + Sized + 'static {
    /// The files' names under [`DATA_URL`], in the order [`FileSet::from_texts`] takes them.
    const NAMES: &'static [&'static str];
    /// The saved set's file in the cache.
    const FILE: &'static str;
    /// What it is called in errors.
    const WHAT: &'static str;
    /// The set, from its files' texts in [`FileSet::NAMES`]' order.
    fn from_texts(texts: Vec<String>) -> Option<Self>;
    /// Whether the core reads it.
    fn reads(&self) -> Result<(), SourceError>;
}

impl FileSet for OsmsFiles {
    const NAMES: &'static [&'static str] = &OsmsFiles::NAMES;
    const FILE: &'static str = "osms-set.json";
    const WHAT: &'static str = "OSMS's file set";

    fn from_texts(texts: Vec<String>) -> Option<Self> {
        let [maps, portals, monsters, map_manifest, lookups]: [String; 5] = texts.try_into().ok()?;
        Some(OsmsFiles { maps, portals, monsters, map_manifest, lookups })
    }

    fn reads(&self) -> Result<(), SourceError> {
        minerva_core::osms::read(self).map(drop)
    }
}

impl FileSet for ItemFiles {
    const NAMES: &'static [&'static str] = &ItemFiles::NAMES;
    const FILE: &'static str = "osms-items.json";
    const WHAT: &'static str = "OSMS's item files";

    fn from_texts(texts: Vec<String>) -> Option<Self> {
        let [items, cash_shop]: [String; 2] = texts.try_into().ok()?;
        Some(ItemFiles { items, cash_shop })
    }

    fn reads(&self) -> Result<(), SourceError> {
        osms_items::read(self).map(drop)
    }
}

/// One of OSMS's sets of files: saved, and fetched when the core asks.
pub struct CachedFileSet<F: FileSet> {
    client: reqwest::Client,
    file: PathBuf,
    /// The set being fetched in the background.
    in_flight: InFlight<F>,
}

impl<F: FileSet> CachedFileSet<F> {
    /// A set saved under `cache_dir`, fetched with `client`.
    pub fn new(client: reqwest::Client, cache_dir: &Path) -> Self {
        Self { client, file: cache_dir.join(F::FILE), in_flight: InFlight::default() }
    }

    /// The set saved, when, and by which version; `None` if none is, or it can't be read
    /// back.
    pub fn saved(&self) -> Option<Saved<F>> {
        let json = std::fs::read(&self.file).ok()?;
        let Stamped { files, minerva_version } = serde_json::from_slice(&json).ok()?;
        let saved_at = std::fs::metadata(&self.file).and_then(|m| m.modified()).ok()?;
        let since_epoch = saved_at.duration_since(UNIX_EPOCH).unwrap_or_default();
        Some(Saved { files, saved_at_unix_ms: since_epoch.as_millis() as u64, saved_by: minerva_version })
    }

    /// A set fetched afresh, in the background: the first call starts fetching it, later
    /// ones collect it.
    pub fn fetch(&mut self) -> Poll<Result<F, SourceError>> {
        if let Some(answer) = self.in_flight.collect("", F::WHAT) {
            return answer;
        }
        let (client, file) = (self.client.clone(), self.file.clone());
        self.in_flight.start("", async move { fetch_and_save::<F>(&client, &file).await })
    }
}

/// Fetch the set's files, and save them at `file` once they read.
async fn fetch_and_save<F: FileSet>(client: &reqwest::Client, file: &Path) -> Result<F, SourceError> {
    let files = fetch::<F>(client).await.map_err(SourceError::Unreachable)?;
    files.reads()?;
    save(file, &files);
    Ok(files)
}

/// Fetch the set's files, one after another: the first that fails fails the set.
async fn fetch<F: FileSet>(client: &reqwest::Client) -> Result<F, String> {
    let mut texts = Vec::new();
    for name in F::NAMES {
        texts.push(http::text(client, &format!("{DATA_URL}{name}")).await?);
    }
    F::from_texts(texts).ok_or_else(|| format!("not {} files", F::NAMES.len()))
}

/// Fetch OSMS's live set now, whatever is saved, for the probe.
pub fn fetch_live<F: FileSet>() -> Result<F, String> {
    let client = http::client().map_err(|e| e.to_string())?;
    tauri::async_runtime::block_on(fetch::<F>(&client))
}

/// Save `files` at `file`, in place of the set saved there, whole, as this version's.
fn save<F: FileSet>(file: &Path, files: &F) {
    let stamped = Stamped { files, minerva_version: Some(MINERVA_VERSION.to_string()) };
    let written = serde_json::to_vec(&stamped)
        .map_err(std::io::Error::other)
        .and_then(|json| crate::files::write_atomically(file, &json));
    if let Err(e) = written {
        error!("{}: couldn't save {}: {e}", F::WHAT, file.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Each set: saved, read back with when and by which version it was saved, one saved
    /// before sets said which version saved them, and one that isn't JSON.
    fn read_back<F: FileSet + Clone + PartialEq + std::fmt::Debug>(files: F) {
        let dir = std::env::temp_dir().join(format!("minerva-{}-{}", F::FILE, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let set = CachedFileSet::<F>::new(reqwest::Client::new(), &dir);
        assert_eq!(set.saved(), None);

        save(&dir.join(F::FILE), &files);
        let saved_at = UNIX_EPOCH + Duration::from_millis(1_790_812_800_000);
        let file = std::fs::File::options().write(true).open(dir.join(F::FILE)).unwrap();
        file.set_modified(saved_at).unwrap();
        drop(file);

        let saved_by = Some(MINERVA_VERSION.to_string());
        let saved = Saved { files: files.clone(), saved_at_unix_ms: 1_790_812_800_000, saved_by };
        assert_eq!(set.saved(), Some(saved));

        std::fs::write(dir.join(F::FILE), serde_json::to_vec(&files).unwrap()).unwrap();
        let saved = set.saved().expect("read back");
        assert_eq!((saved.files, saved.saved_by), (files, None));

        std::fs::write(dir.join(F::FILE), b"not json").unwrap();
        assert_eq!(set.saved(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_saved_set_is_read_back_with_when_and_by_which_version_it_was_saved() {
        read_back(OsmsFiles { maps: "{}".into(), ..OsmsFiles::default() });
        read_back(ItemFiles { items: "{}".into(), ..ItemFiles::default() });
    }
}
