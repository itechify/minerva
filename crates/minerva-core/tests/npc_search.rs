//! NPCs in Search (ADR 0028): what the player types matches NPCs by their name too, in a
//! section of their own between the maps and the items. An NPC on one map makes that map the
//! Destination, with the NPC; an NPC on several lists them, nearest first, to pick from. The
//! Map Preview marks where the NPC stands once the player is on its map.
//!
//! Maps and NPCs are made up in OSMS's shape ([`common::world`]). The player stands on
//! Sunset Sky, titled like the minimap sample.

mod common;

use common::{FakeSource, KERNING_SAMPLE, SUNSET_SAMPLE, arrive, fresh_files};
use common::items::{item_world, use_item};
use common::osms::npc;
use common::world::{
    WorldMap, destination, highlighted_match, map, on_sunset_sky, on_sunset_sky_from, osms_world, stops, typed, world,
};
use minerva_core::{Action, Distance, Minerva, PanelState, Recognition, SearchView};

/// Sunset Sky leads to Kerning City, Kerning City to Henesys. Heena stands in Henesys, the
/// Maple Administrator in Henesys and in Kerning City, and Sam in Henesys's weapon store.
fn towns() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Kerning City", 100, vec![]).to(&["100000000", "100000002"]).npc(npc(9_010_000, "Maple Administrator"), 900, 300),
        map("100000002", "Henesys", 100, vec![])
            .to(&["100000001"])
            .npc(npc(1, "Heena"), 400, 250)
            .npc(npc(9_010_000, "Maple Administrator"), 700, 300),
        map("100000003", "Henesys Weapon Store", 100, vec![]).npc(npc(211, "Sam"), 120, 90),
    ]
}

/// Each NPC matched, as "name · its map", or "name · on N maps".
fn npcs(state: &PanelState) -> Vec<String> {
    state
        .search
        .npcs
        .shown
        .iter()
        .map(|found| match &found.maps[..] {
            [map] => format!("{} · {}", found.name, map.map_name),
            maps => format!("{} · on {} maps", found.name, maps.len()),
        })
        .collect()
}

#[test]
fn typing_matches_npcs_by_name_each_with_its_map() {
    let mut minerva = on_sunset_sky(towns());

    let state = typed(&mut minerva, "heena");

    assert_eq!(state.search.view, Some(SearchView::Matches));
    assert_eq!(npcs(&state), ["Heena · Henesys"]);
    assert!(state.search.maps.shown.is_empty(), "no map holds the name");
}

#[test]
fn an_npc_on_several_maps_is_one_match_and_npcs_sharing_a_name_are_each_listed_with_their_map() {
    let mut maps = towns();
    maps.push(map("100000004", "Truth Booth", 100, vec![]).npc(npc(800_016, "Maple Administrator"), 50, 50));
    let mut minerva = on_sunset_sky(maps);

    let state = typed(&mut minerva, "maple admin");

    assert_eq!(npcs(&state), ["Maple Administrator · on 2 maps", "Maple Administrator · Truth Booth"]);
    let both: Vec<&str> = state.search.npcs.shown[0].maps.iter().map(|m| m.map_name.as_str()).collect();
    assert_eq!(both, ["Henesys", "Kerning City"]);
}

#[test]
fn an_npc_on_no_map_or_only_on_event_and_dev_maps_isnt_matched() {
    let mut maps = towns();
    maps.push(map("100000005", "Snowman Festival", 100, vec![]).in_region("Event").npc(npc(9_000_001, "Snowman Cloto"), 10, 10));
    maps.push(map("100000006", "The Manager's Room", 100, vec![]).in_region("Dev").npc(npc(9_000_002, "Manager Cloto"), 10, 10));
    maps.push(map("100000007", "Event Hall", 100, vec![]).in_region("Event").npc(npc(1, "Heena"), 10, 10));
    let unplaced = osms_world(maps).with_npc(npc(1000, "Agatha Cloto"));
    let mut minerva = on_sunset_sky_from(FakeSource::with_world(unplaced).with_items(item_world(vec![])));

    assert!(npcs(&typed(&mut minerva, "cloto")).is_empty());
    assert_eq!(npcs(&typed(&mut minerva, "heena")), ["Heena · Henesys"], "not the Event Hall");
}

#[test]
fn the_npcs_come_between_the_maps_and_the_items_and_the_highlight_goes_through_them_in_turn() {
    let maps = vec![
        map("100000000", "Sunset Sky", 100, vec![]),
        map("100000001", "Henesys", 100, vec![]).npc(npc(1, "Henesys Guide"), 10, 10),
    ];
    let items = item_world(vec![use_item(2_000_000, "Henesys Return Scroll").sells_for(100)]);
    let mut minerva = on_sunset_sky_from(world(maps).with_items(items));

    let state = typed(&mut minerva, "henesys");
    let seen: Vec<String> = (0..2).filter_map(|_| highlighted_match(minerva.act(Action::SearchNext))).collect();

    assert_eq!(highlighted_match(&state).as_deref(), Some("map: Henesys"));
    assert_eq!(seen, ["npc: Henesys Guide", "item: Henesys Return Scroll"]);
}

#[test]
fn enter_on_an_npc_on_one_map_makes_that_map_the_destination_with_the_npc() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "heena");

    let state = minerva.act(Action::SearchEnter).clone();

    assert_eq!(state.search.view, None);
    let to = destination(&state);
    assert_eq!((to.map_name.as_str(), to.distance), ("Henesys", Distance::MapChanges(2)));
    assert_eq!(stops(&to.route), ["Sunset Sky", "Portal: Kerning City", "Portal: Henesys"]);
    let with = to.npc.as_ref().expect("the NPC");
    assert_eq!((with.id.as_str(), with.name.as_str()), ("1", "Heena"));
    assert_eq!(state.search.hunting, None);
}

#[test]
fn a_click_on_an_npc_on_one_map_makes_that_map_the_destination() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "a");

    let state = minerva.search_pick_npc("211").clone();

    assert_eq!(state.search.view, None);
    assert_eq!(destination(&state).map_name, "Henesys Weapon Store");
    assert_eq!(destination(&state).npc.as_ref().map(|npc| npc.name.as_str()), Some("Sam"));
}

#[test]
fn a_click_on_an_npc_no_longer_shown_does_nothing() {
    let mut minerva = on_sunset_sky(towns());
    let before = typed(&mut minerva, "heena");

    assert_eq!(*minerva.search_pick_npc("211"), before, "Sam isn't shown");
}

#[test]
fn a_map_picked_as_itself_has_no_npc() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "henesys");

    let state = minerva.act(Action::SearchEnter).clone();

    assert_eq!(destination(&state).map_name, "Henesys");
    assert_eq!(destination(&state).npc, None);
}

/// The NPC's map list, as "Map: Distance", a distance in map changes as the number.
fn npc_maps(state: &PanelState) -> Vec<String> {
    let distance = |d: Distance| match d {
        Distance::MapChanges(n) => n.to_string(),
        other => format!("{other:?}"),
    };
    state.search.npc_maps.iter().map(|m| format!("{}: {}", m.map_name, distance(m.distance))).collect()
}

/// The highlighted map's name in the NPC's map list.
fn highlighted_npc_map(state: &PanelState) -> Option<&str> {
    state.search.highlighted_result.map(|at| state.search.npc_maps[at].map_name.as_str())
}

#[test]
fn enter_on_an_npc_on_several_maps_lists_them_nearest_first_with_the_highlight_on_the_nearest() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "maple admin");

    let state = minerva.act(Action::SearchEnter).clone();

    assert_eq!(state.search.view, Some(SearchView::NpcMaps));
    assert_eq!(state.search.picked_npc.as_ref().map(|npc| npc.name.as_str()), Some("Maple Administrator"));
    assert_eq!(npc_maps(&state), ["Kerning City: 1", "Henesys: 2"]);
    let kerning = &state.search.npc_maps[0];
    assert_eq!((kerning.map_id.as_str(), kerning.street_name.as_str()), ("100000001", "Victoria Road"));
    assert_eq!(highlighted_npc_map(&state), Some("Kerning City"));
    assert_eq!(state.search.destination, None, "nothing picked yet");
    assert_eq!(state.search.credit, Some("Data: OSMS Data Explorer (osmsdataexplorer.com)"));
}

#[test]
fn a_click_on_an_npc_on_several_maps_lists_them_too() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "maple admin");

    let state = minerva.search_pick_npc("9010000").clone();

    assert_eq!(state.search.view, Some(SearchView::NpcMaps));
    assert_eq!(npc_maps(&state), ["Kerning City: 1", "Henesys: 2"]);
}

#[test]
fn enter_in_the_npcs_map_list_makes_the_highlighted_map_the_destination_with_the_npc() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "maple admin");
    minerva.act(Action::SearchEnter);
    let moved = minerva.act(Action::SearchNext).clone();
    assert_eq!(highlighted_npc_map(&moved), Some("Henesys"));

    let state = minerva.act(Action::SearchEnter).clone();

    assert_eq!(state.search.view, None);
    let to = destination(&state);
    assert_eq!((to.map_name.as_str(), to.distance), ("Henesys", Distance::MapChanges(2)));
    assert_eq!(to.npc.as_ref().map(|npc| npc.id.as_str()), Some("9010000"));
}

#[test]
fn a_click_on_a_map_in_the_npcs_map_list_makes_it_the_destination_with_the_npc() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "maple admin");
    minerva.act(Action::SearchEnter);

    let state = minerva.search_pick_destination("100000001").clone();

    assert_eq!(destination(&state).map_name, "Kerning City");
    assert_eq!(destination(&state).npc.as_ref().map(|npc| npc.name.as_str()), Some("Maple Administrator"));
}

#[test]
fn a_click_on_a_map_not_in_the_npcs_map_list_does_nothing() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "maple admin");
    let before = minerva.act(Action::SearchEnter).clone();

    assert_eq!(*minerva.search_pick_destination("100000003"), before, "the Administrator isn't in the weapon store");
}

#[test]
fn esc_from_the_npcs_map_list_goes_back_to_the_matches_as_typed() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "maple admin");
    minerva.act(Action::SearchEnter);

    let state = minerva.act(Action::SearchEscape).clone();

    assert_eq!(state.search.view, Some(SearchView::Matches));
    assert_eq!(state.search.query, "maple admin");
    assert_eq!(state.search.picked_npc, None);
    assert!(state.search.npc_maps.is_empty());
}

#[test]
fn with_no_map_shown_the_npcs_maps_are_listed_by_name() {
    let mut minerva = Minerva::new(world(towns()).with_items(item_world(vec![])));
    minerva.tick(0);
    typed(&mut minerva, "maple admin");

    let state = minerva.act(Action::SearchEnter).clone();

    assert_eq!(npc_maps(&state), ["Henesys: Unknown", "Kerning City: Unknown"]);
}

/// Where the Map Preview marks the Destination's NPC, as (x, y).
fn npc_spots(state: &PanelState) -> Vec<(i32, i32)> {
    let preview = state.map_preview.as_ref().expect("a Map Preview");
    preview.npc_spots.iter().map(|at| (at.x, at.y)).collect()
}

#[test]
fn the_map_preview_marks_where_the_npc_stands_once_the_player_is_on_its_map() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "maple admin");
    minerva.act(Action::SearchEnter);
    let picked = minerva.act(Action::SearchEnter).clone();
    assert!(npc_spots(&picked).is_empty(), "Sunset Sky has no Administrator");

    assert!(!destination(&picked).npc_marked, "not there yet");

    let there = arrive(&mut minerva, KERNING_SAMPLE, 2_000);

    assert!(destination(&there).arrived);
    assert_eq!(npc_spots(&there), [(900, 300)]);
    assert!(destination(&there).npc_marked);
}

#[test]
fn an_npc_osms_doesnt_place_on_its_map_is_found_but_not_marked() {
    let maps = vec![map("100000000", "Sunset Sky", 100, vec![]).npc_unplaced(npc(1, "Heena"))];
    let mut minerva = on_sunset_sky(maps);
    typed(&mut minerva, "heena");

    let state = minerva.act(Action::SearchEnter).clone();

    let to = destination(&state);
    assert_eq!((to.map_name.as_str(), to.arrived), ("Sunset Sky", true));
    assert!(npc_spots(&state).is_empty());
    assert!(!to.npc_marked, "nowhere to mark");
}

#[test]
fn an_npc_picked_on_the_map_the_player_is_on_is_marked_in_every_spot_it_stands_at_once() {
    let maps = vec![
        map("100000000", "Sunset Sky", 100, vec![])
            .npc(npc(419, "Treasure Chest"), 100, 200)
            .npc(npc(419, "Treasure Chest"), 300, 400)
            .npc(npc(1, "Heena"), 500, 600),
    ];
    let mut minerva = on_sunset_sky(maps);
    typed(&mut minerva, "treasure");

    let state = minerva.act(Action::SearchEnter).clone();

    assert!(destination(&state).arrived);
    assert_eq!(npc_spots(&state), [(100, 200), (300, 400)], "not Heena");
}

#[test]
fn the_npc_is_no_longer_marked_once_the_destination_is_cleared() {
    let mut minerva = on_sunset_sky(vec![map("100000000", "Sunset Sky", 100, vec![]).npc(npc(1, "Heena"), 500, 600)]);
    typed(&mut minerva, "heena");
    minerva.act(Action::SearchEnter);

    let state = minerva.act(Action::ClearHuntingTarget).clone();

    assert!(npc_spots(&state).is_empty());
}

#[test]
fn the_npcs_show_their_pictures_once_they_are_in() {
    let mut maps = towns();
    maps.push(map("100000004", "Henesys Park", 100, vec![]).npc(npc(2, "Heena's Sister").without_picture(), 10, 10));
    let mut minerva = on_sunset_sky(maps);
    let state = typed(&mut minerva, "heena");
    assert_eq!(state.search.npcs.shown[0].image, None, "not in yet");

    let state = minerva.tick(200).clone();

    let images: Vec<Option<&str>> = state.search.npcs.shown.iter().map(|npc| npc.image.as_deref()).collect();
    assert_eq!(images, [Some("picture:current/images/npcs/0000001.png"), None]);
}

#[test]
fn the_npcs_map_list_and_the_destination_show_the_npcs_picture() {
    let mut minerva = on_sunset_sky(towns());
    typed(&mut minerva, "maple admin");
    minerva.act(Action::SearchEnter);

    let listed = minerva.tick(200).clone();
    let picked = minerva.act(Action::SearchEnter).clone();

    let picture = Some("picture:current/images/npcs/9010000.png");
    assert_eq!(listed.search.picked_npc.and_then(|npc| npc.image).as_deref(), picture);
    assert_eq!(destination(&picked).npc.as_ref().and_then(|npc| npc.image.as_deref()), picture);
}

#[test]
fn an_npcs_picture_path_that_isnt_a_plain_one_is_a_format_change() {
    for path in ["../../settings.json", "/images/npcs/0000001.png", "images//0000001.png", "images/npc pictures/1.png"] {
        let heena = npc(1, "Heena");
        let world = osms_world(towns()).with_npc(common::osms::Npc { thumbnail: Some(path.into()), ..heena });
        let mut minerva = Minerva::new(FakeSource::answering_osms(vec![fresh_files(world.files())]));

        let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);

        assert_eq!(state.recognition, Recognition::FormatChanged, "{path}");
    }
}

#[test]
fn an_npc_the_lookup_doesnt_name_is_left_out() {
    let mut world = osms_world(towns());
    world.npcs.retain(|npc| npc.id != 1);
    let mut minerva = on_sunset_sky_from(FakeSource::with_world(world).with_items(item_world(vec![])));

    let state = typed(&mut minerva, "a");

    assert_eq!(state.recognition, Recognition::Ready);
    assert!(state.search.npcs.shown.iter().all(|npc| npc.id != "1"), "Heena has no name to match");
    assert_eq!(state.search.npcs.shown.len(), 2, "the rest are matched");
}
