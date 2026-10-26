//! Party EXP (GLOSSARY): the Pickup Log tells the player's own EXP from a partymate's. Any 5
//! minutes of active play with Party EXP in them are party; a map's Personal Best is a Solo Best
//! or a Party Best by the window that set it, the Grind Tracker races the kind being played, and
//! the EXP Split shows the shares once Party EXP has come in.

mod common;

use common::player::Player;
use common::{SUNSET_SAMPLE, SUNSET_SKY_ID, S, MIN, tracking};
use minerva_core::{
    BESTS_VERSION, ExpSplit, MapBests, Minerva, PanelControl, PanelState, PickupLine, PickupLogReading, PickupLogSeen, SavedBests,
    ScreenRect, VisitBest,
};

/// A Pickup Log reading at `at_ms` that found `arrived` among its lines, clean.
fn reading(at_ms: u64, arrived: Vec<PickupLine>) -> PickupLogReading {
    let area = ScreenRect { x: 1500, y: 800, width: 420, height: 160 };
    PickupLogReading { at_ms, seen: PickupLogSeen::Clear { area }, rows: vec![], shift: 0, arrived, chose: false, overflowed: false }
}

/// A kill of the player's own, as the log writes it.
fn own_kill(amount: u64) -> Vec<PickupLine> {
    vec![PickupLine::Exp { amount, party: false }]
}

/// A share of a partymate's kill and the bonus on it.
fn party_kill(share: u64, bonus: u64) -> Vec<PickupLine> {
    vec![PickupLine::Exp { amount: share, party: true }, PickupLine::PartyBonus { amount: bonus }]
}

/// Hunt for `ms` more as [`Player::hunt`] does, each step's kill showing in the Pickup Log
/// just after the bar reads it, as `lines` says. The panel state at the last reading.
fn hunt_with(player: &mut Player, ms: u64, gain: u64, lines: impl Fn() -> Vec<PickupLine>) -> PanelState {
    let mut state = player.minerva.panel_state().clone();
    for _ in 0..ms / (10 * S) {
        player.at += 10 * S;
        player.exp += gain;
        player.read();
        state = player.minerva.on_pickup_log_reading(reading(player.at, lines())).clone();
    }
    state
}

fn hunt_solo(player: &mut Player, ms: u64, gain: u64) -> PanelState {
    hunt_with(player, ms, gain, || own_kill(gain / 2))
}

fn hunt_partied(player: &mut Player, ms: u64, gain: u64) -> PanelState {
    hunt_with(player, ms, gain, || party_kill(gain / 4, gain / 10))
}

/// `map`'s Visit bests the core hands out for the Unassigned, highest first, as
/// (EXP/h, party).
fn kept(minerva: &Minerva, map: &str) -> Vec<(u64, bool)> {
    let bests = minerva.bests();
    let maps = bests.maps_of(None);
    maps.iter().find(|m| m.id == map).map_or_else(Vec::new, |m| m.visits.iter().map(|v| (v.exp_per_hour, v.party)).collect())
}

/// A Sunset Sky Visit of 6 minutes alone at 3,600 EXP/h.
fn alone_on_sunset_sky() -> Player {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    hunt_solo(&mut player, 6 * MIN, 10);
    player
}

#[test]
fn a_session_solo_then_partied_then_solo_has_each_window_set_the_best_of_its_kind() {
    let mut player = alone_on_sunset_sky();
    assert_eq!(kept(&player.minerva, SUNSET_SKY_ID), [(3600, false)]);

    // Partied at 10,800 EXP/h: once Party EXP is in the last 5 minutes the window is party,
    // even with most of it solo play, and what it measures is a Party Best, which the Solo Best
    // isn't raced against.
    let first_party = hunt_partied(&mut player, 30 * S, 30);
    assert_eq!(kept(&player.minerva, SUNSET_SKY_ID), [(4320, true), (3600, false)]);
    assert_eq!(first_party.exp_tracking.personal_best, Some(4320));
    assert!(first_party.exp_tracking.personal_best_party);
    let partied = hunt_partied(&mut player, 5 * MIN + 30 * S, 30);
    assert_eq!(kept(&player.minerva, SUNSET_SKY_ID), [(10_800, true), (3600, false)]);
    assert_eq!(partied.exp_tracking.personal_best, Some(10_800));
    assert!(partied.exp_tracking.personal_best_party);

    // The partymate leaves, and the player carries on alone at 7,200 EXP/h: for 5 minutes the
    // windows still have Party EXP in them, and set nothing.
    let leaving = hunt_solo(&mut player, 4 * MIN, 20);
    assert_eq!(kept(&player.minerva, SUNSET_SKY_ID), [(10_800, true), (3600, false)]);
    assert!(leaving.exp_tracking.personal_best_party);

    // Then the window is solo again: the Solo Best is raced, and beaten, the Party Best kept.
    let alone = hunt_solo(&mut player, 3 * MIN, 20);
    assert_eq!(kept(&player.minerva, SUNSET_SKY_ID), [(10_800, true), (7200, false)]);
    assert!(!alone.exp_tracking.personal_best_party);
    assert_eq!(alone.exp_tracking.personal_best, Some(7200));
    assert!(alone.exp_tracking.new_best);
    // One Session throughout: it didn't reset when the party came or left.
    assert!(alone.exp_tracking.active_s > 15 * 60);
}

#[test]
fn bests_saved_before_party_play_was_told_apart_are_solo_bests() {
    let version_2 = format!(
        r#"{{"version": 2, "unassigned": [
            {{"id": "{SUNSET_SKY_ID}", "visits": [{{"expPerHour": 7200, "setAtUnixMs": null}}]}}
        ], "characters": []}}"#
    );
    let saved: SavedBests = serde_json::from_str(&version_2).unwrap();
    let mut minerva = tracking();
    minerva.restore_bests(saved);
    assert_eq!(kept(&minerva, SUNSET_SKY_ID), [(7200, false)]);
    // The shell saves them in this version's shape, and nothing says "party" of a Solo Best.
    let handed = serde_json::to_value(minerva.bests()).unwrap();
    assert_eq!(handed["version"], 3);
    assert!(handed["unassigned"][0]["visits"][0].get("party").is_none());

    // The Grind Tracker races it alone, and a party window doesn't race it.
    let mut player = Player::clocked(minerva);
    let alone = player.arrive(SUNSET_SAMPLE);
    assert_eq!((alone.exp_tracking.personal_best, alone.exp_tracking.personal_best_party), (Some(7200), false));
    let partied = hunt_partied(&mut player, 30 * S, 10);
    assert_eq!((partied.exp_tracking.personal_best, partied.exp_tracking.personal_best_party), (None, false));
}

#[test]
fn a_new_party_visit_is_a_party_best_too_when_it_beats_the_earlier_ones() {
    let mut player = alone_on_sunset_sky();
    hunt_partied(&mut player, 6 * MIN, 20);
    let saved = player.minerva.bests();

    // A later launch's Visit starts from both, each in its own history.
    let mut next = Player::clocked_at(tracking(), common::player::WALL_AT_START + common::DAY);
    next.minerva.restore_bests(saved);
    next.arrive(SUNSET_SAMPLE);
    assert_eq!(kept(&next.minerva, SUNSET_SKY_ID), [(7200, true), (3600, false)]);
}

#[test]
fn the_exp_split_shows_once_party_exp_has_come_in_this_session_with_the_shares() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    // Alone: no split, however long.
    let alone = hunt_solo(&mut player, 2 * MIN, 10);
    assert_eq!(alone.exp_tracking.exp_split, None);

    // The first Party EXP shows it at once. Own EXP so far: 12 kills of 5; now a share of 20
    // and a bonus of 5 each, 3 times in a row: 75 of party against 60 of its own (55.56%).
    let partied = hunt_with(&mut player, 30 * S, 10, || party_kill(20, 5));
    assert_eq!(partied.exp_tracking.exp_split, Some(ExpSplit { own_hundredths: 4444, party_hundredths: 5556 }));

    // The Session's reset starts it afresh: nothing until Party EXP comes in again.
    player.press(PanelControl::ResetSession);
    let again = hunt_solo(&mut player, MIN, 10);
    assert_eq!(again.exp_tracking.exp_split, None);
}

#[test]
fn a_reading_that_saw_nothing_or_was_covered_changes_nothing() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    hunt_solo(&mut player, MIN, 10);
    let area = ScreenRect { x: 1500, y: 800, width: 420, height: 160 };
    let covered = PickupLogReading { seen: PickupLogSeen::Covered { area }, ..reading(player.at, Vec::new()) };
    player.minerva.on_pickup_log_reading(covered);
    let nothing = PickupLogReading { seen: PickupLogSeen::Nothing, ..reading(player.at, Vec::new()) };
    let state = player.minerva.on_pickup_log_reading(nothing).clone();
    assert_eq!(state.exp_tracking.exp_split, None);
}

#[test]
fn the_pace_trace_races_the_party_best_while_the_window_is_party() {
    let player = alone_on_sunset_sky();
    // A Party Best of 14,400 from an earlier run, a Solo Best of 3,600 now.
    let mut saved = player.minerva.bests();
    saved.unassigned[0].visits.insert(0, VisitBest { exp_per_hour: 14_400, party: true, ..VisitBest::default() });
    let mut next = Player::clocked_at(tracking(), common::player::WALL_AT_START + common::DAY);
    next.minerva.restore_bests(saved);
    next.arrive(SUNSET_SAMPLE);
    // Partied at 14,400: within reach of the Party Best after a minute, so the race shows it.
    let state = hunt_with(&mut next, 8 * MIN, 40, || party_kill(20, 5));
    let pace = state.exp_tracking.pace.expect("a pace trace");
    assert!(pace.best.is_some());
    assert!(state.exp_tracking.personal_best_party);
    assert_eq!(state.exp_tracking.personal_best, Some(14_400));
}

#[test]
fn a_visit_moves_to_the_party_bests_and_back_and_the_best_row_follows() {
    let visit = |exp_per_hour| VisitBest { exp_per_hour, ..VisitBest::default() };
    let mut minerva = tracking();
    let maps = vec![MapBests { id: SUNSET_SKY_ID.to_string(), visits: vec![visit(7200), visit(3600)] }];
    minerva.restore_bests(SavedBests { version: BESTS_VERSION, unassigned: maps, characters: Vec::new() });
    let mut player = Player::clocked(minerva);
    assert_eq!(player.arrive(SUNSET_SAMPLE).exp_tracking.personal_best, Some(7200));

    // Moved to the Party Bests, the 7,200 is no longer the Solo Best the player races alone.
    player.minerva.set_visit_party(None, SUNSET_SKY_ID, &visit(7200), true);
    assert_eq!(kept(&player.minerva, SUNSET_SKY_ID), [(7200, true), (3600, false)]);
    let alone = player.minerva.panel_state().clone();
    assert_eq!((alone.exp_tracking.personal_best, alone.exp_tracking.personal_best_party), (Some(3600), false));

    // Moved to where it already is, it changes nothing; moved back, it is the Solo Best again.
    let party_visit = VisitBest { party: true, ..visit(7200) };
    let revision = player.minerva.bests_revision();
    player.minerva.set_visit_party(None, SUNSET_SKY_ID, &party_visit, true);
    assert_eq!(player.minerva.bests_revision(), revision);
    player.minerva.set_visit_party(None, SUNSET_SKY_ID, &party_visit, false);
    assert_eq!(kept(&player.minerva, SUNSET_SKY_ID), [(7200, false), (3600, false)]);
    assert_eq!(player.minerva.panel_state().exp_tracking.personal_best, Some(7200));
}

#[test]
fn the_visit_under_ways_best_cannot_be_moved_yet_and_a_visit_gone_changes_nothing() {
    let mut player = alone_on_sunset_sky();
    let revision = player.minerva.bests_revision();
    let under_way = VisitBest { exp_per_hour: 3600, set_at_unix_ms: player.wall_clock(), ..VisitBest::default() };
    player.minerva.set_visit_party(None, SUNSET_SKY_ID, &under_way, true);
    player.minerva.set_visit_party(None, SUNSET_SKY_ID, &VisitBest { exp_per_hour: 99, ..VisitBest::default() }, true);
    assert_eq!(kept(&player.minerva, SUNSET_SKY_ID), [(3600, false)]);
    assert_eq!(player.minerva.bests_revision(), revision);
}

#[test]
fn the_personal_bests_window_shows_each_maps_solo_best_and_party_best_and_each_visits_kind() {
    let visit = |exp_per_hour, party| VisitBest { exp_per_hour, party, ..VisitBest::default() };
    let mut minerva = tracking();
    let maps = vec![MapBests {
        id: SUNSET_SKY_ID.to_string(),
        visits: vec![visit(9000, true), visit(7200, false), visit(5000, true), visit(3600, false)],
    }];
    minerva.restore_bests(SavedBests { version: BESTS_VERSION, unassigned: maps, characters: Vec::new() });
    let mut player = Player::clocked(minerva);
    player.press(PanelControl::TogglePersonalBests);
    let window = player.minerva.panel_state().personal_bests.clone();
    let map = &window.characters[0].maps[0];
    // The row is the higher of the two, as an upload's entry is.
    assert_eq!(map.exp_per_hour, 9000);
    assert_eq!(map.solo.as_ref().map(|k| k.exp_per_hour), Some(7200));
    assert_eq!(map.party.as_ref().map(|k| k.exp_per_hour), Some(9000));
    let kinds: Vec<(u64, bool)> = map.visits.iter().map(|v| (v.exp_per_hour, v.party)).collect();
    assert_eq!(kinds, [(9000, true), (7200, false), (5000, true), (3600, false)]);
}
