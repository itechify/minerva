//! Updates (#124): Minerva checks for a newer version at launch, then every 6 hours while it
//! runs, never by itself while the player's "Check for updates" is off, and whenever they press
//! Check for updates in Help & Setup. The shell runs each check the core asks for and tells it
//! the latest version the feed names, or that the check failed. The core says what the tray and
//! Help & Setup show: up to date (and when it last checked), a newer version with its release
//! page, or that it couldn't check. A failed check leaves a newer version found before as it is.

mod common;

use common::FakeSource;
use minerva_core::{
    Action, Build, MINERVA_VERSION, Minerva, NewerVersion, PanelControl, PanelState, Request, Settings, UpdateAnswer,
    UpdateStatus,
};

const S: u64 = 1000;
const HOUR: u64 = 60 * 60 * S;

/// The wall clock at capture time 0: 21 Oct 2026, 16:00 UTC.
const WALL: u64 = 1_792_598_400_000;

/// A version newer than any Minerva yet, and one older.
const NEWER: &str = "999.0.0";
const OLDER: &str = "0.0.1";

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
    assert_eq!(state.updates.newer, None);
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

    assert!(minerva.panel_state().updates.on);
    let state = minerva.act(Action::SetCheckForUpdates { on: false }).clone();
    assert!(!state.updates.on);
    assert!(!minerva.settings().check_for_updates);
    assert_eq!(asked(&tick(&mut minerva, 7 * HOUR)), None);

    let state = minerva.act(Action::SetCheckForUpdates { on: true }).clone();
    assert!(state.updates.on);
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
    let release_url = "https://github.com/itechify/minerva/releases/tag/v999.0.0";
    assert_eq!(state.updates.newer, Some(NewerVersion { version: NEWER.into(), release_url: release_url.into() }));
    assert_eq!(state.updates.last_checked_unix_ms, Some(WALL));
    assert_eq!(minerva.press(PanelControl::OpenRelease), Some(Request::OpenInBrowser { url: release_url.into() }));
}

#[test]
fn a_version_named_with_a_v_reads_as_without() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    let state = answer(&mut minerva, latest("v999.0.0"));
    assert_eq!(state.updates.newer.map(|newer| newer.version), Some(NEWER.to_string()));
}

#[test]
fn an_older_version_than_this_one_is_up_to_date() {
    let mut minerva = launched();
    tick(&mut minerva, 0);
    let state = answer(&mut minerva, latest(OLDER));
    assert_eq!(state.updates.status, UpdateStatus::UpToDate);
    assert_eq!(state.updates.newer, None);
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
    assert_eq!(state.updates.newer, None);
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
    let found = answer(&mut minerva, latest(NEWER)).updates.newer;

    tick(&mut minerva, 6 * HOUR);
    let state = answer(&mut minerva, failed());
    assert_eq!(state.updates.status, UpdateStatus::Failed);
    assert_eq!(state.updates.newer, found);
    assert_eq!(state.updates.last_checked_unix_ms, Some(WALL));
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

#[test]
fn a_build_is_notify_only_until_the_shell_says_it_can_install() {
    let mut minerva = launched();
    assert!(minerva.panel_state().updates.notify_only);
    assert!(!minerva.on_build(Build::Nsis).updates.notify_only);
    assert!(!minerva.on_build(Build::Msi).updates.notify_only);
    assert!(minerva.on_build(Build::NotifyOnly).updates.notify_only);
}
