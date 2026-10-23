//! Writing our own files (settings, Personal Bests, market prices, caches).

use crate::errors::error;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::{ErrorKind, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// OSMS's files as the cache keeps them: their texts, beside the Minerva version that
/// fetched and saved them, which the core asks for at startup (a new version fetches them
/// afresh).
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stamped<T> {
    #[serde(flatten)]
    pub files: T,
    /// [`minerva_core::MINERVA_VERSION`] as it saved them; `None` in a file saved before
    /// they said so, which reads as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minerva_version: Option<String>,
}

/// Write via a temporary file and a rename, so a crash never leaves half a file.
pub fn write_atomically(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    let mut file = std::fs::File::create(&tmp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, path)
}

/// What a [`GuardedJsonFile`] keeps: the core's saved shape of it, which says its version.
pub trait JsonShape: Serialize + DeserializeOwned + Default {
    /// What it holds, in errors ("personal bests").
    const WHAT: &'static str;
    /// The shape it was saved in.
    fn version(&self) -> u32;
    /// Whether this version reads that shape (the core's `in_a_known_shape`).
    fn in_a_known_shape(&self) -> bool;
}

/// A file of the player's own data beside the settings (the Personal Bests, the market
/// prices), as this launch found it: the core's saved shape as pretty JSON, read at startup
/// and written whenever it changes, but never over a file that was there and didn't read.
pub struct GuardedJsonFile<T> {
    path: PathBuf,
    /// It was there but didn't read: there is nothing of it this launch.
    lost: bool,
    /// It was there but didn't read, and couldn't be set aside: it is left as it was, and
    /// nothing is saved this launch.
    left: bool,
    shape: PhantomData<fn() -> T>,
}

impl<T: JsonShape> GuardedJsonFile<T> {
    /// What is saved at `path`, or the default when there is no file yet. A file that is
    /// there but doesn't read (reading it fails, it isn't UTF-8 JSON as the core saves it, or
    /// it was saved in a shape this version doesn't read, by a later one) is logged and set
    /// aside as `<name>.json.bad` for diagnosis, and there is nothing. If it can't be set
    /// aside, it is left as it was: [`GuardedJsonFile::save`] never writes over it.
    pub fn load(path: PathBuf) -> (Self, T) {
        let found = |lost, left| Self { path: path.clone(), lost, left, shape: PhantomData };
        let why = match std::fs::read(&path) {
            Err(e) if e.kind() == ErrorKind::NotFound => return (found(false, false), T::default()),
            Err(e) => format!("unreadable ({e})"),
            Ok(bytes) => match serde_json::from_slice::<T>(&bytes) {
                Ok(saved) if saved.in_a_known_shape() => return (found(false, false), saved),
                Ok(saved) => format!("saved in shape {}", saved.version()),
                Err(e) => format!("unreadable ({e})"),
            },
        };
        let what = T::WHAT;
        let left = match std::fs::rename(&path, path.with_extension("json.bad")) {
            Ok(()) => {
                error!("{what} {why}; set aside, starting with none");
                false
            }
            Err(e) => {
                error!("{what} {why}, and not set aside ({e}); starting with none, saving none");
                true
            }
        };
        (found(true, left), T::default())
    }

    /// Whether the file was there but didn't read, so this launch started with nothing.
    pub fn lost(&self) -> bool {
        self.lost
    }

    /// Whether [`GuardedJsonFile::save`] writes: not if the file was left as it was at
    /// [`GuardedJsonFile::load`].
    pub fn saves(&self) -> bool {
        !self.left
    }

    /// Write `saved` atomically: a temp file renamed over the old one. Nothing, if the file
    /// was left as it was at [`GuardedJsonFile::load`].
    pub fn save(&self, saved: &T) -> std::io::Result<()> {
        if self.left {
            return Ok(());
        }
        write_atomically(&self.path, serde_json::to_string_pretty(saved)?.as_bytes())
    }
}
