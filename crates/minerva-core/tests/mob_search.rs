//! Mob search: the player opens the search view (the search hotkey or the 🔍 button),
//! types part of a mob's name and picks the mob from Search's matches, then picks one of
//! the maps it spawns on, ranked by Density Tier and then Density. That mob and map become
//! the Hunting Target. While the search view is open the overlay takes the keyboard (ADR
//! 0001). Maps and items in the matches are `search.rs`'s.

mod common;

use common::world::{SEARCH, WorldMap, loaded, map, mob, orange, search_for, world, sharing};
use minerva_core::{Action, Minerva, DensityTier, PanelState, Request, SearchView};

fn mushroom_world() -> Vec<WorldMap> {
    vec![
        map("100000001", "Mushroom Field", 100, vec![mob("9", "Orange Mushroom", 8, 10), mob("4", "Shroom", 2, 5)]),
        map("100000002", "Green Hill", 100, vec![mob("13", "Green Mushroom", 15, 8)]),
        map("100000003", "Ant Tunnel", 100, vec![mob("2110200", "Horny Mushroom", 22, 12), mob("3", "Blue Snail", 4, 6)]),
        map("100000004", "Mushroom Hill", 100, vec![mob("9", "Orange Mushroom", 8, 4)]),
    ]
}

/// The map list's maps, by name, in order.
fn maps(state: &PanelState) -> Vec<&str> {
    state.search.results.iter().map(|r| r.map_name.as_str()).collect()
}

/// The Hunting Target as "Mob · Map".
fn hunting(state: &PanelState) -> Option<String> {
    state.search.hunting.as_ref().map(|h| format!("{} · {}", h.mob.name, h.map.map_name))
}

#[test]
fn the_player_picks_a_mob_then_one_of_its_maps_and_that_becomes_the_hunting_target() {
    let mut minerva = loaded(mushroom_world());
    minerva.on_hotkey(SEARCH);
    assert_eq!(minerva.panel_state().search.view, Some(SearchView::Matches));

    minerva.search_query("orange");
    let state = minerva.act(Action::SearchEnter);
    assert_eq!(state.search.view, Some(SearchView::Maps), "the mob's map list");
    assert_eq!(picked(state), Some("Orange Mushroom"));
    assert_eq!(maps(state), ["Mushroom Field", "Mushroom Hill"]);
    assert_eq!(hunting(state), None, "nothing is set until a map is picked");

    minerva.act(Action::SearchNext);
    let state = minerva.act(Action::SearchEnter);

    assert_eq!(state.search.view, None, "back to the normal view");
    assert_eq!(hunting(state).as_deref(), Some("Orange Mushroom · Mushroom Hill"));
}

#[test]
fn a_click_picks_a_suggested_mob_and_then_a_listed_map() {
    let mut minerva = loaded(mushroom_world());
    minerva.on_hotkey(SEARCH);
    minerva.search_query("mush");

    let state = minerva.search_pick("9");
    assert_eq!(picked(state), Some("Orange Mushroom"), "not the highlighted Green Mushroom");
    assert_eq!(state.search.view, Some(SearchView::Maps));

    let state = minerva.search_pick("100000004");
    assert_eq!(state.search.view, None);
    assert_eq!(hunting(state).as_deref(), Some("Orange Mushroom · Mushroom Hill"));
}

#[test]
fn a_click_on_something_no_longer_listed_does_nothing() {
    let mut minerva = loaded(mushroom_world());
    minerva.on_hotkey(SEARCH);
    minerva.search_query("green");

    let state = minerva.search_pick("9").clone();
    assert_eq!(state.search.view, Some(SearchView::Matches), "the Orange Mushroom isn't suggested");

    minerva.search_pick("13");
    let state = minerva.search_pick("100000001");
    assert_eq!(state.search.view, Some(SearchView::Maps), "no Green Mushrooms on the Mushroom Field");
    assert_eq!(picked(state), Some("Green Mushroom"));
    assert_eq!(maps(state), ["Green Hill"]);
    assert_eq!(hunting(state), None);
}

#[test]
fn the_search_hotkey_with_the_search_view_open_brings_the_overlay_back_in_front_as_it_is() {
    let mut minerva = loaded(mushroom_world());
    assert_eq!(minerva.on_hotkey(SEARCH), None, "opening the search view brings it in front");
    minerva.search_query("mush");
    let before = minerva.panel_state().clone();

    // The game may have come in front of it since.
    let asked = minerva.on_hotkey(SEARCH);

    assert_eq!(asked, Some(Request::RaiseOverlay));
    assert_eq!(*minerva.panel_state(), before);
}

/// The matched mobs as "Name (level)", in order.
fn suggestions(state: &PanelState) -> Vec<String> {
    state.search.mobs.shown.iter().map(|m| format!("{} ({})", m.mob.name, m.mob.level)).collect()
}

/// The name of the highlighted mob.
fn highlighted(state: &PanelState) -> Option<&str> {
    state.search.highlighted.map(|i| state.search.mobs.shown[i].mob.name.as_str())
}

#[test]
fn typing_suggests_every_known_mob_whose_name_contains_it_whatever_the_case_with_its_level() {
    let mut minerva = loaded(mushroom_world());
    minerva.on_hotkey(SEARCH);

    let state = minerva.search_query("MUSH");

    assert_eq!(state.search.query, "MUSH");
    assert_eq!(suggestions(state), ["Green Mushroom (15)", "Horny Mushroom (22)", "Orange Mushroom (8)"]);
    assert_eq!(highlighted(state), Some("Green Mushroom"), "the first is highlighted");

    let state = minerva.search_query("room");
    assert_eq!(suggestions(state).len(), 4, "anywhere in the name: Shroom too");
    let state = minerva.search_query("dragon");
    assert!(suggestions(state).is_empty());
    assert_eq!(highlighted(state), None);
}

/// The picked mob's name, if any.
fn picked(state: &PanelState) -> Option<&str> {
    state.search.picked.as_ref().map(|m| m.name.as_str())
}

#[test]
fn the_highlight_moves_through_the_suggestions_and_enter_lists_the_highlighted_mobs_maps() {
    let mut minerva = loaded(mushroom_world());
    minerva.on_hotkey(SEARCH);
    minerva.search_query("mush");

    assert_eq!(highlighted(minerva.act(Action::SearchNext)), Some("Horny Mushroom"));
    assert_eq!(highlighted(minerva.act(Action::SearchNext)), Some("Orange Mushroom"));
    assert_eq!(highlighted(minerva.act(Action::SearchPrevious)), Some("Horny Mushroom"));
    let state = minerva.act(Action::SearchEnter);

    assert_eq!(picked(state), Some("Horny Mushroom"));
    assert_eq!(state.search.picked.as_ref().map(|m| m.level), Some(22));
    assert_eq!(state.search.view, Some(SearchView::Maps));
    assert_eq!(maps(state), ["Ant Tunnel"]);
}

#[test]
fn the_highlight_stays_at_the_top_going_up_and_a_new_query_starts_it_at_the_top_again() {
    let mut minerva = loaded(mushroom_world());
    minerva.on_hotkey(SEARCH);
    minerva.search_query("mush");

    assert_eq!(highlighted(minerva.act(Action::SearchPrevious)), Some("Green Mushroom"));
    minerva.act(Action::SearchNext);
    assert_eq!(highlighted(minerva.search_query("mushr")), Some("Green Mushroom"));
}

/// The Orange Mushroom on the Mushroom Field made the Hunting Target.
fn hunting_orange_on_the_field(minerva: &mut Minerva) {
    search_for(minerva, "orange");
    minerva.search_pick("100000001");
    assert_eq!(hunting(minerva.panel_state()).as_deref(), Some("Orange Mushroom · Mushroom Field"));
}

#[test]
fn esc_from_the_matches_returns_to_the_normal_view_with_nothing_changed() {
    let mut minerva = loaded(mushroom_world());
    hunting_orange_on_the_field(&mut minerva);
    let before = minerva.panel_state().clone();

    minerva.on_hotkey(SEARCH);
    minerva.search_query("horny");

    assert_eq!(*minerva.act(Action::SearchEscape), before);
}

#[test]
fn esc_from_the_map_list_goes_back_to_the_matches_as_they_were_and_again_to_the_normal_view() {
    let mut minerva = loaded(mushroom_world());
    hunting_orange_on_the_field(&mut minerva);
    let before = minerva.panel_state().clone();
    minerva.on_hotkey(SEARCH);
    minerva.search_query("mush");
    minerva.act(Action::SearchNext);
    let matches = minerva.panel_state().clone();
    minerva.act(Action::SearchEnter);
    minerva.act(Action::SearchNext);

    assert_eq!(*minerva.act(Action::SearchEscape), matches, "Horny Mushroom still highlighted");
    assert_eq!(*minerva.act(Action::SearchEscape), before);
}

#[test]
fn reopened_the_search_view_starts_afresh() {
    let mut minerva = loaded(mushroom_world());
    search_for(&mut minerva, "orange");
    minerva.act(Action::SearchEscape);
    minerva.act(Action::SearchEscape);

    minerva.on_hotkey(SEARCH);

    let search = &minerva.panel_state().search;
    assert_eq!(search.view, Some(SearchView::Matches));
    assert_eq!(search.query, "");
    assert_eq!(search.picked, None);
}

#[test]
fn collapsing_the_panel_closes_the_search_view_and_keeps_the_hunting_target() {
    let mut minerva = loaded(mushroom_world());
    hunting_orange_on_the_field(&mut minerva);
    search_for(&mut minerva, "horny");

    let state = minerva.act(Action::ToggleExpanded);

    assert!(!state.expanded);
    assert_eq!(state.search.view, None);
    assert_eq!(hunting(state).as_deref(), Some("Orange Mushroom · Mushroom Field"));
}

#[test]
fn enter_on_a_query_that_matches_no_mob_or_with_nothing_typed_does_nothing() {
    let mut minerva = loaded(mushroom_world());

    for query in ["dragon", "  "] {
        let state = search_for(&mut minerva, query);

        assert_eq!(state.search.view, Some(SearchView::Matches), "the player can fix the typing");
        assert_eq!(picked(&state), None);
    }
}

#[test]
fn search_keys_do_nothing_with_the_search_view_closed() {
    let mut minerva = loaded(mushroom_world());
    search_for(&mut minerva, "orange");
    minerva.act(Action::SearchEnter);
    let before = minerva.panel_state().clone();
    assert_eq!(before.search.view, None);

    minerva.act(Action::SearchEnter);
    minerva.act(Action::SearchNext);
    let state = minerva.act(Action::SearchEscape);

    assert_eq!(*state, before);
}

/// Each result as "Map Name: Density (hundredths), Tier".
fn ranked(state: &PanelState) -> Vec<String> {
    state.search.results.iter().map(|r| format!("{}: {}, {:?}", r.map_name, r.density_hundredths, r.tier)).collect()
}

#[test]
fn every_map_the_picked_mob_spawns_on_is_a_result_with_its_density_spawn_share_and_tier() {
    let mut minerva = loaded(vec![
        map("100000001", "Mushroom Field", 150, vec![sharing(40, orange(10)), mob("4", "Shroom", 2, 15)]),
        map("100000002", "Green Hill", 100, vec![mob("13", "Green Mushroom", 15, 8)]),
        map("100000003", "Mushroom Hill", 100, vec![sharing(100, orange(4))]),
    ]);

    let state = search_for(&mut minerva, "orange");

    let results = &state.search.results;
    assert_eq!(results.len(), 2);
    let field = &results[0];
    assert_eq!(field.map_id, "100000001");
    assert_eq!(field.street_name, "Victoria Road");
    assert_eq!(field.map_name, "Mushroom Field");
    assert_eq!(field.density_hundredths, 1_500, "10 spawns at a 1.5x Mob Rate");
    assert_eq!(field.spawn_share_percent, 40);
    assert_eq!(field.tier, DensityTier::High);
    assert_eq!(ranked(&state)[1], "Mushroom Hill: 400, Low");
    assert_eq!(state.search.credit, Some("Data: OSMS Data Explorer (osmsdataexplorer.com)"), "OSMS's data");
}

#[test]
fn a_map_is_high_from_two_thirds_of_the_densest_maps_density_and_medium_from_one_third() {
    // Densities 3.00, 2.00 (exactly 2/3), 1.99, 1.00 (exactly 1/3) and 0.99.
    let mut minerva = loaded(vec![
        map("100000001", "Two Thirds", 200, vec![orange(1)]),
        map("100000002", "Just Under One Third", 99, vec![orange(1)]),
        map("100000003", "Densest", 300, vec![orange(1)]),
        map("100000004", "One Third", 100, vec![orange(1)]),
        map("100000005", "Just Under Two Thirds", 199, vec![orange(1)]),
    ]);

    let state = search_for(&mut minerva, "orange");

    assert_eq!(
        ranked(&state),
        [
            "Densest: 300, High",
            "Two Thirds: 200, High",
            "Just Under Two Thirds: 199, Medium",
            "One Third: 100, Medium",
            "Just Under One Third: 99, Low",
        ]
    );
}

#[test]
fn search_results_are_ordered_by_tier_then_density_with_spawn_count_and_mob_rate_both_counting() {
    let mut minerva = loaded(vec![
        map("100000001", "Slow Refill", 50, vec![orange(20)]),
        map("100000002", "Few But Fast", 200, vec![orange(4)]),
        map("100000003", "Many", 100, vec![orange(12)]),
        map("100000004", "Sparse", 100, vec![orange(2)]),
        map("100000005", "Fast Refill", 160, vec![orange(10)]),
    ]);

    let state = search_for(&mut minerva, "orange");

    assert_eq!(
        ranked(&state),
        ["Fast Refill: 1600, High", "Many: 1200, High", "Slow Refill: 1000, Medium", "Few But Fast: 800, Medium", "Sparse: 200, Low"]
    );
}

#[test]
fn event_maps_are_left_out_and_dont_set_the_densest_map() {
    let mut minerva = loaded(vec![
        map("100000001", "Mushroom Field", 100, vec![orange(10)]),
        map("100000002", "Mushroom Festival", 200, vec![orange(30)]).in_region("Event"),
        map("100000003", "Hidden Mushrooms", 100, vec![orange(4)]).in_region("Hidden Street"),
    ]);

    let state = search_for(&mut minerva, "orange");

    assert_eq!(ranked(&state), ["Mushroom Field: 1000, High", "Hidden Mushrooms: 400, Medium"]);
}

#[test]
fn the_dev_map_is_left_out_like_event_maps() {
    let mut minerva = loaded(vec![
        map("100000001", "Mushroom Field", 100, vec![orange(10)]),
        map("100000002", "The Manager's Room", 200, vec![orange(30)]).in_region("Dev"),
    ]);

    let state = search_for(&mut minerva, "orange");

    assert_eq!(ranked(&state), ["Mushroom Field: 1000, High"]);
}

#[test]
fn a_mob_found_only_on_event_maps_has_no_results_and_search_says_why() {
    let mut minerva = loaded(vec![
        map("100000001", "Snail Field", 100, vec![mob("3", "Blue Snail", 4, 5)]),
        map("100000002", "Mushroom Festival", 200, vec![orange(30)]).in_region("Event"),
    ]);

    let state = search_for(&mut minerva, "orange");

    assert!(state.search.results.is_empty());
    assert!(state.search.only_on_event_maps);
    let state = search_for(&mut minerva, "snail");
    assert!(!state.search.only_on_event_maps, "the Blue Snail has results");
}

#[test]
fn the_map_list_holds_every_map_the_mob_spawns_on_however_many() {
    let fields = (1..=7).map(|n| map(&format!("10000000{n}"), &format!("Field {n}"), 100, vec![orange(10 - n)])).collect();
    let mut minerva = loaded(fields);

    let state = search_for(&mut minerva, "orange");

    assert_eq!(maps(&state),["Field 1", "Field 2", "Field 3", "Field 4", "Field 5", "Field 6", "Field 7"]);
}

/// Thirty empty fields, with Orange Mushrooms on the first and a Green Mushroom with
/// more Orange Mushrooms on the last.
fn thirty_fields() -> Vec<WorldMap> {
    (0..30)
        .map(|n| {
            let id = format!("1000000{n:02}");
            match n {
                0 => map(&id, "Mushroom Field", 100, vec![orange(10)]),
                29 => map(&id, "Mushroom Grove", 100, vec![orange(20), mob("13", "Green Mushroom", 15, 8)]),
                _ => map(&id, &format!("Field {n}"), 100, vec![]),
            }
        })
        .collect()
}

#[test]
fn from_the_first_launch_the_results_cover_every_map_in_the_set() {
    let mut minerva = Minerva::new(world(thirty_fields()));
    minerva.tick(0);

    let state = search_for(&mut minerva, "orange");

    assert_eq!(ranked(&state), ["Mushroom Grove: 2000, High", "Mushroom Field: 1000, Medium"]);
}

#[test]
fn from_the_first_launch_the_suggestions_cover_every_mob_in_the_set() {
    let mut minerva = Minerva::new(world(thirty_fields()));
    minerva.tick(0);
    minerva.on_hotkey(SEARCH);

    let state = minerva.search_query("mushroom");

    assert_eq!(suggestions(state), ["Green Mushroom (15)", "Orange Mushroom (8)"]);
}

#[test]
fn until_a_mob_is_picked_there_is_no_map_list_and_no_credit() {
    let mut minerva = loaded(mushroom_world());
    minerva.on_hotkey(SEARCH);

    let state = minerva.search_query("orange");

    assert!(state.search.results.is_empty());
    assert_eq!(state.search.credit, None);
}

#[test]
fn the_search_hotkey_opens_the_search_view_with_nothing_typed() {
    let mut minerva = loaded(mushroom_world());
    assert_eq!(minerva.panel_state().search.view, None);

    minerva.on_hotkey(SEARCH);

    let search = &minerva.panel_state().search;
    assert_eq!(search.view, Some(SearchView::Matches));
    assert_eq!(search.query, "");
    assert!(search.mobs.shown.is_empty());
}

#[test]
fn the_search_hotkey_expands_a_collapsed_panel_to_show_the_search_view() {
    let mut minerva = loaded(mushroom_world());
    assert!(!minerva.act(Action::ToggleExpanded).expanded);

    minerva.on_hotkey(SEARCH);

    let state = minerva.panel_state();
    assert_eq!(state.search.view, Some(SearchView::Matches));
    assert!(state.expanded);
}
