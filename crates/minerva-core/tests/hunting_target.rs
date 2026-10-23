//! The Hunting Target: the mob and map the player picked in the search view. The panel's
//! Navigation section shows the mob as OSMS animates it, the map's Density Tier, Density,
//! Spawn Share and Distance, and the route there from the player's current map,
//! recomputed as they move. When they arrive it says so, with the trip as it stood when
//! they picked it, and it goes away once they leave that map, or when they clear it with ×.
//!
//! The maps the player stands on are titled like the minimap samples (Sunset Sky,
//! Kerning City), so arriving with that sample puts the panel on them.

mod common;

use common::world::{WorldMap, map, mob, orange, osms_world, world, sharing, on_sunset_sky, on_sunset_sky_from, hunt_orange_on, highlighted_map, stops};
use common::{FakeSource, arrive, osms, SUNSET_SAMPLE, KERNING_SAMPLE, shown_id};
use minerva_core::{Action, DensityTier, Distance, HuntingTarget, PanelState, SearchView, Step};

/// Sunset Sky leads to Kerning City, Kerning City to the Mushroom Field, where Orange
/// Mushrooms are a third as dense as on the Mushroom Grove, which is out of reach.
fn mushroom_roads() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Kerning City", 100, vec![]).to(&["100000000", "100000002"]),
        map("100000002", "Mushroom Field", 150, vec![sharing(40, orange(4)), mob("4", "Shroom", 2, 6)]),
        map("100000003", "Mushroom Grove", 100, vec![orange(18)]),
    ]
}

fn target(state: &PanelState) -> &HuntingTarget {
    state.search.hunting.as_ref().expect("a Hunting Target")
}

/// The target's route, as its map names with how each is reached.
fn route(state: &PanelState) -> Vec<String> {
    stops(&target(state).map.route)
}

#[test]
fn the_navigation_section_shows_the_mob_and_map_with_its_figures() {
    let mut minerva = on_sunset_sky(mushroom_roads());

    let state = hunt_orange_on(&mut minerva, "100000002");

    let target = target(&state);
    assert_eq!((target.mob.name.as_str(), target.mob.level), ("Orange Mushroom", 8));
    assert_eq!((target.map.street_name.as_str(), target.map.map_name.as_str()), ("Victoria Road", "Mushroom Field"));
    assert_eq!(target.map.tier, DensityTier::Medium, "6 is a third of the Grove's 18");
    assert_eq!(target.map.density_hundredths, 600, "4 spawns at a 1.5x Mob Rate");
    assert_eq!(target.map.spawn_share_percent, 40);
    assert_eq!(target.map.distance, Distance::MapChanges(2));
}

/// The mob is shown as its Mob Card shows it: OSMS's default animation, asked for as a
/// picture and loaded from where the source says it is.
#[test]
fn the_hunting_target_shows_the_mobs_animation_once_it_is_in() {
    let mut minerva = on_sunset_sky(mushroom_roads());

    let state = hunt_orange_on(&mut minerva, "100000002");
    assert_eq!(target(&state).image, None, "not asked for yet");
    let state = minerva.tick(200).clone();

    assert_eq!(target(&state).image.as_deref(), Some("picture:images/monsters/gif9.webp"));
}

/// The panel's own pictures come first: the shown map's, then its mobs' animations,
/// then the Hunting Target mob's.
#[test]
fn the_mobs_animation_is_asked_for_after_the_maps_picture_and_its_mobs() {
    let source = world(vec![
        map("100000000", "Sunset Sky", 100, vec![mob("4", "Shroom", 2, 6)]).to(&["100000001"]),
        map("100000001", "Mushroom Field", 100, vec![orange(4)]),
    ]);
    let requests = source.picture_requests.clone();
    let mut minerva = on_sunset_sky_from(source);
    hunt_orange_on(&mut minerva, "100000001");

    minerva.tick(200);

    assert_eq!(
        *requests.lock().unwrap(),
        ["maps/picture100000000.webp", "images/monsters/gif4.webp", "images/monsters/gif9.webp"]
    );
}

#[test]
fn a_mob_osms_has_no_animation_for_shows_none() {
    let world = osms_world(mushroom_roads()).with_mob(osms::mob(9, "Orange Mushroom", 8, 0, 0).without_animation());
    let mut minerva = on_sunset_sky_from(FakeSource::with_world(world));
    hunt_orange_on(&mut minerva, "100000002");

    let state = minerva.tick(200);

    assert_eq!(target(state).image, None);
}

#[test]
fn the_mobs_animation_stays_once_the_player_has_arrived() {
    let mut minerva = on_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Kerning City", 100, vec![orange(10)]).to(&["100000000"]),
    ]);
    hunt_orange_on(&mut minerva, "100000001");
    minerva.tick(200);

    let state = arrive(&mut minerva, KERNING_SAMPLE, 1_000);

    assert!(target(&state).arrived);
    assert_eq!(target(&state).image.as_deref(), Some("picture:images/monsters/gif9.webp"));
}

#[test]
fn the_route_goes_from_the_current_map_and_is_worked_out_again_as_the_player_moves() {
    let mut minerva = on_sunset_sky(mushroom_roads());

    let state = hunt_orange_on(&mut minerva, "100000002");
    assert_eq!(route(&state), ["Sunset Sky", "Portal: Kerning City", "Portal: Mushroom Field"]);

    let state = arrive(&mut minerva, KERNING_SAMPLE, 1_000);
    assert_eq!(target(&state).map.distance, Distance::MapChanges(1));
    assert_eq!(route(&state), ["Kerning City", "Portal: Mushroom Field"]);
}

#[test]
fn a_target_with_no_route_says_so_and_the_travel_toggles_count_at_once() {
    let henesys = "010001000";
    let mut minerva = on_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]),
        map(henesys, "Henesys", 100, vec![orange(10)]),
    ]);

    let state = hunt_orange_on(&mut minerva, henesys);
    assert_eq!(target(&state).map.distance, Distance::NoRoute);
    assert!(route(&state).is_empty());

    let state = minerva.act(Action::ToggleTeleportScrolls).clone();
    assert_eq!(route(&state), ["Sunset Sky", "TeleportScroll: Henesys"]);
    assert_eq!(target(&state).map.travel, [Step::TeleportScroll]);
}

#[test]
fn arriving_on_the_target_map_says_so_with_the_trip_until_the_player_leaves_it() {
    let mut minerva = on_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Kerning City", 100, vec![orange(10)]).to(&["100000000"]),
    ]);
    let state = hunt_orange_on(&mut minerva, "100000001");
    assert!(!target(&state).arrived);

    let state = arrive(&mut minerva, KERNING_SAMPLE, 1_000);
    let here = target(&state);
    assert!(here.arrived);
    assert_eq!(here.map.distance, Distance::MapChanges(0));
    let trip = here.trip.as_ref().expect("the trip as it stood when picked");
    assert_eq!((trip.from.as_str(), trip.map_changes), ("Sunset Sky", 1));
    assert!(trip.travel.is_empty(), "by portals alone");

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 2_000);
    assert_eq!(state.search.hunting, None);
}

#[test]
fn the_trip_names_the_travel_it_took() {
    let henesys = "010001000";
    let mut minerva = on_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]),
        map(henesys, "Henesys", 100, vec![orange(10)]),
    ]);
    minerva.act(Action::ToggleTeleportScrolls);

    let state = hunt_orange_on(&mut minerva, henesys);

    let trip = target(&state).trip.as_ref().expect("a trip");
    assert_eq!((trip.map_changes, trip.travel.as_slice()), (1, [Step::TeleportScroll].as_slice()));
}

#[test]
fn picking_the_map_the_player_is_on_keeps_it_until_they_leave_and_come_back() {
    let mut minerva = on_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![orange(10)]).to(&["100000001"]),
        map("100000001", "Kerning City", 100, vec![]).to(&["100000000"]),
    ]);

    let state = hunt_orange_on(&mut minerva, "100000000");
    assert_eq!(target(&state).map.distance, Distance::MapChanges(0));
    assert!(!target(&state).arrived, "they haven't gone anywhere");
    assert_eq!(target(&state).trip, None);
    let state = arrive(&mut minerva, KERNING_SAMPLE, 1_000);
    assert!(state.search.hunting.is_some());

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 2_000);
    assert!(target(&state).arrived);
    let state = arrive(&mut minerva, KERNING_SAMPLE, 3_000);
    assert_eq!(state.search.hunting, None);
}

#[test]
fn with_candidates_the_player_arrives_once_the_candidate_shown_is_the_target() {
    // Two maps titled Kerning City, told apart by their mobs; the Orange Mushrooms are on
    // the second.
    let mut minerva = on_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]),
        map("100000001", "Kerning City", 100, vec![mob("3", "Blue Snail", 4, 1)]),
        map("100000002", "Kerning City", 100, vec![orange(10)]),
    ]);
    hunt_orange_on(&mut minerva, "100000002");

    let state = arrive(&mut minerva, KERNING_SAMPLE, 1_000);
    assert!(state.candidates.is_some());
    assert_eq!(shown_id(&state), Some("100000001"));
    assert!(state.search.hunting.is_some(), "the candidate shown isn't the target");

    let state = minerva.act(Action::ChooseCandidate);
    assert_eq!(shown_id(state), Some("100000002"));
    assert!(target(state).arrived);
}

#[test]
fn the_sections_x_clears_the_hunting_target() {
    let mut minerva = on_sunset_sky(mushroom_roads());
    hunt_orange_on(&mut minerva, "100000002");

    let state = minerva.act(Action::ClearHuntingTarget);

    assert_eq!(state.search.hunting, None);
    assert_eq!(state.search.credit, None);
}

#[test]
fn the_sections_x_clears_it_once_the_player_has_arrived_too() {
    let mut minerva = on_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Kerning City", 100, vec![orange(10)]),
    ]);
    hunt_orange_on(&mut minerva, "100000001");
    arrive(&mut minerva, KERNING_SAMPLE, 1_000);

    let state = minerva.act(Action::ClearHuntingTarget);

    assert_eq!(state.search.hunting, None);
}

/// Orange Mushrooms on the Near Field, next to Sunset Sky, and the denser Far Field.
fn near_and_far_fields() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Near Field", 100, vec![orange(5)]).to(&["100000002"]),
        map("100000002", "Far Field", 100, vec![orange(10)]),
    ]
}

#[test]
fn the_sections_search_button_reopens_the_mobs_map_list_on_the_target_map() {
    let mut minerva = on_sunset_sky(near_and_far_fields());
    hunt_orange_on(&mut minerva, "100000001");

    let state = minerva.act(Action::ReopenMapList).clone();

    assert_eq!(state.search.view, Some(SearchView::Maps));
    assert_eq!(state.search.picked.as_ref().map(|m| m.name.as_str()), Some("Orange Mushroom"));
    assert_eq!(state.search.results[0].map_name, "Far Field");
    assert_eq!(highlighted_map(&state), Some("Near Field"), "the current target");
    assert_eq!(target(&state).map.map_name, "Near Field", "until another map is picked");
}

#[test]
fn another_map_picked_from_the_reopened_list_becomes_the_target() {
    let mut minerva = on_sunset_sky(near_and_far_fields());
    hunt_orange_on(&mut minerva, "100000001");
    minerva.act(Action::ReopenMapList);

    minerva.act(Action::SearchPrevious);
    let state = minerva.act(Action::SearchEnter);

    assert_eq!(state.search.view, None);
    assert_eq!(target(state).map.map_name, "Far Field");
}

#[test]
fn esc_from_the_reopened_list_goes_back_to_an_empty_search_and_keeps_the_target() {
    let mut minerva = on_sunset_sky(near_and_far_fields());
    let before = hunt_orange_on(&mut minerva, "100000001");
    minerva.act(Action::ReopenMapList);
    minerva.act(Action::SearchPrevious);

    let state = minerva.act(Action::SearchEscape).clone();

    assert_eq!((state.search.view, state.search.query.as_str()), (Some(SearchView::Matches), ""));
    assert_eq!(state.search.hunting, before.search.hunting);
    assert_eq!(*minerva.act(Action::SearchEscape), before);
}

#[test]
fn without_a_target_there_is_no_map_list_to_reopen() {
    let mut minerva = on_sunset_sky(near_and_far_fields());

    let state = minerva.act(Action::ReopenMapList);

    assert_eq!(state.search.view, None);
}

#[test]
fn the_hunting_target_is_kept_in_memory_only() {
    let mut minerva = on_sunset_sky(mushroom_roads());
    hunt_orange_on(&mut minerva, "100000002");

    let json = serde_json::to_string(&minerva.settings()).unwrap();

    assert!(!json.contains("100000002") && !json.to_lowercase().contains("hunting"), "{json}");
}
