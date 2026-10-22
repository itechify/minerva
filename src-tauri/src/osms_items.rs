//! OSMS Data Explorer's item files (ADR 0010): the two files under
//! [`minerva_core::osms::DATA_URL`] the Item Card reads, fetched with Minerva's own
//! user agent, one request at a time and with no other pace. Nothing from OSMS is bundled.
//!
//! Saved apart from the map set ([`crate::osms`]) as one file of their own, so a change in
//! them takes down only the Item Card, and the file's modification time says when they
//! were saved; the file says which Minerva version saved them. The core decides when to
//! fetch them afresh, on the map set's schedule. Fetched files are saved only once the
//! core's reader reads them. If any file fails, or they don't read, nothing is saved, and
//! the core keeps the files it has.

use crate::errors::error;
use crate::files::Stamped;
use crate::http;
use minerva_core::{MINERVA_VERSION, SourceError};
use minerva_core::osms::DATA_URL;
use minerva_core::osms_items::{self, ItemFiles, SavedItemFiles};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::task::Poll;
use std::time::UNIX_EPOCH;

/// The saved files' file in the cache.
const ITEMS_FILE: &str = "osms-items.json";

type Answer = Result<ItemFiles, SourceError>;

/// OSMS's item files: saved, and fetched when the core asks.
pub struct OsmsItems {
    client: reqwest::Client,
    file: PathBuf,
    /// The files being fetched in the background.
    in_flight: Option<Receiver<Answer>>,
}

impl OsmsItems {
    /// Files saved under `cache_dir`, fetched with `client`.
    pub fn new(client: reqwest::Client, cache_dir: &Path) -> Self {
        Self { client, file: cache_dir.join(ITEMS_FILE), in_flight: None }
    }

    /// The files saved, when, and by which version; `None` if none are, or they can't be read
    /// back.
    pub fn saved(&self) -> Option<SavedItemFiles> {
        let json = std::fs::read(&self.file).ok()?;
        let Stamped { files, minerva_version } = serde_json::from_slice(&json).ok()?;
        let saved_at = std::fs::metadata(&self.file).and_then(|m| m.modified()).ok()?;
        let since_epoch = saved_at.duration_since(UNIX_EPOCH).unwrap_or_default();
        Some(SavedItemFiles { files, saved_at_unix_ms: since_epoch.as_millis() as u64, saved_by: minerva_version })
    }

    /// Files fetched afresh, in the background: the first call starts fetching them,
    /// later ones collect them.
    pub fn fetch(&mut self) -> Poll<Answer> {
        if let Some(arriving) = &self.in_flight {
            let Poll::Ready(answer) = http::collect(arriving) else { return Poll::Pending };
            self.in_flight = None;
            if let Err(e) = &answer {
                error!("OSMS's item files: {e}");
            }
            return Poll::Ready(answer);
        }
        let (client, file) = (self.client.clone(), self.file.clone());
        self.in_flight = Some(http::in_background(async move { fetch_and_save(&client, &file).await }));
        Poll::Pending
    }
}

/// Fetch OSMS's two item files, and save them at `file` once they read.
async fn fetch_and_save(client: &reqwest::Client, file: &Path) -> Answer {
    let files = fetch(client).await.map_err(SourceError::Unreachable)?;
    osms_items::read(&files)?;
    save(file, &files);
    Ok(files)
}

/// Fetch OSMS's two item files, one after another: the first that fails fails both.
async fn fetch(client: &reqwest::Client) -> Result<ItemFiles, String> {
    let mut texts = Vec::new();
    for name in ItemFiles::NAMES {
        texts.push(http::text(client, &format!("{DATA_URL}{name}")).await?);
    }
    let [items, cash_shop]: [String; 2] = texts.try_into().map_err(|_| "not two files".to_string())?;
    Ok(ItemFiles { items, cash_shop })
}

/// Fetch OSMS's live item files now, whatever is saved, for the probe.
pub fn fetch_live() -> Result<ItemFiles, String> {
    let client = http::client().map_err(|e| e.to_string())?;
    tauri::async_runtime::block_on(fetch(&client))
}

/// Save `files` at `file`, in place of the ones saved there, whole, as this version's.
fn save(file: &Path, files: &ItemFiles) {
    let stamped = Stamped { files, minerva_version: Some(MINERVA_VERSION.to_string()) };
    let written = serde_json::to_vec(&stamped)
        .map_err(std::io::Error::other)
        .and_then(|json| crate::files::write_atomically(file, &json));
    if let Err(e) = written {
        error!("OSMS items: couldn't save {}: {e}", file.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn the_saved_item_files_are_read_back_with_when_and_by_which_version_they_were_saved() {
        let dir = std::env::temp_dir().join(format!("minerva-osms-items-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let items = OsmsItems::new(reqwest::Client::new(), &dir);
        assert_eq!(items.saved(), None);

        let files = ItemFiles { items: "{}".into(), ..ItemFiles::default() };
        save(&dir.join(ITEMS_FILE), &files);
        let saved_at = UNIX_EPOCH + Duration::from_millis(1_790_812_800_000);
        let file = std::fs::File::options().write(true).open(dir.join(ITEMS_FILE)).unwrap();
        file.set_modified(saved_at).unwrap();
        drop(file);

        let saved_by = Some(MINERVA_VERSION.to_string());
        let saved = SavedItemFiles { files: files.clone(), saved_at_unix_ms: 1_790_812_800_000, saved_by };
        assert_eq!(items.saved(), Some(saved));

        // Item files saved before they said which version saved them.
        std::fs::write(dir.join(ITEMS_FILE), serde_json::to_vec(&files).unwrap()).unwrap();
        let saved = items.saved().expect("read back");
        assert_eq!((saved.files, saved.saved_by), (files, None));

        std::fs::write(dir.join(ITEMS_FILE), b"not json").unwrap();
        assert_eq!(items.saved(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
