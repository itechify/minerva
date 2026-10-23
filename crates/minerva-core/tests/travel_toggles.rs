//! The Taxi and Teleport Scroll toggles: both off by default and remembered between
//! sessions. With one on, Distances, ranking, the headline and routes take it into
//! account straight away.
//!
//! The taxis run between the Victoria Island towns and the scrolls go to the named
//! towns, by their real map IDs, so the hand-built worlds here use those IDs for the
//! towns. No scroll goes to Orbis or El Nath while Ossyria isn't in the game.

mod common;

use common::world::{WorldMap, map, nearest, orange, search_for, world, on_sunset_sky, distances, route_of};
use minerva_core::{Action, Minerva, Distance, Settings, Step, Travel};

/// A core over `maps`, its file set in hand, with the player arrived on the map named
/// Sunset Sky and the Orange Mushroom picked.
fn searched_from_sunset_sky(maps: Vec<WorldMap>) -> Minerva {
    let mut minerva = on_sunset_sky(maps);
    search_for(&mut minerva, "orange");
    minerva
}

#[test]
fn both_toggles_are_off_by_default() {
    let minerva = Minerva::new(world(vec![]));

    let settings = minerva.settings();
    assert!(!settings.travel.taxis);
    assert!(!settings.travel.teleport_scrolls);
    let state = minerva.panel_state();
    assert!(!state.search.travel.taxis);
    assert!(!state.search.travel.teleport_scrolls);
}

#[test]
fn a_settings_file_from_before_the_toggles_has_both_off() {
    let before_toggles = r#"{"version":4,"hotkeys":{"togglePanel":"ctrl+shift+x"}}"#;
    let mut minerva = Minerva::new(world(vec![]));

    let state = minerva.restore(serde_json::from_str(before_toggles).unwrap()).clone();

    assert!(!state.search.travel.taxis && !state.search.travel.teleport_scrolls);
    let settings = minerva.settings();
    assert!(!settings.travel.taxis && !settings.travel.teleport_scrolls, "and saved off");
}

#[test]
fn flipped_toggles_are_saved_and_restored_next_session() {
    let mut minerva = Minerva::new(world(vec![]));
    minerva.act(Action::ToggleTaxis);
    minerva.act(Action::ToggleTeleportScrolls);
    let json = serde_json::to_string(&minerva.settings()).unwrap();
    assert!(json.contains(r#""taxis":true"#) && json.contains(r#""teleportScrolls":true"#), "{json}");

    let mut restarted = Minerva::new(world(vec![]));
    let state = restarted.restore(serde_json::from_str::<Settings>(&json).unwrap()).clone();

    assert!(state.search.travel.taxis);
    assert!(state.search.travel.teleport_scrolls);
    assert!(restarted.settings().travel.taxis && restarted.settings().travel.teleport_scrolls);

    let state = restarted.act(Action::ToggleTaxis);
    assert!(!state.search.travel.taxis, "flipped back off");
    assert!(state.search.travel.teleport_scrolls);
    assert!(!restarted.settings().travel.taxis);
}

const LITH_HARBOR: &str = "010000000";
const HENESYS: &str = "010001000";
const ELLINIA: &str = "010002000";
const KERNING_CITY: &str = "010003000";
const PERION: &str = "010004000";
const SLEEPYWOOD: &str = "010005000";
const ORBIS: &str = "020000000";
const EL_NATH: &str = "020001000";

/// Henesys is next to Sunset Sky; Ellinia, with the mob, is three roads on from it.
fn henesys_then_the_long_way_to_ellinia() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&[HENESYS]),
        map(HENESYS, "Henesys", 100, vec![]).to(&["100000001"]),
        map("100000001", "Road One", 100, vec![]).to(&["100000002"]),
        map("100000002", "Road Two", 100, vec![]).to(&[ELLINIA]),
        map(ELLINIA, "Ellinia", 100, vec![orange(10)]),
    ]
}

#[test]
fn a_taxi_between_towns_shortens_a_route_only_while_taxis_are_on() {
    let mut minerva = searched_from_sunset_sky(henesys_then_the_long_way_to_ellinia());
    let state = minerva.panel_state();
    assert_eq!(distances(state), ["Ellinia: 4"]);
    assert_eq!(state.search.results[0].travel, []);

    let state = minerva.act(Action::ToggleTaxis).clone();

    assert_eq!(distances(&state), ["Ellinia: 2"]);
    assert_eq!(route_of(&state.search.results[0]), "Sunset Sky, Portal: Henesys, Taxi: Ellinia");
    assert_eq!(state.search.results[0].travel, [Step::Taxi]);

    let state = minerva.act(Action::ToggleTaxis).clone();

    assert_eq!(distances(&state), ["Ellinia: 4"]);
    assert_eq!(
        route_of(&state.search.results[0]),
        "Sunset Sky, Portal: Henesys, Portal: Road One, Portal: Road Two, Portal: Ellinia"
    );
    assert_eq!(state.search.results[0].travel, []);
}

#[test]
fn taxis_run_between_every_pair_of_victoria_island_towns() {
    let towns = [LITH_HARBOR, HENESYS, ELLINIA, KERNING_CITY, PERION];
    for from in towns {
        let mut maps = vec![map(from, "Sunset Sky", 100, vec![])];
        maps.extend(towns.iter().filter(|&&to| to != from).map(|&to| map(to, to, 100, vec![orange(10)])));
        let mut minerva = searched_from_sunset_sky(maps);

        let state = minerva.act(Action::ToggleTaxis);

        assert_eq!(state.search.results.len(), 4);
        let all_one = state.search.results.iter().all(|r| r.distance == Distance::MapChanges(1));
        assert!(all_one, "from {from}: {:?}", distances(state));
    }
}

#[test]
fn no_taxi_runs_to_sleepywood() {
    let mut minerva = searched_from_sunset_sky(vec![
        map(HENESYS, "Sunset Sky", 100, vec![]),
        map(SLEEPYWOOD, "Sleepywood", 100, vec![orange(10)]),
    ]);

    let state = minerva.act(Action::ToggleTaxis);

    assert_eq!(distances(state), ["Sleepywood: NoRoute"]);
}

#[test]
fn of_two_equally_short_routes_the_one_by_portals_is_taken_over_a_taxi() {
    let mut minerva = searched_from_sunset_sky(vec![
        map(HENESYS, "Sunset Sky", 100, vec![]).to(&[ELLINIA]),
        map(ELLINIA, "Ellinia", 100, vec![orange(10)]),
    ]);

    let state = minerva.act(Action::ToggleTaxis);

    assert_eq!(route_of(&state.search.results[0]), "Sunset Sky, Portal: Ellinia");
    assert_eq!(state.search.results[0].travel, []);
}

#[test]
fn a_teleport_scroll_takes_the_player_from_any_map_to_each_named_town_only_while_scrolls_are_on() {
    let towns = [
        (HENESYS, "Henesys"),
        (ELLINIA, "Ellinia"),
        (PERION, "Perion"),
        (KERNING_CITY, "Kerning City"),
        (LITH_HARBOR, "Lith Harbor"),
        (SLEEPYWOOD, "Sleepywood"),
    ];
    let mut maps = vec![map("100000000", "Sunset Sky", 100, vec![])];
    maps.extend(towns.iter().map(|&(id, name)| map(id, name, 100, vec![orange(10)])));
    maps.push(map("100000001", "Mushroom Field", 100, vec![orange(10)]));
    let mut minerva = searched_from_sunset_sky(maps);
    assert!(minerva.panel_state().search.results.iter().all(|r| r.distance == Distance::NoRoute));

    let state = minerva.act(Action::ToggleTeleportScrolls).clone();

    for (_, name) in towns {
        let result = state.search.results.iter().find(|r| r.map_name == name).unwrap();
        assert_eq!(result.distance, Distance::MapChanges(1), "{name}");
        assert_eq!(route_of(result), format!("Sunset Sky, TeleportScroll: {name}"));
        assert_eq!(result.travel, [Step::TeleportScroll]);
    }
    let field = state.search.results.iter().find(|r| r.map_name == "Mushroom Field").unwrap();
    assert_eq!(field.distance, Distance::NoRoute, "no scroll goes to a map that isn't a named town");

    let state = minerva.act(Action::ToggleTeleportScrolls);

    assert!(state.search.results.iter().all(|r| r.distance == Distance::NoRoute && r.travel.is_empty()));
}

#[test]
fn no_teleport_scroll_goes_to_orbis_or_el_nath_while_ossyria_is_out_of_the_game() {
    let mut minerva = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]),
        map(ORBIS, "Orbis", 100, vec![orange(10)]),
        map(EL_NATH, "El Nath", 100, vec![orange(10)]),
    ]);

    let state = minerva.act(Action::ToggleTeleportScrolls);

    assert_eq!(distances(state), ["El Nath: NoRoute", "Orbis: NoRoute"]);
}

/// Near Field is two roads from Sunset Sky; Perion is four.
fn near_field_and_far_perion() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001", "100000003"]),
        map("100000001", "Road", 100, vec![]).to(&["100000002"]),
        map("100000002", "Near Field", 100, vec![orange(10)]),
        map("100000003", "Long Road One", 100, vec![]).to(&["100000004"]),
        map("100000004", "Long Road Two", 100, vec![]).to(&["100000005"]),
        map("100000005", "Long Road Three", 100, vec![]).to(&[PERION]),
        map(PERION, "Perion", 100, vec![orange(10)]),
    ]
}

#[test]
fn flipping_a_toggle_reranks_the_results_and_moves_the_headline_straight_away() {
    let mut minerva = searched_from_sunset_sky(near_field_and_far_perion());
    let state = minerva.panel_state();
    assert_eq!(distances(state), ["Near Field: 2", "Perion: 4"]);
    assert_eq!(nearest(state).unwrap().map_name, "Near Field");

    let state = minerva.act(Action::ToggleTeleportScrolls).clone();

    assert_eq!(distances(&state), ["Perion: 1", "Near Field: 2"]);
    let perion = nearest(&state).unwrap();
    assert_eq!(perion.map_name, "Perion");
    assert_eq!(perion.travel, [Step::TeleportScroll]);

    let state = minerva.act(Action::ToggleTeleportScrolls);

    assert_eq!(distances(state), ["Near Field: 2", "Perion: 4"]);
    assert_eq!(nearest(state).unwrap().map_name, "Near Field");
}

#[test]
fn restored_toggles_count_in_the_distances_at_once() {
    let mut minerva = searched_from_sunset_sky(near_field_and_far_perion());

    let state = minerva.restore(Settings { travel: Travel { teleport_scrolls: true, taxis: false }, ..Settings::default() });

    assert!(state.search.travel.teleport_scrolls);
    assert_eq!(distances(state), ["Perion: 1", "Near Field: 2"]);
}

#[test]
fn a_nearest_town_scroll_takes_the_player_to_their_own_maps_nearest_town() {
    let mut minerva = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).returning_to("100000010"),
        map("100000010", "Amherst", 100, vec![orange(10)]),
        // Another map's nearest town is no nearer.
        map("100000030", "Far Field", 100, vec![]).returning_to("100000020"),
        map("100000020", "Other Town", 100, vec![orange(10)]),
    ]);
    assert_eq!(distances(minerva.panel_state()), ["Amherst: NoRoute", "Other Town: NoRoute"]);

    let state = minerva.act(Action::ToggleTeleportScrolls).clone();

    assert_eq!(distances(&state), ["Amherst: 1", "Other Town: NoRoute"]);
    assert_eq!(route_of(&state.search.results[0]), "Sunset Sky, TeleportScroll: Amherst");
    assert_eq!(state.search.results[0].travel, [Step::TeleportScroll]);
    assert_eq!(nearest(&state).unwrap().map_name, "Amherst");
}

/// OSMS gives no nearest town for a few maps that aren't Towns: a nearest-town scroll
/// takes the player nowhere from them.
#[test]
fn from_a_map_osms_gives_no_nearest_town_a_nearest_town_scroll_goes_nowhere() {
    let mut minerva = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]),
        map("100000010", "Amherst", 100, vec![orange(10)]),
    ]);

    let state = minerva.act(Action::ToggleTeleportScrolls).clone();

    assert_eq!(distances(&state), ["Amherst: NoRoute"]);
}

#[test]
fn a_nearest_town_scroll_can_be_read_partway_along_a_route() {
    let mut minerva = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).returning_to("100000000").to(&["100000001"]),
        map("100000001", "Deep Cave", 100, vec![]).returning_to("100000010"),
        map("100000010", "Amherst", 100, vec![]).to(&["100000011"]),
        map("100000011", "Amherst Field", 100, vec![orange(10)]),
    ]);

    let state = minerva.act(Action::ToggleTeleportScrolls);

    assert_eq!(
        route_of(&state.search.results[0]),
        "Sunset Sky, Portal: Deep Cave, TeleportScroll: Amherst, Portal: Amherst Field"
    );
}
