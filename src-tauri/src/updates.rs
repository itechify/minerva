//! Updates (ADR 0019): the core says when a check is due and when to download or install a
//! newer version, and this runs each through the updater plugin, driven from Rust only, then
//! tells the core what came of it.
//!
//! - **Checks** read the version the feed names: an installed build through its own entry
//!   (the plugin's lookup, `windows-x86_64-nsis` or `-msi`), the portable `.exe` and dev builds
//!   through `windows-x86_64-nsis`. Those never download anything.
//! - **Downloads** ask the feed again and fetch the installer it names, which the plugin
//!   checks against the public key built in; the core loop keeps it.
//! - **Installs** run on the core loop's thread, once it has saved whatever Quit Minerva
//!   saves: the plugin's `install` is synchronous, starts the installer (`passive`), and ends
//!   Minerva with `process::exit(0)`, so nothing on the quit path runs after.

use crate::Command;
use crate::http;
use minerva_core::{Build, DownloadAnswer, UpdateAnswer, UpdateJob, UpdateStep, Updates};
use std::fmt;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tauri::AppHandle;
use tauri::utils::config::BundleType;
use tauri::utils::platform::bundle_type;
use tauri_plugin_updater::{Update, UpdaterExt};

/// The feed: the newest release's `latest.json`, which every release carries.
const FEED: &str = "https://github.com/itechify/minerva/releases/latest/download/latest.json";

/// Replaces [`FEED`] in any build, for testing. That is safe: an update is still checked
/// against the public key built in before anything is installed.
const ENDPOINT_VAR: &str = "MINERVA_UPDATE_ENDPOINT";

/// The feed's entry a build that can't install reads its version through: never a bare
/// `windows-x86_64`, which `latest.json` never has (the portable would install the setup
/// it points at beside itself).
const NOTIFY_ONLY_TARGET: &str = "windows-x86_64-nsis";

/// How long a check may take before it counts as failed.
const TIMEOUT: Duration = Duration::from_secs(30);

/// How long the installer's download may take, on a slow connection.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

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

/// The feed's entry `build` reads: `None` for the plugin's own lookup, which an installed
/// build makes by how it was installed.
fn target(build: Build) -> Option<&'static str> {
    match build {
        Build::Nsis | Build::Msi => None,
        Build::NotifyOnly => Some(NOTIFY_ONLY_TARGET),
    }
}

/// A newer version's installer, downloaded and checked against the public key, for the
/// core loop to keep until the core says to install it.
#[derive(Clone)]
pub struct Fetched {
    update: Box<Update>,
    bytes: Arc<Vec<u8>>,
}

impl fmt::Debug for Fetched {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fetched {{ version: {:?}, {} bytes }}", self.update.version, self.bytes.len())
    }
}

/// An installer the core said to start: [`Install::run`] it once what Quit Minerva saves is
/// saved.
pub struct Install {
    pub id: u64,
    update: Box<Update>,
    bytes: Arc<Vec<u8>>,
    relaunch: bool,
}

impl Install {
    /// Start the installer, which ends Minerva at once (`process::exit(0)`). Returns only if
    /// it couldn't be started: why.
    pub fn run(self) -> String {
        let update = (*self.update).restart_after_install(self.relaunch);
        match update.install(self.bytes.as_slice()) {
            Ok(()) => "the installer started, but Minerva didn't end".to_string(),
            Err(e) => e.to_string(),
        }
    }
}

/// Runs the core's checks, downloads and installs.
pub struct Updater {
    app: AppHandle,
    build: Build,
    /// The id of the last check run: the core's ids only grow.
    checked: u64,
    /// The id of the last download run, likewise.
    downloaded: u64,
    /// The id of the last install started.
    installed: u64,
    /// The installer the last download fetched, by its id.
    fetched: Option<(u64, Fetched)>,
    /// Where the answers go, for the core loop.
    commands: Sender<Command>,
}

impl Updater {
    pub fn new(app: AppHandle, build: Build, commands: Sender<Command>) -> Self {
        Self { app, build, checked: 0, downloaded: 0, installed: 0, fetched: None, commands }
    }

    /// Run the core's check under way and its download, each once, in the background; what
    /// comes of them comes back to the core loop as [`Command::UpdateAnswer`] and
    /// [`Command::UpdateDownloaded`].
    pub fn sync(&mut self, updates: &Updates) {
        if let Some(id) = updates.check.filter(|&id| id > self.checked) {
            self.checked = id;
            let (app, build, commands) = (self.app.clone(), self.build, self.commands.clone());
            tauri::async_runtime::spawn(async move {
                let answer = match latest(&app, build).await {
                    Ok(update) => UpdateAnswer::Latest { version: update.version },
                    Err(why) => {
                        // Shown only as Help & Setup's "Couldn't check": the answer, with
                        // why, goes into a Bug Report.
                        eprintln!("checking for updates: {why}");
                        UpdateAnswer::Failed { why }
                    }
                };
                let _ = commands.send(Command::UpdateAnswer { id, answer });
            });
        }
        if let Some(UpdateJob { id, step: UpdateStep::Download }) = updates.job
            && id > self.downloaded
            && self.build != Build::NotifyOnly
        {
            self.downloaded = id;
            let (app, build, commands) = (self.app.clone(), self.build, self.commands.clone());
            tauri::async_runtime::spawn(async move {
                let fetched = download(&app, build).await;
                if let Err(why) = &fetched {
                    eprintln!("downloading the update: {why}");
                }
                let _ = commands.send(Command::UpdateDownloaded { id, fetched });
            });
        }
    }

    /// Download `id` came to `fetched`: keep the installer, and say what came of it to the
    /// core.
    pub fn downloaded(&mut self, id: u64, fetched: Result<Fetched, String>) -> DownloadAnswer {
        match fetched {
            Ok(fetched) => {
                let version = fetched.update.version.clone();
                self.fetched = Some((id, fetched));
                DownloadAnswer::Downloaded { version }
            }
            Err(why) => DownloadAnswer::Failed { why },
        }
    }

    /// The installer the core says to start now, once: `Err` with the id if it is gone.
    pub fn install(&mut self, updates: &Updates) -> Option<Result<Install, u64>> {
        let Some(UpdateJob { id, step: UpdateStep::Install { relaunch } }) = updates.job else { return None };
        if id <= self.installed {
            return None;
        }
        self.installed = id;
        Some(match self.fetched.take() {
            Some((fetched_id, Fetched { update, bytes })) if fetched_id == id => {
                Ok(Install { id, update, bytes, relaunch })
            }
            _ => Err(id),
        })
    }
}

/// The update the feed names as the latest, newer or not (the core says which), or why it
/// couldn't be had: the feed unreachable, missing (the plugin's `ReleaseNotFound`), without
/// the build's entry (`TargetsNotFound`), or not reading.
async fn latest(app: &AppHandle, build: Build) -> Result<Update, String> {
    let url = endpoint(std::env::var(ENDPOINT_VAR).ok());
    let url = url.parse().map_err(|e| format!("the feed {url:?}: {e}"))?;
    let mut builder = app
        .updater_builder()
        .endpoints(vec![url])
        .and_then(|builder| builder.header(reqwest::header::USER_AGENT, http::USER_AGENT))
        .map_err(|e| e.to_string())?
        .timeout(TIMEOUT)
        .version_comparator(|_, _| true);
    if let Some(target) = target(build) {
        builder = builder.target(target);
    }
    let updater = builder.build().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(update),
        Ok(None) => Err("the feed answered with no release".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// The installer for the version the feed names now, downloaded and checked against the
/// public key (and its version, `requireSignedVersion`), or why not.
async fn download(app: &AppHandle, build: Build) -> Result<Fetched, String> {
    let mut update = latest(app, build).await?;
    update.timeout = Some(DOWNLOAD_TIMEOUT);
    let bytes = update.download(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    Ok(Fetched { update: Box::new(update), bytes: Arc::new(bytes) })
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

    #[test]
    fn an_installed_build_reads_its_own_entry_and_one_that_cannot_install_the_setups() {
        // The plugin's lookup: `windows-x86_64-nsis` for the setup, `-msi` for the MSI.
        assert_eq!(target(Build::Nsis), None);
        assert_eq!(target(Build::Msi), None);
        assert_eq!(target(Build::NotifyOnly), Some("windows-x86_64-nsis"));
    }
}
