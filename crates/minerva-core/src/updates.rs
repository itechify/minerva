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
//!   before stays, and the next check tries again. A check unanswered for
//!   [`CHECK_GIVES_UP_MS`] has failed, so a later one can be asked.
//! - **The notice** of a newer version is decided here, whole: the one stage it is at, what
//!   choosing to update offers then, and what Quit Minerva does ([`UpdateNotice`]). The tray
//!   and Help & Setup only put it into words.
//! - **Updating** (#128): choosing to update, in a build that can install, downloads the newer
//!   version (the shell checks its signature) and installs it, starting it again after. While
//!   a Session is running, Help & Setup asks first: Update now, Update when I quit (the
//!   download comes now, and Quit Minerva installs it without starting it again), or Later.
//!   The portable `.exe` and dev builds only open the release page. A failed download or an
//!   installer that doesn't start says so, installs nothing, and can be tried again.

use crate::MINERVA_VERSION;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// How often a check is due while Minerva runs: every 6 hours.
const CHECK_EVERY_MS: u64 = 6 * 60 * 60 * 1000;

/// How long a check may go unanswered before it counts as failed, so the next can be asked.
const CHECK_GIVES_UP_MS: u64 = 2 * 60 * 1000;

/// Minerva's releases, each version's page under it.
const RELEASES_URL: &str = "https://github.com/itechify/minerva/releases";

/// Updates, as the tray, Help & Setup and the settings view show them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Updates {
    /// "Check for updates" is on (the settings view's switch): Minerva checks by itself.
    pub checks_on: bool,
    /// What the last check came to, or one under way.
    pub status: UpdateStatus,
    /// When a check last answered (ms since the Unix epoch); `None` until one has, or while
    /// the wall clock wasn't known then.
    pub last_checked_unix_ms: Option<u64>,
    /// The newer version the last check that answered found, and how updating to it goes;
    /// `None` while none is known. A failed check leaves it as it is.
    pub notice: Option<UpdateNotice>,
    /// The player chose Quit Minerva, and it waits for the update they chose to install when
    /// they quit to finish downloading: nothing shows but the tray, which says so.
    pub quitting: bool,
    /// The check under way, by its id, for the shell to run once and tell the answer to
    /// [`crate::Minerva::on_update_answer`]; `None` while there is none. Not sent to the
    /// pages.
    #[serde(skip_serializing)]
    pub check: Option<u64>,
    /// What the shell must do to update, by its id: download the newer version, then
    /// install it. Not sent to the pages.
    #[serde(skip_serializing)]
    pub job: Option<UpdateJob>,
}

/// A newer version than this one, as the tray and Help & Setup show it: each puts `stage`,
/// `offer` and `quit` into words of its own, and decides nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotice {
    pub version: String,
    /// Its release page, which says what's new.
    pub release_url: String,
    pub stage: UpdateStage,
    /// What the tray's update item and Help & Setup's button do now.
    pub offer: UpdateOffer,
    /// What the tray's Quit Minerva does now.
    pub quit: QuitItem,
}

/// Where a newer version stands: one at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateStage {
    /// This build can't install it (the portable `.exe` and dev builds): its release page is
    /// where to get it.
    NotifyOnly,
    /// It is out, and nothing is under way: the player hasn't chosen to update, or chose Later.
    Out,
    /// The player chose to update while a Session is running: Help & Setup asks first,
    /// "Updating restarts Minerva and ends this Session".
    Asking,
    /// It is downloading, and installs and starts once it has.
    Downloading,
    /// It is downloading, and installs when the player chooses Quit Minerva.
    DownloadingForQuit,
    /// It downloaded, and installs when the player chooses Quit Minerva.
    InstallsWhenYouQuit,
    /// The player chose Quit Minerva while it downloaded: Minerva quits and installs it once
    /// it has, or quits without it if Quit Minerva is chosen again.
    FinishingBeforeQuit,
    /// The installer is starting: Minerva ends once it has.
    Installing,
    /// The download failed, or its signature didn't check out: nothing was installed.
    DownloadFailed,
    /// The installer couldn't be started: this version stays.
    InstallFailed,
    /// The download was another version than the one offered (the feed changed since it was
    /// found: a release deleted, or a newer one out): nothing was installed, and a check says
    /// what is out now.
    Changed,
}

/// What choosing to update (the tray's update item, Help & Setup's button) does now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateOffer {
    /// Nothing: updating is under way, or Minerva is ending.
    Nothing,
    /// Opens the release page: this build can't install it.
    ReleasePage,
    /// Updates (asking first while a Session is running).
    Update,
    /// Help & Setup asks: Update now, Update when I quit, or Later. The tray's item asks again.
    Choices,
    /// Installs it now and starts it, rather than when the player quits.
    UpdateNow,
    /// Tries again, after a download or an installer that failed.
    TryAgain,
}

/// What the tray's Quit Minerva does now.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuitItem {
    /// Quits.
    #[default]
    Quit,
    /// Quits, and installs the newer version without starting it: the player chose Update
    /// when I quit.
    QuitAndUpdate,
    /// Quits at once, without installing what is still downloading.
    QuitWithoutUpdating,
}

/// What the shell must do to update: download the newer version, then install it. The id
/// stays the same from one step to the next, and grows with each download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJob {
    pub id: u64,
    pub step: UpdateStep,
}

/// A step of updating, for the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UpdateStep {
    /// Download the newer version and check its signature against the public key.
    Download,
    /// Save what Quit Minerva saves, then start the installer it downloaded, which ends
    /// Minerva; and once it has installed, start the new version if `relaunch`.
    Install { relaunch: bool },
}

/// What a download came to, as the shell tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DownloadAnswer {
    /// `version` downloaded, and its signature checks out.
    Downloaded { version: String },
    /// The download failed, or its signature didn't check out: why, for a Bug Report.
    Failed { why: String },
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
    /// The feed names a newer version: [`Updates::notice`].
    NewerVersion,
    /// The check failed: the feed couldn't be reached, or doesn't read.
    Failed,
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

/// Where choosing to update has got to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Updating {
    /// Not chosen, or the player chose Later.
    #[default]
    Idle,
    Downloading,
    /// Downloaded, to install when the player quits.
    Downloaded,
    Installing,
    DownloadFailed,
    InstallFailed,
    /// The download was another version than the one offered.
    Changed,
}

/// Checks for a newer version, and downloads and installs it when the player chooses: when
/// each is due, and what the last came to.
#[derive(Default)]
pub(crate) struct Updater {
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
    /// The newer version the last check that answered found.
    newer: Option<String>,
    last_checked_unix_ms: Option<u64>,
    /// When the check under way began (capture time).
    in_flight_since: u64,
    updating: Updating,
    /// Help & Setup asks first, while a Session is running.
    asking: bool,
    /// The player chose Update when I quit.
    when_i_quit: bool,
    /// The player chose Quit Minerva, and it waits for the download to install.
    quitting: bool,
    /// The id of the last download asked for: one more with each.
    last_job_id: u64,
    job: Option<UpdateJob>,
    /// The download that came, by its id, waiting to install when the player quits.
    downloaded: Option<u64>,
}

impl Updater {
    /// A build that can install, with a newer version to install.
    fn can_install(&self) -> bool {
        self.build != Build::NotifyOnly && self.newer.is_some()
    }

    /// `version` is the newer version the notice offers, and newer than this one.
    fn offers(&self, version: &str) -> bool {
        self.newer.as_deref() == Some(version) && compare(version, MINERVA_VERSION) == Some(Ordering::Greater)
    }

    /// The player chose to update (the tray's "Update to X.Y.Z…", or Help & Setup's Update),
    /// with a Session `running` or not. Returns whether it did anything, which Help & Setup
    /// shows: nothing in a build that can't install, or with no newer version known.
    pub fn choose(&mut self, running: bool) -> bool {
        if !self.can_install() {
            return false;
        }
        if self.updating == Updating::Installing {
            // Minerva ends at any moment.
        } else if running {
            self.asking = true;
        } else {
            self.update_now();
        }
        true
    }

    /// Update now: install as soon as it has downloaded, and start the new version.
    pub fn update_now(&mut self) {
        if !self.can_install() {
            return;
        }
        self.asking = false;
        self.when_i_quit = false;
        match self.updating {
            Updating::Downloaded => self.install(true),
            // It installs once it has downloaded.
            Updating::Downloading | Updating::Installing => {}
            Updating::Idle | Updating::DownloadFailed | Updating::InstallFailed | Updating::Changed => self.download(),
        }
    }

    /// Update when I quit: download now, and install when the player quits, without
    /// starting the new version.
    pub fn when_i_quit(&mut self) {
        if !self.can_install() || self.updating == Updating::Installing {
            return;
        }
        self.asking = false;
        self.when_i_quit = true;
        if !matches!(self.updating, Updating::Downloading | Updating::Downloaded) {
            self.download();
        }
    }

    /// Later: nothing more for now.
    pub fn later(&mut self) {
        self.asking = false;
    }

    /// The player chose Quit Minerva. Returns whether Minerva quits now: not while what
    /// they chose to install when they quit installs, or has yet to download.
    pub fn quit(&mut self) -> bool {
        if !self.when_i_quit {
            return true;
        }
        match self.updating {
            Updating::Downloaded => self.install(false),
            Updating::Downloading => {}
            _ => return true,
        }
        self.quitting = true;
        false
    }

    fn download(&mut self) {
        self.last_job_id += 1;
        self.job = Some(UpdateJob { id: self.last_job_id, step: UpdateStep::Download });
        self.updating = Updating::Downloading;
        self.downloaded = None;
    }

    fn install(&mut self, relaunch: bool) {
        let Some(id) = self.downloaded else { return };
        self.job = Some(UpdateJob { id, step: UpdateStep::Install { relaunch } });
        self.updating = Updating::Installing;
    }

    /// Download `id` came to `answer`. Returns whether Minerva quits now: the player chose
    /// Quit Minerva while it downloaded, and it failed. One no longer under way changes
    /// nothing. Only the version offered installs, and only if it is newer than this one:
    /// the shell asks the feed again to download, and the feed may name another by then.
    pub fn downloaded(&mut self, id: u64, answer: DownloadAnswer) -> bool {
        if self.job != Some(UpdateJob { id, step: UpdateStep::Download }) {
            return false;
        }
        self.job = None;
        match answer {
            DownloadAnswer::Downloaded { version } if !self.offers(&read_version(&version)) => {
                self.ask();
                self.failed(Updating::Changed)
            }
            DownloadAnswer::Downloaded { .. } => {
                self.downloaded = Some(id);
                self.updating = Updating::Downloaded;
                if !self.when_i_quit || self.quitting {
                    self.install(!self.when_i_quit);
                }
                false
            }
            DownloadAnswer::Failed { .. } => self.failed(Updating::DownloadFailed),
        }
    }

    /// The installer downloaded as `id` couldn't be started. Returns whether Minerva quits
    /// now, as [`Updater::downloaded`]. One no longer under way changes nothing.
    pub fn install_failed(&mut self, id: u64) -> bool {
        if !matches!(self.job, Some(UpdateJob { id: job, step: UpdateStep::Install { .. } }) if job == id) {
            return false;
        }
        self.job = None;
        self.failed(Updating::InstallFailed)
    }

    /// Updating came to `failed`: nothing is installed, and the player can try again.
    fn failed(&mut self, failed: Updating) -> bool {
        self.updating = failed;
        self.downloaded = None;
        self.when_i_quit = false;
        std::mem::take(&mut self.quitting)
    }

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
            if at_ms < self.in_flight_since + CHECK_GIVES_UP_MS {
                return;
            }
            // It never answered: it failed, and its answer, if it ever comes, is ignored.
            self.in_flight = None;
            self.status = UpdateStatus::Failed;
            return;
        }
        if self.asked || (on && at_ms >= self.due_at) {
            self.asked = false;
            self.due_at = at_ms + CHECK_EVERY_MS;
            self.last_id += 1;
            self.in_flight = Some(self.last_id);
            self.in_flight_since = at_ms;
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
            UpdateAnswer::Latest { version } => Some(read_version(&version)),
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
                self.newer = newer;
                // What is out now is known again.
                if self.updating == Updating::Changed {
                    self.updating = Updating::Idle;
                }
                self.status = if self.newer.is_some() { UpdateStatus::NewerVersion } else { UpdateStatus::UpToDate };
                self.last_checked_unix_ms = wall_clock;
            }
            None => self.status = UpdateStatus::Failed,
        }
    }

    /// The newer version's release page, if one is known.
    pub fn release_url(&self) -> Option<String> {
        self.newer.as_deref().map(release_url)
    }

    /// As the tray, Help & Setup and the settings view show it, with "Check for updates"
    /// `checks_on` or off.
    pub fn state(&self, checks_on: bool) -> Updates {
        Updates {
            checks_on,
            status: if self.in_flight.is_some() { UpdateStatus::Checking } else { self.status },
            last_checked_unix_ms: self.last_checked_unix_ms,
            notice: self.newer.as_ref().map(|version| self.notice(version)),
            quitting: self.quitting,
            check: self.in_flight,
            job: self.job,
        }
    }

    /// The notice of the newer `version`: the one stage that shows, of all that may hold at
    /// once, what choosing to update offers then, and what Quit Minerva does.
    fn notice(&self, version: &str) -> UpdateNotice {
        let (stage, offer) = if self.build == Build::NotifyOnly {
            (UpdateStage::NotifyOnly, UpdateOffer::ReleasePage)
        } else {
            match self.updating {
                Updating::Installing => (UpdateStage::Installing, UpdateOffer::Nothing),
                _ if self.quitting => (UpdateStage::FinishingBeforeQuit, UpdateOffer::Nothing),
                _ if self.asking => (UpdateStage::Asking, UpdateOffer::Choices),
                Updating::Downloading if self.when_i_quit => (UpdateStage::DownloadingForQuit, UpdateOffer::UpdateNow),
                Updating::Downloading => (UpdateStage::Downloading, UpdateOffer::Nothing),
                Updating::Downloaded => (UpdateStage::InstallsWhenYouQuit, UpdateOffer::UpdateNow),
                Updating::Idle => (UpdateStage::Out, UpdateOffer::Update),
                Updating::DownloadFailed => (UpdateStage::DownloadFailed, UpdateOffer::TryAgain),
                Updating::InstallFailed => (UpdateStage::InstallFailed, UpdateOffer::TryAgain),
                Updating::Changed => (UpdateStage::Changed, UpdateOffer::TryAgain),
            }
        };
        let quit = if self.quitting {
            QuitItem::QuitWithoutUpdating
        } else if self.when_i_quit && self.updating != Updating::Installing {
            QuitItem::QuitAndUpdate
        } else {
            QuitItem::Quit
        };
        UpdateNotice { version: version.to_string(), release_url: release_url(version), stage, offer, quit }
    }
}

/// `version`'s release page.
fn release_url(version: &str) -> String {
    format!("{RELEASES_URL}/tag/v{version}")
}

/// A version as the feed or the plugin names it, without the space around it or a `v` before.
fn read_version(version: &str) -> String {
    version.trim().trim_start_matches('v').to_string()
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
