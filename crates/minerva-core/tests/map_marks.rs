//! Map Marks (GLOSSARY): beside a map's name, wherever it shows, the emblem the game's
//! Mini Map draws beside its title, for the area the map belongs to. OSMS's map data names
//! each map's mark and where its picture is (made up here, in OSMS's shape); a map names
//! its mark by OSMS's name for it, and the panel state holds one table of the marks, each
//! with the area's name as players know it and its picture once it's in. Every mark's
//! picture is asked for first, before any other picture, and once.

mod common;


use common::osms::{self, CONSTRUCTION_SITE, KERNING_CITY, NORTHERN_TOP, SUNSET_SKY};
use common::world::{search_for, tick_through};
use common::{FakeSource, SUNSET_SKY_ID, arrive, fresh_files, SUNSET_SAMPLE, KERNING_SAMPLE, CONSTRUCTION_SAMPLE, shown_id, NORTHERN_TOP_ID};
use minerva_core::osms::OsmsFiles;
use minerva_core::{
    BESTS_VERSION, Minerva, MapBests, MapDataStatus, MapMark, PanelControl, PanelState, Recognition, SavedBests,
    VisitBest,
};

/// Where OSMS keeps mark `mark`'s picture, as the source is asked for it: below its
/// pictures, in the current data set.
fn mark_picture(mark: &str) -> String {
    format!("current/images/map_marks/{mark}.png")
}

/// The made-up game world with marks: Kerning City and the maps up from it Kerning
/// City's, the Northern Top Lith Harbor's (OSMS's `Rith`), and pictures of those and of
/// Mushroom Town's and Henesys's marks.
fn marked() -> osms::World {
    osms::game()
        .with_mark_icon("KerningCity")
        .with_mark_icon("Rith")
        .with_mark_icon("MushroomVillage")
        .with_mark_icon("Henesys")
        .changing(KERNING_CITY, |m| m.marked("KerningCity"))
        .changing(SUNSET_SKY, |m| m.marked("KerningCity"))
        .changing(CONSTRUCTION_SITE, |m| m.marked("KerningCity"))
        .changing(NORTHERN_TOP, |m| m.marked("Rith"))
}

/// The shown map's mark.
fn title_mark(state: &PanelState) -> Option<&str> {
    state.map.as_ref().and_then(|map| map.mark.as_deref())
}

#[test]
fn the_title_names_its_maps_mark_and_the_table_each_marks_area_and_picture() {
    let mut minerva = Minerva::new(FakeSource::with_world(marked()));

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);
    assert_eq!(title_mark(&state), Some("KerningCity"));
    let names: Vec<(&str, &str)> = state.map_marks.iter().map(|(mark, m)| (mark.as_str(), m.name.as_str())).collect();
    assert_eq!(
        names,
        [("Henesys", "Henesys"), ("KerningCity", "Kerning City"), ("MushroomVillage", "Mushroom Town"), ("Rith", "Lith Harbor")]
    );
    assert!(state.map_marks.values().all(|mark| mark.image.is_none()), "not in yet");

    let state = tick_through(&mut minerva, 400, 1_000);
    assert_eq!(
        state.map_marks["Rith"],
        MapMark { name: "Lith Harbor".into(), image: Some(format!("picture:{}", mark_picture("Rith"))) }
    );
}

#[test]
fn every_marks_picture_is_asked_for_first_and_once() {
    let source = FakeSource::with_world(marked());
    let requests = source.picture_requests.clone();
    let mut minerva = Minerva::new(source);

    arrive(&mut minerva, SUNSET_SAMPLE, 0);
    tick_through(&mut minerva, 400, 5_000);
    arrive(&mut minerva, CONSTRUCTION_SAMPLE, 5_050);
    tick_through(&mut minerva, 5_400, 10_000);

    let asked = requests.lock().unwrap().clone();
    let marks: Vec<String> = ["Henesys", "KerningCity", "MushroomVillage", "Rith"].map(mark_picture).into();
    assert_eq!(asked[..4], marks[..], "before the map's picture: {asked:?}");
    assert_eq!(asked[4], "maps/picture010003070.webp");
    for mark in &marks {
        assert_eq!(asked.iter().filter(|path| *path == mark).count(), 1, "{mark}");
    }
}

#[test]
fn a_map_without_a_mark_or_with_one_osms_has_no_picture_of_has_none() {
    // Kerning City has none ("None" in maps.json); Sunset Sky's has no picture.
    let world = osms::game().with_mark_icon("KerningCity").changing(SUNSET_SKY, |m| m.marked("Amherst"));
    let mut minerva = Minerva::new(FakeSource::with_world(world));

    assert_eq!(title_mark(&arrive(&mut minerva, KERNING_SAMPLE, 0)), None);
    let state = arrive(&mut minerva, SUNSET_SAMPLE, 1_000);
    assert_eq!(title_mark(&state), None);
    assert!(!state.map_marks.contains_key("Amherst"));
}

#[test]
fn without_map_mark_icons_the_set_reads_and_no_map_has_a_mark() {
    let mut minerva = Minerva::new(FakeSource::with_world(marked().without_mark_icons()));

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);

    assert_eq!(state.recognition, Recognition::Ready);
    assert_eq!(title_mark(&state), None);
    assert!(state.map_marks.is_empty());
}

#[test]
fn a_marks_picture_path_that_isnt_a_plain_one_is_a_format_change() {
    for path in ["../../settings.json", "/images/map_marks/Rith.png", "images//Rith.png", "images/map marks/Rith.png"] {
        let files: OsmsFiles = marked().with_mark_icon_at("Dungeon", path).files();
        let mut minerva = Minerva::new(FakeSource::answering_osms(vec![fresh_files(files)]));

        let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

        assert_eq!(state.recognition, Recognition::FormatChanged, "{path}");
    }
}

#[test]
fn hidden_portals_and_the_map_previews_marks_name_their_destinations_mark() {
    let mut minerva = Minerva::new(FakeSource::with_world(marked()));

    let state = arrive(&mut minerva, CONSTRUCTION_SAMPLE, 0);

    let MapDataStatus::Ready(data) = &state.data else { panic!("no data: {:?}", state.data) };
    let hidden: Vec<(&str, Option<&str>)> =
        data.hidden_portals.iter().map(|p| (p.to_map_id.as_str(), p.to_mark.as_deref())).collect();
    assert_eq!(hidden, [(NORTHERN_TOP_ID, Some("Rith"))]);
    let preview = state.map_preview.as_ref().expect("a Map Preview");
    let marks: Vec<(&str, Option<&str>)> =
        preview.marks.iter().map(|m| (m.to_map_id.as_str(), m.to_mark.as_deref())).collect();
    assert_eq!(marks, [(SUNSET_SKY_ID, Some("KerningCity")), (NORTHERN_TOP_ID, Some("Rith"))]);
}

#[test]
fn search_results_and_their_routes_name_each_maps_mark() {
    let mut minerva = Minerva::new(FakeSource::with_world(marked()));
    arrive(&mut minerva, KERNING_SAMPLE, 0);

    // Octopuses spawn on the Construction Site and the Northern Top above it.
    let state = search_for(&mut minerva, "octopus");

    let results: Vec<(&str, Option<&str>)> =
        state.search.results.iter().map(|r| (r.map_name.as_str(), r.mark.as_deref())).collect();
    assert_eq!(
        results,
        [("Construction Site North of Kerning City", Some("KerningCity")), ("Northern Top of Construction Site", Some("Rith"))]
    );
    let route: Vec<Option<&str>> = state.search.results[1].route.iter().map(|stop| stop.mark.as_deref()).collect();
    assert_eq!(route, [Some("KerningCity"), Some("KerningCity"), Some("Rith")]);
}

/// The made-up game world with the Construction Site and Sunset Sky sharing the Kerning
/// City sample's title, the Construction Site's mark `construction_site`'s, Sunset Sky's
/// Kerning City's.
fn sharing_a_title_marked(construction_site: &str) -> FakeSource {
    let world = marked()
        .changing(CONSTRUCTION_SITE, |m| m.marked(construction_site))
        .titled("Victoria Road", "Kerning City", &[CONSTRUCTION_SITE, SUNSET_SKY]);
    FakeSource::with_world(world)
}

#[test]
fn while_candidates_wait_for_a_choice_the_title_shows_a_mark_only_if_they_share_it() {
    let mut minerva = Minerva::new(sharing_a_title_marked("Rith"));
    let unsure = arrive(&mut minerva, KERNING_SAMPLE, 0);
    assert_eq!(unsure.candidates.as_ref().map(|c| c.chosen), Some(false));
    assert_eq!(title_mark(&unsure), None, "the Construction Site's or Sunset Sky's");

    // Chosen, it's the shown map's.
    minerva.on_hotkey("ctrl+shift+n");
    let chosen = minerva.panel_state();
    assert_eq!(shown_id(chosen), Some(SUNSET_SKY_ID));
    assert_eq!(title_mark(chosen), Some("KerningCity"));

    // Sharing it, they show it from the start.
    let mut minerva = Minerva::new(sharing_a_title_marked("KerningCity"));
    assert_eq!(title_mark(&arrive(&mut minerva, KERNING_SAMPLE, 0)), Some("KerningCity"));
}

#[test]
fn the_personal_bests_window_names_each_maps_mark() {
    let mut minerva = Minerva::new(FakeSource::with_world(marked()));
    let best = |id: &str| MapBests { id: id.into(), visits: vec![VisitBest { exp_per_hour: 7200, ..VisitBest::default() }] };
    let unassigned = vec![best(SUNSET_SKY_ID), best(NORTHERN_TOP_ID)];
    minerva.restore_bests(SavedBests { version: BESTS_VERSION, unassigned, characters: Vec::new() });
    arrive(&mut minerva, KERNING_SAMPLE, 0);

    minerva.press(PanelControl::TogglePersonalBests);

    let rows: Vec<(&str, Option<&str>)> =
        minerva.panel_state().personal_bests.characters[0].maps.iter().map(|m| (m.id.as_str(), m.mark.as_deref())).collect();
    // Tied on EXP/h, by ID.
    assert_eq!(rows, [(SUNSET_SKY_ID, Some("KerningCity")), (NORTHERN_TOP_ID, Some("Rith"))]);
}

#[test]
fn a_marks_area_is_named_as_players_know_it() {
    let marks = ["Dungeon", "ElNath", "ForgottenHollow", "KerningCity", "MushroomVillage", "Rith", "SouthPerry"];
    let world = marks.iter().fold(marked(), |world, mark| world.with_mark_icon(mark));
    let mut minerva = Minerva::new(FakeSource::with_world(world));

    let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

    let names: Vec<&str> = marks.iter().map(|mark| state.map_marks[*mark].name.as_str()).collect();
    assert_eq!(
        names,
        ["Dungeon", "El Nath", "Forgotten Hollow", "Kerning City", "Mushroom Town", "Lith Harbor", "Southperry"]
    );
}

#[test]
fn the_overlay_gets_the_marks_as_it_renders_them() {
    let mut minerva = Minerva::new(FakeSource::with_world(marked()));
    arrive(&mut minerva, SUNSET_SAMPLE, 0);
    let state = serde_json::to_value(tick_through(&mut minerva, 400, 1_000)).unwrap();

    assert_eq!(state["map"]["mark"], "KerningCity");
    assert_eq!(
        state["mapMarks"]["KerningCity"],
        serde_json::json!({ "name": "Kerning City", "image": format!("picture:{}", mark_picture("KerningCity")) })
    );
}
