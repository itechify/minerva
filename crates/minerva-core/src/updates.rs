//! Updates (#124, ADR 0019): whether a newer Minerva is out, and what the tray and Help &
//! Setup say of it. The shell runs each check the core asks for (through the updater plugin,
//! with the feed's `latest.json`) and tells the core the version it names, or that the check
//! failed; the core says whether that version is newer than this one.
//!
//! - **When**: at launch (the first tick), then every [`CHECK_EVERY_MS`] by capture time, never
//!   by itself while the player's "Check for updates" is off, and whenever they press Check for
//!   updates, setting or not. One check at a time.
//! - **What shows**: up to date, with when it last checked; a newer version, with its release
//!   page; or that it couldn't check. A failed check shows nothing more: a newer version found
//!   before stays, and the next check tries again.

use crate::MINERVA_VERSION;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// How often a check is due while Minerva runs: every 6 hours.
const CHECK_EVERY_MS: u64 = 6 * 60 * 60 * 1000;

/// Minerva's releases, each version's page under it.
const RELEASES_URL: &str = "https://github.com/itechify/minerva/releases";

/// Updates, as the tray and Help & Setup show them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Updates {
    /// "Check for updates" is on (the settings view's switch): Minerva checks by itself.
    pub on: bool,
    /// What the last check came to, or one under way.
    pub status: UpdateStatus,
    /// The newer version the last check that answered found; `None` while none is known. A
    /// failed check leaves it as it is.
    pub newer: Option<NewerVersion>,
    /// When a check last answered (ms since the Unix epoch); `None` until one has, or while
    /// the wall clock wasn't known then.
    pub last_checked_unix_ms: Option<u64>,
    /// This build can only say a newer version is out, with its release page: it can't
    /// install one (the portable `.exe` and dev builds).
    pub notify_only: bool,
    /// The check under way, by its id, for the shell to run once and tell the answer to
    /// [`crate::Minerva::on_update_answer`]; `None` while there is none. Not sent to the
    /// overlay.
    #[serde(skip_serializing)]
    pub check: Option<u64>,
}

/// What the last check came to, or one under way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateStatus {
    /// No check yet: it is due at the first tick, unless "Check for updates" is off.
    #[default]
    NotChecked,
    Checking,
    /// The feed names this version, or an older one.
    UpToDate,
    /// The feed names a newer version: [`Updates::newer`].
    NewerVersion,
    /// The check failed: the feed couldn't be reached, or doesn't read.
    Failed,
}

/// A newer version than this one, and its release page, which says what's new.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewerVersion {
    pub version: String,
    pub release_url: String,
}

/// What a check came to, as the shell tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UpdateAnswer {
    /// The feed names `version` as the latest, newer or not.
    Latest { version: String },
    /// The check failed: why, for the log and a Bug Report.
    Failed { why: String },
}

/// What this build can do with a newer version, as the shell tells it at launch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Build {
    /// Installed with the setup `.exe`: it can update in place.
    Nsis,
    /// Installed with the `.msi`: it can update in place.
    Msi,
    /// The portable `.exe`, or a dev build: it only says a newer version is out.
    #[default]
    NotifyOnly,
}

/// Checks for a newer version: when one is due, and what the last came to.
#[derive(Default)]
pub(crate) struct UpdateChecks {
    build: Build,
    /// When the next check is due by itself (capture time); 0 at launch.
    due_at: u64,
    /// The player pressed Check for updates: one starts at the next tick.
    asked: bool,
    /// The id of the last check asked for: one more with each.
    last_id: u64,
    /// The check under way.
    in_flight: Option<u64>,
    status: UpdateStatus,
    newer: Option<NewerVersion>,
    last_checked_unix_ms: Option<u64>,
}

impl UpdateChecks {
    pub fn set_build(&mut self, build: Build) {
        self.build = build;
    }

    /// The player pressed Check for updates: one under way counts.
    pub fn ask(&mut self) {
        self.asked = self.in_flight.is_none();
    }

    /// Time passed to `at_ms`: start a check if one is due, by itself only while `on`.
    pub fn tick(&mut self, at_ms: u64, on: bool) {
        if self.in_flight.is_some() {
            return;
        }
        if self.asked || (on && at_ms >= self.due_at) {
            self.asked = false;
            self.due_at = at_ms + CHECK_EVERY_MS;
            self.last_id += 1;
            self.in_flight = Some(self.last_id);
        }
    }

    /// Check `id` came to `answer`, at `wall_clock`. An answer to a check no longer under way
    /// changes nothing.
    pub fn answer(&mut self, id: u64, answer: UpdateAnswer, wall_clock: Option<u64>) {
        if self.in_flight != Some(id) {
            return;
        }
        self.in_flight = None;
        let latest = match answer {
            UpdateAnswer::Latest { version } => Some(version.trim().trim_start_matches('v').to_string()),
            UpdateAnswer::Failed { .. } => None,
        };
        let newer = match latest.as_deref().map(|latest| (latest, compare(latest, MINERVA_VERSION))) {
            Some((latest, Some(Ordering::Greater))) => Some(Some(latest.to_string())),
            Some((_, Some(_))) => Some(None),
            // A version that doesn't read is a failed check.
            Some((_, None)) | None => None,
        };
        match newer {
            Some(newer) => {
                self.newer = newer.map(|version| NewerVersion { release_url: format!("{RELEASES_URL}/tag/v{version}"), version });
                self.status = if self.newer.is_some() { UpdateStatus::NewerVersion } else { UpdateStatus::UpToDate };
                self.last_checked_unix_ms = wall_clock;
            }
            None => self.status = UpdateStatus::Failed,
        }
    }

    /// The newer version's release page, if one is known.
    pub fn release_url(&self) -> Option<String> {
        self.newer.as_ref().map(|newer| newer.release_url.clone())
    }

    /// As the tray, Help & Setup and the settings view show it, with "Check for updates" `on`
    /// or off.
    pub fn state(&self, on: bool) -> Updates {
        Updates {
            on,
            status: if self.in_flight.is_some() { UpdateStatus::Checking } else { self.status },
            newer: self.newer.clone(),
            last_checked_unix_ms: self.last_checked_unix_ms,
            notify_only: self.build == Build::NotifyOnly,
            check: self.in_flight,
        }
    }
}

/// How version `a` compares with `b`, both `X.Y.Z` with an optional `-pre-release` (which
/// comes before the release) and `+build` (which counts for nothing); `None` if either
/// doesn't read.
fn compare(a: &str, b: &str) -> Option<Ordering> {
    let (a, b) = (Version::read(a)?, Version::read(b)?);
    Some(a.numbers.cmp(&b.numbers).then_with(|| match (a.pre, b.pre) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => a.cmp(b),
    }))
}

struct Version<'a> {
    numbers: [u64; 3],
    pre: Option<&'a str>,
}

impl<'a> Version<'a> {
    fn read(version: &'a str) -> Option<Self> {
        let version = version.split('+').next()?;
        let (release, pre) = match version.split_once('-') {
            Some((release, pre)) => (release, Some(pre)),
            None => (version, None),
        };
        let mut parts = release.split('.').map(|part| part.parse::<u64>().ok());
        let numbers = [parts.next()??, parts.next()??, parts.next()??];
        if parts.next().is_some() {
            return None;
        }
        Some(Self { numbers, pre })
    }
}
