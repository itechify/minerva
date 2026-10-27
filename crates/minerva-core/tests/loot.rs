//! Loot (GLOSSARY): the mesos the Pickup Log says the player picked up in the Session, and
//! mesos an hour over its active time (ADR 0026, #170). Readings are built by hand; the
//! Session is the EXP Session, started by the EXP bar's first change.
//!
//! The level is the ~4,199.5 EXP one of `exp_session.rs`: 160 [3.81%] at 0 s, then
//! 266 [6.33%] at 10 s starts the Session, and 335 [7.98%] at 70 s is a minute in.

mod common;

use common::{S, read, tracking};
use minerva_core::{
    Action, Minerva, PanelControl, PanelSizes, PanelState, PickupLine, PickupLogReading, PickupLogSeen, ScreenRect, Size,
    StatusBar, StatusBarReading, TrackingState,
};

use common::{LOG, picked_up, pickup_reading as arrived};

fn mesos(amount: u64) -> PickupLine {
    PickupLine::Mesos { amount }
}

/// A Session a minute in: started at 10 s, 69 EXP gained by 70 s.
fn a_minute_in() -> Minerva {
    let mut minerva = tracking();
    common::start_session(&mut minerva, 0, 10 * S);
    read(&mut minerva, 70 * S, 335, 798);
    minerva
}

#[test]
fn the_sessions_mesos_are_the_total_of_the_mesos_lines_that_arrived() {
    let mut minerva = a_minute_in();
    assert_eq!(minerva.panel_state().exp_tracking.loot.mesos, 0);

    picked_up(&mut minerva, 71 * S, vec![mesos(29)]);
    let state = picked_up(&mut minerva, 72 * S, vec![mesos(1_200), mesos(29)]);

    assert_eq!(state.exp_tracking.loot.mesos, 1_258);
}

#[test]
fn only_mesos_lines_of_a_clean_capture_count() {
    let mut minerva = a_minute_in();
    let exp = PickupLine::Exp { amount: 14, party: false };
    let item = PickupLine::Item { name: "Orange Mushroom Cap".into(), quantity: 2, tab: "Etc".into(), cut_short: false };

    picked_up(&mut minerva, 71 * S, vec![exp, PickupLine::PartyBonus { amount: 1 }, item, PickupLine::Unreadable]);
    // A capture some row of which couldn't be read, or none at all, brings no lines to count.
    minerva.on_pickup_log_reading(arrived(72 * S, PickupLogSeen::Covered { area: LOG }, vec![mesos(500)]));
    let state = minerva.on_pickup_log_reading(arrived(73 * S, PickupLogSeen::Nothing, vec![mesos(500)])).clone();

    assert_eq!(state.exp_tracking.loot.mesos, 0);
}

#[test]
fn mesos_picked_up_before_the_session_starts_are_not_its_own() {
    let mut minerva = tracking();
    read(&mut minerva, 0, 160, 381);
    picked_up(&mut minerva, 5 * S, vec![mesos(300)]);

    read(&mut minerva, 10 * S, 266, 633);
    let state = picked_up(&mut minerva, 11 * S, vec![mesos(40)]);

    assert_eq!(state.exp_tracking.loot.mesos, 40);
}

/// A clean reading delivered at `at_ms` of lines that arrived at `arrived_ms` (the captures where
/// the stack shifted them in; a line is read a few captures after it arrives, and so comes later).
fn arriving(at_ms: u64, arrived_ms: u64, lines: Vec<PickupLine>) -> PickupLogReading {
    PickupLogReading { arrived_at: vec![arrived_ms; lines.len()], ..common::clean_pickup_reading(at_ms, lines) }
}

#[test]
fn a_line_that_arrived_before_the_session_started_is_not_its_own_though_it_is_read_after() {
    let mut minerva = tracking();
    read(&mut minerva, 0, 160, 381);
    read(&mut minerva, 10 * S, 266, 633);

    minerva.on_pickup_log_reading(arriving(10 * S + 300, 10 * S - 100, vec![mesos(300)]));
    let state = minerva.on_pickup_log_reading(arriving(10 * S + 400, 10 * S, vec![mesos(40)])).clone();

    assert_eq!(state.exp_tracking.loot.mesos, 40);
}

#[test]
fn a_line_that_arrived_before_a_reset_the_player_undid_is_not_counted_after_the_undo() {
    let mut minerva = a_minute_in();
    picked_up(&mut minerva, 71 * S, vec![mesos(1_500)]);
    minerva.tick(80 * S);
    minerva.act(Action::ResetSession);
    minerva.tick(85 * S);
    minerva.act(Action::UndoReset);

    // Arrived while the Session was set aside, read just after it came back.
    minerva.on_pickup_log_reading(arriving(85 * S + 300, 82 * S, vec![mesos(300)]));
    let state = minerva.on_pickup_log_reading(arriving(85 * S + 400, 85 * S + 100, vec![mesos(40)])).clone();

    assert_eq!(state.exp_tracking.loot.mesos, 1_540);
}

#[test]
fn a_party_line_that_arrived_before_a_reset_does_not_join_the_next_session() {
    let mut minerva = a_minute_in();
    minerva.tick(80 * S);
    minerva.act(Action::ResetSession);
    let party = PickupLine::Exp { amount: 30, party: true };

    // Read after the reset, but it arrived before.
    minerva.on_pickup_log_reading(arriving(80 * S + 300, 79 * S + 900, vec![party]));
    let state = read(&mut minerva, 82 * S, 400, 952);

    assert_eq!(state.exp_tracking.exp_split, None);
}

#[test]
fn a_reset_ends_the_log_readers_stream_so_the_lines_it_was_still_reading_are_dropped() {
    let mut minerva = a_minute_in();
    let before = minerva.panel_state().pickup_log_generation;

    minerva.act(Action::ResetSession);

    assert!(minerva.panel_state().pickup_log_generation > before);
}

#[test]
fn with_the_grind_tracker_off_nothing_is_counted() {
    let mut minerva = a_minute_in();
    minerva.act(Action::SetGrindTracker { on: false });
    picked_up(&mut minerva, 71 * S, vec![mesos(300)]);
    minerva.act(Action::SetGrindTracker { on: true });

    let state = picked_up(&mut minerva, 72 * S, vec![mesos(40)]);

    assert_eq!(state.exp_tracking.loot.mesos, 40);
}

#[test]
fn mesos_an_hour_are_over_the_sessions_active_time_and_hold_with_the_other_figures() {
    let mut minerva = a_minute_in();
    assert_eq!(minerva.panel_state().exp_tracking.loot.mesos_per_hour, Some(0));

    picked_up(&mut minerva, 71 * S, vec![mesos(1_500)]);
    // The rate holds until the figures are measured afresh, at 90 s.
    assert_eq!(minerva.tick(80 * S).exp_tracking.loot.mesos_per_hour, Some(0));

    // 1,500 mesos in 90 s of active play (10 s to 100 s).
    let state = minerva.tick(100 * S).clone();
    assert_eq!(state.exp_tracking.loot.mesos_per_hour, Some(60_000));
    assert_eq!(state.exp_tracking.active_s, 90);
}

#[test]
fn mesos_picked_up_while_no_exp_comes_in_keep_the_session_from_pausing() {
    let mut minerva = a_minute_in();

    // The last EXP change was at 70 s; a break is more than 2 minutes without play, and the
    // mesos are play.
    picked_up(&mut minerva, 150 * S, vec![mesos(100)]);
    picked_up(&mut minerva, 220 * S, vec![mesos(100)]);
    let state = minerva.tick(300 * S).clone();

    assert_eq!(state.exp_tracking.state, TrackingState::Active);
    assert_eq!(state.exp_tracking.active_s, 290, "10 s to 300 s, no break");
    assert_eq!(state.exp_tracking.loot.mesos, 200);

    // The EXP alone would have been paused, and 2 minutes after the last mesos it is.
    let paused = minerva.tick(341 * S).clone();
    assert_eq!(paused.exp_tracking.state, TrackingState::Paused);
}

#[test]
fn mesos_do_not_move_the_pace_trace_or_the_exp_rates() {
    let mut minerva = a_minute_in();
    let before = minerva.panel_state().exp_tracking.clone();

    let after = picked_up(&mut minerva, 70 * S, vec![mesos(5_000)]);

    assert_eq!(after.exp_tracking.pace, before.pace);
    assert_eq!(after.exp_tracking.rolling, before.rolling);
    assert_eq!(after.exp_tracking.session_average, before.session_average);
}

#[test]
fn a_reset_clears_the_mesos_and_its_undo_brings_them_back() {
    let mut minerva = a_minute_in();
    picked_up(&mut minerva, 71 * S, vec![mesos(1_500)]);

    let reset = minerva.act(Action::ResetSession).clone();
    assert_eq!(reset.exp_tracking.loot.mesos, 0);
    assert_eq!(reset.exp_tracking.loot.mesos_per_hour, None);

    let undone = minerva.act(Action::UndoReset).clone();
    assert_eq!(undone.exp_tracking.loot.mesos, 1_500);
}

#[test]
fn the_new_session_after_a_reset_counts_from_zero() {
    let mut minerva = a_minute_in();
    picked_up(&mut minerva, 71 * S, vec![mesos(1_500)]);
    minerva.act(Action::ResetSession);

    read(&mut minerva, 80 * S, 400, 952);
    let state = picked_up(&mut minerva, 81 * S, vec![mesos(40)]);

    assert_eq!(state.exp_tracking.loot.mesos, 40);
    assert!(!state.exp_tracking.reset_undoable);
}

#[test]
fn switching_character_clears_the_mesos_with_the_session() {
    let mut minerva = tracking();
    let named = |minerva: &mut Minerva, at_ms: u64, name: &str| {
        let area = ScreenRect { x: 334, y: 1145, width: 331, height: 55 };
        let bar = StatusBar { area, level: Some(13), job: None, name: Some(name.into()), exp_figures: None };
        minerva.on_status_bar_reading(StatusBarReading { bar: Some(bar), at_ms, near_miss: None }).clone()
    };
    for at in [0, 1, 2] {
        named(&mut minerva, at * S, "Egbert");
    }
    common::start_session(&mut minerva, 3 * S, 10 * S);
    let before = picked_up(&mut minerva, 11 * S, vec![mesos(1_500)]);
    assert_eq!(before.exp_tracking.loot.mesos, 1_500);

    let mut state = before;
    for at in [12, 13, 14] {
        state = named(&mut minerva, at * S, "Bob");
    }

    assert!(state.exp_tracking.character_switched);
    assert_eq!(state.exp_tracking.loot.mesos, 0);
}

/// A core with the Grind Tracker's window showing, and where it is on screen.
fn with_the_tracker_showing() -> (Minerva, ScreenRect) {
    let mut minerva = a_minute_in();
    minerva.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    let tracker = minerva.set_grind_tracker_size(Size { width: 300, height: 150 }).grind_tracker.placement;
    (minerva, tracker.expect("the Grind Tracker shows beside the panel"))
}

/// The log read, with nothing arriving, at `at_ms`, where it is at `area`.
fn log_at(minerva: &mut Minerva, at_ms: u64, area: ScreenRect) -> PanelState {
    minerva.on_pickup_log_reading(arrived(at_ms, PickupLogSeen::Clear { area }, vec![])).clone()
}

#[test]
fn the_log_is_covered_once_one_of_our_windows_has_been_over_it_for_more_than_a_few_seconds() {
    let (mut minerva, tracker) = with_the_tracker_showing();
    assert!(!minerva.panel_state().pickup_log_covered);

    // Over where the log was seen, at 10 readings a second: a moment's cover says nothing.
    let mut state = log_at(&mut minerva, 100 * S, tracker);
    for tenth in 1..=30 {
        state = log_at(&mut minerva, 100 * S + tenth * 100, tracker);
    }
    assert!(!state.pickup_log_covered, "3 s is not more than a few seconds");

    let state = log_at(&mut minerva, 103 * S + 100, tracker);
    assert!(state.pickup_log_covered);
}

#[test]
fn a_window_that_misses_the_log_covers_nothing() {
    let (mut minerva, tracker) = with_the_tracker_showing();
    let beside = ScreenRect { x: tracker.x, y: tracker.y + tracker.height as i32 + 500, width: 100, height: 100 };

    log_at(&mut minerva, 100 * S, beside);
    let state = log_at(&mut minerva, 110 * S, beside);

    assert!(!state.pickup_log_covered);
}

#[test]
fn a_covered_log_that_was_last_seen_under_the_window_stays_covered_while_no_lines_are_found() {
    let (mut minerva, tracker) = with_the_tracker_showing();
    log_at(&mut minerva, 100 * S, tracker);
    minerva.on_pickup_log_reading(arrived(102 * S, PickupLogSeen::Covered { area: tracker }, vec![]));

    let state = minerva.on_pickup_log_reading(arrived(104 * S, PickupLogSeen::Nothing, vec![])).clone();

    assert!(state.pickup_log_covered);
}

#[test]
fn the_note_goes_as_soon_as_the_window_is_off_the_log() {
    let (mut minerva, tracker) = with_the_tracker_showing();
    log_at(&mut minerva, 100 * S, tracker);
    assert!(log_at(&mut minerva, 110 * S, tracker).pickup_log_covered);

    // The player closes the window.
    minerva.press(PanelControl::CloseGrindTracker);
    assert!(!minerva.panel_state().pickup_log_covered);

    // Or the log is read somewhere else: the cover starts afresh.
    minerva.press(PanelControl::ToggleGrindTracker);
    let elsewhere = ScreenRect { x: 0, y: 0, width: 10, height: 10 };
    assert!(!log_at(&mut minerva, 111 * S, elsewhere).pickup_log_covered);
    assert!(!log_at(&mut minerva, 120 * S, tracker).pickup_log_covered, "the 9 s were not under it");
}

#[test]
fn there_is_no_rate_before_the_session_has_active_time() {
    let mut minerva = tracking();
    let state = common::start_session(&mut minerva, 0, 10 * S);

    assert_eq!(state.exp_tracking.loot.mesos_per_hour, None);
}
