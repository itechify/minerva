//! Updates (#124): Minerva checks for a newer version at launch, then every 6 hours while it
//! runs, never by itself while the player's "Check for updates" is off, and whenever they press
//! Check for updates in Help & Setup. The shell runs each check the core asks for and tells it
//! the latest version the feed names, or that the check failed. The core says what the tray and
//! Help & Setup show: up to date (and when it last checked), a newer version with its release
//! page, or that it couldn't check. A failed check leaves a newer version found before as it is.
//! Choosing to update (#128) downloads and installs it in a build that can install, asking first
//! while a Session is running, and only opens the release page in one that can't. The notice
//! the tray and Help & Setup turn into words is the core's: which stage it is at, what choosing
//! to update offers, and what Quit Minerva does. A newer version Minerva found by itself is
//! said over the game too, by the owl (ADR 0025), once a launch.

mod common;

use common::{FakeSource, HOUR, S};
use common::player::Player;
use minerva_core::{
    Action, Build, DownloadAnswer, Foreground, ItemTooltip, ItemTooltipReading, MINERVA_VERSION, Minerva, OwlLine,
    OwlMood, PanelControl, PanelState, QuitItem, Request, ScreenRect, Settings, Size, UpdateAnswer, UpdateJob,
    UpdateNotice, UpdateOffer, UpdateStage, UpdateStatus, UpdateStep,
};

/// The wall clock at capture time 0: 21 Oct 2026, 16:00 UTC.
const WALL: u64 = 1_792_598_400_000;

/// A version newer than any Minerva yet, and one older.
const NEWER: &str = "999.0.0";
const OLDER: &str = "0.0.1";

/// The newer version's release page.
const RELEASE_URL: &str = "https://github.com/itechify/minerva/releases/tag/v999.0.0";

/// A core at launch with `settings` restored and the wall clock told, before its first tick.
fn launched_with(settings: Settings) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(settings);
    minerva.on_wall_clock(WALL);
    minerva
}

fn launched() -> Minerva {
    launched_with(Settings::default())
}

/// The check the core asks the shell to run, if any.
fn asked(state: &PanelState) -> Option<u64> {
    state.updates.check
}

/// Tick to `at_ms`, with the wall clock moving with it.
fn tick(minerva: &mut Minerva, at_ms: u64) -> PanelState {
    minerva.on_wall_clock(WALL + at_ms);
    minerva.tick(at_ms).clone()
}

/// Answer the check under way with `answer`.
fn answer(minerva: &mut Minerva, answer: UpdateAnswer) -> PanelState {
    let id = asked(minerva.panel_state()).expect("a check under way");
    minerva.on_update_answer(id, answer).clone()
}

fn latest(version: &str) -> UpdateAnswer {
    UpdateAnswer::Latest { version: version.to_string() }
}

fn failed() -> UpdateAnswer {
    UpdateAnswer::Failed { why: "offline".to_string() }
}

/// The notice of a newer version the tray and Help & Setup show.
fn notice(state: &PanelState) -> &UpdateNotice {
    state.updates.notice.as_ref().expect("a notice of a newer version")
}

/// The notice's stage, what choosing to update offers, and what Quit Minerva does.
fn shown(state: &PanelState) -> (UpdateStage, UpdateOffer, QuitItem) {
    let notice = notice(state);
    (notice.stage, notice.offer, notice.quit)
}

fn version(state: &PanelState) -> Option<&str> {
    state.updates.notice.as_ref().map(|notice| notice.version.as_str())
}

#[test]
fn a_check_is_asked_for_at_launch_and_says_up_to_date_when_the_feed_names_this_version() {
    let mut minerva = launched();
    assert_eq!(minerva.panel_state().updates.status, UpdateStatus::NotChecked);

    let state = tick(&mut minerva, 0);
    assert!(asked(&state).is_some());
    assert_eq!(state.updates.status, UpdateStatus::Checking);

    tick(&mut minerva, 2 * S);
    let state = answer(&mut minerva, latest(MINERVA_VERSION));
    assert_eq!(asked(&state), None);
    assert_eq!(state.updates.status, UpdateStatus::UpToDate);
    assert_eq!(state.updates.notice, None);
    assert_eq!(state.updates.last_checked_unix_ms, Some(WALL + 2 * S));
}

#[test]
fn a_check_is_due_again_6_hours_after_the_last_began() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    answer(&mut minerva, latest(MINERVA_VERSION));

    assert_eq!(asked(&tick(&mut minerva, 6 * HOUR - S)), None);
    let state = tick(&mut minerva, 6 * HOUR);
    assert!(asked(&state).is_some());
    assert_eq!(state.updates.status, UpdateStatus::Checking);
    answer(&mut minerva, latest(MINERVA_VERSION));

    assert_eq!(asked(&tick(&mut minerva, 12 * HOUR - S)), None);
    assert!(asked(&tick(&mut minerva, 12 * HOUR)).is_some());
}

#[test]
fn with_check_for_updates_off_there_is_no_check_at_launch_nor_later() {
    let mut minerva = launched_with(Settings { check_for_updates: false, ..Settings::default() });
    for at in [0, S, 6 * HOUR, 13 * HOUR] {
        let state = tick(&mut minerva, at);
        assert_eq!(asked(&state), None);
        assert_eq!(state.updates.status, UpdateStatus::NotChecked);
    }
}

#[test]
fn turning_check_for_updates_off_stops_the_checks_and_on_again_checks_if_one_is_due() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    answer(&mut minerva, latest(MINERVA_VERSION));

    assert!(minerva.panel_state().updates.checks_on);
    let state = minerva.act(Action::SetCheckForUpdates { on: false }).clone();
    assert!(!state.updates.checks_on);
    assert!(!minerva.settings().check_for_updates);
    assert_eq!(asked(&tick(&mut minerva, 7 * HOUR)), None);

    let state = minerva.act(Action::SetCheckForUpdates { on: true }).clone();
    assert!(state.updates.checks_on);
    assert!(minerva.settings().check_for_updates);
    assert!(asked(&tick(&mut minerva, 7 * HOUR + S)).is_some());
}

#[test]
fn check_for_updates_is_on_in_a_settings_file_from_before_it() {
    let old: Settings = serde_json::from_str(r#"{"version":7}"#).unwrap();
    assert!(old.check_for_updates);
}

#[test]
fn pressing_check_for_updates_checks_at_once_with_the_setting_off() {
    let mut minerva = launched_with(Settings { check_for_updates: false, ..Settings::default() });
    tick(&mut minerva, 0);
    tick(&mut minerva, HOUR);

    minerva.act(Action::CheckForUpdates);
    let state = tick(&mut minerva, HOUR + 50);
    assert!(asked(&state).is_some());
    assert_eq!(state.updates.status, UpdateStatus::Checking);
    let state = answer(&mut minerva, latest(MINERVA_VERSION));
    assert_eq!(state.updates.status, UpdateStatus::UpToDate);
    assert_eq!(state.updates.last_checked_unix_ms, Some(WALL + HOUR + 50));

    // Only the once: the setting is still off.
    assert_eq!(asked(&tick(&mut minerva, 8 * HOUR)), None);
}

#[test]
fn pressing_check_for_updates_while_one_is_under_way_starts_no_second() {
    let mut minerva = launched();
    let first = asked(&tick(&mut minerva, 0));
    minerva.act(Action::CheckForUpdates);
    assert_eq!(asked(&tick(&mut minerva, 50)), first);

    answer(&mut minerva, latest(MINERVA_VERSION));
    assert_eq!(asked(&tick(&mut minerva, 100)), None);
}

#[test]
fn a_newer_version_shows_with_its_release_page_which_opens_from_the_tray_and_whats_new() {
    let mut minerva = launched();
    assert_eq!(minerva.press(PanelControl::OpenRelease), None);
    tick(&mut minerva, 0);

    let state = answer(&mut minerva, latest(NEWER));
    assert_eq!(state.updates.status, UpdateStatus::NewerVersion);
    assert_eq!(version(&state), Some(NEWER));
    assert_eq!(notice(&state).release_url, RELEASE_URL);
    assert_eq!(state.updates.last_checked_unix_ms, Some(WALL));
    assert_eq!(minerva.press(PanelControl::OpenRelease), Some(Request::OpenInBrowser { url: RELEASE_URL.into() }));
}

#[test]
fn a_version_named_with_a_v_reads_as_without() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    let state = answer(&mut minerva, latest("v999.0.0"));
    assert_eq!(version(&state), Some(NEWER));
}

#[test]
fn an_older_version_than_this_one_is_up_to_date() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    let state = answer(&mut minerva, latest(OLDER));
    assert_eq!(state.updates.status, UpdateStatus::UpToDate);
    assert_eq!(state.updates.notice, None);
}

#[test]
fn a_pre_release_of_this_version_is_older_and_a_pre_release_of_a_later_one_newer() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    let state = answer(&mut minerva, latest(&format!("{MINERVA_VERSION}-rc.1")));
    assert_eq!(state.updates.status, UpdateStatus::UpToDate);

    minerva.act(Action::CheckForUpdates);
    tick(&mut minerva, 50);
    let state = answer(&mut minerva, latest("999.0.0-rc.1"));
    assert_eq!(state.updates.status, UpdateStatus::NewerVersion);
}

#[test]
fn a_version_that_does_not_read_is_a_failed_check() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    let state = answer(&mut minerva, latest("soon"));
    assert_eq!(state.updates.status, UpdateStatus::Failed);
    assert_eq!(state.updates.notice, None);
    assert_eq!(state.updates.last_checked_unix_ms, None);
}

#[test]
fn a_failed_check_says_so_and_the_next_good_one_says_what_it_found() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    let state = answer(&mut minerva, failed());
    assert_eq!(state.updates.status, UpdateStatus::Failed);
    assert_eq!(state.updates.last_checked_unix_ms, None);
    // Tried again at the next due check, not before.
    assert_eq!(asked(&tick(&mut minerva, HOUR)), None);

    tick(&mut minerva, 6 * HOUR);
    let state = answer(&mut minerva, latest(NEWER));
    assert_eq!(state.updates.status, UpdateStatus::NewerVersion);
    assert_eq!(state.updates.last_checked_unix_ms, Some(WALL + 6 * HOUR));
}

#[test]
fn a_failed_check_after_a_newer_version_was_found_leaves_it_showing() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    let found = answer(&mut minerva, latest(NEWER)).updates.notice;

    tick(&mut minerva, 6 * HOUR);
    let state = answer(&mut minerva, failed());
    assert_eq!(state.updates.status, UpdateStatus::Failed);
    assert_eq!(state.updates.notice, found);
    assert_eq!(state.updates.last_checked_unix_ms, Some(WALL));
}

#[test]
fn a_check_unanswered_for_2_minutes_has_failed_and_the_next_can_be_asked() {
    let mut minerva = launched();
    let stuck = asked(&tick(&mut minerva, 0)).unwrap();

    let state = tick(&mut minerva, 2 * 60 * S - 1);
    assert_eq!(asked(&state), Some(stuck));
    let state = tick(&mut minerva, 2 * 60 * S);
    assert_eq!(asked(&state), None);
    assert_eq!(state.updates.status, UpdateStatus::Failed);

    // Check for updates asks again, and a late answer to the stuck one changes nothing.
    minerva.act(Action::CheckForUpdates);
    let next = asked(&tick(&mut minerva, 3 * 60 * S)).unwrap();
    assert!(next > stuck);
    let before = minerva.panel_state().clone();
    assert_eq!(minerva.on_update_answer(stuck, latest(NEWER)), &before);
    let state = answer(&mut minerva, latest(NEWER));
    assert_eq!(state.updates.status, UpdateStatus::NewerVersion);
}

#[test]
fn an_answer_to_a_check_no_longer_under_way_changes_nothing() {
    let mut minerva = launched();
    let id = asked(&tick(&mut minerva, 0)).unwrap();
    answer(&mut minerva, latest(MINERVA_VERSION));

    let before = minerva.panel_state().clone();
    let after = minerva.on_update_answer(id, latest(NEWER)).clone();
    assert_eq!(after, before);
}

// The notice: one stage at a time, what choosing to update offers then, and what Quit
// Minerva does, for the tray and Help & Setup to turn into words.

#[test]
fn with_no_newer_version_known_there_is_no_notice() {
    let mut minerva = launched();
    minerva.on_build(Build::Nsis);
    assert_eq!(minerva.panel_state().updates.notice, None);
    tick(&mut minerva, 0);
    assert_eq!(answer(&mut minerva, latest(MINERVA_VERSION)).updates.notice, None);
}

#[test]
fn a_newer_version_in_a_build_that_can_install_offers_to_update() {
    for build in [Build::Nsis, Build::Msi] {
        let minerva = newer_known(build);
        let state = minerva.panel_state();
        assert_eq!(notice(state).version, NEWER);
        assert_eq!(notice(state).release_url, RELEASE_URL);
        assert_eq!(shown(state), (UpdateStage::Out, UpdateOffer::Update, QuitItem::Quit));
    }
}

#[test]
fn a_build_is_notify_only_until_the_shell_says_it_can_install() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    answer(&mut minerva, latest(NEWER));
    assert_eq!(shown(minerva.panel_state()), (UpdateStage::NotifyOnly, UpdateOffer::ReleasePage, QuitItem::Quit));
    assert_eq!(notice(minerva.on_build(Build::Nsis)).stage, UpdateStage::Out);
    assert_eq!(notice(minerva.on_build(Build::Msi)).stage, UpdateStage::Out);
    assert_eq!(notice(minerva.on_build(Build::NotifyOnly)).stage, UpdateStage::NotifyOnly);
}

// Updating in place (#128): choosing to update downloads the newer version, which the shell
// checks against the public key, then installs it. A running Session is warned of first.

/// A core set up, of `build`, that knows of a newer version: no Session is running.
fn newer_known(build: Build) -> Minerva {
    let mut minerva = common::tracking();
    find_newer(&mut minerva, build, 0);
    minerva
}

/// The shell tells `minerva` it is `build`, and a check at `at_ms` finds a newer version.
fn find_newer(minerva: &mut Minerva, build: Build, at_ms: u64) {
    minerva.on_build(build);
    minerva.act(Action::CheckForUpdates);
    tick(minerva, at_ms);
    answer(minerva, latest(NEWER));
    assert!(!minerva.panel_state().setup.window_open);
}

/// The player hunting, so a Session is running, in a build that can install and knows of a
/// newer version.
fn hunting() -> Player {
    let mut player = Player::new(common::tracking());
    let at = player.at;
    find_newer(&mut player.minerva, Build::Nsis, at);
    player
}

/// What the core asks the shell to do to update, if anything.
fn job(state: &PanelState) -> Option<UpdateJob> {
    state.updates.job
}

/// The download under way, by its id.
fn downloading(state: &PanelState) -> u64 {
    match job(state) {
        Some(UpdateJob { id, step: UpdateStep::Download }) => id,
        other => panic!("a download, not {other:?}"),
    }
}

/// `version`, downloaded, its signature checked.
fn got(version: &str) -> DownloadAnswer {
    DownloadAnswer::Downloaded { version: version.into() }
}

#[test]
fn with_no_session_running_choosing_to_update_downloads_then_installs_and_relaunches() {
    let mut minerva = newer_known(Build::Nsis);
    assert_eq!(job(minerva.panel_state()), None);

    assert_eq!(minerva.press(PanelControl::Update), None);
    let state = minerva.panel_state().clone();
    assert_eq!(shown(&state), (UpdateStage::Downloading, UpdateOffer::Nothing, QuitItem::Quit));
    let id = downloading(&state);
    // Help & Setup opens, to show it.
    assert!(state.setup.window_open);

    let state = minerva.on_update_download(id, got(NEWER)).clone();
    assert_eq!(shown(&state), (UpdateStage::Installing, UpdateOffer::Nothing, QuitItem::Quit));
    assert_eq!(job(&state), Some(UpdateJob { id, step: UpdateStep::Install { relaunch: true } }));
}

#[test]
fn a_build_that_cannot_install_opens_the_release_page_and_never_downloads() {
    let mut minerva = newer_known(Build::NotifyOnly);

    assert_eq!(minerva.press(PanelControl::Update), Some(Request::OpenInBrowser { url: RELEASE_URL.into() }));
    let state = minerva.panel_state().clone();
    assert_eq!(shown(&state), (UpdateStage::NotifyOnly, UpdateOffer::ReleasePage, QuitItem::Quit));
    assert_eq!(job(&state), None);
    assert!(!state.setup.window_open);

    // Nor do Update now or Update when I quit, and Quit Minerva just quits.
    assert_eq!(job(minerva.act(Action::UpdateNow)), None);
    assert_eq!(job(minerva.act(Action::UpdateWhenIQuit)), None);
    let state = minerva.act(Action::Quit).clone();
    assert_eq!(job(&state), None);
    assert!(state.quit);
}

#[test]
fn with_a_session_running_choosing_to_update_asks_first_and_downloads_nothing() {
    let mut player = hunting();
    assert_eq!(player.minerva.press(PanelControl::Update), None);
    let state = player.minerva.panel_state().clone();
    assert_eq!(shown(&state), (UpdateStage::Asking, UpdateOffer::Choices, QuitItem::Quit));
    assert_eq!(job(&state), None);
    // Help & Setup opens, to ask.
    assert!(state.setup.window_open);
}

#[test]
fn update_now_after_the_warning_downloads_then_installs_and_relaunches() {
    let mut player = hunting();
    player.minerva.press(PanelControl::Update);

    let state = player.minerva.act(Action::UpdateNow).clone();
    assert_eq!(notice(&state).stage, UpdateStage::Downloading);
    let id = downloading(&state);

    let state = player.minerva.on_update_download(id, got(NEWER)).clone();
    assert_eq!(job(&state), Some(UpdateJob { id, step: UpdateStep::Install { relaunch: true } }));
    assert_eq!(notice(&state).stage, UpdateStage::Installing);
}

#[test]
fn later_leaves_the_newer_version_showing_and_downloads_nothing() {
    let mut player = hunting();
    player.minerva.press(PanelControl::Update);

    let state = player.minerva.act(Action::UpdateLater).clone();
    assert_eq!(shown(&state), (UpdateStage::Out, UpdateOffer::Update, QuitItem::Quit));
    assert_eq!(job(&state), None);
    assert_eq!(state.updates.status, UpdateStatus::NewerVersion);
    assert_eq!(version(&state), Some(NEWER));

    // And choosing to update again asks again.
    player.minerva.press(PanelControl::Update);
    assert_eq!(notice(player.minerva.panel_state()).stage, UpdateStage::Asking);
}

#[test]
fn update_when_i_quit_downloads_now_and_installs_without_relaunching_on_quit_minerva() {
    let mut player = hunting();
    player.minerva.press(PanelControl::Update);

    let state = player.minerva.act(Action::UpdateWhenIQuit).clone();
    assert_eq!(shown(&state), (UpdateStage::DownloadingForQuit, UpdateOffer::UpdateNow, QuitItem::QuitAndUpdate));
    let id = downloading(&state);

    // Downloaded, it waits for the player to quit: the Session carries on.
    let state = player.minerva.on_update_download(id, got(NEWER)).clone();
    assert_eq!(shown(&state), (UpdateStage::InstallsWhenYouQuit, UpdateOffer::UpdateNow, QuitItem::QuitAndUpdate));
    assert_eq!(job(&state), None);
    let state = player.hunt(60 * S, 10);
    assert_eq!(job(&state), None);
    assert_eq!(notice(&state).stage, UpdateStage::InstallsWhenYouQuit);

    let state = player.minerva.act(Action::Quit).clone();
    assert!(!state.quit);
    assert_eq!(notice(&state).stage, UpdateStage::Installing);
    assert_eq!(job(&state), Some(UpdateJob { id, step: UpdateStep::Install { relaunch: false } }));
}

#[test]
fn asked_again_after_update_when_i_quit_the_question_shows_and_quit_minerva_still_updates() {
    let mut player = hunting();
    player.minerva.press(PanelControl::Update);
    let id = downloading(&player.minerva.act(Action::UpdateWhenIQuit).clone());
    player.minerva.on_update_download(id, got(NEWER));

    player.minerva.press(PanelControl::Update);
    let state = player.minerva.panel_state();
    assert_eq!(shown(state), (UpdateStage::Asking, UpdateOffer::Choices, QuitItem::QuitAndUpdate));
}

#[test]
fn quitting_while_the_update_for_when_i_quit_downloads_waits_to_install_it() {
    let mut player = hunting();
    player.minerva.press(PanelControl::Update);
    let id = downloading(&player.minerva.act(Action::UpdateWhenIQuit).clone());

    let state = player.minerva.act(Action::Quit).clone();
    assert!(!state.quit);
    assert!(state.updates.quitting);
    assert_eq!(shown(&state), (UpdateStage::FinishingBeforeQuit, UpdateOffer::Nothing, QuitItem::QuitWithoutUpdating));

    let state = player.minerva.on_update_download(id, got(NEWER)).clone();
    assert!(!state.quit);
    assert_eq!(job(&state), Some(UpdateJob { id, step: UpdateStep::Install { relaunch: false } }));
}

#[test]
fn quit_minerva_chosen_again_while_it_waits_quits_at_once_and_installs_nothing() {
    let mut player = hunting();
    player.minerva.press(PanelControl::Update);
    let id = downloading(&player.minerva.act(Action::UpdateWhenIQuit).clone());
    assert!(!player.minerva.act(Action::Quit).quit);

    let state = player.minerva.act(Action::Quit).clone();
    assert!(state.quit);
    assert_eq!(job(&state), None);

    // The download that comes after installs nothing.
    let state = player.minerva.on_update_download(id, got(NEWER)).clone();
    assert!(state.quit);
    assert_eq!(job(&state), None);
}

#[test]
fn while_quit_minerva_waits_choosing_to_update_changes_nothing() {
    let mut player = hunting();
    player.minerva.press(PanelControl::Update);
    let id = downloading(&player.minerva.act(Action::UpdateWhenIQuit).clone());
    let waiting = player.minerva.act(Action::Quit).clone();

    assert_eq!(player.minerva.press(PanelControl::Update), None);
    for action in [Action::UpdateWhenIQuit, Action::UpdateLater, Action::UpdateNow] {
        player.minerva.act(action);
    }
    let state = player.minerva.panel_state();
    assert_eq!(state.updates, waiting.updates);
    assert_eq!(downloading(state), id);

    // Once it has downloaded, it installs as Quit Minerva would: without starting again.
    let state = player.minerva.on_update_download(id, got(NEWER)).clone();
    assert_eq!(job(&state), Some(UpdateJob { id, step: UpdateStep::Install { relaunch: false } }));
}

#[test]
fn quitting_while_the_update_for_when_i_quit_downloads_quits_if_it_fails() {
    let mut player = hunting();
    player.minerva.press(PanelControl::Update);
    let id = downloading(&player.minerva.act(Action::UpdateWhenIQuit).clone());
    player.minerva.act(Action::Quit);

    let state = player.minerva.on_update_download(id, DownloadAnswer::Failed { why: "offline".into() }).clone();
    assert!(state.quit);
    assert_eq!(job(&state), None);
}

#[test]
fn quit_minerva_just_quits_when_update_when_i_quit_was_not_chosen() {
    let mut minerva = newer_known(Build::Nsis);
    let state = minerva.act(Action::Quit).clone();
    assert!(state.quit);
    assert_eq!(job(&state), None);
}

#[test]
fn update_now_after_update_when_i_quit_installs_at_once_and_relaunches() {
    let mut player = hunting();
    player.minerva.press(PanelControl::Update);
    let id = downloading(&player.minerva.act(Action::UpdateWhenIQuit).clone());
    player.minerva.on_update_download(id, got(NEWER));

    // Asked again, as a Session is running.
    player.minerva.press(PanelControl::Update);
    let state = player.minerva.act(Action::UpdateNow).clone();
    assert_eq!(shown(&state), (UpdateStage::Installing, UpdateOffer::Nothing, QuitItem::Quit));
    assert_eq!(job(&state), Some(UpdateJob { id, step: UpdateStep::Install { relaunch: true } }));
}

#[test]
fn a_failed_download_says_so_installs_nothing_and_can_be_tried_again() {
    let mut minerva = newer_known(Build::Msi);
    minerva.press(PanelControl::Update);
    let id = downloading(minerva.panel_state());

    let state = minerva.on_update_download(id, DownloadAnswer::Failed { why: "signature didn't check out".into() }).clone();
    assert_eq!(shown(&state), (UpdateStage::DownloadFailed, UpdateOffer::TryAgain, QuitItem::Quit));
    assert_eq!(job(&state), None);
    assert!(!state.quit);
    // The release page, to download it from, is still there.
    assert_eq!(notice(&state).release_url, RELEASE_URL);

    minerva.press(PanelControl::Update);
    let again = downloading(minerva.panel_state());
    assert!(again > id);
    assert_eq!(notice(minerva.panel_state()).stage, UpdateStage::Downloading);
    // A late answer to the first changes nothing.
    let before = minerva.panel_state().clone();
    assert_eq!(minerva.on_update_download(id, got(NEWER)), &before);
}

#[test]
fn an_installer_that_does_not_start_says_so_and_can_be_tried_again() {
    let mut minerva = newer_known(Build::Nsis);
    minerva.press(PanelControl::Update);
    let id = downloading(minerva.panel_state());
    minerva.on_update_download(id, got(NEWER));

    let state = minerva.on_update_install_failed(id, "the installer was blocked").clone();
    assert_eq!(shown(&state), (UpdateStage::InstallFailed, UpdateOffer::TryAgain, QuitItem::Quit));
    assert_eq!(job(&state), None);
    assert!(!state.quit);

    minerva.press(PanelControl::Update);
    assert!(downloading(minerva.panel_state()) > id);
}

#[test]
fn choosing_to_update_with_no_newer_version_known_does_nothing() {
    let mut minerva = launched();
    minerva.on_build(Build::Nsis);
    tick(&mut minerva, 0);
    answer(&mut minerva, latest(MINERVA_VERSION));

    assert_eq!(minerva.press(PanelControl::Update), None);
    assert_eq!(job(minerva.panel_state()), None);
    assert_eq!(minerva.panel_state().updates.notice, None);
}

// Only the newer version offered installs: never this one or an older one, which the feed
// may name by the time the download asks it again (a release deleted since the check).

#[test]
fn a_download_of_another_version_than_the_one_offered_installs_nothing_and_checks_again() {
    let mut minerva = newer_known(Build::Nsis);
    minerva.press(PanelControl::Update);
    let id = downloading(minerva.panel_state());

    let state = minerva.on_update_download(id, got("998.0.0")).clone();
    assert_eq!(shown(&state), (UpdateStage::Changed, UpdateOffer::TryAgain, QuitItem::Quit));
    assert_eq!(job(&state), None);
    assert!(!state.quit);

    // The check asked at once says what is out now.
    let state = tick(&mut minerva, S);
    assert!(asked(&state).is_some());
    let state = answer(&mut minerva, latest(NEWER));
    assert_eq!(shown(&state), (UpdateStage::Out, UpdateOffer::Update, QuitItem::Quit));
}

#[test]
fn a_download_of_this_version_or_an_older_one_never_installs() {
    for version in [MINERVA_VERSION, OLDER] {
        let mut minerva = newer_known(Build::Nsis);
        minerva.press(PanelControl::Update);
        let id = downloading(minerva.panel_state());

        let state = minerva.on_update_download(id, got(version)).clone();
        assert_eq!(notice(&state).stage, UpdateStage::Changed, "{version}");
        assert_eq!(job(&state), None, "{version}");

        // And the check after finds nothing newer: the notice goes.
        tick(&mut minerva, S);
        let state = answer(&mut minerva, latest(version));
        assert_eq!(state.updates.notice, None, "{version}");
        assert_eq!(job(&state), None, "{version}");
    }
}

#[test]
fn the_version_offered_named_with_a_v_installs() {
    let mut minerva = newer_known(Build::Nsis);
    minerva.press(PanelControl::Update);
    let id = downloading(minerva.panel_state());

    let state = minerva.on_update_download(id, got("v999.0.0")).clone();
    assert_eq!(job(&state), Some(UpdateJob { id, step: UpdateStep::Install { relaunch: true } }));
}

#[test]
fn a_release_withdrawn_while_it_downloads_installs_nothing() {
    let mut minerva = newer_known(Build::Nsis);
    minerva.press(PanelControl::Update);
    let id = downloading(minerva.panel_state());

    // A check while it downloads finds the release gone: nothing newer is out.
    minerva.act(Action::CheckForUpdates);
    tick(&mut minerva, S);
    let state = answer(&mut minerva, latest(MINERVA_VERSION));
    assert_eq!(state.updates.notice, None);

    let state = minerva.on_update_download(id, got(NEWER)).clone();
    assert_eq!(job(&state), None);
    assert_eq!(state.updates.notice, None);
    assert!(!state.quit);
}

#[test]
fn quitting_while_the_update_for_when_i_quit_downloads_quits_without_it_if_another_version_comes() {
    let mut player = hunting();
    player.minerva.press(PanelControl::Update);
    let id = downloading(&player.minerva.act(Action::UpdateWhenIQuit).clone());
    player.minerva.act(Action::Quit);

    let state = player.minerva.on_update_download(id, got(MINERVA_VERSION)).clone();
    assert!(state.quit);
    assert_eq!(job(&state), None);
}

// The Owl Bubble's update line (ADR 0025).

/// The Owl Bubble's size.
const OWL: Size = Size { width: 420, height: 56 };

/// A core at launch, set up, over the game window, its Owl Bubble's size known.
fn over_the_game() -> Minerva {
    let mut minerva = launched_with(common::set_up());
    minerva.set_owl_bubble_size(OWL);
    minerva.on_game_window(Some(common::window_on_wide_screen(common::GAME)));
    minerva
}

/// Minerva's own check at launch finds `version`.
fn found_at_launch(minerva: &mut Minerva, version: &str) -> PanelState {
    tick(minerva, 0);
    answer(minerva, latest(version))
}

/// What the owl says, if anything.
fn owl_line(state: &PanelState) -> Option<OwlLine> {
    state.owl_bubble.as_ref().map(|owl| owl.line.clone())
}

fn update_line(version: &str) -> Option<OwlLine> {
    Some(OwlLine::Update { version: version.into() })
}

#[test]
fn a_newer_version_minerva_found_by_itself_is_said_by_the_owl_at_the_top_centre_of_the_game_window() {
    let mut minerva = over_the_game();

    let state = found_at_launch(&mut minerva, NEWER);

    let owl = state.owl_bubble.expect("the owl");
    assert_eq!(owl.line, OwlLine::Update { version: NEWER.into() });
    assert_eq!(owl.mood, OwlMood::News);
    assert!(owl.clickable, "its × and the bubble take clicks");
    // Centred on the 1920 px client at x 812, a twentieth of its 1200 px down from y 110.
    assert_eq!(owl.placement, ScreenRect { x: 1562, y: 170, width: 420, height: 56 });
}

#[test]
fn the_owl_says_nothing_of_this_version_nor_without_a_game_window_nor_while_another_app_is_in_front() {
    let mut minerva = over_the_game();
    assert_eq!(found_at_launch(&mut minerva, MINERVA_VERSION).owl_bubble, None);

    let mut minerva = launched_with(common::set_up());
    minerva.set_owl_bubble_size(OWL);
    assert_eq!(found_at_launch(&mut minerva, NEWER).owl_bubble, None, "no game window");
    let state = minerva.on_game_window(Some(common::window_on_wide_screen(common::GAME)));
    assert_eq!(owl_line(state), update_line(NEWER));

    assert_eq!(minerva.on_foreground(Foreground::OtherApp).owl_bubble, None);
    assert_eq!(owl_line(minerva.on_foreground(Foreground::Game)), update_line(NEWER));
}

#[test]
fn a_newer_version_the_players_check_for_updates_found_is_not_said_by_the_owl() {
    let mut minerva = over_the_game();
    found_at_launch(&mut minerva, MINERVA_VERSION);

    minerva.act(Action::CheckForUpdates);
    tick(&mut minerva, S);
    let state = answer(&mut minerva, latest(NEWER));
    assert_eq!(version(&state), Some(NEWER), "Help & Setup says it");
    assert_eq!(state.owl_bubble, None);

    // Nor when Minerva's own next check finds it again: the player has been told.
    tick(&mut minerva, 6 * HOUR + S);
    assert_eq!(answer(&mut minerva, latest(NEWER)).owl_bubble, None);
}

#[test]
fn the_owls_x_dismisses_the_update_line_until_a_newer_version_is_found() {
    let mut minerva = over_the_game();
    found_at_launch(&mut minerva, NEWER);

    let state = minerva.act(Action::DismissUpdateLine).clone();
    assert_eq!(state.owl_bubble, None);
    assert_eq!(version(&state), Some(NEWER), "the tray and Help & Setup still say it");

    tick(&mut minerva, 6 * HOUR);
    assert_eq!(answer(&mut minerva, latest(NEWER)).owl_bubble, None, "the same version");

    tick(&mut minerva, 12 * HOUR);
    assert_eq!(owl_line(&answer(&mut minerva, latest("999.0.1"))), update_line("999.0.1"));
}

#[test]
fn a_dismissed_update_line_is_said_again_at_the_next_launch() {
    let mut minerva = over_the_game();
    found_at_launch(&mut minerva, NEWER);
    minerva.act(Action::DismissUpdateLine);

    let mut minerva = over_the_game();
    assert_eq!(owl_line(&found_at_launch(&mut minerva, NEWER)), update_line(NEWER));
}

#[test]
fn a_click_on_the_owls_update_line_opens_about_and_credits_and_dismisses_it() {
    let mut minerva = over_the_game();
    let about = found_at_launch(&mut minerva, NEWER).setup.about;

    let state = minerva.act(Action::OpenUpdateLine).clone();
    assert!(state.setup.window_open);
    assert_eq!(state.setup.about, about + 1, "Help & Setup turns to About & Credits");

    let state = minerva.act(Action::CancelSetup).clone();
    assert!(!state.setup.window_open);
    assert_eq!(state.owl_bubble, None);
}

#[test]
fn the_owl_says_nothing_of_the_update_while_help_and_setup_shows() {
    let mut minerva = over_the_game();
    found_at_launch(&mut minerva, NEWER);

    minerva.press(PanelControl::Setup);
    assert_eq!(minerva.panel_state().owl_bubble, None);

    let state = minerva.act(Action::CancelSetup);
    assert_eq!(owl_line(state), update_line(NEWER), "closed without a click on the owl");
}

#[test]
fn choosing_to_update_ends_the_owls_update_line_for_this_launch() {
    let mut minerva = over_the_game();
    minerva.on_build(Build::Nsis);
    found_at_launch(&mut minerva, NEWER);
    minerva.press(PanelControl::Update);
    let id = downloading(minerva.panel_state());
    minerva.act(Action::CancelSetup);
    assert_eq!(minerva.panel_state().owl_bubble, None, "downloading");

    let state = minerva.on_update_download(id, DownloadAnswer::Failed { why: "offline".into() }).clone();
    assert_eq!(notice(&state).stage, UpdateStage::DownloadFailed);
    assert_eq!(state.owl_bubble, None, "Help & Setup says it failed");

    // A build that can't install only opens the release page.
    let mut minerva = over_the_game();
    found_at_launch(&mut minerva, NEWER);
    assert_eq!(minerva.press(PanelControl::Update), Some(Request::OpenInBrowser { url: RELEASE_URL.into() }));
    assert_eq!(notice(minerva.panel_state()).stage, UpdateStage::NotifyOnly);
    assert_eq!(minerva.panel_state().owl_bubble, None);
}

#[test]
fn the_check_after_a_download_of_another_version_is_the_players() {
    let mut minerva = over_the_game();
    minerva.on_build(Build::Nsis);
    found_at_launch(&mut minerva, NEWER);
    minerva.press(PanelControl::Update);
    let id = downloading(minerva.panel_state());
    minerva.on_update_download(id, got("998.0.0"));
    minerva.act(Action::CancelSetup);

    tick(&mut minerva, S);
    let state = answer(&mut minerva, latest("999.0.1"));
    assert_eq!(notice(&state).stage, UpdateStage::Out);
    assert_eq!(state.owl_bubble, None, "Help & Setup said what is out now");
}

#[test]
fn an_item_lookups_line_goes_before_the_update_line_which_comes_back_after() {
    let mut minerva = over_the_game();
    found_at_launch(&mut minerva, NEWER);

    assert_eq!(minerva.on_hotkey("ctrl+shift+u"), Some(Request::CaptureItemTooltip));
    let reading = ItemTooltipReading { pointer: common::POINTER, tooltip: ItemTooltip::None, at_ms: S };
    let state = minerva.on_item_tooltip_reading(Some(reading)).clone();
    let owl = state.owl_bubble.expect("the owl");
    assert_eq!(owl.line, OwlLine::ItemLookup(state.item_lookup.clone().expect("a line")));
    assert_eq!(owl.mood, OwlMood::Problem);
    assert!(!owl.clickable, "clicks go through to the game");

    assert!(matches!(owl_line(&tick(&mut minerva, 7 * S - 1)), Some(OwlLine::ItemLookup(_))));
    assert_eq!(owl_line(&tick(&mut minerva, 7 * S)), update_line(NEWER), "once the lookup's line goes");
}
