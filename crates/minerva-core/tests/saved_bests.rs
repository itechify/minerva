//! Personal Bests outlive the launch: the core hands the shell every map's Visit bests
//! as plain data, the Visit under way's included, and takes them back at the next
//! launch, the same way as the settings. Closing Minerva ends the Visit.

mod common;

use common::player::{Player, WALL_AT_START};
use common::{
    CONSTRUCTION_SITE_ID, NORTHERN_TOP_ID, SUNSET_SKY_ID, sharing_a_title, three_sharing_a_title, tracking,
    tracking_with,
};
use minerva_core::{BESTS_VERSION, Minerva, ExpBar, ExpBarReading, PanelControl, PanelState, SavedBests, VisitBest};

const S: u64 = 1000;
const MIN: u64 = 60 * S;
const DAY: u64 = 24 * 60 * MIN;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const ROCKY_ROAD: &str = "victoria-road__rocky-road-i--title-region";

/// The bests a launch hands out, as the shell would save them (JSON).
fn handed_out(minerva: &Minerva) -> SavedBests {
    let json = serde_json::to_string(&minerva.bests()).unwrap();
    serde_json::from_str(&json).unwrap()
}

/// The first launch, its wall clock at [`WALL_AT_START`] at capture time 0.
fn first_launch() -> Player {
    Player::clocked(tracking())
}

/// The next launch, its shell restoring `bests` at startup, its wall clock at
/// `wall_at_start` at capture time 0: the player's Session under way, on a map the
/// panel doesn't know.
fn relaunch(bests: SavedBests, wall_at_start: u64) -> Player {
    let mut minerva = tracking();
    minerva.restore_bests(bests);
    Player::clocked_at(minerva, wall_at_start)
}

/// The Best row's EXP/h: the map's Personal Best from earlier Visits.
fn best_row(state: &PanelState) -> Option<u64> {
    state.exp_tracking.personal_best
}

/// `map`'s Visit bests in what `minerva` hands out, highest first.
fn saved(minerva: &Minerva, map: &str) -> Vec<VisitBest> {
    handed_out(minerva).unassigned.into_iter().find(|m| m.id == map).map_or_else(Vec::new, |m| m.visits)
}

/// A best of `exp_per_hour`, set at `set_at` by the wall clock.
fn best(exp_per_hour: u64, set_at: Option<u64>) -> VisitBest {
    VisitBest { exp_per_hour, set_at_unix_ms: set_at, ..VisitBest::default() }
}

#[test]
fn bests_handed_out_by_one_launch_show_the_same_best_row_and_star_in_the_next() {
    // A first Visit to Sunset Sky at 7,200 EXP/h, its best set 5 minutes in; 2 minutes on
    // the Construction Site, and back.
    let mut first = first_launch();
    first.arrive(SUNSET);
    first.hunt(5 * MIN, 20);
    let set = first.wall_clock();
    first.hunt(MIN, 20);
    first.arrive(CONSTRUCTION_SITE);
    first.hunt(2 * MIN, 30);
    let back = first.arrive(SUNSET);
    assert_eq!(best_row(&back), Some(7200));
    let saved_then = handed_out(&first.minerva);
    // 5 minutes at 10,800 EXP/h: a new best on it.
    let five = first.hunt(5 * MIN, 30);
    assert!(five.exp_tracking.new_best);

    // A day later, the next launch shows the same, and keeps when the best was set.
    let mut next = relaunch(saved_then, WALL_AT_START + DAY);
    assert_eq!(best_row(&next.arrive(SUNSET)), best_row(&back));
    let next_five = next.hunt(5 * MIN, 30);
    assert_eq!((best_row(&next_five), next_five.exp_tracking.new_best), (Some(10_800), true));
    assert_eq!(saved(&next.minerva, SUNSET_SKY_ID)[1], best(7200, set));
}

#[test]
fn a_visit_cut_short_by_closing_minerva_keeps_its_best() {
    // A first Visit to Sunset Sky at 7,200 EXP/h, its best set 5 minutes in, closed a
    // minute later.
    let mut first = first_launch();
    first.arrive(SUNSET);
    first.hunt(5 * MIN, 20);
    let set = first.wall_clock();
    first.hunt(MIN, 20);

    // Closing ended the Visit: the next launch has its best as the earlier Visits'.
    let mut next = relaunch(handed_out(&first.minerva), WALL_AT_START + DAY);
    assert_eq!(best_row(&next.arrive(SUNSET)), Some(7200));
    assert_eq!(saved(&next.minerva, SUNSET_SKY_ID), [best(7200, set)]);

    // 10,800 EXP/h beats it, and that Visit is closed in turn: it is the best after.
    next.hunt(5 * MIN, 30);
    let beaten = next.wall_clock();
    let mut third = relaunch(handed_out(&next.minerva), WALL_AT_START + 2 * DAY);
    assert_eq!(best_row(&third.arrive(SUNSET)), Some(10_800));
    assert_eq!(saved(&third.minerva, SUNSET_SKY_ID), [best(10_800, beaten), best(7200, set)]);
}

/// A first Visit to Sunset Sky at 7,200 EXP/h for 6 minutes, closed then: the bests the
/// launch hands out.
fn a_best_of_7200_on_sunset_sky() -> SavedBests {
    let mut first = first_launch();
    first.arrive(SUNSET);
    first.hunt(6 * MIN, 20);
    handed_out(&first.minerva)
}

#[test]
fn bests_saved_in_another_shape_are_left_and_the_app_starts_with_none() {
    let other_shape = SavedBests { version: BESTS_VERSION + 1, ..a_best_of_7200_on_sunset_sky() };

    let mut next = relaunch(other_shape, WALL_AT_START + DAY);
    assert_eq!(best_row(&next.arrive(SUNSET)), None);
    // 5 minutes at 3,600 EXP/h: a first Visit's best.
    next.hunt(5 * MIN, 10);
    assert_eq!(saved(&next.minerva, SUNSET_SKY_ID), [best(3600, next.wall_clock())]);
}

#[test]
fn bests_of_no_exp_in_a_hand_edited_file_are_left_out() {
    // The file edited by hand: Sunset Sky's only best, and the Construction Site's
    // second, are 0 EXP/h.
    let edited = format!(
        r#"{{"version": {BESTS_VERSION}, "maps": [
            {{"id": "{SUNSET_SKY_ID}", "visits": [{{"expPerHour": 0, "setAtUnixMs": null}}]}},
            {{"id": "{CONSTRUCTION_SITE_ID}", "visits": [
                {{"expPerHour": 7200, "setAtUnixMs": {WALL_AT_START}}},
                {{"expPerHour": 0, "setAtUnixMs": {WALL_AT_START}}}
            ]}}
        ]}}"#
    );
    let mut next = relaunch(serde_json::from_str(&edited).unwrap(), WALL_AT_START + DAY);
    let handed = handed_out(&next.minerva);
    assert!(handed.unassigned.iter().all(|m| m.id != SUNSET_SKY_ID));
    assert_eq!(saved_rates(&handed, CONSTRUCTION_SITE_ID), [7200]);

    // Sunset Sky has no best: a first Visit, with its own best from 5 minutes there.
    assert_eq!(best_row(&next.arrive(SUNSET)), None);
    next.hunt(5 * MIN, 10);
    assert_eq!(saved(&next.minerva, SUNSET_SKY_ID), [best(3600, next.wall_clock())]);

    // The Construction Site's best is the 7,200; removing it leaves the map no history,
    // and this Visit's 3,600 on the row and to save.
    assert_eq!(best_row(&next.arrive(CONSTRUCTION_SITE)), Some(7200));
    next.hunt(5 * MIN, 10);
    let set = next.wall_clock();
    assert_eq!(best_row(&next.press(PanelControl::RemoveBest)), Some(3600));
    assert_eq!(saved(&next.minerva, CONSTRUCTION_SITE_ID), [best(3600, set)]);
}

/// The EXP/h of each Visit's best a map's saved bests hold, in their order.
fn saved_rates(bests: &SavedBests, map: &str) -> Vec<u64> {
    let map = bests.unassigned.iter().find(|m| m.id == map);
    map.map_or_else(Vec::new, |m| m.visits.iter().map(|v| v.exp_per_hour).collect())
}

#[test]
fn each_map_keeps_only_its_10_best_visits() {
    // 11 Visits to Sunset Sky, 5 minutes each, at 3,600 to 7,200 EXP/h in steps of 360
    // (the slowest the third), each but the last ended by stepping onto the Construction
    // Site. The last is under way when the bests are handed out.
    let mut first = Player::saved_by_the_shell(tracking());
    for (i, gain) in [15, 11, 10, 20, 18, 12, 19, 13, 17, 14, 16].into_iter().enumerate() {
        if i > 0 {
            first.arrive(CONSTRUCTION_SITE);
        }
        first.arrive(SUNSET);
        first.hunt(5 * MIN, gain);
    }

    // The slowest Visit didn't make the 10, highest first.
    let kept: Vec<u64> = (11..=20).rev().map(|gain| gain * 360).collect();
    assert_eq!(saved_rates(&handed_out(&first.minerva), SUNSET_SKY_ID), kept);

    // A twelfth Visit at 3,240 EXP/h doesn't make them either, ended or not, so the
    // shell has nothing to save.
    first.arrive(CONSTRUCTION_SITE);
    first.arrive(SUNSET);
    let saves = first.saves();
    first.hunt(5 * MIN, 9);
    assert_eq!(saved_rates(&handed_out(&first.minerva), SUNSET_SKY_ID), kept);
    first.arrive(CONSTRUCTION_SITE);
    assert_eq!(saved_rates(&handed_out(&first.minerva), SUNSET_SKY_ID), kept);
    assert_eq!(first.saves(), saves);
}

#[test]
fn the_next_launch_shows_the_best_row_on_arriving_before_the_sessions_first_gain() {
    let mut minerva = tracking();
    minerva.restore_bests(a_best_of_7200_on_sunset_sky());
    // The bar read twice, the same: nothing gained yet.
    for at_ms in [0, 10 * S] {
        let bar = Some(ExpBar { exp: 1000, percent_hundredths: 500 });
        minerva.on_exp_bar_reading(ExpBarReading { bar, at_ms, place: None });
    }

    common::arrive(&mut minerva, SUNSET, 20 * S);

    // What the Grind Tracker gets: the Best row while the Session waits for EXP.
    let state = serde_json::to_value(minerva.panel_state()).unwrap();
    assert_eq!(state["expTracking"]["state"], "waiting");
    assert_eq!(state["expTracking"]["personalBest"], 7200);
}

#[test]
fn with_no_bests_saved_the_app_starts_with_none() {
    let mut next = relaunch(SavedBests::default(), WALL_AT_START + DAY);
    assert_eq!(best_row(&next.arrive(SUNSET)), None);
}

#[test]
fn the_shell_saves_the_bests_whenever_they_change_and_only_then() {
    let mut player = Player::saved_by_the_shell(tracking_with(sharing_a_title()));

    // 10,800 EXP/h while the Candidates wait for a choice: no best, nothing to save.
    player.arrive(KERNING);
    player.hunt(6 * MIN, 30);
    assert_eq!(player.saves(), 0);

    // The hotkey shows Sunset Sky, which takes this Visit's best; on to the Construction
    // Site, which takes it from Sunset Sky.
    player.choose_next();
    assert_eq!(player.saves(), 1);
    let site = player.choose_next();
    assert_eq!(site.map.map(|m| m.id), Some(CONSTRUCTION_SITE_ID.to_string()));
    assert_eq!(player.saves(), 2);

    // The same rate again is no new best; 14,400 EXP/h raises it at refreshes.
    player.hunt(2 * MIN, 30);
    assert_eq!(player.saves(), 2);
    player.hunt(5 * MIN, 40);
    let raised = player.saves();
    assert!(raised > 2);

    // A reset, the bar unread, leaving the map, and play on a map without a Map Summary
    // change no best.
    player.press(PanelControl::ResetSession);
    player.unread(MIN);
    player.arrive(ROCKY_ROAD);
    player.hunt(6 * MIN, 30);
    assert_eq!(player.saves(), raised);

    // Back, choosing the Construction Site past Sunset Sky: a slower Visit's best joins
    // its others.
    player.arrive(KERNING);
    player.choose_next();
    player.choose_next();
    player.hunt(6 * MIN, 20);
    let joined = player.saves();
    assert!(joined > raised);
    let site_rates = |player: &Player| saved_rates(&player.minerva.bests(), CONSTRUCTION_SITE_ID);
    assert_eq!(site_rates(&player), [14_400, 7200]);

    // With no rate since a reset, choosing Sunset Sky gives it no best of its own: the
    // Visit's best stays the Construction Site's, so closing Minerva now keeps it,
    // and it joins the Construction Site's others when the Visit ends there.
    player.press(PanelControl::ResetSession);
    let sunset = player.choose_next();
    assert_eq!(sunset.map.map(|m| m.id), Some(SUNSET_SKY_ID.to_string()));
    assert_eq!((player.saves(), site_rates(&player)), (joined, vec![14_400, 7200]));
    player.arrive(ROCKY_ROAD);
    assert_eq!((player.saves(), site_rates(&player)), (joined, vec![14_400, 7200]));
}

fn shown_map(state: &PanelState) -> Option<&str> {
    state.map.as_ref().map(|m| m.id.as_str())
}

#[test]
fn a_candidate_chosen_again_with_no_rate_removes_nothing_and_leaves_the_visits_best_where_it_counted() {
    let mut player = Player::saved_by_the_shell(tracking_with(sharing_a_title()));
    let site_rates = |player: &Player| saved_rates(&player.minerva.bests(), CONSTRUCTION_SITE_ID);
    let sunset_rates = |player: &Player| saved_rates(&player.minerva.bests(), SUNSET_SKY_ID);

    // 10,800 EXP/h with the Candidates waiting, then the hotkey shows Sunset Sky, which
    // takes it as this Visit's best.
    player.arrive(KERNING);
    player.hunt(6 * MIN, 30);
    player.choose_next();
    // 3,600 EXP/h, and on to the Construction Site, which takes the Visit's best.
    player.hunt(5 * MIN, 10);
    player.choose_next();
    let set = player.wall_clock();
    assert_eq!((sunset_rates(&player), site_rates(&player)), (vec![], vec![3600]));

    // With no rate since a reset, back on Sunset Sky: it had a best this Visit, but the
    // Visit's best stays the Construction Site's, and Sunset Sky has no history to show...
    player.press(PanelControl::ResetSession);
    let saves = player.saves();
    let sunset = player.choose_next();
    assert_eq!(shown_map(&sunset), Some(SUNSET_SKY_ID));
    assert_eq!(best_row(&sunset), None);
    assert_eq!((sunset_rates(&player), site_rates(&player)), (vec![], vec![3600]));

    // ...and removing there takes nothing from what the shell saves.
    player.press(PanelControl::RemoveBest);
    assert_eq!(player.saves(), saves);
    assert_eq!((sunset_rates(&player), site_rates(&player)), (vec![], vec![3600]));

    // The Visit ends with the Construction Site's 3,600 joining its bests, as it was set.
    player.arrive(ROCKY_ROAD);
    assert_eq!(saved(&player.minerva, CONSTRUCTION_SITE_ID), [best(3600, set)]);
    assert_eq!(player.saves(), saves);
}

#[test]
fn the_hotkey_passing_a_candidate_that_had_a_best_this_visit_leaves_the_visits_best_where_it_counted() {
    let mut player = Player::saved_by_the_shell(tracking_with(three_sharing_a_title()));
    let rates = |player: &Player, map| saved_rates(&player.minerva.bests(), map);

    // Three Candidates: the Northern Top shows first, waiting for a choice.
    let arrived = player.arrive(KERNING);
    assert_eq!(shown_map(&arrived), Some(NORTHERN_TOP_ID));
    assert_eq!(arrived.candidates.as_ref().map(|c| (c.maps.len(), c.chosen)), Some((3, false)));
    assert!(arrived.data.solo_exp_ceiling().is_some());

    // 10,800 EXP/h, then the player chooses the Construction Site, and the hotkey goes on
    // to Sunset Sky, which takes the Visit's best: 10,800 there.
    player.hunt(6 * MIN, 30);
    assert_eq!(shown_map(&player.choose_next()), Some(CONSTRUCTION_SITE_ID));
    assert_eq!(shown_map(&player.choose_next()), Some(SUNSET_SKY_ID));
    assert_eq!((rates(&player, CONSTRUCTION_SITE_ID), rates(&player, SUNSET_SKY_ID)), (vec![], vec![10_800]));

    // After a reset, the hotkey goes past the Northern Top back to the Construction Site,
    // and 21,600 EXP/h for 6 minutes there: the Visit's best is the Construction Site's.
    player.press(PanelControl::ResetSession);
    player.choose_next();
    assert_eq!(shown_map(&player.choose_next()), Some(CONSTRUCTION_SITE_ID));
    player.hunt(6 * MIN, 60);
    assert_eq!((rates(&player, CONSTRUCTION_SITE_ID), rates(&player, SUNSET_SKY_ID)), (vec![21_600], vec![]));

    // After another reset, the hotkey passes Sunset Sky, with no rate since, on its way to
    // the Northern Top: the Visit's best stays the Construction Site's 21,600.
    player.press(PanelControl::ResetSession);
    let saves = player.saves();
    let sunset = player.choose_next();
    assert_eq!(shown_map(&sunset), Some(SUNSET_SKY_ID));
    assert_eq!((rates(&player, CONSTRUCTION_SITE_ID), rates(&player, SUNSET_SKY_ID)), (vec![21_600], vec![]));
    let top = player.choose_next();
    assert_eq!(shown_map(&top), Some(NORTHERN_TOP_ID));
    assert_eq!(player.saves(), saves);
    assert_eq!((rates(&player, CONSTRUCTION_SITE_ID), rates(&player, SUNSET_SKY_ID)), (vec![21_600], vec![]));

    // The Visit ends with the Construction Site's 21,600 joining its bests.
    player.arrive(ROCKY_ROAD);
    let maps: Vec<_> = player.minerva.bests().unassigned.into_iter().map(|m| m.id).collect();
    assert_eq!(maps, [CONSTRUCTION_SITE_ID]);
    assert_eq!(rates(&player, CONSTRUCTION_SITE_ID), [21_600]);
}

#[test]
fn removing_a_best_from_the_history_changes_the_bests_the_shell_saves() {
    let mut player = Player::saved_by_the_shell(tracking());
    let sunset_rates = |player: &Player| saved_rates(&player.minerva.bests(), SUNSET_SKY_ID);

    // A first Visit's best at 7,200 EXP/h.
    player.arrive(SUNSET);
    player.hunt(5 * MIN, 20);
    let set = player.wall_clock();
    player.hunt(MIN, 20);
    assert_eq!(sunset_rates(&player), [7200]);

    // A second Visit at 10,800 EXP/h, then back a third time: removing the 10,800 leaves
    // the 7,200, and the shell saves that.
    player.arrive(CONSTRUCTION_SITE);
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 30);
    player.arrive(CONSTRUCTION_SITE);
    player.arrive(SUNSET);
    assert_eq!(sunset_rates(&player), [10_800, 7200]);
    let saves = player.saves();
    player.press(PanelControl::RemoveBest);
    assert_eq!((player.saves(), sunset_rates(&player)), (saves + 1, vec![7200]));

    // The next launch has it gone: the 7,200 is the best, set when it was.
    let mut next = relaunch(handed_out(&player.minerva), WALL_AT_START + DAY);
    assert_eq!(best_row(&next.arrive(SUNSET)), Some(7200));
    assert_eq!(saved(&next.minerva, SUNSET_SKY_ID), [best(7200, set)]);

    // Removing the map's only best leaves it none to save.
    next.press(PanelControl::RemoveBest);
    assert!(next.minerva.bests().unassigned.iter().all(|m| m.id != SUNSET_SKY_ID));
}

#[test]
fn a_visit_removed_in_the_personal_bests_window_is_gone_from_what_the_shell_saves_and_the_next_launch() {
    // Three Visits to Sunset Sky, at 10,800, 7,200 and 3,600 EXP/h; the player then on the
    // Construction Site.
    let mut player = Player::saved_by_the_shell(tracking());
    let mut set = Vec::new();
    for gain in [30, 20, 10] {
        player.arrive(SUNSET);
        player.hunt(5 * MIN, gain);
        set.push(player.wall_clock());
        player.hunt(MIN, gain);
        player.arrive(CONSTRUCTION_SITE);
    }
    assert_eq!(saved_rates(&player.minerva.bests(), SUNSET_SKY_ID), [10_800, 7200, 3600]);

    // The middle one, from another map: the shell saves the bests without it.
    let saves = player.saves();
    player.remove_visit(SUNSET_SKY_ID, best(7200, set[1]));
    assert_eq!(player.saves(), saves + 1);

    // The next launch has it gone, the others as they were set.
    let next = relaunch(handed_out(&player.minerva), WALL_AT_START + DAY);
    assert_eq!(saved(&next.minerva, SUNSET_SKY_ID), [best(10_800, set[0]), best(3600, set[2])]);
}

#[test]
fn removing_the_best_from_the_history_leaves_every_candidates_best_from_this_visit() {
    let mut player = Player::saved_by_the_shell(tracking_with(sharing_a_title()));

    // An earlier Visit's best on Sunset Sky: 72,000 EXP/h.
    player.arrive(KERNING);
    player.choose_next();
    player.hunt(5 * MIN, 200);
    let earlier_set = player.wall_clock();
    player.hunt(MIN, 200);
    player.arrive(ROCKY_ROAD);
    player.hunt(MIN, 10);

    // Back, 36,000 EXP/h on Sunset Sky, then on the Construction Site with the hotkey,
    // and back on Sunset Sky: half its best.
    player.arrive(KERNING);
    player.choose_next();
    player.hunt(5 * MIN, 100);
    let sunset_set = player.wall_clock();
    player.hunt(MIN, 100);
    let site = player.choose_next();
    let site_set = player.wall_clock();
    // The Construction Site has no earlier Visits': its row is this Visit's, from the choice.
    assert_eq!(best_row(&site), Some(36_000));
    player.hunt(MIN, 100);
    assert_eq!(best_row(&player.choose_next()), Some(72_000));
    assert_eq!(saved(&player.minerva, SUNSET_SKY_ID), [best(72_000, earlier_set), best(36_000, sunset_set)]);

    // Removing takes the 72,000: Sunset Sky's best from this Visit is left as it was
    // set, and takes the row...
    assert_eq!(best_row(&player.press(PanelControl::RemoveBest)), Some(36_000));
    assert_eq!(saved(&player.minerva, SUNSET_SKY_ID), [best(36_000, sunset_set)]);
    // ...and so is the Construction Site's, the Visit's once it shows again.
    player.hunt(MIN, 100);
    player.choose_next();
    assert_eq!(saved(&player.minerva, CONSTRUCTION_SITE_ID), [best(36_000, site_set)]);
}
