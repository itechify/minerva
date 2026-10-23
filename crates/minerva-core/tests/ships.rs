//! Ships: a ride between regions (Southperry → Lith Harbor) is always one map change, so
//! maps past it have a Distance from Maple Island. Each result has its route (the maps in
//! order, each with how it is reached) and says which kinds of travel besides portals
//! that route uses. No ship sails into Ossyria while it isn't in the game.
//!
//! The ships run between the real stations (by their map IDs), so the hand-built
//! worlds here use those IDs for the stations.

mod common;

use common::osms;
use common::world::{WorldMap, map, orange, search_for, distances, route_of};
use common::{FakeSource, arrive, SUNSET_SAMPLE, KERNING_SAMPLE};
use minerva_core::{Minerva, Distance, PanelState, PortalKind, Step};

const SOUTHPERRY: &str = "000000060";
const LITH_HARBOR: &str = "010000000";
const ELLINIA_STATION: &str = "010002090";
const ORBIS_TICKETING_BOOTH: &str = "020000010";

/// A core over `maps`, its file set in hand, with the player arrived on the map named
/// Sunset Sky and the Orange Mushroom picked.
fn searched_from_sunset_sky(maps: Vec<WorldMap>) -> PanelState {
    let mut minerva = Minerva::new(common::world::world(maps));
    minerva.tick(0);
    arrive(&mut minerva, SUNSET_SAMPLE, 100);
    search_for(&mut minerva, "orange")
}

/// Sunset Sky leads to Southperry; Shanks sails on to Lith Harbor, next to the Mushroom
/// Field.
fn by_ship_to_the_mushroom_field() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&[SOUTHPERRY]),
        map(SOUTHPERRY, "Southperry", 100, vec![]),
        map(LITH_HARBOR, "Lith Harbor", 100, vec![]).to(&["100000001"]),
        map("100000001", "Mushroom Field", 100, vec![orange(10)]),
    ]
}

#[test]
fn a_ship_ride_is_one_map_change() {
    let state = searched_from_sunset_sky(by_ship_to_the_mushroom_field());

    assert_eq!(distances(&state), ["Mushroom Field: 3"]);
}

#[test]
fn a_result_reached_by_ship_has_its_route_and_says_it_takes_a_ship() {
    let state = searched_from_sunset_sky(by_ship_to_the_mushroom_field());

    let result = &state.search.results[0];
    assert_eq!(route_of(result), "Sunset Sky, Portal: Southperry, Ship: Lith Harbor, Portal: Mushroom Field");
    assert_eq!(result.travel, [Step::Ship]);
}

#[test]
fn the_ship_runs_from_the_real_southperry_to_the_real_lith_harbor() {
    // The game's map list (the map names fixture), with a made-up portal from Kerning
    // City to Southperry to keep the route short, and Orange Mushrooms in Lith Harbor.
    let world = osms::map_names()
        .with_mob(osms::mob(9, "Orange Mushroom", 8, 0, 0))
        .changing(10_003_000, |m| m.portal("toSouthperry", 2, 60))
        .changing(10_000_000, |m| m.spawns(9, 10));
    let mut minerva = Minerva::new(FakeSource::with_world(world));
    minerva.tick(0);
    arrive(&mut minerva, KERNING_SAMPLE, 100);

    let state = search_for(&mut minerva, "orange");

    let result = &state.search.results[0];
    assert_eq!(result.distance, Distance::MapChanges(2));
    assert_eq!(route_of(result), "Kerning City, Portal: Southperry, Ship: Lith Harbor");
    assert_eq!(result.travel, [Step::Ship]);
}

#[test]
fn no_ship_sails_between_ellinia_station_and_orbis_while_ossyria_is_out_of_the_game() {
    let state = searched_from_sunset_sky(vec![
        map(ELLINIA_STATION, "Sunset Sky", 100, vec![]),
        map(ORBIS_TICKETING_BOOTH, "Orbis Ticketing Booth", 100, vec![orange(10)]),
    ]);
    assert_eq!(distances(&state), ["Orbis Ticketing Booth: NoRoute"]);

    let state = searched_from_sunset_sky(vec![
        map(ORBIS_TICKETING_BOOTH, "Sunset Sky", 100, vec![]),
        map(ELLINIA_STATION, "Ellinia Station", 100, vec![orange(10)]),
    ]);
    assert_eq!(distances(&state), ["Ellinia Station: NoRoute"]);
}

#[test]
fn shanks_sails_from_southperry_to_lith_harbor_and_no_ship_sails_back() {
    let state = searched_from_sunset_sky(vec![
        map(SOUTHPERRY, "Sunset Sky", 100, vec![orange(10)]),
        map(LITH_HARBOR, "Lith Harbor", 100, vec![orange(10)]),
    ]);
    assert_eq!(distances(&state), ["Sunset Sky: 0", "Lith Harbor: 1"]);
    assert_eq!(state.search.results[1].travel, [Step::Ship]);

    let state = searched_from_sunset_sky(vec![
        map(SOUTHPERRY, "Southperry", 100, vec![orange(10)]),
        map(LITH_HARBOR, "Sunset Sky", 100, vec![orange(10)]),
    ]);
    assert_eq!(distances(&state), ["Sunset Sky: 0", "Southperry: NoRoute"]);
}

#[test]
fn a_result_reached_by_portals_alone_has_its_route_and_no_travel_marker() {
    let state = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Road", 100, vec![]).to(&["100000002"]),
        map("100000002", "Mushroom Field", 100, vec![orange(10)]),
    ]);

    let result = &state.search.results[0];
    assert_eq!(route_of(result), "Sunset Sky, Portal: Road, Portal: Mushroom Field");
    assert_eq!(result.travel, []);
}

#[test]
fn of_two_equally_short_routes_the_one_by_portals_is_taken() {
    // Southperry leads straight to Lith Harbor by ship, and by a (made-up) portal too.
    let state = searched_from_sunset_sky(vec![
        map(SOUTHPERRY, "Sunset Sky", 100, vec![]).to(&[LITH_HARBOR]),
        map(LITH_HARBOR, "Lith Harbor", 100, vec![orange(10)]),
    ]);

    let result = &state.search.results[0];
    assert_eq!(route_of(result), "Sunset Sky, Portal: Lith Harbor");
    assert_eq!(result.travel, []);
}

#[test]
fn on_the_map_with_the_mob_the_route_is_that_map_alone() {
    let state = searched_from_sunset_sky(vec![map("100000000", "Sunset Sky", 100, vec![orange(10)])]);

    assert_eq!(route_of(&state.search.results[0]), "Sunset Sky");
}

#[test]
fn a_map_with_no_route_or_only_a_quest_route_has_no_route_shown() {
    let state = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).exit(PortalKind::Quest, "100000001"),
        map("100000001", "Quest Mushrooms", 100, vec![orange(10)]),
        map("100000002", "Lost Mushrooms", 100, vec![orange(10)]),
    ]);

    assert_eq!(distances(&state), ["Lost Mushrooms: NoRoute", "Quest Mushrooms: QuestOnly"]);
    assert!(state.search.results.iter().all(|r| r.route.is_empty() && r.travel.is_empty()));
}

#[test]
fn with_no_map_recognised_no_route_is_shown() {
    let mut minerva = Minerva::new(common::world::world(vec![
        map(SOUTHPERRY, "Southperry", 100, vec![]),
        map(LITH_HARBOR, "Lith Harbor", 100, vec![orange(10)]),
    ]));
    minerva.tick(0);

    let state = search_for(&mut minerva, "orange");

    assert_eq!(distances(&state), ["Lith Harbor: Unknown"]);
    assert!(state.search.results[0].route.is_empty());
    assert!(state.search.results[0].travel.is_empty());
}
