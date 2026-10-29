//! Settings persistence: the core's `Settings` as pretty JSON in the app config
//! directory (`%APPDATA%\com.itechify.minerva\settings.json` on Windows).

use crate::errors::error;
use minerva_core::Settings;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("settings.json"))
}

/// The saved settings, or the defaults when there are none. An unreadable file is kept
/// beside it as `settings.json.bad` for diagnosis ([`crate::files::set_aside`]).
pub fn load(path: &Path) -> Settings {
    let Ok(text) = fs::read_to_string(path) else { return Settings::default() };
    serde_json::from_str(&text).unwrap_or_else(|e| {
        error!("settings unreadable ({e}); using defaults");
        let _ = crate::files::set_aside(path, &path.with_extension("json.bad"));
        Settings::default()
    })
}

/// Write the settings atomically: a temp file renamed over the old one.
pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    crate::files::write_atomically(path, serde_json::to_string_pretty(settings)?.as_bytes())
}
