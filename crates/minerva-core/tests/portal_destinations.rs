//! The hand-kept destinations of portals OSMS gives the dead end, though
//! they lead somewhere. OSMS can't follow a script, so the two ways down into
//! Forgotten Hollow, at the foot of The Tree Tunnel At the Forest Up North and of Sleepy
//! Dungeon V, lead nowhere in its files; Forgotten Hollow's ways back up land at them. With
//! their destinations, routes go down through them, and the Map Preview marks them.
//!
//! The list names the real maps by their IDs, so the hand-built worlds here use those IDs
//! for the entrances and the maps they lead to, under made-up names. The map the player
//! stands on is titled as the Kerning City sample, so arriving with it puts the panel there.

mod common;

use common::osms::DEAD_END;
use common::world::{WorldMap, distances, hunt_orange_on, map, orange, route_of, searched_from_sunset_sky};
use common::{KERNING_SAMPLE, arrive};
use minerva_core::{Minerva, PanelState, PortalKind};

const TREE_TUNNEL: &str = "010002071";
const FORGOTTEN_ENTRANCE_III: &str = "010006060";
const SLEEPY_DUNGEON_V: &str = "010005054";
const FORGOTTEN_DUNGEON_TUNNEL: &str = "010006071";

/// The type OSMS gives both ways down (8): one the Hidden Portal rule doesn't know, and
/// that OSMS gives the dead end wherever it appears.
const TYPE_OF_THE_WAYS_DOWN: u64 = 8;

/// A map, `entrance`, with a portal named `down00` to the dead end, as OSMS has the
/// ways down into Forgotten Hollow.
fn entrance(id: &str, name: &str) -> WorldMap {
    let mut entrance = map(id, name, 100, vec![]);
    entrance.map = entrance.map.portal("down00", TYPE_OF_THE_WAYS_DOWN, DEAD_END);
    entrance
}

/// Sunset Sky leads to the Tree Tunnel, whose way down OSMS gives the dead end; below it,
/// Forgotten Entrance III has Orange Mushrooms and its way back up.
fn down_through_the_tree_tunnel() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&[TREE_TUNNEL]),
        entrance(TREE_TUNNEL, "Tree Tunnel"),
        map(FORGOTTEN_ENTRANCE_III, "Forgotten Entrance III", 100, vec![orange(10)]).to(&[TREE_TUNNEL]),
    ]
}

/// As [`down_through_the_tree_tunnel`], through Sleepy Dungeon V to the Forgotten Dungeon
/// Tunnel.
fn down_through_the_sleepy_dungeon() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&[SLEEPY_DUNGEON_V]),
        entrance(SLEEPY_DUNGEON_V, "Sleepy Dungeon V"),
        map(FORGOTTEN_DUNGEON_TUNNEL, "Forgotten Dungeon Tunnel", 100, vec![orange(10)]).to(&[SLEEPY_DUNGEON_V]),
    ]
}

/// Each Map Preview mark's destination and kind, and whether it is next on the route.
fn marks(state: &PanelState) -> Vec<(&str, PortalKind, bool)> {
    let preview = state.map_preview.as_ref().expect("a Map Preview");
    preview.marks.iter().map(|m| (m.to_map_name.as_str(), m.kind, m.next_on_route)).collect()
}

#[test]
fn a_route_goes_down_into_forgotten_hollow_through_the_tree_tunnel() {
    let state = searched_from_sunset_sky(down_through_the_tree_tunnel());

    assert_eq!(distances(&state), ["Forgotten Entrance III: 2"]);
    assert_eq!(route_of(&state.search.results[0]), "Sunset Sky, Portal: Tree Tunnel, Portal: Forgotten Entrance III");
}

#[test]
fn a_route_goes_down_into_forgotten_hollow_through_the_sleepy_dungeon() {
    let state = searched_from_sunset_sky(down_through_the_sleepy_dungeon());

    assert_eq!(distances(&state), ["Forgotten Dungeon Tunnel: 2"]);
    assert_eq!(
        route_of(&state.search.results[0]),
        "Sunset Sky, Portal: Sleepy Dungeon V, Portal: Forgotten Dungeon Tunnel"
    );
}

#[test]
fn the_map_preview_marks_the_way_down_and_lights_it_on_the_route() {
    // The player stands on the Tree Tunnel, titled as the Kerning City sample is.
    let maps = vec![
        entrance(TREE_TUNNEL, "Kerning City"),
        map(FORGOTTEN_ENTRANCE_III, "Forgotten Entrance III", 100, vec![orange(10)]).to(&[TREE_TUNNEL]),
    ];
    let mut minerva = Minerva::new(common::world::world(maps));
    minerva.tick(0);
    arrive(&mut minerva, KERNING_SAMPLE, 100);

    let state = hunt_orange_on(&mut minerva, FORGOTTEN_ENTRANCE_III);

    assert_eq!(marks(&state), [("Forgotten Entrance III", PortalKind::Visible, true)]);
}

#[test]
fn a_destination_osms_gives_is_kept() {
    // Should OSMS come to follow the script, its destination stands: here, Sunset Sky.
    let mut maps = down_through_the_tree_tunnel();
    maps[1] = map(TREE_TUNNEL, "Tree Tunnel", 100, vec![]);
    maps[1].map = maps[1].map.clone().portal("down00", TYPE_OF_THE_WAYS_DOWN, 100_000_000);

    let state = searched_from_sunset_sky(maps);

    assert_eq!(distances(&state), ["Forgotten Entrance III: NoRoute"]);
}

#[test]
fn a_portal_to_the_dead_end_off_the_list_still_leads_nowhere() {
    let mut maps = down_through_the_tree_tunnel();
    maps[1] = entrance("010002070", "Tree Tunnel");
    maps[0] = map("100000000", "Sunset Sky", 100, vec![]).to(&["010002070"]);

    let state = searched_from_sunset_sky(maps);

    assert_eq!(distances(&state), ["Forgotten Entrance III: NoRoute"]);
}

