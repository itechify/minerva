//! Characters (GLOSSARY): the Status Bar names who is playing, and their level. A new name
//! is another Character, and a new Session at once; the level read there is the one the
//! Session learns, and a level-up it shows is taken at once. While no name can be read, the
//! Session tells another Character from the EXP bar alone, as before.
//!
//! The Status Bar's readings here are made up, and so are the levels' totals: 4,200 EXP for
//! level 13 (the samples' Egbert's), 4,600 for level 14, and 16,682 for another Character's.

mod common;

use common::{tracking, S, overlaps, percent, unread};
use minerva_core::{
    Action, Character, Deaths, ExpBar, ExpTracking, Foreground, Minerva, PanelSizes, PanelState, STATUS_BAR_STRIP,
    ScreenRect, Size, StatusBar, StatusBarReading, TrackingState,
};

const LV13: u64 = 4200;
const LV14: u64 = 4600;
const OTHER: u64 = 16_682;

/// The EXP bar read as `exp` on a level of `total` EXP, at `at_ms`.
fn read(minerva: &mut Minerva, at_ms: u64, exp: u64, total: u64) -> PanelState {
    common::read(minerva, at_ms, exp, percent(exp, total))
}

/// The Status Bar read at `at_ms`, showing a Magician named `name` (`None`: unreadable) at
/// `level`.
fn status(minerva: &mut Minerva, at_ms: u64, name: Option<&str>, level: Option<u32>) -> PanelState {
    let area = ScreenRect { x: 334, y: 1145, width: 331, height: 55 };
    let bar = StatusBar { area, level, job: Some("Magician".to_string()), name: name.map(String::from), exp_figures: None };
    minerva.on_status_bar_reading(StatusBarReading { bar: Some(bar), at_ms }).clone()
}

/// A Status Bar capture at `at_ms` with no Status Bar in it: the character select screen.
fn no_status_bar(minerva: &mut Minerva, at_ms: u64) -> PanelState {
    minerva.on_status_bar_reading(StatusBarReading { bar: None, at_ms }).clone()
}

/// Egbert, a level 13 Magician, a minute and more into a Session: 100 EXP every 10 s up to
/// 860 EXP at 70 s, the Status Bar read every second.
fn egbert_a_minute_in() -> Minerva {
    let mut minerva = tracking();
    for k in 0..=7 {
        status(&mut minerva, k * 10 * S, Some("Egbert"), Some(13));
        read(&mut minerva, k * 10 * S, 160 + 100 * k, LV13);
    }
    minerva
}

/// The character select screen from `from` to `to` seconds: no Status Bar, no EXP bar.
fn character_select(minerva: &mut Minerva, from: u64, to: u64) {
    for at in from..=to {
        no_status_bar(minerva, at * S);
        unread(minerva, at * S);
    }
}

fn egbert(level: u32) -> Option<Character> {
    Some(Character { name: "Egbert".to_string(), job: Some("Magician".to_string()), level: Some(level) })
}

#[test]
fn no_character_shows_until_the_status_bar_names_one_and_it_stays_through_a_moment_unread() {
    let mut minerva = tracking();
    assert_eq!(minerva.panel_state().character, None);
    assert_eq!(status(&mut minerva, 0, None, Some(13)).character, None, "a level alone names no one");

    assert_eq!(status(&mut minerva, S, Some("Egbert"), Some(13)).character, egbert(13));
    assert_eq!(status(&mut minerva, 2 * S, None, None).character, egbert(13), "the pointer over the name");
    assert_eq!(no_status_bar(&mut minerva, 3 * S).character, egbert(13), "the Session is still theirs");
}

#[test]
fn a_new_name_on_the_status_bar_starts_a_new_session_at_once() {
    let mut minerva = egbert_a_minute_in();
    character_select(&mut minerva, 75, 79);

    // Bob, at the same level as Egbert: no EXP reading has come yet, and none needs to.
    let switched = status(&mut minerva, 80 * S, Some("Bob"), Some(13));
    assert_eq!(
        switched.exp_tracking,
        ExpTracking { state: TrackingState::Waiting, character_switched: true, ..ExpTracking::default() }
    );
    assert_eq!(switched.character.as_ref().map(|c| c.name.as_str()), Some("Bob"));

    // Bob's bar fits Egbert's level, with less EXP than Egbert had: it is no death, and
    // what Bob gains next is all the new Session counts.
    let first = read(&mut minerva, 80 * S, 500, LV13);
    assert_eq!(first.exp_tracking.state, TrackingState::Waiting);
    let started = read(&mut minerva, 90 * S, 600, LV13);
    assert_eq!(started.exp_tracking.state, TrackingState::WarmingUp);
    assert_eq!(started.exp_tracking.deaths, Deaths::default());
    let on = read(&mut minerva, 100 * S, 700, LV13);
    assert_eq!(on.exp_tracking.session_average.map(|r| r.exp_per_hour), Some(36_000), "100 EXP in 10 s");
}

#[test]
fn another_name_read_while_the_status_bar_stayed_in_sight_is_a_misread() {
    let mut minerva = egbert_a_minute_in();

    let misread = status(&mut minerva, 71 * S, Some("Egbart"), Some(13));

    assert!(!misread.exp_tracking.character_switched);
    assert_eq!(misread.exp_tracking.state, TrackingState::Active);
    assert_eq!(misread.character, egbert(13));
}

/// Egbert's Session, then the character select screen, then another Character's EXP bar
/// on a level of 16,682 EXP, three readings in a row, while the Status Bar shows `name`.
fn another_level_after_the_character_select_screen(name: Option<&str>) -> PanelState {
    let mut minerva = egbert_a_minute_in();
    character_select(&mut minerva, 75, 79);
    for (k, exp) in [149, 157, 157].into_iter().enumerate() {
        let at = (80 + k as u64) * S;
        status(&mut minerva, at, name, None);
        read(&mut minerva, at, exp, OTHER);
    }
    minerva.panel_state().clone()
}

#[test]
fn with_no_name_read_readings_that_fit_another_level_are_another_character() {
    let state = another_level_after_the_character_select_screen(None);

    assert_eq!(state.exp_tracking.state, TrackingState::Waiting);
    assert!(state.exp_tracking.character_switched);
    assert_eq!(state.character, None, "it isn't Egbert, whoever it is");
}

#[test]
fn readings_that_fit_another_level_are_no_switch_while_the_status_bar_names_the_same_character() {
    let state = another_level_after_the_character_select_screen(Some("Egbert"));

    // The level was learned wrong, or the EXP moved while the bar was out of sight: the
    // Session goes on, from the reading as it is.
    assert!(!state.exp_tracking.character_switched);
    assert_eq!(state.exp_tracking.state, TrackingState::Active);
    assert_eq!(state.exp_tracking.bar, Some(ExpBar { exp: 157, percent_hundredths: 94 }));
    assert_eq!(state.character, egbert(13));
}

#[test]
fn with_no_name_read_another_level_after_the_character_select_screen_is_another_character() {
    let mut minerva = tracking();
    for k in 0..=7 {
        status(&mut minerva, k * 10 * S, None, Some(13));
        read(&mut minerva, k * 10 * S, 160 + 100 * k, LV13);
    }
    character_select(&mut minerva, 75, 79);

    let switched = status(&mut minerva, 80 * S, None, Some(45));

    assert!(switched.exp_tracking.character_switched);
    assert_eq!(switched.exp_tracking.state, TrackingState::Waiting);
}

/// Egbert at level 13, 2,000 EXP to 2,100 (50.00%) by 20 s.
fn egbert_halfway() -> Minerva {
    let mut minerva = tracking();
    for (k, exp) in [2000, 2050, 2100].into_iter().enumerate() {
        status(&mut minerva, k as u64 * 10 * S, Some("Egbert"), Some(13));
        read(&mut minerva, k as u64 * 10 * S, exp, LV13);
    }
    minerva
}

#[test]
fn the_status_bars_level_overrides_a_reading_that_fits_the_level_before() {
    // 1 [0.02%] allows totals from 4,000 to 6,666 EXP: it fits level 13, with less EXP than
    // before, which alone reads as a death.
    let mut alone = egbert_halfway();
    let died = read(&mut alone, 30 * S, 1, LV14);
    assert_eq!(died.exp_tracking.deaths.count, 1);

    // The Status Bar shows level 14 first: the reading is the new level's, and what was left
    // of level 13 counts, with the one EXP in 14.
    let mut minerva = egbert_halfway();
    status(&mut minerva, 30 * S, Some("Egbert"), Some(14));
    let levelled = read(&mut minerva, 30 * S, 1, LV14);
    assert_eq!(levelled.exp_tracking.deaths, Deaths::default());
    assert_eq!(levelled.exp_tracking.bar, Some(ExpBar { exp: 1, percent_hundredths: 2 }));
    assert_eq!(levelled.character, egbert(14));
    // The Session started at 2,050, at 10 s: 50 EXP more by 20 s, then ~2,100 to finish
    // level 13 and 1 into 14, ~2,151 EXP in its 20 s.
    let average = levelled.exp_tracking.session_average.map(|r| r.exp_per_hour).unwrap_or_default();
    assert!((386_000..=388_000).contains(&average), "{average} EXP/h");
}

/// Egbert at level 13, 3,200 EXP to 3,360 (80.00%) by 20 s: far from a level-up.
fn egbert_at_80_percent() -> Minerva {
    let mut minerva = tracking();
    for (k, exp) in [3200, 3280, 3360].into_iter().enumerate() {
        status(&mut minerva, k as u64 * 10 * S, Some("Egbert"), Some(13));
        read(&mut minerva, k as u64 * 10 * S, exp, LV13);
    }
    minerva
}

#[test]
fn a_level_up_the_status_bar_shows_is_taken_at_once_however_far_the_bar_was_from_full() {
    // A quest turned in at 80%, into level 14 with 500 EXP: from the EXP bar alone, that is
    // a misread to set aside, as the bar was nowhere near 100%.
    let mut alone = egbert_at_80_percent();
    let set_aside = read(&mut alone, 30 * S, 500, LV14);
    assert_eq!(set_aside.exp_tracking.bar, Some(ExpBar { exp: 3360, percent_hundredths: 8000 }));

    // The Status Bar shows level 14: the reading is a level-up, counted at once.
    let mut minerva = egbert_at_80_percent();
    status(&mut minerva, 30 * S, Some("Egbert"), Some(14));
    let levelled = read(&mut minerva, 30 * S, 500, LV14);
    assert_eq!(levelled.exp_tracking.bar, Some(ExpBar { exp: 500, percent_hundredths: percent(500, LV14) }));
    assert_eq!(levelled.exp_tracking.deaths, Deaths::default());

    // Read the other way round, the reading set aside comes before the Status Bar shows the
    // level: the next reading is the level-up.
    let mut late = egbert_at_80_percent();
    read(&mut late, 30 * S, 500, LV14);
    status(&mut late, 31 * S, Some("Egbert"), Some(14));
    let levelled_late = read(&mut late, 40 * S, 520, LV14);
    assert_eq!(levelled_late.exp_tracking.bar, Some(ExpBar { exp: 520, percent_hundredths: percent(520, LV14) }));
    assert_eq!(levelled_late.exp_tracking.deaths, Deaths::default());
}

#[test]
fn a_panel_dragged_onto_the_status_bars_character_is_moved_clear_of_it() {
    let mut minerva = tracking();
    minerva.set_panel_sizes(PanelSizes { expanded: Size { width: 340, height: 72 }, collapsed: Size { width: 44, height: 44 } });
    let area = status(&mut minerva, 0, Some("Egbert"), Some(13)).status_bar.expect("found").area;

    let state = minerva.act(Action::PanelMoved { x: 300, y: 1120 }).clone();

    let placement = state.placement.expect("placed");
    assert!(!overlaps(placement, area), "{placement:?} over {area:?}");
}

#[test]
fn the_status_bar_is_captured_across_the_bottom_of_the_client_area_while_the_game_can_show() {
    let mut minerva = tracking();
    assert_eq!(
        minerva.panel_state().status_bar_capture,
        Some(ScreenRect { x: 0, y: 1200 - STATUS_BAR_STRIP as i32, width: 1920, height: STATUS_BAR_STRIP })
    );

    let state = minerva.on_foreground(Foreground::OtherApp);

    assert_eq!(state.status_bar_capture, None, "another app's pixels are never read");
}
