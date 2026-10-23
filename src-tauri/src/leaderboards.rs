//! Uploading to the Map Leaderboards (#110, ADR 0015): the shell makes each upload the core
//! has under way (`PanelState::leaderboard_uploads`) once, on the async runtime, and tells the
//! core the answer. What to upload, when, and what an answer means are the core's.
//!
//! The Leaderboard Key, which ties a Character's name to this install, is kept here and never
//! handed to the core: everything the core takes in goes into a Bug Report, and those go into
//! public issues. It lives in a file of its own beside `settings.json`, `leaderboard-key`, so
//! the settings being set aside as unreadable never loses it. It is made on the first upload,
//! never shown to the player, and sent only to the Map Leaderboards.

use crate::errors::error;
use crate::{Command, http};
use minerva_core::{LeaderboardAnswer, LeaderboardUpload, SITE_URL, read_leaderboard_answer};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Manager};

/// Where the key is kept: `%APPDATA%\com.itechify.minerva\leaderboard-key` on Windows.
pub fn key_path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("leaderboard-key"))
}

/// The site the uploads go to: owlofminerva.app, or in a debug build the one
/// `MINERVA_SITE_URL` names (`http://localhost:8787/` for `wrangler dev`).
fn site() -> String {
    let given = if cfg!(debug_assertions) { std::env::var("MINERVA_SITE_URL").ok() } else { None };
    let site = given.unwrap_or_else(|| SITE_URL.to_string());
    if site.ends_with('/') { site } else { format!("{site}/") }
}

/// The Leaderboard Key's file.
pub struct KeyFile {
    path: PathBuf,
    /// The key, once read or made.
    key: Option<String>,
}

impl KeyFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path, key: None }
    }

    /// The key, made on first use. A file that doesn't hold one (edited, say) is set aside as
    /// `leaderboard-key.bad`, where the maintainer could still find it, and another is made: the
    /// names it held are then refused as claimed until the maintainer hands them to the new
    /// one. One that can't be read at all is left as it is, and there is no key this time.
    pub fn key(&mut self) -> Result<String, String> {
        if let Some(key) = &self.key {
            return Ok(key.clone());
        }
        let key = match fs::read_to_string(&self.path) {
            Ok(text) if is_key(text.trim()) => text.trim().to_string(),
            Ok(_) => {
                error!("the Leaderboard Key's file doesn't hold one: set aside, and another made");
                fs::rename(&self.path, self.path.with_extension("bad")).map_err(|e| format!("setting the key aside: {e}"))?;
                make(&self.path)?
            }
            Err(e) if e.kind() == ErrorKind::NotFound => make(&self.path)?,
            Err(e) => return Err(format!("reading the Leaderboard Key: {e}")),
        };
        self.key = Some(key.clone());
        Ok(key)
    }
}

/// Whether `text` can be a Leaderboard Key: 32 to 256 letters, digits, `-` or `_`.
fn is_key(text: &str) -> bool {
    (32..=256).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Make a key, 32 random bytes as hex, and keep it at `path`.
fn make(path: &Path) -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| format!("making a Leaderboard Key: {e}"))?;
    let key: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    crate::files::write_atomically(path, key.as_bytes()).map_err(|e| format!("keeping the Leaderboard Key: {e}"))?;
    Ok(key)
}

/// Makes the core's uploads.
pub struct Uploader {
    key: KeyFile,
    /// The data source's: one HTTP client for all that Minerva fetches and sends.
    client: reqwest::Client,
    /// The id of the last upload made: the core's ids only grow.
    made: u64,
    /// Where the answers go, for the core loop.
    commands: Sender<Command>,
}

impl Uploader {
    pub fn new(key: KeyFile, client: reqwest::Client, commands: Sender<Command>) -> Self {
        Self { key, client, made: 0, commands }
    }

    /// Make each of `uploads` (the core's under way) not made yet, in the background; each
    /// answer comes back to the core loop as [`Command::LeaderboardAnswer`].
    pub fn sync(&mut self, uploads: &[LeaderboardUpload]) {
        let made = self.made;
        for upload in uploads.iter().filter(|upload| upload.id > made) {
            self.made = self.made.max(upload.id);
            let id = upload.id;
            let key = match self.key.key() {
                Ok(key) => key,
                Err(why) => {
                    error!("Map Leaderboards: {why}");
                    let _ = self.commands.send(Command::LeaderboardAnswer { id, answer: LeaderboardAnswer::Failed { why } });
                    continue;
                }
            };
            let url = format!("{}{}", site(), upload.path());
            let (method, body) = match upload.body() {
                Some(body) => (reqwest::Method::PUT, Some(body)),
                None => (reqwest::Method::DELETE, None),
            };
            let client = self.client.clone();
            http::answer_in_background(self.commands.clone(), async move {
                let answer = match http::send(&client, method, &url, &key, body).await {
                    Ok(reply) => read_leaderboard_answer(reply.status, reply.retry_after.as_deref(), &reply.body),
                    Err(why) => LeaderboardAnswer::Failed { why },
                };
                Command::LeaderboardAnswer { id, answer }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `leaderboard-key` in a folder of `test`'s own under the temp folder, emptied first.
    fn file(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("minerva-key-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("leaderboard-key")
    }

    #[test]
    fn a_key_is_made_on_first_use_kept_and_the_same_at_the_next_launch() {
        let path = file("made");
        assert!(!path.exists());
        let key = KeyFile::new(path.clone()).key().unwrap();
        assert!(is_key(&key), "{key}");
        assert_eq!(fs::read_to_string(&path).unwrap(), key);
        assert_eq!(KeyFile::new(path.clone()).key().unwrap(), key);
        // Another install makes another.
        assert_ne!(KeyFile::new(file("another")).key().unwrap(), key);
    }

    #[test]
    fn a_file_that_holds_no_key_is_set_aside_and_another_made() {
        let path = file("edited");
        fs::write(&path, "not a key").unwrap();
        let key = KeyFile::new(path.clone()).key().unwrap();
        assert!(is_key(&key));
        assert_eq!(fs::read_to_string(path.with_extension("bad")).unwrap(), "not a key");
        assert_eq!(fs::read_to_string(&path).unwrap(), key);
    }

    #[test]
    fn the_settings_set_aside_leave_the_key_as_it_was() {
        let path = file("settings");
        let key = KeyFile::new(path.clone()).key().unwrap();
        let settings = path.with_file_name("settings.json");
        fs::write(&settings, "{ not json").unwrap();
        crate::settings::load(&settings);
        assert!(settings.with_extension("json.bad").exists());
        assert_eq!(KeyFile::new(path).key().unwrap(), key);
    }
}
