//! The Personal Best (GLOSSARY): the best Rolling Rate, in EXP/h, the player has had on
//! the map they're on. A Visit's best is the highest Rolling Rate shown at a refresh over
//! a full 5 minutes of active play on the map; the map's Personal Best is the best of its
//! Visits'. The Grind Tracker's Best row shows the one from earlier Visits, and a star
//! marks Last 5 min while it beats it.

mod common;

use common::player::Player;
use common::{CONSTRUCTION_SITE_ID, SUNSET_SKY_ID, sharing_a_title, tracking, tracking_with};
use minerva_core::{ExpBar, ExpBarReading, PanelControl, PanelState, TrackingState, VisitBest};

const S: u64 = 1000;
const MIN: u64 = 60 * S;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const ROCKY_ROAD: &str = "victoria-road__rocky-road-i--title-region";

/// The Rolling Rate's EXP/h as the Grind Tracker shows it ("Last 5 min").
fn rolling(state: &PanelState) -> Option<i64> {
    state.exp_tracking.rolling.map(|r| r.exp_per_hour)
}

/// The Best row's EXP/h: the map's Personal Best from earlier Visits.
fn best_row(state: &PanelState) -> Option<u64> {
    state.exp_tracking.personal_best
}

/// Whether the star marks Last 5 min as a new best on the map.
fn starred(state: &PanelState) -> bool {
    state.exp_tracking.new_best
}

/// `map`'s Visit bests as the core hands them out to save, highest first: the Visit under
/// way's included.
fn saved(player: &Player, map: &str) -> Vec<VisitBest> {
    let bests = player.minerva.bests();
    bests.unassigned.into_iter().find(|m| m.id == map).map_or_else(Vec::new, |m| m.visits)
}

/// The EXP/h of each of [`saved`].
fn saved_rates(player: &Player, map: &str) -> Vec<u64> {
    saved(player, map).iter().map(|v| v.exp_per_hour).collect()
}

#[test]
fn a_first_visits_best_is_dated_by_the_wall_clock_at_the_refresh_that_showed_it() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    let best = |exp_per_hour, set_at_unix_ms| vec![VisitBest { exp_per_hour, set_at_unix_ms, ..VisitBest::default() }];

    // 3,600 EXP/h: the first best is the refresh's 5 minutes in, and was set then.
    let five = player.hunt(5 * MIN, 10);
    let first_set = player.wall_clock();
    assert_eq!(rolling(&five), Some(3600));
    assert_eq!(saved(&player, SUNSET_SKY_ID), best(3600, first_set));

    // The same rate shown again isn't a new best: it keeps when it was first set.
    player.hunt(MIN, 10);
    assert_eq!(saved(&player, SUNSET_SKY_ID), best(3600, first_set));

    // Faster, the best rises with each refresh, set at the last.
    let faster = player.hunt(5 * MIN, 20);
    let faster_set = player.wall_clock();
    assert_eq!(rolling(&faster), Some(7200));
    assert_eq!(saved(&player, SUNSET_SKY_ID), best(7200, faster_set));

    // Slower, it stays as it was, and when.
    let slower = player.hunt(5 * MIN, 10);
    assert_eq!(rolling(&slower), Some(3600));
    assert_eq!(saved(&player, SUNSET_SKY_ID), best(7200, faster_set));
}

#[test]
fn a_first_visit_has_no_best_row_and_its_best_counts_from_five_minutes_of_play_there() {
    let mut player = Player::new(tracking());
    // 10,800 EXP/h on the Construction Site for 6 minutes, then on to Sunset Sky.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(6 * MIN, 30);
    assert_eq!(best_row(&player.arrive(SUNSET)), None);

    // 3,600 EXP/h there. Under 5 minutes in, the Rolling Rate still counts some of the
    // Construction Site's play: no best yet.
    let under = player.hunt(4 * MIN + 30 * S, 10);
    assert_eq!(rolling(&under), Some(4320));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [] as [u64; 0]);

    // 5 minutes there: the Rolling Rate is all Sunset Sky's, and this Visit's first best.
    let five = player.hunt(30 * S, 10);
    assert_eq!(rolling(&five), Some(3600));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [3600]);

    // Faster, the best rises with what the Rolling Rate shows, and stays when it falls
    // back. A first Visit has no history to show in a Best row, nor a star.
    let faster = player.hunt(5 * MIN, 20);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [7200]);
    let slower = player.hunt(5 * MIN, 10);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [7200]);
    for state in [&under, &five, &faster, &slower] {
        assert_eq!((best_row(state), starred(state)), (None, false));
    }
}

/// A first Visit to Sunset Sky at 7,200 EXP/h for 6 minutes, then 2 minutes on the
/// Construction Site at 10,800, and back to Sunset Sky.
fn back_on_sunset_sky_after_a_best_of_7200() -> (Player, PanelState) {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(2 * MIN, 30);
    let back = player.arrive(SUNSET);
    (player, back)
}

#[test]
fn coming_back_shows_the_maps_best_from_its_history_from_arriving() {
    let (mut player, back) = back_on_sunset_sky_after_a_best_of_7200();
    assert_eq!(best_row(&back), Some(7200));

    // No 5 minutes to wait: it shows all along, at whatever rate.
    assert_eq!(best_row(&player.hunt(30 * S, 10)), Some(7200));
    let five = player.hunt(5 * MIN, 10);
    assert_eq!(rolling(&five), Some(3600));
    assert_eq!((best_row(&five), starred(&five)), (Some(7200), false));
}

#[test]
fn the_star_marks_last_5_min_while_it_beats_the_best_over_five_minutes_here() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();

    // 10,800 EXP/h: under 5 minutes here, the Rolling Rate isn't all Sunset Sky's yet, so
    // it is no best here, however high.
    let under = player.hunt(4 * MIN + 30 * S, 30);
    assert_eq!(rolling(&under), Some(10_800));
    assert!(!starred(&under));

    // 5 minutes here: a new best on the map.
    let five = player.hunt(30 * S, 30);
    assert_eq!((rolling(&five), best_row(&five), starred(&five)), (Some(10_800), Some(7200), true));

    // Back to the best, and below it: the star goes.
    let level = player.hunt(5 * MIN, 20);
    assert_eq!((rolling(&level), starred(&level)), (Some(7200), false));
    let below = player.hunt(5 * MIN, 10);
    assert_eq!((rolling(&below), starred(&below)), (Some(3600), false));

    // Above it again, it comes back.
    let again = player.hunt(5 * MIN, 30);
    assert_eq!((rolling(&again), starred(&again)), (Some(10_800), true));
}

#[test]
fn the_star_compares_the_rates_as_the_grind_tracker_writes_them() {
    // Sunset Sky's best: 10,800 EXP/h on a first Visit.
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 30);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(2 * MIN, 30);
    assert_eq!(best_row(&player.arrive(SUNSET)), Some(10_800));

    // 3 EXP more in 5 minutes: 10,836 EXP/h is more, but reads 10,800 as the best does.
    player.hunt(4 * MIN + 30 * S, 30);
    let same = player.hunt(30 * S, 31);
    assert_eq!((rolling(&same), starred(&same)), (Some(10_836), false));

    // 11,016 EXP/h reads 11,000: a new best.
    let more = player.hunt(30 * S, 35);
    assert_eq!((rolling(&more), starred(&more)), (Some(11_016), true));
}

#[test]
fn the_best_row_holds_all_visit_and_a_best_set_in_it_shows_from_the_next() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();

    // 10,800 EXP/h, then 9,000: both beat the 7,200 from earlier Visits, which the row
    // shows all Visit, never the 10,800 set in it.
    player.hunt(5 * MIN, 30);
    let after = player.hunt(5 * MIN, 25);
    assert_eq!((rolling(&after), best_row(&after), starred(&after)), (Some(9000), Some(7200), true));

    // The next Visit shows it: 9,000 EXP/h is no new best there.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 30);
    assert_eq!(best_row(&player.arrive(SUNSET)), Some(10_800));
    let five = player.hunt(5 * MIN, 25);
    assert_eq!((rolling(&five), starred(&five)), (Some(9000), false));
}

#[test]
fn nothing_is_recorded_on_a_map_without_a_map_summary() {
    let mut player = Player::new(tracking());

    // A town has none: no best there, however long the play.
    player.arrive(KERNING);
    assert_eq!(best_row(&player.hunt(6 * MIN, 30)), None);
    assert!(player.minerva.bests().unassigned.is_empty());

    // Sunset Sky has one: its best from 5 minutes there.
    player.arrive(SUNSET);
    player.hunt(5 * MIN, 10);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [3600]);
}

fn shown_map(state: &PanelState) -> Option<&str> {
    state.map.as_ref().map(|m| m.id.as_str())
}

#[test]
fn nothing_is_recorded_while_candidates_wait_for_a_choice() {
    let mut player = Player::new(tracking_with(sharing_a_title()));
    let arrived = player.arrive(KERNING);
    assert_eq!(shown_map(&arrived), Some(CONSTRUCTION_SITE_ID));
    assert_eq!(arrived.candidates.map(|c| c.chosen), Some(false));

    // 10,800 EXP/h, then 3,600, with nothing chosen: no best.
    player.hunt(6 * MIN, 30);
    player.hunt(5 * MIN, 10);
    assert!(player.minerva.bests().unassigned.is_empty());

    // The player chooses the Construction Site: the hotkey shows Sunset Sky, then it.
    player.choose_next();
    let chosen = player.choose_next();
    assert_eq!(shown_map(&chosen), Some(CONSTRUCTION_SITE_ID));
    // Its first best is the Rolling Rate as it shows now: the 10,800 shown before the
    // choice never counted.
    assert_eq!(saved_rates(&player, CONSTRUCTION_SITE_ID), [3600]);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [] as [u64; 0]);
}

#[test]
fn the_best_row_shows_only_once_the_player_has_chosen_among_candidates() {
    let mut player = Player::new(tracking_with(sharing_a_title()));
    // A first Visit, choosing the Construction Site past Sunset Sky: 3,600 EXP/h there.
    player.arrive(KERNING);
    player.hunt(6 * MIN, 10);
    player.choose_next();
    player.choose_next();

    // On Rocky Road I, whose page doesn't settle which map the title is on the way back.
    player.arrive(ROCKY_ROAD);
    player.hunt(MIN, 10);
    let back = player.arrive(KERNING);
    assert_eq!(back.candidates.as_ref().map(|c| c.chosen), Some(false));
    assert_eq!(shown_map(&back), Some(CONSTRUCTION_SITE_ID));
    // Waiting for a choice, the map isn't certain: no Best row, though the one shown has
    // a best.
    assert_eq!(best_row(&back), None);
    let waiting = player.hunt(6 * MIN, 30);
    assert_eq!((best_row(&waiting), starred(&waiting)), (None, false));

    // Sunset Sky has none: the best from the Visit is the Construction Site's...
    let sunset = player.choose_next();
    assert_eq!(shown_map(&sunset), Some(SUNSET_SKY_ID));
    assert_eq!(best_row(&sunset), None);
    // ...which shows its own.
    let site = player.choose_next();
    assert_eq!(shown_map(&site), Some(CONSTRUCTION_SITE_ID));
    assert_eq!(best_row(&site), Some(3600));
}

#[test]
fn nothing_is_recorded_while_the_exp_bar_cannot_be_read() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    player.hunt(4 * MIN, 30);

    // The bar goes unread past 5 minutes on the map: nothing is known of the EXP.
    let unreadable = player.unread(2 * MIN);
    assert_eq!(unreadable.exp_tracking.state, TrackingState::Unreadable);

    // Off to the Construction Site before it reads again, then back: nothing came of the
    // Visit.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 10);
    let back = player.arrive(SUNSET);
    assert_eq!(best_row(&back), None);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [] as [u64; 0]);
}

#[test]
fn a_best_is_last_5_min_as_shown_at_a_refresh_never_a_peak_between() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 10);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [3600]);

    // 10 s into a half minute, a burst of 310 EXP: the Rolling Rate would read 7,200
    // EXP/h (600 EXP in 5 minutes), but the figures shown hold till the half minute is
    // out.
    let burst = player.hunt(10 * S, 310);
    assert_eq!(rolling(&burst), Some(3600));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [3600]);

    // Nothing more till then: the Rolling Rate shows 6,960 EXP/h (580 EXP), and that is
    // the best, as long as the burst counts in it.
    let shown = player.hunt(20 * S, 0);
    assert_eq!(rolling(&shown), Some(6960));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [6960]);
    let later = player.hunt(5 * MIN, 10);
    assert_eq!(rolling(&later), Some(3600));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [6960]);
}

#[test]
fn a_session_reset_or_a_character_switch_leaves_the_bests_as_they_were() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    // A new best this Visit: 10,800 EXP/h.
    player.hunt(5 * MIN, 30);

    // The player starts the Session afresh: the best and this Visit's new one stay.
    player.minerva.press(PanelControl::ResetSession);
    let reset = player.minerva.panel_state().clone();
    assert_eq!(reset.exp_tracking.state, TrackingState::Waiting);
    assert_eq!(best_row(&reset), Some(7200));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [10_800, 7200]);
    // The new Session's 5 minutes are on this map too: 3,600 EXP/h, no new best.
    player.hunt(5 * MIN + 10 * S, 10);
    let five = player.hunt(20 * S, 10);
    assert_eq!((rolling(&five), best_row(&five), starred(&five)), (Some(3600), Some(7200), false));

    // Another character's bar, 157 [0.94%], read three times: a Session of their own.
    for _ in 0..3 {
        player.at += 10 * S;
        let bar = Some(ExpBar { exp: 157, percent_hundredths: 94 });
        player.minerva.on_exp_bar_reading(ExpBarReading { bar, at_ms: player.at, place: None });
    }
    let switched = player.minerva.panel_state().clone();
    assert!(switched.exp_tracking.character_switched);
    assert_eq!(best_row(&switched), Some(7200));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [10_800, 7200]);

    // This Visit's best joins Sunset Sky's when it ends, whoever plays.
    player.arrive(CONSTRUCTION_SITE);
    let back = player.arrive(SUNSET);
    assert_eq!(best_row(&back), Some(10_800));
}

#[test]
fn with_the_bar_unreadable_or_paused_the_best_row_and_star_hold_like_the_other_figures() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    let five = player.hunt(5 * MIN, 30);
    assert_eq!((best_row(&five), starred(&five)), (Some(7200), true));

    // The bar can't be read for 10 minutes: everything shown holds.
    let unreadable = player.unread(10 * MIN);
    assert_eq!(unreadable.exp_tracking.state, TrackingState::Unreadable);
    assert_eq!((rolling(&unreadable), best_row(&unreadable), starred(&unreadable)), (Some(10_800), Some(7200), true));

    // It reads again, unchanged: the Session is paused at the Rolling Rate's 10,800
    // EXP/h, and the star with it, however long the break.
    let paused = player.hunt(10 * S, 0);
    assert_eq!(paused.exp_tracking.state, TrackingState::Paused);
    assert_eq!((rolling(&paused), best_row(&paused), starred(&paused)), (Some(10_800), Some(7200), true));
    let later = player.hunt(10 * MIN, 0);
    assert_eq!(later.exp_tracking.state, TrackingState::Paused);
    assert_eq!((best_row(&later), starred(&later)), (Some(7200), true));
}

#[test]
fn the_overlay_gets_the_best_row_and_star_as_the_grind_tracker_renders_them() {
    // A first Visit: no Best row, no star.
    let mut first = Player::new(tracking());
    first.arrive(SUNSET);
    let five = serde_json::to_value(first.hunt(5 * MIN, 10)).unwrap();
    assert_eq!(five["expTracking"]["personalBest"], serde_json::Value::Null);
    assert_eq!(five["expTracking"]["newBest"], false);

    // Back after a best of 7,200, beaten.
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    let beaten = serde_json::to_value(player.hunt(5 * MIN, 30)).unwrap();
    assert_eq!(beaten["expTracking"]["personalBest"], 7200);
    assert_eq!(beaten["expTracking"]["newBest"], true);
}

/// Sunset Sky's best Visits: 10,800 EXP/h on the first, 7,200 on the second; then back on
/// it a third time.
fn back_on_sunset_sky_after_bests_of_10800_and_7200() -> (Player, PanelState) {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 30);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(2 * MIN, 30);
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(2 * MIN, 30);
    let back = player.arrive(SUNSET);
    (player, back)
}

#[test]
fn removing_the_best_takes_it_from_the_history_at_once_and_the_next_best_visit_takes_over() {
    let (mut player, back) = back_on_sunset_sky_after_bests_of_10800_and_7200();
    assert_eq!(best_row(&back), Some(10_800));

    // 9,000 EXP/h for 5 minutes: no new best on the 10,800.
    let five = player.hunt(5 * MIN, 25);
    assert_eq!((rolling(&five), starred(&five)), (Some(9000), false));

    // Removed: the 7,200 shows at once, and 9,000 beats it.
    let removed = player.press(PanelControl::RemoveBest);
    assert_eq!((best_row(&removed), starred(&removed)), (Some(7200), true));

    // That removed too: the map has no history, so no Best row and no star. This Visit's
    // best is left.
    let none = player.press(PanelControl::RemoveBest);
    assert_eq!((best_row(&none), starred(&none)), (None, false));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [9000]);
}

#[test]
fn removing_the_best_takes_the_maps_best_from_its_history_never_this_visits() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();

    // 10,800 EXP/h for 5 minutes beats the 7,200: removing takes the 7,200 all the same,
    // and this Visit's 10,800 is left to join the history when the Visit ends.
    player.hunt(5 * MIN, 30);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [10_800, 7200]);
    let removed = player.press(PanelControl::RemoveBest);
    assert_eq!((best_row(&removed), starred(&removed)), (None, false));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [10_800]);
}

#[test]
fn a_first_visits_best_cannot_be_removed_until_it_has_joined_the_history() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 30);

    // No history here: removing takes nothing.
    player.press(PanelControl::RemoveBest);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [10_800]);

    // Once the Visit has ended, it can go.
    player.arrive(CONSTRUCTION_SITE);
    assert_eq!(best_row(&player.arrive(SUNSET)), Some(10_800));
    let removed = player.press(PanelControl::RemoveBest);
    assert_eq!(best_row(&removed), None);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [] as [u64; 0]);
}

/// The Pace Trace's race with the Best row, once within reach: how far its pace line
/// rises, and how far the trace has come over the same stretch.
fn race(state: &PanelState) -> Option<(i64, i64)> {
    let pace = state.exp_tracking.pace.as_ref()?;
    let [from, to] = pace.best?;
    let last = pace.points.last()?;
    Some((to.exp - from.exp, last.exp - from.exp))
}

#[test]
fn the_pace_trace_shows_the_bests_pace_once_within_reach_for_the_rest_of_the_visit() {
    let (mut player, back) = back_on_sunset_sky_after_a_best_of_7200();
    // Arriving, the race on the map hasn't started: the trace races the 5 minutes before.
    let trace = back.exp_tracking.pace.clone().unwrap();
    assert_eq!((trace.best, trace.arrived_ago_ms), (None, Some(0)));

    // A minute at half the best's 7,200 EXP/h: out of reach.
    let slow = player.hunt(MIN, 10);
    assert_eq!(race(&slow), None);
    assert_eq!(slow.exp_tracking.pace.and_then(|p| p.arrived_ago_ms), Some(MIN));

    // Two more at 10,800: 420 EXP since arriving to the best's 360, so within reach.
    let close = player.hunt(2 * MIN, 30);
    assert_eq!(race(&close), Some((360, 420)));

    // It stays for the rest of the Visit, fallen back too: 444 EXP to its 600.
    let fallen = player.hunt(2 * MIN, 2);
    assert_eq!(race(&fallen), Some((600, 444)));

    // On the next Visit it is out of reach until it comes within reach there.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 30);
    assert_eq!(race(&player.arrive(SUNSET)), None);
    assert_eq!(race(&player.hunt(MIN, 10)), None);
}

#[test]
fn a_first_visit_races_only_the_five_minutes_before() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    let state = player.hunt(11 * MIN, 20);
    let trace = state.exp_tracking.pace.clone().unwrap();
    assert_eq!(trace.best, None);
    // 20 EXP every 10 s, as in the 5 minutes before: level with them.
    assert_eq!(trace.points.last().map(|p| p.exp), Some(600));
    assert_eq!(trace.earlier[1].exp, 600);
}
