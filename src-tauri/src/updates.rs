//! Checks for a newer version (ADR 0019): the core says when one is due, and this runs it
//! through the updater plugin, driven from Rust only, then tells the core the version the
//! feed names, or why the check failed. Every build is notify-only for now: the check reads
//! the version through the feed's `windows-x86_64-nsis` entry, and nothing is downloaded.

use crate::Command;
use crate::http;
use minerva_core::{Build, UpdateAnswer, Updates};
use std::sync::mpsc::Sender;
use std::time::Duration;
use tauri::AppHandle;
use tauri::utils::config::BundleType;
use tauri::utils::platform::bundle_type;
use tauri_plugin_updater::UpdaterExt;

/// The feed: the newest release's `latest.json`, which every release carries.
const FEED: &str = "https://github.com/itechify/minerva/releases/latest/download/latest.json";

/// Replaces [`FEED`] in any build, for testing. That is safe: an update is still checked
/// against the public key built in before anything is installed.
const ENDPOINT_VAR: &str = "MINERVA_UPDATE_ENDPOINT";

/// The feed's entry the check reads: never a bare `windows-x86_64`, which `latest.json`
/// never has (the portable would install the setup it points at beside itself).
const TARGET: &str = "windows-x86_64-nsis";

/// How long a check may take before it counts as failed.
const TIMEOUT: Duration = Duration::from_secs(30);

/// The feed to check: `MINERVA_UPDATE_ENDPOINT`'s value when it is set, otherwise [`FEED`].
fn endpoint(set: Option<String>) -> String {
    set.map(|url| url.trim().to_string()).filter(|url| !url.is_empty()).unwrap_or_else(|| FEED.to_string())
}

/// Whether the feed `url` is on this computer (`127.0.0.1`, `localhost` or `[::1]`): the
/// only place a feed may be served over http, for the update smoke test.
fn is_loopback(url: &str) -> bool {
    let Ok(url) = url.trim().parse::<tauri::Url>() else { return false };
    matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
}

/// Before the app runs: let the updater take an http feed when `MINERVA_UPDATE_ENDPOINT`
/// names one on this computer, and never otherwise. The plugin reads it only from its
/// config, never `tauri.conf.json`'s here. An update is still checked against the public key.
pub fn allow_local_feed(config: &mut tauri::Config) {
    let set = std::env::var(ENDPOINT_VAR).ok();
    if set.as_deref().is_some_and(is_loopback)
        && let Some(updater) = config.plugins.0.get_mut("updater").and_then(|updater| updater.as_object_mut())
    {
        updater.insert("dangerousInsecureTransportProtocol".into(), true.into());
    }
}

/// What this build can do with a newer version, from how the bundler patched it: the
/// portable `.exe` and dev builds aren't patched.
pub fn build() -> Build {
    match bundle_type() {
        Some(BundleType::Nsis) => Build::Nsis,
        Some(BundleType::Msi) => Build::Msi,
        _ => Build::NotifyOnly,
    }
}

/// Runs the core's checks.
pub struct Checker {
    app: AppHandle,
    /// The id of the last check run: the core's ids only grow.
    made: u64,
    /// Where the answers go, for the core loop.
    commands: Sender<Command>,
}

impl Checker {
    pub fn new(app: AppHandle, commands: Sender<Command>) -> Self {
        Self { app, made: 0, commands }
    }

    /// Run the core's check under way, once, in the background; its answer comes back to
    /// the core loop as [`Command::UpdateAnswer`].
    pub fn sync(&mut self, updates: &Updates) {
        let Some(id) = updates.check.filter(|&id| id > self.made) else { return };
        self.made = id;
        let (app, commands) = (self.app.clone(), self.commands.clone());
        tauri::async_runtime::spawn(async move {
            let answer = match latest(&app).await {
                Ok(version) => UpdateAnswer::Latest { version },
                Err(why) => {
                    // Shown only as Help & Setup's "Couldn't check": the answer, with why,
                    // goes into a Bug Report.
                    eprintln!("checking for updates: {why}");
                    UpdateAnswer::Failed { why }
                }
            };
            let _ = commands.send(Command::UpdateAnswer { id, answer });
        });
    }
}

/// The version the feed names as the latest, newer or not (the core says which), or why it
/// couldn't be had: the feed unreachable, missing (the plugin's `ReleaseNotFound`), without
/// the entry read (`TargetsNotFound`), or not reading.
async fn latest(app: &AppHandle) -> Result<String, String> {
    let url = endpoint(std::env::var(ENDPOINT_VAR).ok());
    let url = url.parse().map_err(|e| format!("the feed {url:?}: {e}"))?;
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .and_then(|builder| builder.header(reqwest::header::USER_AGENT, http::USER_AGENT))
        .map(|builder| builder.target(TARGET).timeout(TIMEOUT).version_comparator(|_, _| true))
        .and_then(|builder| builder.build())
        .map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(update.version),
        Ok(None) => Err("the feed answered with no release".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_feed_is_the_latest_releases_unless_minerva_update_endpoint_is_set() {
        assert_eq!(endpoint(None), "https://github.com/itechify/minerva/releases/latest/download/latest.json");
        assert_eq!(endpoint(Some("http://localhost:8000/latest.json".into())), "http://localhost:8000/latest.json");
        assert_eq!(endpoint(Some(" http://localhost:8000/latest.json\n".into())), "http://localhost:8000/latest.json");
        assert_eq!(endpoint(Some("  ".into())), FEED);
    }

    #[test]
    fn only_a_feed_on_this_computer_may_be_http() {
        for url in ["http://127.0.0.1:8000/latest.json", "http://localhost:8000/latest.json", "http://[::1]:8000/latest.json"] {
            assert!(is_loopback(url), "{url}");
        }
        for url in [
            "http://example.com/latest.json",
            "http://127.0.0.1.example.com/latest.json",
            "http://localhost.example.com/latest.json",
            "http://192.168.1.10/latest.json",
            "not a url",
            "",
        ] {
            assert!(!is_loopback(url), "{url}");
        }
    }

    #[test]
    fn a_build_cargo_made_is_notify_only() {
        // Not patched by the bundler, as the portable `.exe` isn't.
        assert_eq!(build(), Build::NotifyOnly);
    }
}
