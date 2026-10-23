//! The recognised map shows with its data from OSMS's file set, which the core holds
//! whole: no page is asked for, and no map waits for one. The panel lists every mob,
//! credits both sites, and still opens the map's MeowDB page.

mod common;

use common::osms::{self, SUNSET_SKY, map, mob};
use common::{FakeSource, arrive, sample_frame, SUNSET_SAMPLE, KERNING_SAMPLE, shown_map_name};
use minerva_core::{Minerva, MapData, MapDataStatus, OSMS_AND_MEOWDB_CREDIT, Request};

#[test]
fn the_recognised_map_shows_at_once_with_its_data() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_frame(sample_frame(SUNSET_SAMPLE, 0));

    let state = minerva.on_frame(sample_frame(SUNSET_SAMPLE, 333));

    assert_eq!(shown_map_name(state), Some("Sunset Sky"));
    assert!(matches!(&state.data, MapDataStatus::Ready(data) if data.mobs.len() == 5));
}

#[test]
fn before_any_map_is_shown_there_is_no_data_and_no_credit() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.on_frame(sample_frame(SUNSET_SAMPLE, 0));

    assert_eq!(state.data, MapDataStatus::NoMap);
    assert_eq!(state.credit, None);
}

/// The panel when the shown map, Sunset Sky, has these mobs: (ID, name, level, spawn
/// points).
fn sunset_with(mobs: &[(u64, &str, u32, u32)]) -> MapData {
    let mut world = osms::game().with(map(SUNSET_SKY, "Victoria Road", "Sunset Sky"));
    for &(id, name, level, points) in mobs {
        world = world.with_mob(mob(id, name, level, 100, 10)).changing(SUNSET_SKY, |m| m.spawns(id, points));
    }
    let mut minerva = Minerva::new(FakeSource::with_world(world));
    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);
    let MapDataStatus::Ready(data) = &state.data else { panic!("{:?}", state.data) };
    data.clone()
}

#[test]
fn mobs_are_sorted_by_level_then_name_whatever_order_osms_gives() {
    let data = sunset_with(&[(101, "Stump", 5, 1), (102, "Ice Sentinel", 30, 1), (103, "Fire Sentinel", 30, 1), (104, "Snail", 1, 1)]);

    let order: Vec<_> = data.mobs.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(order, ["Snail", "Stump", "Fire Sentinel", "Ice Sentinel"]);
}

/// However many there are and however rare: the section folds to its heading if it's
/// too long.
#[test]
fn the_panel_lists_every_mob_on_the_map_by_level() {
    let data = sunset_with(&[
        (101, "Wild Boar", 25, 20),
        (102, "Snail", 1, 2),
        (103, "Stump", 5, 18),
        (104, "Fire Boar", 32, 3),
        (105, "Horny Mushroom", 22, 15),
        (106, "Axe Stump", 17, 12),
        (107, "Ghost Stump", 19, 25),
        (108, "Ligator", 33, 5),
    ]);

    let listed: Vec<_> = data.mobs.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        listed,
        ["Snail", "Stump", "Axe Stump", "Ghost Stump", "Horny Mushroom", "Wild Boar", "Fire Boar", "Ligator"]
    );
}

/// The data is OSMS Data Explorer's; the title and the mobs' names open NiaMeowDB's pages.
#[test]
fn the_shown_map_credits_osms_for_its_data_and_meowdb_for_its_pages() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

    assert_eq!(state.credit, Some(OSMS_AND_MEOWDB_CREDIT));
    assert_eq!(OSMS_AND_MEOWDB_CREDIT, "Data: OSMS Data Explorer (osmsdataexplorer.com) · Pages: NiaMeowDB (meowdb.com)");
}

#[test]
fn the_shown_map_still_links_to_its_meowdb_page() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);

    assert_eq!(state.map.as_ref().map(|m| m.url.as_str()), Some("https://meowdb.com/msclassic/maps/010003070"));
}

/// Settings from before this hotkey existed get Ctrl+Shift+M.
#[test]
fn ctrl_shift_m_opens_the_shown_maps_meowdb_page_even_with_settings_from_before_it_existed() {
    let older = r#"{"version":1,"expanded":true,"hotkeys":{"togglePanel":"ctrl+shift+m","editMode":"ctrl+shift+e"}}"#;
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(serde_json::from_str(older).unwrap());
    arrive(&mut minerva, SUNSET_SAMPLE, 0);

    let opened = minerva.on_hotkey("ctrl+shift+m");

    assert_eq!(opened, Some(Request::OpenInBrowser { url: "https://meowdb.com/msclassic/maps/010003070".into() }));
}

/// The overlay gets the panel state as JSON: what the panel renders, and nothing of the
/// world graph.
#[test]
fn the_overlay_receives_the_shown_maps_data_without_its_portal_graph() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let json = serde_json::to_value(arrive(&mut minerva, SUNSET_SAMPLE, 0)).unwrap();

    assert_eq!(json["data"]["status"], "ready");
    assert_eq!(json["data"]["mobs"].as_array().map(Vec::len), Some(5));
    assert!(json["data"]["summary"].is_object() && json["data"]["hiddenPortals"].is_array());
    assert!(json["data"].get("connectedMaps").is_none() && json["data"].get("portals").is_none());
    assert_eq!(json["recognition"], serde_json::json!({ "status": "ready" }));
}
