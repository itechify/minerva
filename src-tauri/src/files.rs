//! Writing our own files (settings, Personal Bests, caches).

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

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
