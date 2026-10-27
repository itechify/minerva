//! Search: the player opens the search view (the search hotkey or the 🔍 button) and
//! types. What the query matches shows in sections, mobs, then maps, then items, each its
//! best five with a count of the rest. A mob lists its maps, to pick a Hunting Target
//! (`mob_search.rs`); a map becomes the Destination, which the Navigation section shows
//! the way to; items show their Item Card beside the panel.
//!
//! Maps are made up in OSMS's shape ([`common::world`]), as are the items
//! ([`common::items`]). The player stands on Sunset Sky, titled like the minimap sample.

mod common;

use common::items::{ItemWorld, cash, equip, item_world, use_item};
use common::world::{SEARCH, WorldMap, destination, highlighted_match as highlighted, map, mob, orange, world, on_sunset_sky_from, stops, typed};
use common::{ItemSetAnswer, arrive, tracking_with, SUNSET_SAMPLE, KERNING_SAMPLE, overlaps};
use minerva_core::{
    Action, Minerva, Distance, ItemCardView, ItemsMissing, PanelSizes, PanelState, ScreenRect, SearchView,
    Size, Step,
};
use std::task::Poll;

/// Sunset Sky leads to Kerning City, Kerning City to the Mushroom Field. Orange Mushrooms
/// on the Field and the Mushroom Grove; a Mushroom Festival in the Event region.
fn mushroom_roads() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Kerning City", 100, vec![]).to(&["100000000", "100000002"]),
        map("100000002", "Mushroom Field", 150, vec![orange(4), mob("4", "Shroom", 2, 6)]).to(&["100000001"]),
        map("100000003", "Mushroom Grove", 100, vec![orange(18)]),
        map("100000004", "Mushroom Festival", 200, vec![orange(30)]).in_region("Event"),
    ]
}

/// Items, a few of them mushroom-named.
fn mushroom_items() -> ItemWorld {
    item_world(vec![
        use_item(2_000_000, "Red Potion").sells_for(25),
        use_item(2_022_000, "Mushroom Stew").sells_for(40),
        equip(1_002_000, "Mushroom Hat", "Hat").sells_for(500),
    ])
}

/// A core over `maps` and `items`, both in, with the player arrived on Sunset Sky.
fn on_sunset_sky(maps: Vec<WorldMap>, items: ItemWorld) -> Minerva {
    on_sunset_sky_from(world(maps).with_items(items))
}

fn mobs(state: &PanelState) -> Vec<&str> {
    state.search.mobs.shown.iter().map(|m| m.mob.name.as_str()).collect()
}

fn maps(state: &PanelState) -> Vec<&str> {
    state.search.maps.shown.iter().map(|m| m.map_name.as_str()).collect()
}

fn items(state: &PanelState) -> Vec<String> {
    state.search.items.shown.iter().map(|i| format!("{} ({})", i.name, i.kind)).collect()
}

#[test]
fn typing_matches_mobs_maps_and_items_each_in_a_section_of_its_own() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());

    let state = typed(&mut minerva, "mushroom");

    assert_eq!(state.search.view, Some(SearchView::Matches));
    assert_eq!(mobs(&state), ["Orange Mushroom"]);
    assert_eq!(maps(&state), ["Mushroom Field", "Mushroom Grove"], "not the Event map");
    assert_eq!(items(&state), ["Mushroom Hat (Hat)", "Mushroom Stew (Use)"]);
    assert_eq!(state.search.items_missing, None);
}

#[test]
fn a_kind_with_nothing_matching_has_nothing_shown() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());

    let state = typed(&mut minerva, "potion");

    assert!(mobs(&state).is_empty() && maps(&state).is_empty());
    assert_eq!(items(&state), ["Red Potion (Use)"]);
}

#[test]
fn each_kind_shows_its_best_five_and_counts_the_rest() {
    let fields = (1..=8).map(|n| map(&format!("10000000{n}"), &format!("Snail Field {n}"), 100, vec![])).collect();
    let snails = (1..=6).map(|n| use_item(2_000_000 + n, &format!("Snail Shell {n}")).sells_for(n)).collect();
    let mut minerva = on_sunset_sky(fields, item_world(snails));

    let state = typed(&mut minerva, "snail");

    assert_eq!(maps(&state), ["Snail Field 1", "Snail Field 2", "Snail Field 3", "Snail Field 4", "Snail Field 5"]);
    assert_eq!(state.search.maps.more, 3);
    assert_eq!(state.search.items.shown.len(), 5);
    assert_eq!(state.search.items.more, 1);
    assert_eq!(state.search.mobs.more, 0);
}

#[test]
fn names_that_start_with_the_query_come_first_then_those_with_a_word_that_does_then_the_rest() {
    let snails = vec![
        mob("1", "Blue Snail", 2, 1),
        mob("2", "Mossnail", 3, 1),
        mob("3", "Snail", 1, 1),
        mob("4", "Red Snail", 4, 1),
        mob("5", "Snail-Shell Crab", 5, 1),
    ];
    let mut minerva = on_sunset_sky(vec![map("100000001", "Beach", 100, snails)], item_world(vec![]));

    let state = typed(&mut minerva, "SNAIL");

    assert_eq!(mobs(&state), ["Snail", "Snail-Shell Crab", "Blue Snail", "Red Snail", "Mossnail"]);
}

#[test]
fn a_map_matches_by_its_map_name_alone_and_shows_its_street() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());

    let state = typed(&mut minerva, "victoria");
    assert!(maps(&state).is_empty(), "every map is on Victoria Road");

    let state = typed(&mut minerva, "kerning");
    let kerning = &state.search.maps.shown[0];
    assert_eq!((kerning.map_id.as_str(), kerning.street_name.as_str()), ("100000001", "Victoria Road"));
}

#[test]
fn the_dev_map_is_left_out_like_event_maps() {
    let mut maps_ = mushroom_roads();
    maps_.push(map("100000009", "The Mushroom Manager's Room", 100, vec![]).in_region("Dev"));
    let mut minerva = on_sunset_sky(maps_, mushroom_items());

    let state = typed(&mut minerva, "mushroom");

    assert_eq!(maps(&state), ["Mushroom Field", "Mushroom Grove"]);
}

#[test]
fn items_that_look_alike_are_one_match_and_those_that_dont_are_each_listed_with_their_kind() {
    let namesakes = item_world(vec![
        use_item(2_000_000, "Red Potion").sells_for(25),
        use_item(2_000_100, "Red Potion").sells_for(25),
        equip(1_002_067, "Blue Moon", "Overall").sells_for(11_000),
        equip(1_032_001, "Blue Moon", "Earrings").sells_for(10_500),
    ])
    .with_cash(cash(5_000_100, "Blue Moon Mask", "Accessory"));
    let mut minerva = on_sunset_sky(mushroom_roads(), namesakes);

    let potions = typed(&mut minerva, "red potion");
    let moons = typed(&mut minerva, "blue moon");

    assert_eq!(items(&potions), ["Red Potion (Use)"]);
    assert_eq!(potions.search.items.shown[0].id, "02000000", "under the first");
    assert_eq!(items(&moons), ["Blue Moon (Earrings)", "Blue Moon (Overall)", "Blue Moon Mask (Cash)"]);
}

#[test]
fn until_the_item_files_are_in_search_says_so_in_the_items_place_once_something_is_typed() {
    let pending: Vec<ItemSetAnswer> = vec![Poll::Pending];
    let mut minerva = on_sunset_sky_from(world(mushroom_roads()).answering_item_set(pending));

    minerva.on_hotkey(SEARCH);
    assert_eq!(minerva.panel_state().search.items_missing, None, "nothing typed");
    let state = minerva.search_query("mushroom").clone();

    assert_eq!(state.search.items_missing, Some(ItemsMissing::Loading));
    assert_eq!(maps(&state), ["Mushroom Field", "Mushroom Grove"], "the rest are matched");
}

#[test]
fn with_osms_unreachable_and_no_item_files_saved_search_says_items_couldnt_be_had() {
    let mut minerva = on_sunset_sky_from(world(mushroom_roads()));

    let state = typed(&mut minerva, "mushroom");

    assert_eq!(state.search.items_missing, Some(ItemsMissing::Unavailable));
}

#[test]
fn items_arriving_while_the_search_view_is_open_are_matched_at_once() {
    let answers: Vec<ItemSetAnswer> = vec![Poll::Pending, Poll::Ready(Ok(mushroom_items().files()))];
    let mut minerva = on_sunset_sky_from(world(mushroom_roads()).answering_item_set(answers));
    typed(&mut minerva, "mushroom");

    let state = minerva.tick(200).clone();

    assert_eq!(state.search.items_missing, None);
    assert_eq!(items(&state), ["Mushroom Hat (Hat)", "Mushroom Stew (Use)"]);
}

#[test]
fn the_highlight_starts_on_the_first_match_and_goes_through_the_mobs_then_the_maps_then_the_items() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());
    let state = typed(&mut minerva, "mushroom");
    assert_eq!(highlighted(&state).as_deref(), Some("mob: Orange Mushroom"));

    let seen: Vec<String> = (0..5).filter_map(|_| highlighted(minerva.act(Action::SearchNext))).collect();

    assert_eq!(
        seen,
        ["map: Mushroom Field", "map: Mushroom Grove", "item: Mushroom Hat", "item: Mushroom Stew", "item: Mushroom Stew"]
    );
    assert_eq!(highlighted(minerva.act(Action::SearchPrevious)).as_deref(), Some("item: Mushroom Hat"));
}

#[test]
fn with_no_mob_matching_the_highlight_starts_on_the_first_map() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());

    let state = typed(&mut minerva, "field");

    assert_eq!(highlighted(&state).as_deref(), Some("map: Mushroom Field"));
}

#[test]
fn a_new_query_starts_the_highlight_at_the_top_again() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());
    typed(&mut minerva, "mushroom");
    minerva.act(Action::SearchNext);
    minerva.act(Action::SearchNext);

    let state = minerva.search_query("mushroo").clone();

    assert_eq!(highlighted(&state).as_deref(), Some("mob: Orange Mushroom"));
}

/// The Destination's route, as its map names with how each is reached.
fn route(state: &PanelState) -> Vec<String> {
    stops(&destination(state).route)
}

#[test]
fn enter_on_a_map_makes_it_the_destination_and_closes_the_search_view() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());
    typed(&mut minerva, "field");

    let state = minerva.act(Action::SearchEnter).clone();

    assert_eq!(state.search.view, None);
    let to = destination(&state);
    assert_eq!((to.map_id.as_str(), to.street_name.as_str(), to.map_name.as_str()), ("100000002", "Victoria Road", "Mushroom Field"));
    assert_eq!(to.distance, Distance::MapChanges(2));
    assert_eq!(route(&state), ["Sunset Sky", "Portal: Kerning City", "Portal: Mushroom Field"]);
    assert!(!to.arrived);
    assert_eq!(state.search.hunting, None);
    assert_eq!(state.search.credit, Some("Data: OSMS Data Explorer (osmsdataexplorer.com)"));
}

#[test]
fn a_click_on_a_map_makes_it_the_destination() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());
    typed(&mut minerva, "mushroom");

    let state = minerva.search_pick_destination("100000003").clone();

    assert_eq!(state.search.view, None);
    assert_eq!(destination(&state).map_name, "Mushroom Grove");
    assert_eq!(destination(&state).distance, Distance::NoRoute);
}

#[test]
fn a_click_on_a_map_or_item_no_longer_shown_does_nothing() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());
    let before = typed(&mut minerva, "grove");

    assert_eq!(*minerva.search_pick_destination("100000002"), before, "the Field isn't shown");
    assert_eq!(*minerva.search_pick_item("01002000"), before, "nor is the Hat");
    assert_eq!(*minerva.search_pick_destination("100000004"), before, "nor ever the Festival");
}

#[test]
fn the_way_to_the_destination_is_worked_out_again_as_the_player_moves() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());
    typed(&mut minerva, "field");
    minerva.act(Action::SearchEnter);

    let state = arrive(&mut minerva, KERNING_SAMPLE, 2_000);

    assert_eq!(destination(&state).distance, Distance::MapChanges(1));
    assert_eq!(route(&state), ["Kerning City", "Portal: Mushroom Field"]);
}

#[test]
fn the_map_preview_rings_the_next_portal_on_the_way_to_the_destination() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());
    typed(&mut minerva, "field");

    let state = minerva.act(Action::SearchEnter).clone();

    let preview = state.map_preview.as_ref().expect("Sunset Sky's Map Preview");
    let next: Vec<&str> = preview.marks.iter().filter(|mark| mark.next_on_route).map(|mark| mark.to_map_id.as_str()).collect();
    assert_eq!(next, ["100000001"]);
}

/// Sunset Sky and Kerning City, each a map change from the other.
fn two_towns() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Kerning City", 100, vec![orange(5)]).to(&["100000000"]),
    ]
}

#[test]
fn arriving_at_the_destination_says_so_with_the_trip_until_the_player_leaves_it() {
    let mut minerva = on_sunset_sky(two_towns(), mushroom_items());
    typed(&mut minerva, "kerning");
    minerva.act(Action::SearchEnter);

    let there = arrive(&mut minerva, KERNING_SAMPLE, 2_000);
    let to = destination(&there);
    assert!(to.arrived);
    let trip = to.trip.as_ref().expect("the trip");
    assert_eq!((trip.from.as_str(), trip.map_changes, trip.travel.as_slice()), ("Sunset Sky", 1, &[] as &[Step]));

    let left = arrive(&mut minerva, SUNSET_SAMPLE, 4_000);
    assert_eq!(left.search.destination, None);
}

#[test]
fn picking_the_map_the_player_is_on_says_they_are_there_at_once_until_they_leave() {
    let mut minerva = on_sunset_sky(two_towns(), mushroom_items());
    typed(&mut minerva, "sunset");

    let state = minerva.act(Action::SearchEnter).clone();

    let to = destination(&state);
    assert!(to.arrived);
    assert_eq!((to.distance, to.trip.as_ref()), (Distance::MapChanges(0), None));
    assert_eq!(arrive(&mut minerva, KERNING_SAMPLE, 2_000).search.destination, None);
}

#[test]
fn a_destination_takes_the_hunting_targets_place_and_a_hunting_target_the_destinations() {
    let mut minerva = on_sunset_sky(two_towns(), mushroom_items());
    typed(&mut minerva, "orange");
    minerva.act(Action::SearchEnter);
    let hunting = minerva.act(Action::SearchEnter).clone();
    assert!(hunting.search.hunting.is_some());

    typed(&mut minerva, "sunset");
    let state = minerva.act(Action::SearchEnter).clone();
    assert_eq!((state.search.hunting.is_some(), state.search.destination.is_some()), (false, true));

    typed(&mut minerva, "orange");
    minerva.act(Action::SearchEnter);
    let state = minerva.act(Action::SearchEnter).clone();
    assert_eq!((state.search.hunting.is_some(), state.search.destination.is_some()), (true, false));
}

#[test]
fn the_navigation_sections_x_clears_the_destination() {
    let mut minerva = on_sunset_sky(two_towns(), mushroom_items());
    typed(&mut minerva, "kerning");
    minerva.act(Action::SearchEnter);

    let state = minerva.act(Action::ClearHuntingTarget);

    assert_eq!(state.search.destination, None);
}

#[test]
fn a_destination_has_no_map_list_to_reopen() {
    let mut minerva = on_sunset_sky(two_towns(), mushroom_items());
    typed(&mut minerva, "kerning");
    let before = minerva.act(Action::SearchEnter).clone();

    let state = minerva.act(Action::ReopenMapList);

    assert_eq!(state.search.view, None);
    assert_eq!(state.search.destination, before.search.destination);
}

#[test]
fn the_matches_show_their_pictures_once_they_are_in() {
    let mut minerva = on_sunset_sky(mushroom_roads(), mushroom_items());
    let state = typed(&mut minerva, "mushroom");
    assert_eq!(state.search.mobs.shown[0].image, None, "not in yet");

    let state = minerva.tick(200).clone();

    assert_eq!(state.search.mobs.shown[0].image.as_deref(), Some("picture:images/monsters/gif9.webp"));
    let icons: Vec<Option<&str>> = state.search.items.shown.iter().map(|i| i.icon.as_deref()).collect();
    assert_eq!(
        icons,
        [Some("picture:current/images/items/01002000.png"), Some("picture:current/images/items/02022000.png")]
    );
}

/// A core with EXP tracking on, its windows' sizes known, over `maps` and the made-up
/// items, both in, on a 1920x1200 game window at the screen's corner.
fn over_the_game(maps: Vec<WorldMap>) -> Minerva {
    let mut minerva = tracking_with(world(maps).with_items(mushroom_items()));
    minerva.set_panel_sizes(PanelSizes { expanded: Size { width: 340, height: 400 }, collapsed: Size { width: 44, height: 44 } });
    minerva.set_item_card_size(CARD);
    minerva.tick(0);
    minerva
}

const CARD: Size = Size { width: 300, height: 160 };

#[test]
fn enter_on_items_closes_the_search_view_and_shows_their_card_beside_the_panel() {
    let mut minerva = over_the_game(mushroom_roads());
    minerva.act(Action::ToggleGrindTracker);
    typed(&mut minerva, "hat");

    let state = minerva.act(Action::SearchEnter).clone();

    assert_eq!(state.search.view, None);
    let Some(ItemCardView::Card(card)) = state.item_card.as_ref().map(|shown| &shown.view) else { panic!("a card") };
    assert_eq!((card.id.as_str(), card.name.as_str()), ("01002000", "Mushroom Hat"));
    let panel = state.placement.expect("the panel");
    let at = state.item_card_placement.expect("where the card shows");
    let towards_the_middle = if panel.x + panel.width as i32 / 2 >= 960 { panel.x - 8 - 300 } else { panel.x + panel.width as i32 + 8 };
    assert_eq!(at, ScreenRect { x: towards_the_middle, y: panel.y, width: 300, height: 160 });
}

#[test]
fn a_click_on_items_shows_their_card_and_closes_the_search_view() {
    let mut minerva = over_the_game(mushroom_roads());
    typed(&mut minerva, "mushroom");

    let state = minerva.search_pick_item("02022000").clone();

    assert_eq!(state.search.view, None);
    let Some(ItemCardView::Card(card)) = state.item_card.as_ref().map(|shown| &shown.view) else { panic!("a card") };
    assert_eq!(card.name, "Mushroom Stew");
}

#[test]
fn a_card_from_search_keeps_off_the_grind_tracker_beside_the_panel() {
    let mut minerva = over_the_game(mushroom_roads());
    minerva.set_grind_tracker_size(Size { width: 300, height: 160 });
    typed(&mut minerva, "hat");

    let state = minerva.act(Action::SearchEnter).clone();

    let tracker = state.grind_tracker.placement.expect("the Grind Tracker beside the panel");
    let card = state.item_card_placement.expect("the card");
    let panel = state.placement.expect("the panel");
    assert!(!overlaps(card, tracker), "{card:?} over the Grind Tracker at {tracker:?}");
    assert!(!overlaps(card, panel), "{card:?} over the panel at {panel:?}");
    let client = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    assert!(card.x >= client.x && card.x + card.width as i32 <= 1920 && card.y + card.height as i32 <= 1200);
}

#[test]
fn the_card_from_search_follows_the_panel_until_dragged() {
    let mut minerva = over_the_game(mushroom_roads());
    minerva.act(Action::ToggleGrindTracker);
    typed(&mut minerva, "hat");
    let before = minerva.act(Action::SearchEnter).clone();
    let (panel, card) = (before.placement.unwrap(), before.item_card_placement.unwrap());

    let state = minerva.act(Action::PanelMoved { x: panel.x, y: panel.y + 100 }).clone();

    let moved = state.placement.unwrap();
    assert_eq!(state.item_card_placement.map(|at| (at.x - moved.x, at.y - moved.y)), Some((card.x - panel.x, card.y - panel.y)));
}

#[test]
fn with_no_map_shown_the_destination_has_no_distance_or_route() {
    let mut minerva = Minerva::new(world(two_towns()).with_items(mushroom_items()));
    minerva.tick(0);
    typed(&mut minerva, "kerning");

    let state = minerva.act(Action::SearchEnter).clone();

    let to = destination(&state);
    assert_eq!((to.map_name.as_str(), to.distance), ("Kerning City", Distance::Unknown), "no map shown to measure from");
    assert!(to.route.is_empty() && to.trip.is_none() && !to.arrived);
}
