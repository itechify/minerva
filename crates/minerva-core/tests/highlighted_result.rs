//! The map list's highlight: the search view lists the picked mob's maps, and Enter makes
//! the highlighted one the Hunting Target. The arrow keys move the highlight.

mod common;

use common::{arrive, KERNING_SAMPLE};
use common::world::{WorldMap, map, mob, orange, search_for, on_sunset_sky, highlighted_map};
use minerva_core::Action;

/// Sunset Sky leads to the Near Field, the Near Field to the Far Field, and the Far
/// Field to Kerning City; Orange Mushrooms on both fields, Blue Snails on the Far Field.
fn fields() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Near Field", 100, vec![orange(10)]).to(&["100000002"]),
        map("100000002", "Far Field", 100, vec![orange(10), mob("3", "Blue Snail", 2, 5)]).to(&["100000003"]),
        map("100000003", "Kerning City", 100, vec![]).to(&["100000002"]),
    ]
}

#[test]
fn a_picked_mobs_first_map_is_highlighted() {
    let mut minerva = on_sunset_sky(fields());

    let state = search_for(&mut minerva, "orange");

    assert_eq!(highlighted_map(&state), Some("Near Field"));
}

#[test]
fn the_arrows_move_the_highlight_through_the_map_list_stopping_at_either_end() {
    let mut minerva = on_sunset_sky(fields());
    search_for(&mut minerva, "orange");

    assert_eq!(highlighted_map(minerva.act(Action::SearchNext)), Some("Far Field"));
    assert_eq!(highlighted_map(minerva.act(Action::SearchNext)), Some("Far Field"), "it stops at the last");
    assert_eq!(highlighted_map(minerva.act(Action::SearchPrevious)), Some("Near Field"));
    assert_eq!(highlighted_map(minerva.act(Action::SearchPrevious)), Some("Near Field"), "it stops at the first");
}

#[test]
fn the_highlight_stays_on_its_map_when_a_map_change_reorders_the_list() {
    let mut minerva = on_sunset_sky(fields());
    search_for(&mut minerva, "orange");
    minerva.act(Action::SearchNext);

    let state = arrive(&mut minerva, KERNING_SAMPLE, 1_000);

    assert_eq!(state.search.results[0].map_name, "Far Field", "the Far Field is now nearer");
    assert_eq!(highlighted_map(&state), Some("Far Field"));
}

#[test]
fn picking_another_mob_highlights_its_first_map() {
    let mut minerva = on_sunset_sky(fields());
    search_for(&mut minerva, "orange");
    minerva.act(Action::SearchNext);
    let state = search_for(&mut minerva, "orange");
    assert_eq!(highlighted_map(&state), Some("Near Field"), "picked afresh");

    let state = search_for(&mut minerva, "snail");

    assert_eq!(highlighted_map(&state), Some("Far Field"));
}

#[test]
fn with_no_maps_listed_nothing_is_highlighted_and_enter_does_nothing() {
    let mut minerva = on_sunset_sky(vec![map("100000000", "Sunset Sky", 100, vec![]), event_field()]);

    let state = search_for(&mut minerva, "orange");
    assert!(state.search.results.is_empty());
    assert_eq!(state.search.highlighted_result, None);

    let state = minerva.act(Action::SearchEnter);
    assert!(state.search.view.is_some());
    assert_eq!(state.search.hunting, None);
}

/// An Event map with Orange Mushrooms: never listed.
fn event_field() -> WorldMap {
    map("100000001", "Event Field", 100, vec![orange(10)]).in_region("Event")
}
