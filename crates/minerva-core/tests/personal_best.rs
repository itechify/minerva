//! The Personal Best (GLOSSARY): the best Rolling Rate, in EXP/h, the player has had on
//! the map they're on. A Visit's best is the highest Rolling Rate shown at a refresh over
//! a full 5 minutes of active play on the map; the map's Personal Best is the best of its
//! Visits'. The Grind Tracker's Best row shows the one from earlier Visits, and a star
//! marks Last 5 min while it beats it.

mod common;

use common::player::Player;
use common::{CONSTRUCTION_SITE_ID, SUNSET_SKY_ID, sharing_a_title, tracking, tracking_with, SUNSET_SAMPLE, CONSTRUCTION_SAMPLE, KERNING_SAMPLE, S, MIN, best_row, shown_id};
use minerva_core::{ExpBar, ExpBarReading, PanelControl, PanelState, TrackingState, VisitBest};

const ROCKY_ROAD: &str = "victoria-road__rocky-road-i--title-region";

/// The Rolling Rate's EXP/h as the Grind Tracker shows it ("Last 5 min").
fn rolling(state: &PanelState) -> Option<i64> {
    state.exp_tracking.rolling.map(|r| r.exp_per_hour)
}

/// Whether the Best row's figure was set on this Visit, and is marked so.
fn this_visits(state: &PanelState) -> bool {
    state.exp_tracking.personal_best_this_visit
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
    player.arrive(SUNSET_SAMPLE);
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
fn a_first_visits_best_shows_on_the_best_row_from_five_minutes_of_play_there_marked_as_this_visits() {
    let mut player = Player::new(tracking());
    // 10,800 EXP/h on the Construction Site for 6 minutes, then on to Sunset Sky.
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(6 * MIN, 30);
    assert_eq!(best_row(&player.arrive(SUNSET_SAMPLE)), None);

    // 3,600 EXP/h there. Under 5 minutes in, the Rolling Rate still counts some of the
    // Construction Site's play: no best yet, so no Best row.
    let under = player.hunt(4 * MIN + 30 * S, 10);
    assert_eq!(rolling(&under), Some(4320));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [] as [u64; 0]);
    assert_eq!((best_row(&under), this_visits(&under)), (None, false));

    // 5 minutes there: the Rolling Rate is all Sunset Sky's, this Visit's first best, and
    // the Best row shows it at once, marked as this Visit's.
    let five = player.hunt(30 * S, 10);
    assert_eq!(rolling(&five), Some(3600));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [3600]);
    assert_eq!((best_row(&five), this_visits(&five)), (Some(3600), true));

    // Faster, the row rises with what the Rolling Rate shows, and stays when it falls back.
    let faster = player.hunt(5 * MIN, 20);
    assert_eq!((best_row(&faster), this_visits(&faster)), (Some(7200), true));
    let slower = player.hunt(5 * MIN, 10);
    assert_eq!((rolling(&slower), best_row(&slower), this_visits(&slower)), (Some(3600), Some(7200), true));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [7200]);
}

/// A first Visit to Sunset Sky at 7,200 EXP/h for 6 minutes, then 2 minutes on the
/// Construction Site at 10,800, and back to Sunset Sky.
fn back_on_sunset_sky_after_a_best_of_7200() -> (Player, PanelState) {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(2 * MIN, 30);
    let back = player.arrive(SUNSET_SAMPLE);
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
fn the_star_marks_last_5_min_while_it_is_the_best_this_visit_set_on_the_map() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();

    // 10,800 EXP/h: under 5 minutes here, the Rolling Rate isn't all Sunset Sky's yet, so
    // it is no best here, however high.
    let under = player.hunt(4 * MIN + 30 * S, 30);
    assert_eq!((rolling(&under), best_row(&under), starred(&under)), (Some(10_800), Some(7200), false));

    // 5 minutes here: a new best on the map, on the Best row at once, and Last 5 min is it.
    let five = player.hunt(30 * S, 30);
    assert_eq!((rolling(&five), best_row(&five), starred(&five)), (Some(10_800), Some(10_800), true));
    assert!(this_visits(&five));

    // Back to the earlier Visits' best, and below it: the star goes, and the row keeps
    // this Visit's best, to show where the player was.
    let level = player.hunt(5 * MIN, 20);
    assert_eq!((rolling(&level), best_row(&level), starred(&level)), (Some(7200), Some(10_800), false));
    let below = player.hunt(5 * MIN, 10);
    assert_eq!((rolling(&below), best_row(&below), starred(&below)), (Some(3600), Some(10_800), false));

    // Back at it, the star comes back: Last 5 min is the best again.
    let again = player.hunt(5 * MIN, 30);
    assert_eq!((rolling(&again), best_row(&again), starred(&again)), (Some(10_800), Some(10_800), true));
}

#[test]
fn a_first_visits_star_marks_last_5_min_while_it_is_the_visits_best() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET_SAMPLE);
    let five = player.hunt(5 * MIN, 20);
    assert_eq!((rolling(&five), best_row(&five), starred(&five)), (Some(7200), Some(7200), true));
    let slower = player.hunt(5 * MIN, 10);
    assert_eq!((rolling(&slower), best_row(&slower), starred(&slower)), (Some(3600), Some(7200), false));
}

#[test]
fn this_visits_best_takes_the_row_once_it_reads_higher_as_the_grind_tracker_writes_them() {
    // Sunset Sky's best: 10,800 EXP/h on a first Visit.
    let mut player = Player::new(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 30);
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(2 * MIN, 30);
    assert_eq!(best_row(&player.arrive(SUNSET_SAMPLE)), Some(10_800));

    // 3 EXP more in 5 minutes: 10,836 EXP/h is more, but reads 10,800 as the best does.
    // The tie stays the earlier Visit's: no mark, and no star.
    player.hunt(4 * MIN + 30 * S, 30);
    let same = player.hunt(30 * S, 31);
    assert_eq!((rolling(&same), best_row(&same)), (Some(10_836), Some(10_800)));
    assert_eq!((this_visits(&same), starred(&same)), (false, false));

    // 11,016 EXP/h reads 11,000: a new best, this Visit's.
    let more = player.hunt(30 * S, 35);
    assert_eq!((rolling(&more), best_row(&more)), (Some(11_016), Some(11_016)));
    assert_eq!((this_visits(&more), starred(&more)), (true, true));
}

#[test]
fn a_best_set_this_visit_is_an_earlier_visits_on_the_next() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    player.hunt(5 * MIN, 30);
    let after = player.hunt(5 * MIN, 25);
    assert_eq!((rolling(&after), best_row(&after), this_visits(&after)), (Some(9000), Some(10_800), true));

    // The next Visit shows it from arriving, as the earlier Visits' best: no mark.
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(MIN, 30);
    let back = player.arrive(SUNSET_SAMPLE);
    assert_eq!((best_row(&back), this_visits(&back)), (Some(10_800), false));
    let five = player.hunt(5 * MIN, 25);
    assert_eq!((rolling(&five), starred(&five), this_visits(&five)), (Some(9000), false, false));
}

#[test]
fn nothing_is_recorded_on_a_map_without_a_map_summary() {
    let mut player = Player::new(tracking());

    // A town has none: no best there, however long the play.
    player.arrive(KERNING_SAMPLE);
    assert_eq!(best_row(&player.hunt(6 * MIN, 30)), None);
    assert!(player.minerva.bests().unassigned.is_empty());

    // Sunset Sky has one: its best from 5 minutes there.
    player.arrive(SUNSET_SAMPLE);
    player.hunt(5 * MIN, 10);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [3600]);
}

#[test]
fn nothing_is_recorded_while_candidates_wait_for_a_choice() {
    let mut player = Player::new(tracking_with(sharing_a_title()));
    let arrived = player.arrive(KERNING_SAMPLE);
    assert_eq!(shown_id(&arrived), Some(CONSTRUCTION_SITE_ID));
    assert_eq!(arrived.candidates.map(|c| c.chosen), Some(false));

    // 10,800 EXP/h, then 3,600, with nothing chosen: no best.
    player.hunt(6 * MIN, 30);
    player.hunt(5 * MIN, 10);
    assert!(player.minerva.bests().unassigned.is_empty());

    // The player chooses the Construction Site: the hotkey shows Sunset Sky, then it.
    player.choose_next();
    let chosen = player.choose_next();
    assert_eq!(shown_id(&chosen), Some(CONSTRUCTION_SITE_ID));
    // Its first best is the Rolling Rate as it shows now: the 10,800 shown before the
    // choice never counted.
    assert_eq!(saved_rates(&player, CONSTRUCTION_SITE_ID), [3600]);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [] as [u64; 0]);
}

#[test]
fn the_best_row_shows_only_once_the_player_has_chosen_among_candidates() {
    let mut player = Player::new(tracking_with(sharing_a_title()));
    // A first Visit, choosing the Construction Site past Sunset Sky: 3,600 EXP/h there.
    player.arrive(KERNING_SAMPLE);
    player.hunt(6 * MIN, 10);
    player.choose_next();
    player.choose_next();

    // On Rocky Road I, whose page doesn't settle which map the title is on the way back.
    player.arrive(ROCKY_ROAD);
    player.hunt(MIN, 10);
    let back = player.arrive(KERNING_SAMPLE);
    assert_eq!(back.candidates.as_ref().map(|c| c.chosen), Some(false));
    assert_eq!(shown_id(&back), Some(CONSTRUCTION_SITE_ID));
    // Waiting for a choice, the map isn't certain: no Best row, though the one shown has
    // a best.
    assert_eq!(best_row(&back), None);
    let waiting = player.hunt(6 * MIN, 30);
    assert_eq!((best_row(&waiting), starred(&waiting)), (None, false));

    // Chosen, the map is certain: the Rolling Rate shown now, 10,800 EXP/h over 5 minutes
    // of the Visit, is Sunset Sky's first best, on its row at once...
    let sunset = player.choose_next();
    assert_eq!(shown_id(&sunset), Some(SUNSET_SKY_ID));
    assert_eq!((best_row(&sunset), this_visits(&sunset)), (Some(10_800), true));
    // ...and the Construction Site's, chosen instead, beating its 3,600 from the first Visit.
    let site = player.choose_next();
    assert_eq!(shown_id(&site), Some(CONSTRUCTION_SITE_ID));
    assert_eq!((best_row(&site), this_visits(&site)), (Some(10_800), true));
    assert_eq!(saved_rates(&player, CONSTRUCTION_SITE_ID), [10_800, 3600]);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [] as [u64; 0]);
}

#[test]
fn nothing_is_recorded_while_the_exp_bar_cannot_be_read() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(4 * MIN, 30);

    // The bar goes unread past 5 minutes on the map: nothing is known of the EXP.
    let unreadable = player.unread(2 * MIN);
    assert_eq!(unreadable.exp_tracking.state, TrackingState::Unreadable);

    // Off to the Construction Site before it reads again, then back: nothing came of the
    // Visit.
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(MIN, 10);
    let back = player.arrive(SUNSET_SAMPLE);
    assert_eq!(best_row(&back), None);
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [] as [u64; 0]);
}

#[test]
fn a_best_is_last_5_min_as_shown_at_a_refresh_never_a_peak_between() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET_SAMPLE);
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
    assert_eq!((best_row(&reset), this_visits(&reset)), (Some(10_800), true));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [10_800, 7200]);
    // The new Session's 5 minutes are on this map too: 3,600 EXP/h, no new best.
    player.hunt(5 * MIN + 10 * S, 10);
    let five = player.hunt(20 * S, 10);
    assert_eq!((rolling(&five), best_row(&five), starred(&five)), (Some(3600), Some(10_800), false));

    // Another character's bar, 157 [0.94%], read three times: a Session of their own.
    for _ in 0..3 {
        player.at += 10 * S;
        let bar = Some(ExpBar { exp: 157, percent_hundredths: 94 });
        player.minerva.on_exp_bar_reading(ExpBarReading { bar, at_ms: player.at, place: None });
    }
    let switched = player.minerva.panel_state().clone();
    assert!(switched.exp_tracking.character_switched);
    assert_eq!((best_row(&switched), this_visits(&switched)), (Some(10_800), true));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [10_800, 7200]);

    // This Visit's best joins Sunset Sky's when it ends, whoever plays.
    player.arrive(CONSTRUCTION_SAMPLE);
    let back = player.arrive(SUNSET_SAMPLE);
    assert_eq!((best_row(&back), this_visits(&back)), (Some(10_800), false));
}

#[test]
fn with_the_bar_unreadable_or_paused_the_best_row_and_star_hold_like_the_other_figures() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    let five = player.hunt(5 * MIN, 30);
    assert_eq!((best_row(&five), starred(&five)), (Some(10_800), true));

    // The bar can't be read for 10 minutes: everything shown holds.
    let unreadable = player.unread(10 * MIN);
    assert_eq!(unreadable.exp_tracking.state, TrackingState::Unreadable);
    assert_eq!((rolling(&unreadable), best_row(&unreadable), starred(&unreadable)), (Some(10_800), Some(10_800), true));

    // It reads again, unchanged: the Session is paused at the Rolling Rate's 10,800
    // EXP/h, and the star with it, however long the break.
    let paused = player.hunt(10 * S, 0);
    assert_eq!(paused.exp_tracking.state, TrackingState::Paused);
    assert_eq!((rolling(&paused), best_row(&paused), starred(&paused)), (Some(10_800), Some(10_800), true));
    let later = player.hunt(10 * MIN, 0);
    assert_eq!(later.exp_tracking.state, TrackingState::Paused);
    assert_eq!((best_row(&later), starred(&later)), (Some(10_800), true));
}

#[test]
fn the_overlay_gets_the_best_row_and_star_as_the_grind_tracker_renders_them() {
    // A first Visit, under 5 minutes: no Best row, no star.
    let mut first = Player::new(tracking());
    first.arrive(SUNSET_SAMPLE);
    let under = serde_json::to_value(first.hunt(4 * MIN, 10)).unwrap();
    assert_eq!(under["expTracking"]["personalBest"], serde_json::Value::Null);
    assert_eq!(under["expTracking"]["personalBestThisVisit"], false);
    assert_eq!(under["expTracking"]["newBest"], false);

    // Back after a best of 7,200, from arriving: the earlier Visits'.
    let (mut player, back) = back_on_sunset_sky_after_a_best_of_7200();
    let back = serde_json::to_value(back).unwrap();
    assert_eq!(back["expTracking"]["personalBest"], 7200);
    assert_eq!(back["expTracking"]["personalBestThisVisit"], false);

    // Beaten: this Visit's, and Last 5 min is it.
    let beaten = serde_json::to_value(player.hunt(5 * MIN, 30)).unwrap();
    assert_eq!(beaten["expTracking"]["personalBest"], 10_800);
    assert_eq!(beaten["expTracking"]["personalBestThisVisit"], true);
    assert_eq!(beaten["expTracking"]["newBest"], true);
}

/// Sunset Sky's best Visits: 10,800 EXP/h on the first, 7,200 on the second; then back on
/// it a third time.
fn back_on_sunset_sky_after_bests_of_10800_and_7200() -> (Player, PanelState) {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 30);
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(2 * MIN, 30);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(2 * MIN, 30);
    let back = player.arrive(SUNSET_SAMPLE);
    (player, back)
}

#[test]
fn removing_the_earlier_visits_best_lets_the_next_best_take_the_row_this_visits_too() {
    let (mut player, back) = back_on_sunset_sky_after_bests_of_10800_and_7200();
    assert_eq!(best_row(&back), Some(10_800));

    // 9,000 EXP/h for 5 minutes: no new best on the 10,800.
    let five = player.hunt(5 * MIN, 25);
    assert_eq!((rolling(&five), best_row(&five), starred(&five)), (Some(9000), Some(10_800), false));

    // Removed: the next best is this Visit's 9,000 over the earlier 7,200, and Last 5 min is it.
    let removed = player.press(PanelControl::RemoveBest);
    assert_eq!((best_row(&removed), this_visits(&removed), starred(&removed)), (Some(9000), true, true));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [9000, 7200]);
}

#[test]
fn removing_this_visits_best_falls_back_to_the_earlier_visits_and_counts_only_play_since() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();

    // 10,800 EXP/h for 5 minutes, from a misread say: this Visit's best, on the row.
    let five = player.hunt(5 * MIN, 30);
    assert_eq!((best_row(&five), this_visits(&five)), (Some(10_800), true));

    // Removed at once: the earlier Visits' 7,200 takes the row back, with no star.
    let removed = player.press(PanelControl::RemoveBest);
    assert_eq!((best_row(&removed), this_visits(&removed), starred(&removed)), (Some(7200), false, false));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [7200]);

    // Last 5 min still holds the play before the removal: it isn't counted again.
    let still = player.hunt(4 * MIN + 30 * S, 30);
    assert_eq!((rolling(&still), best_row(&still), starred(&still)), (Some(10_800), Some(7200), false));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [7200]);

    // 5 minutes all since the removal count, and set this Visit's best afresh.
    let since = player.hunt(30 * S, 30);
    assert_eq!((best_row(&since), this_visits(&since), starred(&since)), (Some(10_800), true, true));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [10_800, 7200]);
}

#[test]
fn this_visits_best_removed_after_a_reset_isnt_counted_again_when_the_reset_is_undone() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    player.hunt(5 * MIN, 30);

    // Reset, then this Visit's 10,800 removed while the Session waits for its Undo.
    player.press(PanelControl::ResetSession);
    let removed = player.press(PanelControl::RemoveBest);
    assert_eq!((best_row(&removed), this_visits(&removed)), (Some(7200), false));

    // Undone: Last 5 min is the 10,800 again, all play before the removal, so no best.
    let undone = player.press(PanelControl::UndoReset);
    assert_eq!((rolling(&undone), best_row(&undone), starred(&undone)), (Some(10_800), Some(7200), false));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [7200]);
}

#[test]
fn a_first_visits_best_can_be_removed_at_once() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 30);

    // With no history, the row goes with it, and nothing joins the history when the Visit ends.
    let removed = player.press(PanelControl::RemoveBest);
    assert_eq!((best_row(&removed), starred(&removed)), (None, false));
    assert_eq!(saved_rates(&player, SUNSET_SKY_ID), [] as [u64; 0]);
    player.arrive(CONSTRUCTION_SAMPLE);
    assert_eq!(best_row(&player.arrive(SUNSET_SAMPLE)), None);
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
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(MIN, 30);
    assert_eq!(race(&player.arrive(SUNSET_SAMPLE)), None);
    assert_eq!(race(&player.hunt(MIN, 10)), None);
}

#[test]
fn a_first_visit_races_its_own_best_and_keeps_it_when_the_pace_falls_back() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET_SAMPLE);
    let state = player.hunt(11 * MIN, 20);
    let trace = state.exp_tracking.pace.clone().unwrap();
    // 20 EXP every 10 s, as in the 5 minutes before: level with them, and with this
    // Visit's best of 7,200 EXP/h, set at that pace.
    assert_eq!(trace.points.last().map(|p| p.exp), Some(600));
    assert_eq!(trace.earlier[1].exp, 600);
    assert_eq!(race(&state), Some((600, 600)));

    // Half that pace for 5 minutes: the best's line shows where the player was.
    let slower = player.hunt(5 * MIN, 10);
    assert_eq!((best_row(&slower), race(&slower)), (Some(7200), Some((600, 300))));
}
