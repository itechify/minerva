//! Personal Bests per Character (GLOSSARY): each best is stamped with the Character, level
//! and job the Status Bar showed at the refresh that set it, and kept with that Character's.
//! The Grind Tracker races the current Character's best on the map, or the Unassigned ones
//! while no Character is read. Bests saved before Characters existed are Unassigned.

mod common;

use common::player::{Player, WALL_AT_START};
use common::{CONSTRUCTION_SITE_ID, SUNSET_SKY_ID, tracking, SUNSET_SAMPLE, CONSTRUCTION_SAMPLE, S, MIN, best_row};
use minerva_core::{Minerva, PanelControl, PanelState, SavedBests, VisitBest};

/// The bests `minerva` hands out, as the shell saves them: JSON.
fn handed_out(minerva: &Minerva) -> serde_json::Value {
    serde_json::to_value(minerva.bests()).unwrap()
}

#[test]
fn bests_saved_before_characters_read_in_as_unassigned_with_nothing_lost() {
    // A file as version 1 saved it, with no Characters.
    let version_1 = format!(
        r#"{{"version": 1, "maps": [
            {{"id": "{SUNSET_SKY_ID}", "visits": [
                {{"expPerHour": 7200, "setAtUnixMs": {WALL_AT_START}}},
                {{"expPerHour": 3600, "setAtUnixMs": null}}
            ]}},
            {{"id": "{CONSTRUCTION_SITE_ID}", "visits": [{{"expPerHour": 5000, "setAtUnixMs": null}}]}}
        ]}}"#
    );
    let saved: SavedBests = serde_json::from_str(&version_1).unwrap();
    let mut minerva = tracking();
    minerva.restore_bests(saved);

    // Every Visit is Unassigned, and the shell saves them in the new shape.
    let expected = serde_json::json!({
        "version": 2,
        "unassigned": [
            { "id": SUNSET_SKY_ID, "visits": [
                { "expPerHour": 7200, "setAtUnixMs": WALL_AT_START },
                { "expPerHour": 3600, "setAtUnixMs": null }
            ]},
            { "id": CONSTRUCTION_SITE_ID, "visits": [{ "expPerHour": 5000, "setAtUnixMs": null }] }
        ],
        "characters": []
    });
    assert_eq!(handed_out(&minerva), expected);

    // With no Character read, the Grind Tracker races them.
    let mut player = Player::new(minerva);
    assert_eq!(best_row(&player.arrive(SUNSET_SAMPLE)), Some(7200));
    player.hunt(MIN, 10);
    assert_eq!(best_row(&player.arrive(CONSTRUCTION_SAMPLE)), Some(5000));
}

/// What the bests `minerva` hands out keep for `character` (`None`: Unassigned) on `map`:
/// each Visit's (EXP/h, level), highest first.
fn kept(minerva: &Minerva, character: Option<&str>, map: &str) -> Vec<(u64, Option<u32>)> {
    let bests = minerva.bests();
    let maps = bests.maps_of(character);
    maps.iter().find(|m| m.id == map).map_or_else(Vec::new, |m| m.visits.iter().map(|v| (v.exp_per_hour, v.level)).collect())
}

/// Egbert, a level 13 Magician, a Visit to Sunset Sky at 7,200 EXP/h for 6 minutes, then on
/// to the Construction Site.
fn egbert_after_a_best_of_7200_on_sunset_sky() -> Player {
    let mut player = Player::clocked(tracking());
    player.status(Some("Egbert"), 13);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SAMPLE);
    player
}

#[test]
fn a_best_is_stamped_with_the_character_level_and_job_the_status_bar_showed_when_it_was_set() {
    let mut player = Player::clocked(tracking());
    player.status(Some("Egbert"), 13);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(5 * MIN, 20);
    let set = player.wall_clock().unwrap();

    let expected = serde_json::json!({
        "version": 2,
        "unassigned": [],
        "characters": [{ "name": "Egbert", "maps": [{ "id": SUNSET_SKY_ID, "visits": [
            { "expPerHour": 7200, "setAtUnixMs": set, "level": 13, "job": "Magician" }
        ]}]}]
    });
    assert_eq!(handed_out(&player.minerva), expected);
}

#[test]
fn each_characters_bests_are_kept_apart_and_the_grind_tracker_races_the_one_playing() {
    let mut player = egbert_after_a_best_of_7200_on_sunset_sky();
    assert_eq!(best_row(&player.arrive(SUNSET_SAMPLE)), Some(7200));

    // Bob, at the character select screen and back on Sunset Sky: he has no best there.
    player.status(Some("Bob"), 40);
    player.hunt(MIN, 10);
    player.arrive(CONSTRUCTION_SAMPLE);
    assert_eq!(best_row(&player.arrive(SUNSET_SAMPLE)), None);
    // 3,600 EXP/h: his own first best, on his row, never measured against Egbert's 7,200.
    let five = player.hunt(6 * MIN, 10);
    assert_eq!((best_row(&five), five.exp_tracking.personal_best_this_visit), (Some(3600), true));
    assert_eq!(kept(&player.minerva, Some("Bob"), SUNSET_SKY_ID), [(3600, Some(40))]);
    assert_eq!(kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID), [(7200, Some(13))]);

    // Back as Egbert: his best again.
    player.status(Some("Egbert"), 13);
    player.hunt(MIN, 10);
    player.arrive(CONSTRUCTION_SAMPLE);
    assert_eq!(best_row(&player.arrive(SUNSET_SAMPLE)), Some(7200));
}

#[test]
fn the_best_row_races_the_unassigned_bests_until_a_name_is_read_then_the_characters_at_once() {
    let mut minerva = tracking();
    let saved = serde_json::json!({
        "version": 2,
        "unassigned": [{ "id": SUNSET_SKY_ID, "visits": [{ "expPerHour": 9000, "setAtUnixMs": null }] }],
        "characters": [{ "name": "Egbert", "maps": [{ "id": SUNSET_SKY_ID, "visits": [
            { "expPerHour": 7200, "setAtUnixMs": null, "level": 13, "job": "Magician" }
        ]}]}]
    });
    minerva.restore_bests(serde_json::from_value(saved).unwrap());
    let mut player = Player::new(minerva);
    // The Status Bar's name can't be read.
    player.status(None, 13);
    assert_eq!(best_row(&player.arrive(SUNSET_SAMPLE)), Some(9000));

    // Named once the name is read three times in a row.
    player.status(Some("Egbert"), 13);
    assert_eq!(best_row(&player.hunt(20 * S, 10)), Some(9000), "not yet");
    assert_eq!(best_row(&player.hunt(10 * S, 10)), Some(7200));
}

#[test]
fn a_best_set_while_no_character_is_named_is_unassigned_and_the_player_can_move_it_to_one() {
    let mut player = Player::saved_by_the_shell(tracking());
    player.status(None, 13);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(5 * MIN, 20);
    let set = player.wall_clock();
    player.arrive(CONSTRUCTION_SAMPLE);
    assert_eq!(kept(&player.minerva, None, SUNSET_SKY_ID), [(7200, Some(13))]);

    // Egbert named: he has no best on Sunset Sky, and the window offers him and Unassigned.
    player.status(Some("Egbert"), 13);
    player.hunt(30 * S, 10);
    assert_eq!(best_row(&player.arrive(SUNSET_SAMPLE)), None);
    let window = player.press(PanelControl::TogglePersonalBests).personal_bests;
    let choices: Vec<(Option<&str>, bool, usize)> =
        window.characters.iter().map(|c| (c.name.as_deref(), c.current, c.maps.len())).collect();
    assert_eq!(choices, [(Some("Egbert"), true, 0), (None, false, 1)]);
    let listed = &window.characters[1].maps[0].visits[0];
    assert_eq!((listed.exp_per_hour, listed.set_at_unix_ms, listed.level), (7200, set, Some(13)));

    // Moved to Egbert, it is his best at once, and Unassigned has none left to show.
    let visit = VisitBest { exp_per_hour: 7200, set_at_unix_ms: set, level: Some(13), job: None };
    let moved = player.move_visit(None, SUNSET_SKY_ID, visit, "Egbert");
    assert_eq!(best_row(&moved), Some(7200));
    assert_eq!(kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID), [(7200, Some(13))]);
    assert_eq!(kept(&player.minerva, None, SUNSET_SKY_ID), []);
    let names: Vec<Option<&str>> = moved.personal_bests.characters.iter().map(|c| c.name.as_deref()).collect();
    assert_eq!(names, [Some("Egbert")]);
}

#[test]
fn a_character_that_levels_up_keeps_its_best_in_each_lower_level_band_past_the_10_best() {
    // Egbert at level 19: a Visit to Sunset Sky at 3,600 EXP/h.
    let mut player = Player::saved_by_the_shell(tracking());
    player.status(Some("Egbert"), 19);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(5 * MIN, 10);
    // Level 20, the next Level Band: 10 Visits there at 3,960 to 7,200 EXP/h, then one at
    // 3,600, each ended by stepping onto the Construction Site. (The level-up's reading isn't
    // counted: the Status Bar showed it before the EXP bar did.)
    player.status(Some("Egbert"), 20);
    player.hunt(MIN, 10);
    for gain in [15, 11, 20, 18, 12, 19, 13, 17, 14, 16, 10] {
        player.arrive(CONSTRUCTION_SAMPLE);
        player.arrive(SUNSET_SAMPLE);
        player.hunt(5 * MIN, gain);
    }
    player.arrive(CONSTRUCTION_SAMPLE);

    // The 10 best, all at level 20, and the best at 10 to 19 under them. The last Visit at
    // level 20 is neither, and isn't kept.
    let mut expected: Vec<(u64, Option<u32>)> = (11..=20).rev().map(|gain| (gain * 360, Some(20))).collect();
    expected.push((3600, Some(19)));
    assert_eq!(kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID), expected);

    // The window lists them all, each with its level.
    let window = player.press(PanelControl::TogglePersonalBests).personal_bests;
    let listed: Vec<(u64, Option<u32>)> =
        window.characters[0].maps[0].visits.iter().map(|v| (v.exp_per_hour, v.level)).collect();
    assert_eq!(listed, expected);

    // Shown on the Map Leaderboards, his best in each band on the map is an entry.
    player.minerva.show_on_leaderboards("Egbert", true);
    let at = player.at;
    let state = player.minerva.tick(at);
    let entries: Vec<(String, u32, u64)> = state.leaderboard_uploads[0]
        .entries
        .iter()
        .flatten()
        .map(|e| (e.map.clone(), e.level, e.exp_per_hour))
        .collect();
    assert_eq!(entries, [(SUNSET_SKY_ID.to_string(), 19, 3600), (SUNSET_SKY_ID.to_string(), 20, 7200)]);
}

/// A Magician's Visit best of `exp_per_hour` at `level`, with no date.
fn at_level(exp_per_hour: u64, level: u32) -> serde_json::Value {
    serde_json::json!({ "expPerHour": exp_per_hour, "setAtUnixMs": null, "level": level, "job": "Magician" })
}

/// Bests saved at an earlier launch: each Character's Visits on each map.
fn saved(characters: serde_json::Value) -> SavedBests {
    serde_json::from_value(serde_json::json!({ "version": 2, "unassigned": [], "characters": characters })).unwrap()
}

/// The Personal Bests window's choices, as (name, current, how many maps).
fn choices(state: &PanelState) -> Vec<(Option<&str>, bool, usize)> {
    state.personal_bests.characters.iter().map(|c| (c.name.as_deref(), c.current, c.maps.len())).collect()
}

/// The player hunting as `name`, with `bests` restored at launch, the Personal Bests window
/// open on Sunset Sky.
fn playing_with(name: &str, bests: SavedBests) -> (Player, PanelState) {
    let mut minerva = tracking();
    minerva.restore_bests(bests);
    let mut player = Player::saved_by_the_shell(minerva);
    // Read three times in a row, as a name must be to name the Character.
    player.status(Some(name), 41);
    player.hunt(30 * S, 10);
    player.arrive(SUNSET_SAMPLE);
    let state = player.press(PanelControl::TogglePersonalBests);
    (player, state)
}

#[test]
fn removing_a_character_removes_their_bests_and_the_one_playing_stays_a_choice() {
    let (mut player, open) = playing_with(
        "Bob",
        saved(serde_json::json!([
            { "name": "Egbert", "maps": [{ "id": SUNSET_SKY_ID, "visits": [at_level(7200, 13)] }] },
            { "name": "Bob", "maps": [
                { "id": SUNSET_SKY_ID, "visits": [at_level(3600, 40)] },
                { "id": CONSTRUCTION_SITE_ID, "visits": [at_level(5000, 41)] }
            ]}
        ])),
    );
    assert_eq!(best_row(&open), Some(3600));
    assert_eq!(choices(&open), [(Some("Bob"), true, 2), (Some("Egbert"), false, 1)]);

    // Bob removed: his bests go, at once, and he stays a choice, with none, while he plays.
    let removed = player.remove_character("Bob");
    assert_eq!(best_row(&removed), None);
    assert_eq!(choices(&removed), [(Some("Bob"), true, 0), (Some("Egbert"), false, 1)]);
    let names: Vec<String> = player.minerva.bests().characters.into_iter().map(|c| c.name).collect();
    assert_eq!(names, ["Egbert"]);

    // Egbert removed too: no best is left anywhere, and the window closes.
    let none = player.remove_character("Egbert");
    assert!(!none.personal_bests.open);
    assert!(!none.grind_tracker.personal_bests);
}

#[test]
fn merging_a_character_into_another_moves_its_bests_keeping_the_10_best_and_the_best_in_each_band() {
    // Egbert's 10 Visits to Sunset Sky at level 30; Egbert2, his new name, with two at level
    // 25, one at 31, and a Visit to the Construction Site.
    let egbert: Vec<serde_json::Value> = (10..20).rev().map(|k| at_level(k * 1000, 30)).collect();
    let (mut player, open) = playing_with(
        "Egbert2",
        saved(serde_json::json!([
            { "name": "Egbert", "maps": [{ "id": SUNSET_SKY_ID, "visits": egbert }] },
            { "name": "Egbert2", "maps": [
                { "id": SUNSET_SKY_ID, "visits": [at_level(9000, 31), at_level(5000, 25), at_level(4000, 25)] },
                { "id": CONSTRUCTION_SITE_ID, "visits": [at_level(6000, 31)] }
            ]}
        ])),
    );
    assert_eq!(best_row(&open), Some(9000));

    let merged = player.merge_characters("Egbert2", "Egbert");
    // On Sunset Sky the 10 best are Egbert's, and the best at levels 20 to 29 joins them; the
    // others aren't kept. The Construction Site's moves across.
    let mut sunset: Vec<(u64, Option<u32>)> = (10..20).rev().map(|k| (k * 1000, Some(30))).collect();
    sunset.push((5000, Some(25)));
    assert_eq!(kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID), sunset);
    assert_eq!(kept(&player.minerva, Some("Egbert"), CONSTRUCTION_SITE_ID), [(6000, Some(31))]);
    // Egbert2 has none left: playing, he is a choice with none, and the Best row has none.
    assert_eq!(choices(&merged), [(Some("Egbert"), false, 2), (Some("Egbert2"), true, 0)]);
    assert_eq!(best_row(&merged), None);
}

#[test]
fn an_unassigned_visit_moved_to_a_character_is_kept_only_if_their_history_keeps_it() {
    // Egbert's 10 Visits to Sunset Sky at level 30, and an Unassigned one from before
    // Characters, with no level, slower than all of them.
    let egbert: Vec<serde_json::Value> = (10..20).rev().map(|k| at_level(k * 1000, 30)).collect();
    let mut bests = saved(serde_json::json!([{ "name": "Egbert", "maps": [{ "id": SUNSET_SKY_ID, "visits": egbert }] }]));
    let before = VisitBest { exp_per_hour: 3600, ..VisitBest::default() };
    bests.unassigned = serde_json::from_value(serde_json::json!([{ "id": SUNSET_SKY_ID, "visits": [before] }])).unwrap();
    let (mut player, _) = playing_with("Egbert", bests);

    // Moved, it is neither among his 10 best nor the best in a band: it isn't kept.
    let moved = player.move_visit(None, SUNSET_SKY_ID, before, "Egbert");
    let rates: Vec<u64> = kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID).into_iter().map(|(rate, _)| rate).collect();
    assert_eq!(rates, (10..20).rev().map(|k| k * 1000).collect::<Vec<u64>>());
    assert_eq!(kept(&player.minerva, None, SUNSET_SKY_ID), []);
    assert_eq!(choices(&moved), [(Some("Egbert"), true, 1)]);
}

#[test]
fn a_visit_is_removed_by_its_character_map_best_level_and_job_all() {
    let (mut player, _) = playing_with(
        "Egbert",
        saved(serde_json::json!([
            { "name": "Egbert", "maps": [{ "id": SUNSET_SKY_ID, "visits": [at_level(7200, 14), at_level(7200, 13)] }] }
        ])),
    );
    let visit = |level, job: &str| VisitBest {
        exp_per_hour: 7200,
        set_at_unix_ms: None,
        level: Some(level),
        job: Some(job.to_string()),
    };
    // Another job, the Unassigned bests, or another Character's: nothing is removed.
    player.remove_visit_of(Some("Egbert"), SUNSET_SKY_ID, visit(13, "Wizard (F/P)"));
    player.remove_visit_of(None, SUNSET_SKY_ID, visit(13, "Magician"));
    player.remove_visit_of(Some("Bob"), SUNSET_SKY_ID, visit(13, "Magician"));
    assert_eq!(kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID), [(7200, Some(14)), (7200, Some(13))]);

    // The one at level 13 goes, and only it.
    player.remove_visit_of(Some("Egbert"), SUNSET_SKY_ID, visit(13, "Magician"));
    assert_eq!(kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID), [(7200, Some(14))]);
}

/// A Magician's Visit best of `exp_per_hour` at `level`, with no date, as the window lists it.
fn best_at(exp_per_hour: u64, level: u32) -> VisitBest {
    VisitBest { exp_per_hour, set_at_unix_ms: None, level: Some(level), job: Some("Magician".to_string()) }
}

#[test]
fn a_visit_moved_from_one_character_to_another_joins_theirs_and_the_best_row_follows() {
    // Bob's best on Sunset Sky, at level 13, was Egbert's: the Status Bar misread the name.
    let (mut player, open) = playing_with(
        "Egbert",
        saved(serde_json::json!([
            { "name": "Egbert", "maps": [{ "id": SUNSET_SKY_ID, "visits": [at_level(7200, 13)] }] },
            { "name": "Bob", "maps": [{ "id": SUNSET_SKY_ID, "visits": [at_level(9000, 13), at_level(3600, 40)] }] }
        ])),
    );
    assert_eq!(best_row(&open), Some(7200));

    // Moved to the one it's already with, or a Visit Bob doesn't have: nothing changes.
    player.move_visit(Some("Bob"), SUNSET_SKY_ID, best_at(9000, 13), "Bob");
    player.move_visit(Some("Bob"), SUNSET_SKY_ID, best_at(9000, 14), "Egbert");
    assert_eq!(kept(&player.minerva, Some("Bob"), SUNSET_SKY_ID), [(9000, Some(13)), (3600, Some(40))]);

    let moved = player.move_visit(Some("Bob"), SUNSET_SKY_ID, best_at(9000, 13), "Egbert");
    assert_eq!(kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID), [(9000, Some(13)), (7200, Some(13))]);
    assert_eq!(kept(&player.minerva, Some("Bob"), SUNSET_SKY_ID), [(3600, Some(40))]);
    // Egbert plays on Sunset Sky: the Best row races his new best at once.
    assert_eq!(best_row(&moved), Some(9000));
}

#[test]
fn every_unassigned_visit_can_be_given_to_a_character_at_once() {
    // Two Unassigned Visits on Sunset Sky and one on the Construction Site; Egbert has one on
    // Sunset Sky.
    let mut bests = saved(serde_json::json!([
        { "name": "Egbert", "maps": [{ "id": SUNSET_SKY_ID, "visits": [at_level(7200, 13)] }] }
    ]));
    bests.unassigned = serde_json::from_value(serde_json::json!([
        { "id": SUNSET_SKY_ID, "visits": [at_level(9000, 14), at_level(3600, 12)] },
        { "id": CONSTRUCTION_SITE_ID, "visits": [at_level(5000, 15)] }
    ]))
    .unwrap();
    let (mut player, open) = playing_with("Egbert", bests);
    assert_eq!(choices(&open), [(Some("Egbert"), true, 1), (None, false, 2)]);

    // Each joins Egbert's on its map, and Unassigned is no longer a choice.
    let assigned = player.assign_unassigned("Egbert");
    assert_eq!(kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID), [(9000, Some(14)), (7200, Some(13)), (3600, Some(12))]);
    assert_eq!(kept(&player.minerva, Some("Egbert"), CONSTRUCTION_SITE_ID), [(5000, Some(15))]);
    assert!(player.minerva.bests().unassigned.is_empty());
    assert_eq!(choices(&assigned), [(Some("Egbert"), true, 2)]);
    assert_eq!(best_row(&assigned), Some(9000));
}
