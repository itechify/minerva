//! While there is a Hunting Target the player hasn't reached, and the route's next step
//! from the shown map is through a portal, the Map Preview marks every portal on the shown
//! map that leads to the route's next map as next on the route. A Quest Portal there isn't:
//! routes never go through one. A next step by ship, taxi or Teleport Scroll marks nothing.
//!
//! The maps the player stands on are titled like the minimap samples (Sunset Sky,
//! Kerning City), so arriving with that sample puts the panel on them.

mod common;

use common::{arrive, SUNSET_SAMPLE, KERNING_SAMPLE};
use common::world::{WorldMap, map, orange, world, hunt_orange_on};
use minerva_core::{Action, Minerva, PanelState, PortalKind};

/// Sunset Sky leads to Kerning City by two portals and a Quest Portal, and to the Back
/// Street by a Hidden Portal; Kerning City leads on to the Mushroom Field and back to
/// Sunset Sky. Orange Mushrooms are on Kerning City and the Mushroom Field.
fn roads_to_the_mushrooms() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![])
            .exit(PortalKind::Visible, "100000001")
            .exit(PortalKind::Hidden, "100000003")
            .exit(PortalKind::Visible, "100000001")
            .exit(PortalKind::Quest, "100000001"),
        map("100000001", "Kerning City", 100, vec![orange(2)]).to(&["100000000", "100000002"]),
        map("100000002", "Mushroom Field", 100, vec![orange(4)]),
        map("100000003", "Back Street", 100, vec![]),
    ]
}

/// A core over `maps`, its file set in hand, with the player arrived on Sunset Sky.
fn on_sunset_sky(maps: Vec<WorldMap>) -> Minerva {
    let mut minerva = Minerva::new(world(maps));
    minerva.tick(0);
    arrive(&mut minerva, SUNSET_SAMPLE, 100);
    minerva
}

/// Each mark's destination and kind, and whether it is next on the route.
fn marks(state: &PanelState) -> Vec<(&str, PortalKind, bool)> {
    let preview = state.map_preview.as_ref().expect("a Map Preview");
    preview.marks.iter().map(|m| (m.to_map_name.as_str(), m.kind, m.next_on_route)).collect()
}

/// The destinations of the marks next on the route.
fn next_on_route(state: &PanelState) -> Vec<&str> {
    marks(state).into_iter().filter(|&(_, _, next)| next).map(|(to, _, _)| to).collect()
}

#[test]
fn every_portal_to_the_routes_next_map_is_marked_but_a_quest_portal() {
    let mut minerva = on_sunset_sky(roads_to_the_mushrooms());

    let state = hunt_orange_on(&mut minerva, "100000002");

    assert_eq!(
        marks(&state),
        [
            ("Kerning City", PortalKind::Visible, true),
            ("Back Street", PortalKind::Hidden, false),
            ("Kerning City", PortalKind::Visible, true),
            ("Kerning City", PortalKind::Quest, false),
        ]
    );
}

#[test]
fn without_a_hunting_target_no_portal_is_next_on_the_route() {
    let mut minerva = on_sunset_sky(roads_to_the_mushrooms());

    let state = minerva.tick(200);

    assert_eq!(next_on_route(state), Vec::<&str>::new());
}

#[test]
fn the_marks_follow_the_route_as_the_player_moves() {
    let mut minerva = on_sunset_sky(roads_to_the_mushrooms());
    hunt_orange_on(&mut minerva, "100000002");

    let state = arrive(&mut minerva, KERNING_SAMPLE, 1_000);

    assert_eq!(marks(&state), [("Sunset Sky", PortalKind::Visible, false), ("Mushroom Field", PortalKind::Visible, true)]);
}

#[test]
fn once_the_player_has_arrived_no_portal_is_next_on_the_route() {
    let mut minerva = on_sunset_sky(roads_to_the_mushrooms());
    hunt_orange_on(&mut minerva, "100000001");

    let state = arrive(&mut minerva, KERNING_SAMPLE, 1_000);

    assert!(state.search.hunting.as_ref().is_some_and(|target| target.arrived));
    assert_eq!(next_on_route(&state), Vec::<&str>::new());
}

#[test]
fn clearing_the_hunting_target_clears_the_marks_at_once() {
    let mut minerva = on_sunset_sky(roads_to_the_mushrooms());
    hunt_orange_on(&mut minerva, "100000002");

    let state = minerva.act(Action::ClearHuntingTarget);

    assert_eq!(next_on_route(state), Vec::<&str>::new());
}

/// Henesys is a Teleport Scroll away from Sunset Sky, whose portals lead elsewhere.
#[test]
fn a_next_step_by_teleport_scroll_marks_nothing() {
    let henesys = "010001000";
    let mut minerva = on_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]).exit(PortalKind::Quest, henesys),
        map("100000001", "Kerning City", 100, vec![]),
        map(henesys, "Henesys", 100, vec![orange(10)]),
    ]);
    minerva.act(Action::ToggleTeleportScrolls);

    let state = hunt_orange_on(&mut minerva, henesys);

    let route: Vec<_> = state.search.hunting.as_ref().expect("a Hunting Target").map.route.iter().map(|s| s.by).collect();
    assert_eq!(route, [None, Some(minerva_core::Step::TeleportScroll)]);
    assert_eq!(next_on_route(&state), Vec::<&str>::new());
}

/// The overlay gets each mark's destination by ID, and whether it is next on the route.
#[test]
fn the_overlay_receives_which_marks_are_next_on_the_route() {
    let mut minerva = on_sunset_sky(roads_to_the_mushrooms());

    let state = hunt_orange_on(&mut minerva, "100000002");

    let json = serde_json::to_value(&state).unwrap();
    let mark = &json["mapPreview"]["marks"][0];
    assert_eq!((&mark["toMapId"], &mark["nextOnRoute"]), (&serde_json::json!("100000001"), &serde_json::json!(true)));
}
