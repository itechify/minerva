//! The Personal Bests window (GLOSSARY): every map's Personal Best side by side, over the
//! game, to compare maps. The Grind Tracker's star and its Best row's "Best" open it while
//! some map has a best, and close it; so does its ×. It shows by the Grind Tracker's rule
//! (with the game in front, not while setup is open), EXP tracking on or not. The core
//! hands the shell its rows as plain data, one map a row, highest EXP/h first, live: the
//! bests as saved, the Visit under way's included, with each map's names, region, mob
//! levels and share of its solo EXP/h ceiling from OSMS's file set, and the map the player
//! is on marked. Each row holds its map's Visit History, the Visit under way's best among
//! them, and the player can remove any Visit in it, on any map.
//!
//! Where it goes, worked as in `grind_tracker_window.rs`: the game's client area at
//! (812,110), 1920x1200 physical px, on one 3440x1440 monitor, and a Personal Bests window
//! of 920x600, so centred at (1312,410). The Minimap Title is found at (882,150)
//! 620x48.

mod common;

use common::player::Player;
use common::{CONSTRUCTION_SITE_ID, FakeSource, SUNSET_SKY_ID, osms, sharing_a_title, tracking, tracking_with, SUNSET_SAMPLE, CONSTRUCTION_SAMPLE, KERNING_SAMPLE, S, MIN, GAME, game, rect};
use minerva_core::{
    Action, BESTS_VERSION, CharacterBests, ClientOffset, Minerva, Foreground, LevelRange, MapBests,
    PanelControl, PanelSizes, PanelState, PersonalBestsMap, PersonalBestsVisit,
    PersonalBestsWindow, Request, SavedBests, ScreenRect, Settings, Size, VisitBest, meowdb,
};

const ROCKY_ROAD: &str = "victoria-road__rocky-road-i--title-region";

/// The player presses the Grind Tracker's star: the Personal Bests window then.
fn toggle(player: &mut Player) -> PersonalBestsWindow {
    player.press(PanelControl::TogglePersonalBests).personal_bests
}

/// Whether the Grind Tracker's window bar shows the Personal Bests button.
fn button(state: &PanelState) -> bool {
    state.grind_tracker.personal_bests
}

/// `map`'s Map Leaderboard, as the window links it for the Character named `name` with no
/// level read.
fn leaderboard(map: &str, name: &str) -> Option<String> {
    Some(format!("https://owlofminerva.app/leaderboards/map/?map={map}&name={name}"))
}

/// The window's rows of the Unassigned bests, which are all there are with no Character
/// named: none if there are none.
fn maps(window: &PersonalBestsWindow) -> &[PersonalBestsMap] {
    window.characters.iter().find(|c| c.name.is_none()).map_or(&[], |c| &c.maps)
}

/// The window's rows as (map ID, EXP/h, marked here), in its order.
fn rows(window: &PersonalBestsWindow) -> Vec<(&str, u64, bool)> {
    maps(window).iter().map(|m| (m.id.as_str(), m.exp_per_hour, m.here)).collect()
}

/// The window's row for `map`.
fn row<'a>(window: &'a PersonalBestsWindow, map: &str) -> &'a PersonalBestsMap {
    maps(window).iter().find(|m| m.id == map).unwrap_or_else(|| panic!("no row for {map}"))
}

/// Bests saved at an earlier launch: one Visit's, with no date, on each map.
fn bests_of(maps: &[(&str, u64)]) -> SavedBests {
    let histories: Vec<(&str, Vec<u64>)> = maps.iter().map(|&(id, exp)| (id, vec![exp])).collect();
    histories_of(&histories)
}

/// Bests saved at an earlier launch: each map's Visits' at these EXP/h, highest first,
/// with no date.
fn histories_of(maps: &[(&str, Vec<u64>)]) -> SavedBests {
    let maps = maps
        .iter()
        .map(|(id, rates)| MapBests { id: (*id).into(), visits: rates.iter().map(|&exp| saved_best(exp)).collect() })
        .collect();
    SavedBests { version: BESTS_VERSION, unassigned: maps, characters: Vec::new() }
}

/// A Visit's best of `exp_per_hour` saved with no date: what the player removes it by.
fn saved_best(exp_per_hour: u64) -> VisitBest {
    VisitBest { exp_per_hour, ..VisitBest::default() }
}

/// One of a map's Visits as the window lists it.
fn visit(exp_per_hour: u64, set_at_unix_ms: Option<u64>, share: Option<u64>, this_visit: bool) -> PersonalBestsVisit {
    PersonalBestsVisit { exp_per_hour, set_at_unix_ms, level: None, job: None, ceiling_share_hundredths: share, this_visit }
}

/// `map`'s Visits in the window as (EXP/h, the Visit under way's), highest first.
fn visits(window: &PersonalBestsWindow, map: &str) -> Vec<(u64, bool)> {
    row(window, map).visits.iter().map(|v| (v.exp_per_hour, v.this_visit)).collect()
}

/// Henesys Hunting Ground I's map ID: its Map Summary is the one the spec works out.
const HENESYS_HUNTING_GROUND_I_ID: &str = "010001010";

/// Maple Hill: a Hidden Street map in the Event region.
const MAPLE_HILL: u64 = 90_000_000;
const MAPLE_HILL_ID: &str = "090000000";

/// Sunset Sky's mob levels in the made-up game world: Blue Snail to Green Mushroom.
const SUNSET_SKY_LEVELS: LevelRange = LevelRange { lowest: 2, highest: 14 };

/// The Construction Site's mob levels in the made-up game world: Red Snail to Blue
/// Mushroom.
const CONSTRUCTION_SITE_LEVELS: LevelRange = LevelRange { lowest: 4, highest: 19 };

#[test]
fn with_no_best_anywhere_there_is_no_button_and_the_window_doesnt_open() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    // 4 minutes: not a best yet.
    let four = player.hunt(4 * MIN, 20);
    assert!(!button(&four));

    let window = toggle(&mut player);
    assert!(!window.open);
    assert_eq!(window.placement, None);
    assert!(window.characters.is_empty());
}

#[test]
fn a_first_visits_best_is_in_the_window_with_the_maps_names_levels_and_share_marked_here() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    // 7,200 EXP/h, the best set 5 minutes in.
    let five = player.hunt(5 * MIN, 20);
    let set = player.wall_clock();
    assert!(button(&five));
    player.hunt(MIN, 20);

    let window = toggle(&mut player);
    assert!(window.open);
    assert_eq!(
        maps(&window),
        [PersonalBestsMap {
            id: SUNSET_SKY_ID.into(),
            street_name: "Victoria Road".into(),
            map_name: "Sunset Sky".into(),
            region: "Victoria Island".into(),
            mark: None,
            url: meowdb::map_url(SUNSET_SKY_ID),
            // The Unassigned bests are no one's on the site: no Map Leaderboard. Nor a star,
            // with only one map.
            leaderboard_url: None,
            best_of_all: false,
            mob_levels: Some(SUNSET_SKY_LEVELS),
            exp_per_hour: 7200,
            // Of its solo ceiling, 213,571 EXP/h: 3.37%.
            solo_exp_ceiling: Some(213_571),
            ceiling_share_hundredths: Some(337),
            set_at_unix_ms: set,
            here: true,
            visits: vec![visit(7200, set, Some(337), true)],
        }]
    );
}

#[test]
fn maps_are_highest_first_and_only_the_one_the_player_is_on_is_marked_here() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(6 * MIN, 30);

    let window = toggle(&mut player);
    assert_eq!(rows(&window), [(CONSTRUCTION_SITE_ID, 10_800, true), (SUNSET_SKY_ID, 7200, false)]);
    // Of its solo ceiling, 281,786 EXP/h: 3.83%.
    let site = row(&window, CONSTRUCTION_SITE_ID);
    assert_eq!(
        (site.mob_levels, site.solo_exp_ceiling, site.ceiling_share_hundredths),
        (Some(CONSTRUCTION_SITE_LEVELS), Some(281_786), Some(383))
    );
}

#[test]
fn open_it_follows_the_bests_and_the_player_as_they_hunt() {
    // Open on Sunset Sky's first best.
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    assert_eq!(rows(&toggle(&mut player)), [(SUNSET_SKY_ID, 7200, true)]);

    // Off to the Construction Site, where a best comes without opening it again.
    let arrived = player.arrive(CONSTRUCTION_SAMPLE);
    assert_eq!(rows(&arrived.personal_bests), [(SUNSET_SKY_ID, 7200, false)]);
    let hunted = player.hunt(6 * MIN, 30);
    assert_eq!(rows(&hunted.personal_bests), [(CONSTRUCTION_SITE_ID, 10_800, true), (SUNSET_SKY_ID, 7200, false)]);
}

#[test]
fn a_best_on_a_map_the_player_has_not_been_on_has_its_names_region_levels_and_share_at_once() {
    // Bests saved at an earlier launch: Henesys Hunting Ground I (Victoria Island) and
    // Maple Hill, an Event map given Blue Snails.
    let world = osms::game().changing(MAPLE_HILL, |m| m.spawns(3, 4));
    let mut minerva = tracking_with(FakeSource::with_world(world));
    minerva.restore_bests(bests_of(&[(HENESYS_HUNTING_GROUND_I_ID, 7661), (MAPLE_HILL_ID, 2857)]));
    let mut player = Player::clocked(minerva);
    player.arrive(SUNSET_SAMPLE);

    let window = toggle(&mut player);
    assert_eq!(
        maps(&window),
        [
            PersonalBestsMap {
                id: HENESYS_HUNTING_GROUND_I_ID.into(),
                street_name: "Victoria Road".into(),
                map_name: "Henesys Hunting Ground I".into(),
                region: "Victoria Island".into(),
                mark: None,
                url: meowdb::map_url(HENESYS_HUNTING_GROUND_I_ID),
                leaderboard_url: None,
                // The Unassigned bests' best of all maps isn't the player's: no star.
                best_of_all: false,
                // Blue Snail and Shroom to Green Mushroom.
                mob_levels: Some(LevelRange { lowest: 2, highest: 14 }),
                exp_per_hour: 7661,
                // Of its solo ceiling, 153,216 EXP/h: 5.0%.
                solo_exp_ceiling: Some(153_216),
                ceiling_share_hundredths: Some(500),
                set_at_unix_ms: None,
                here: false,
                visits: vec![visit(7661, None, Some(500), false)],
            },
            PersonalBestsMap {
                id: MAPLE_HILL_ID.into(),
                street_name: "Hidden Street".into(),
                map_name: "Maple Hill".into(),
                region: "Event".into(),
                mark: None,
                url: meowdb::map_url(MAPLE_HILL_ID),
                leaderboard_url: None,
                best_of_all: false,
                mob_levels: Some(LevelRange { lowest: 2, highest: 2 }),
                exp_per_hour: 2857,
                // Of its solo ceiling, 4 Blue Snails' 5,714 EXP/h: 50%.
                solo_exp_ceiling: Some(5714),
                ceiling_share_hundredths: Some(5000),
                set_at_unix_ms: None,
                here: false,
                visits: vec![visit(2857, None, Some(5000), false)],
            },
        ]
    );
}

#[test]
fn the_window_has_this_visits_best_as_the_best_row_does() {
    // Sunset Sky at 7,200 EXP/h, then 2 minutes on the Construction Site, and back.
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(2 * MIN, 30);
    player.arrive(SUNSET_SAMPLE);

    // 10,800 EXP/h beats it: both show this Visit's.
    let beaten = player.hunt(5 * MIN, 30);
    let set = player.wall_clock();
    assert_eq!(beaten.exp_tracking.personal_best, Some(10_800));
    let window = toggle(&mut player);
    assert_eq!(rows(&window), [(SUNSET_SKY_ID, 10_800, true)]);
    assert_eq!(row(&window, SUNSET_SKY_ID).set_at_unix_ms, set);
}

#[test]
fn while_candidates_wait_for_a_choice_no_map_is_marked_here() {
    // A first Visit, choosing the Construction Site past Sunset Sky: 3,600 EXP/h there.
    let mut player = Player::clocked(tracking_with(sharing_a_title()));
    player.arrive(KERNING_SAMPLE);
    player.hunt(6 * MIN, 10);
    player.choose_next();
    player.choose_next();
    assert_eq!(rows(&toggle(&mut player)), [(CONSTRUCTION_SITE_ID, 3600, true)]);

    // Back by Rocky Road I, which doesn't settle which map the title is: the panel shows
    // the Construction Site, but the player may be on Sunset Sky.
    player.arrive(ROCKY_ROAD);
    player.hunt(MIN, 10);
    let back = player.arrive(KERNING_SAMPLE);
    assert_eq!(back.candidates.map(|c| c.chosen), Some(false));
    assert_eq!(rows(&back.personal_bests), [(CONSTRUCTION_SITE_ID, 3600, false)]);
}

#[test]
fn a_best_on_a_map_not_in_the_map_list_is_kept_and_goes_by_its_id() {
    let gone = "999999999";
    let mut minerva = tracking();
    minerva.restore_bests(bests_of(&[(gone, 5000)]));
    let mut player = Player::clocked(minerva);
    player.arrive(SUNSET_SAMPLE);

    let window = toggle(&mut player);
    assert_eq!(
        maps(&window),
        [PersonalBestsMap {
            id: gone.into(),
            street_name: String::new(),
            map_name: String::new(),
            region: String::new(),
            mark: None,
            url: meowdb::map_url(gone),
            leaderboard_url: None,
            best_of_all: false,
            mob_levels: None,
            exp_per_hour: 5000,
            solo_exp_ceiling: None,
            ceiling_share_hundredths: None,
            set_at_unix_ms: None,
            here: false,
            visits: vec![visit(5000, None, None, false)],
        }]
    );
}

/// The window's rows of the Character named `name` as (map ID, starred, Map Leaderboard), in
/// its order.
fn starred_and_linked(window: &PersonalBestsWindow, name: Option<&str>) -> Vec<(String, bool, Option<String>)> {
    let character = window.characters.iter().find(|c| c.name.as_deref() == name).unwrap();
    character.maps.iter().map(|m| (m.id.clone(), m.best_of_all, m.leaderboard_url.clone())).collect()
}

#[test]
fn a_characters_best_of_all_maps_is_starred_and_each_map_in_the_list_links_to_its_map_leaderboard() {
    // Egbert's bests on Sunset Sky, the Construction Site and a map not in the map list; Bob's
    // on Sunset Sky alone; and the same three as Egbert's Unassigned.
    let gone = "999999999";
    let mut bests = histories_of(&[(SUNSET_SKY_ID, vec![7200]), (CONSTRUCTION_SITE_ID, vec![10_800]), (gone, vec![5000])]);
    let bob = histories_of(&[(SUNSET_SKY_ID, vec![3600])]).unassigned;
    bests.characters = vec![
        CharacterBests { name: "Egbert".into(), maps: bests.unassigned.clone() },
        CharacterBests { name: "Bob".into(), maps: bob },
    ];
    let mut minerva = tracking();
    minerva.restore_bests(bests);
    let mut player = Player::clocked(minerva);
    player.arrive(SUNSET_SAMPLE);
    let window = toggle(&mut player);

    // Egbert's best of all maps has the star; a map not in the list has no Map Leaderboard.
    assert_eq!(
        starred_and_linked(&window, Some("Egbert")),
        [
            (CONSTRUCTION_SITE_ID.into(), true, leaderboard(CONSTRUCTION_SITE_ID, "Egbert")),
            (SUNSET_SKY_ID.into(), false, leaderboard(SUNSET_SKY_ID, "Egbert")),
            (gone.into(), false, None),
        ]
    );
    // With one map, there is nothing to compare: no star.
    assert_eq!(starred_and_linked(&window, Some("Bob")), [(SUNSET_SKY_ID.into(), false, leaderboard(SUNSET_SKY_ID, "Bob"))]);
    // The Unassigned bests are no one's: no star, and nothing of theirs is on the site.
    assert_eq!(
        starred_and_linked(&window, None),
        [(CONSTRUCTION_SITE_ID.into(), false, None), (SUNSET_SKY_ID.into(), false, None), (gone.into(), false, None)]
    );
}

#[test]
fn removing_the_last_best_closes_the_window_and_the_button_goes() {
    // Sunset Sky's only best, 7,200 EXP/h, then back on it with the window open.
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SAMPLE);
    player.hunt(MIN, 30);
    let back = player.arrive(SUNSET_SAMPLE);
    assert!(button(&back));
    assert!(toggle(&mut player).open);

    let removed = player.press(PanelControl::RemoveBest);
    assert!(!button(&removed));
    assert!(!removed.personal_bests.open);
    assert_eq!(removed.personal_bests.placement, None);
}

#[test]
fn without_the_wall_clock_its_bests_have_no_date() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);

    let window = toggle(&mut player);
    assert_eq!(row(&window, SUNSET_SKY_ID).set_at_unix_ms, None);
}

#[test]
fn pressing_the_star_again_closes_it_and_closed_it_holds_no_rows() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    assert!(toggle(&mut player).open);

    let closed = toggle(&mut player);
    assert!(!closed.open);
    assert!(closed.characters.is_empty());

    assert!(toggle(&mut player).open);
    let closed = player.press(PanelControl::ClosePersonalBests).personal_bests;
    assert!(!closed.open);
}

#[test]
fn clicking_a_maps_name_opens_its_meowdb_page() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    assert_eq!(player.minerva.open_bests_map_page(SUNSET_SKY_ID), None, "closed, it lists none");
    toggle(&mut player);

    let url = meowdb::map_url(SUNSET_SKY_ID);
    assert_eq!(player.minerva.open_bests_map_page(SUNSET_SKY_ID), Some(Request::OpenInBrowser { url }));
    assert_eq!(player.minerva.open_bests_map_page(CONSTRUCTION_SITE_ID), None);
}

#[test]
fn the_shell_gets_the_window_and_the_button_as_they_render_them() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    let state = serde_json::to_value(player.hunt(6 * MIN, 20)).unwrap();
    assert_eq!(state["grindTracker"]["personalBests"], true);

    let opened = player.press(PanelControl::TogglePersonalBests);
    // Its rows go to its own window, not with the overlay's state.
    assert!(serde_json::to_value(&opened).unwrap().get("personalBests").is_none());
    let window = serde_json::to_value(&opened.personal_bests).unwrap();
    assert_eq!(window["open"], true);
    let sunset = &window["characters"][0]["maps"][0];
    assert_eq!(sunset["id"], SUNSET_SKY_ID);
    assert_eq!(sunset["streetName"], "Victoria Road");
    assert_eq!(sunset["mapName"], "Sunset Sky");
    assert_eq!(sunset["region"], "Victoria Island");
    assert_eq!(sunset["url"], meowdb::map_url(SUNSET_SKY_ID));
    assert_eq!(sunset["mobLevels"], serde_json::json!({ "lowest": 2, "highest": 14 }));
    assert_eq!(sunset["expPerHour"], 7200);
    assert_eq!(sunset["soloExpCeiling"], 213_571);
    assert_eq!(sunset["ceilingShareHundredths"], 337);
    assert!(sunset["setAtUnixMs"].is_u64());
    assert_eq!(sunset["here"], true);
    let visit = &sunset["visits"][0];
    assert_eq!(visit["expPerHour"], 7200);
    assert_eq!(visit["setAtUnixMs"], sunset["setAtUnixMs"]);
    assert_eq!(visit["ceilingShareHundredths"], 337);
    assert_eq!(visit["thisVisit"], true);
}

// Each map's Visit History.

#[test]
fn each_row_holds_its_maps_visit_history_highest_first_this_visits_best_marked() {
    // Sunset Sky at 7,200 EXP/h, then at 3,600, each Visit ended by stepping onto the
    // Construction Site; then a third Visit, under way, at 10,800.
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET_SAMPLE);
    player.hunt(5 * MIN, 20);
    let first = player.wall_clock();
    player.hunt(MIN, 20);
    player.arrive(CONSTRUCTION_SAMPLE);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(5 * MIN, 10);
    let second = player.wall_clock();
    player.hunt(MIN, 10);
    player.arrive(CONSTRUCTION_SAMPLE);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(5 * MIN, 30);
    let third = player.wall_clock();

    // Each with its share of Sunset Sky's solo ceiling, 213,571 EXP/h. The row's figures
    // are the first's.
    let window = toggle(&mut player);
    assert_eq!(
        row(&window, SUNSET_SKY_ID).visits,
        [
            visit(10_800, third, Some(506), true),
            visit(7200, first, Some(337), false),
            visit(3600, second, Some(169), false),
        ]
    );
    assert_eq!(rows(&window), [(SUNSET_SKY_ID, 10_800, true)]);
}

#[test]
fn this_visits_best_is_listed_only_once_it_makes_its_maps_10() {
    // Ten earlier Visits to Sunset Sky, at 20,000 down to 11,000 EXP/h.
    let earlier: Vec<u64> = (11..=20).rev().map(|k| k * 1000).collect();
    let mut minerva = tracking();
    minerva.restore_bests(histories_of(&[(SUNSET_SKY_ID, earlier.clone())]));
    let mut player = Player::clocked(minerva);
    player.arrive(SUNSET_SAMPLE);

    // 7,200 EXP/h doesn't make them: the window lists the ten.
    player.hunt(6 * MIN, 20);
    let window = toggle(&mut player);
    assert_eq!(visits(&window, SUNSET_SKY_ID), earlier.iter().map(|&exp| (exp, false)).collect::<Vec<_>>());

    // 14,400 does: it is listed among them, and the slowest goes.
    let faster = player.hunt(5 * MIN, 40);
    let listed = visits(&faster.personal_bests, SUNSET_SKY_ID);
    assert_eq!(listed.len(), 10);
    assert_eq!(listed[5..8], [(15_000, false), (14_400, true), (14_000, false)]);
    assert_eq!(listed.last(), Some(&(12_000, false)));
}

#[test]
fn removing_a_visit_takes_exactly_it_on_any_map_and_the_maps_row_follows() {
    // Sunset Sky's Visits at 10,800, 7,200 and 3,600 EXP/h, and the Construction Site's
    // at 5,000; the player on the Construction Site, with no best there yet this Visit.
    let mut minerva = tracking();
    minerva
        .restore_bests(histories_of(&[(SUNSET_SKY_ID, vec![10_800, 7200, 3600]), (CONSTRUCTION_SITE_ID, vec![5000])]));
    let mut player = Player::clocked(minerva);
    player.arrive(CONSTRUCTION_SAMPLE);
    toggle(&mut player);

    // Sunset Sky's middle Visit: its row stays at its best.
    let middle = player.remove_visit(SUNSET_SKY_ID, saved_best(7200));
    assert_eq!(visits(&middle.personal_bests, SUNSET_SKY_ID), [(10_800, false), (3600, false)]);
    assert_eq!(rows(&middle.personal_bests), [(SUNSET_SKY_ID, 10_800, false), (CONSTRUCTION_SITE_ID, 5000, true)]);

    // Its best: the next Visit takes over its row, now under the Construction Site's.
    let best = player.remove_visit(SUNSET_SKY_ID, saved_best(10_800));
    assert_eq!(rows(&best.personal_bests), [(CONSTRUCTION_SITE_ID, 5000, true), (SUNSET_SKY_ID, 3600, false)]);

    // The Construction Site's only Visit: its row goes, though the player is on it.
    let only = player.remove_visit(CONSTRUCTION_SITE_ID, saved_best(5000));
    assert_eq!(rows(&only.personal_bests), [(SUNSET_SKY_ID, 3600, false)]);
    assert!(only.personal_bests.open);
}

#[test]
fn removing_the_best_on_the_map_the_player_is_on_moves_the_best_row_at_once() {
    let mut minerva = tracking();
    minerva.restore_bests(histories_of(&[(SUNSET_SKY_ID, vec![10_800, 7200])]));
    let mut player = Player::clocked(minerva);
    let arrived = player.arrive(SUNSET_SAMPLE);
    assert_eq!(arrived.exp_tracking.personal_best, Some(10_800));
    toggle(&mut player);

    let removed = player.remove_visit(SUNSET_SKY_ID, saved_best(10_800));

    assert_eq!(removed.exp_tracking.personal_best, Some(7200));
    assert_eq!(rows(&removed.personal_bests), [(SUNSET_SKY_ID, 7200, true)]);
}

#[test]
fn this_visits_best_is_removed_at_once_and_only_the_play_since_counts_again() {
    // An earlier Visit to Sunset Sky at 3,600 EXP/h, and this one's at 7,200.
    let mut minerva = tracking();
    minerva.restore_bests(histories_of(&[(SUNSET_SKY_ID, vec![3600])]));
    let mut player = Player::saved_by_the_shell(minerva);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    let window = toggle(&mut player);
    let this_visit = row(&window, SUNSET_SKY_ID).visits[0].clone();
    assert!(this_visit.this_visit);
    let saves = player.saves();

    // Removed: the earlier Visit's best is the row's, here and on the Grind Tracker, and saved.
    let this_one = VisitBest { exp_per_hour: this_visit.exp_per_hour, set_at_unix_ms: this_visit.set_at_unix_ms, ..VisitBest::default() };
    let removed = player.remove_visit(SUNSET_SKY_ID, this_one);
    assert_eq!(visits(&removed.personal_bests, SUNSET_SKY_ID), [(3600, false)]);
    assert_eq!((removed.exp_tracking.personal_best, removed.exp_tracking.personal_best_this_visit), (Some(3600), false));
    assert_eq!(player.saves(), saves + 1);

    // Last 5 min still holds the play before: it isn't counted again, until 5 minutes since.
    let still = player.hunt(4 * MIN + 30 * S, 20);
    assert_eq!(visits(&still.personal_bests, SUNSET_SKY_ID), [(3600, false)]);
    let since = player.hunt(30 * S, 20);
    assert_eq!(visits(&since.personal_bests, SUNSET_SKY_ID), [(7200, true), (3600, false)]);
}

#[test]
fn a_visit_no_longer_listed_cannot_be_removed() {
    // An earlier Visit to Sunset Sky at 3,600 EXP/h, and this one's at 7,200.
    let mut minerva = tracking();
    minerva.restore_bests(histories_of(&[(SUNSET_SKY_ID, vec![3600])]));
    let mut player = Player::saved_by_the_shell(minerva);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    let window = toggle(&mut player);
    let saves = player.saves();

    assert_eq!(player.remove_visit(SUNSET_SKY_ID, saved_best(5000)).personal_bests, window);
    assert_eq!(player.remove_visit(CONSTRUCTION_SITE_ID, saved_best(3600)).personal_bests, window);
    assert_eq!(player.saves(), saves);
}

#[test]
fn removing_the_last_visit_anywhere_closes_the_window_and_the_button_goes() {
    let mut minerva = tracking();
    minerva.restore_bests(bests_of(&[(SUNSET_SKY_ID, 7200)]));
    let mut player = Player::clocked(minerva);
    player.arrive(CONSTRUCTION_SAMPLE);
    assert!(toggle(&mut player).open);

    let removed = player.remove_visit(SUNSET_SKY_ID, saved_best(7200));

    assert!(!button(&removed));
    assert!(!removed.personal_bests.open);
    assert_eq!(removed.personal_bests.placement, None);
}

// Where it goes.

const WINDOW: Size = Size { width: 920, height: 600 };

/// A core restored from `settings` with a best on Sunset Sky, the game window found and
/// the windows' sizes known.
fn started(settings: Settings) -> Minerva {
    started_on(GAME, settings)
}

/// As [`started`], the game's client area at `client`.
fn started_on(client: ScreenRect, settings: Settings) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms_offline());
    minerva.restore(settings);
    minerva.restore_bests(bests_of(&[(SUNSET_SKY_ID, 7200)]));
    minerva.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    minerva.set_grind_tracker_size(Size { width: 300, height: 150 });
    minerva.set_personal_bests_size(WINDOW);
    minerva.on_game_window(game(client));
    common::found_title(&mut minerva, common::title_on(client), 0);
    minerva
}

/// As [`started`], with the Personal Bests window opened.
fn opened(settings: Settings) -> Minerva {
    let mut minerva = started(settings);
    minerva.press(PanelControl::TogglePersonalBests);
    minerva
}

fn placed(state: &PanelState) -> Option<ScreenRect> {
    state.personal_bests.placement
}

/// What the shell does on restart: save the settings as JSON, load them into a new core.
fn restart(minerva: &Minerva) -> Minerva {
    started(common::settings_after_restart(minerva))
}

#[test]
fn it_opens_centred_over_the_game() {
    let minerva = opened(common::set_up());

    assert_eq!(placed(minerva.panel_state()), Some(rect(1312, 410, 920, 600)));
}

#[test]
fn as_its_rows_grow_it_stays_centred() {
    let mut minerva = opened(common::set_up());

    let state = minerva.set_personal_bests_size(Size { width: 920, height: 200 });

    assert_eq!(placed(state), Some(rect(1312, 610, 920, 200)));
}

#[test]
fn over_a_game_narrower_than_it_it_is_as_wide_as_the_game() {
    // A 900x700 client area, its Minimap Title found at (882,150) 620x48.
    let narrow = rect(812, 110, 900, 700);
    let mut minerva = started_on(narrow, common::set_up());

    minerva.press(PanelControl::TogglePersonalBests);

    // Centred at (812,160), 900 wide, it would cover the Minimap Title: it moves below it.
    assert_eq!(placed(minerva.panel_state()), Some(rect(812, 198, 900, 600)));
}

#[test]
fn dragged_it_keeps_its_spot_relative_to_the_game_window_after_a_restart_too() {
    let mut minerva = opened(common::set_up());

    let state = minerva.act(Action::PersonalBestsMoved { x: 1000, y: 300 });
    assert_eq!(placed(state), Some(rect(1000, 300, 920, 600)));

    // The game window moving takes it along.
    let moved = rect(GAME.x + 100, GAME.y + 50, GAME.width, GAME.height);
    assert_eq!(placed(minerva.on_game_window(game(moved))), Some(rect(1100, 350, 920, 600)));

    assert_eq!(minerva.settings().personal_bests_at, Some(ClientOffset { x: 188, y: 190 }));
    let mut restarted = restart(&minerva);
    restarted.press(PanelControl::TogglePersonalBests);
    assert_eq!(placed(restarted.panel_state()), Some(rect(1000, 300, 920, 600)));
}

#[test]
fn dragged_back_to_the_middle_it_stays_centred_again() {
    let mut minerva = opened(common::set_up());
    minerva.act(Action::PersonalBestsMoved { x: 1000, y: 300 });

    // Let go within an eighth of its width (115 px) of the middle (1312,410).
    let state = minerva.act(Action::PersonalBestsMoved { x: 1312 + 100, y: 410 - 60 });
    assert_eq!(placed(state), Some(rect(1312, 410, 920, 600)));
    assert_eq!(minerva.settings().personal_bests_at, None);
}

#[test]
fn dragged_onto_a_captured_region_it_moves_off_it() {
    let mut minerva = opened(common::set_up());
    let area = rect(1817, 1254, 384, 32);
    common::found_exp_figures(&mut minerva, area);

    // Over the EXP figures found in the Status Bar (1817..2201, 1254..1286): our windows
    // show in the capture, so it moves the shortest way off them, here above.
    let state = minerva.act(Action::PersonalBestsMoved { x: 1700, y: 700 });

    assert_eq!(placed(state), Some(rect(1700, 654, 920, 600)));
}

#[test]
fn it_hides_without_the_game_while_setup_is_open_and_while_another_app_is_in_front() {
    let mut minerva = opened(common::set_up());
    let shown = placed(minerva.panel_state());
    assert!(shown.is_some());

    assert_eq!(placed(minerva.on_game_window(None)), None);
    assert!(minerva.panel_state().personal_bests.open, "hidden, not closed");
    minerva.on_game_window(game(GAME));

    minerva.press(PanelControl::Setup);
    assert_eq!(placed(minerva.panel_state()), None);
    minerva.act(Action::CancelSetup);
    assert_eq!(placed(minerva.panel_state()), shown);

    assert_eq!(placed(minerva.on_foreground(Foreground::OtherApp)), None);
    assert_eq!(placed(minerva.on_foreground(Foreground::Minerva)), shown);
}

#[test]
fn it_shows_with_exp_tracking_off() {
    let mut minerva = opened(common::set_up());

    let state = minerva.act(Action::SetGrindTracker { on: false });

    assert_eq!(state.grind_tracker.placement, None);
    assert_eq!(placed(state), Some(rect(1312, 410, 920, 600)));
}

#[test]
fn each_launch_starts_with_it_closed() {
    let minerva = opened(common::set_up());

    let restarted = restart(&minerva);

    assert!(!restarted.panel_state().personal_bests.open);
    assert_eq!(placed(restarted.panel_state()), None);
}
