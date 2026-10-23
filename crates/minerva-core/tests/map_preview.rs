//! The Map section shows a Map Preview: OSMS's picture of the shown map, named by its
//! content hash in `map_manifest.json`, with every portal marked where OSMS puts it in the
//! picture's pixels. Each mark says where it leads (another map, within the map, or a map
//! not in the data), what kind of portal it is by the Hidden Portal rule, the destination's
//! name and street, its Hidden Portal row if it has one, and for a teleport within the map
//! where it lands. Served from made-up worlds in OSMS's shape.
//!
//! The picture itself is asked of the source by its path under OSMS's data while the
//! preview shows, and kept for the run.

mod common;

use common::osms::{self, CONSTRUCTION_SITE, DEAD_END, KERNING_CITY, NORTHERN_TOP, SUNSET_SKY, map};
use common::{FakeSource, arrive, fresh_files};
use minerva_core::{
    Minerva, Landing, Leads, MapDataStatus, MapPreview, PanelState, PicturePoint, PortalKind, PortalMark, PreviewImage,
    SourceError, TitleLines, TitleReading,
};

/// A made-up map whose portals the tests give, on a street of its own.
const FIELD: u64 = 100_000_000;

fn read_as(street: &str, map: &str, at_ms: u64) -> TitleReading {
    TitleReading { lines: Some(TitleLines { street: street.into(), map: map.into() }), found: Some(common::TITLE_FOUND), at_ms }
}

/// The panel on the made-up Field, in `world`.
fn on_field_in(world: osms::World, change: impl FnOnce(osms::Map) -> osms::Map) -> PanelState {
    let world = world.with(change(map(FIELD, "Made-up Street", "Field")));
    let mut minerva = Minerva::new(FakeSource::with_world(world));
    minerva.on_reading(read_as("Made-up Street", "Field", 0));
    minerva.on_reading(read_as("Made-up Street", "Field", 333)).clone()
}

/// The panel on the made-up Field, with the game's maps around it.
fn on_field(change: impl FnOnce(osms::Map) -> osms::Map) -> PanelState {
    on_field_in(osms::game(), change)
}

fn preview(state: &PanelState) -> &MapPreview {
    state.map_preview.as_ref().expect("a Map Preview")
}

fn marks(state: &PanelState) -> &[PortalMark] {
    &preview(state).marks
}

#[test]
fn each_portal_is_marked_where_osms_puts_it_in_the_maps_picture() {
    let state = on_field(|m| m.portal_at("east00", 2, KERNING_CITY, 1480, 612));

    assert_eq!(preview(&state).picture, "picture100000000");
    assert_eq!(
        marks(&state),
        [PortalMark {
            at: PicturePoint { x: 1480, y: 612 },
            leads: Leads::AnotherMap,
            kind: PortalKind::Visible,
            to_map_id: "010003000".into(),
            to_map_name: "Kerning City".into(),
            to_street_name: "Victoria Road".into(),
            to_mark: None,
            lands_at: None,
            hidden_portal: None,
            next_on_route: false,
        }]
    );
}

/// Each mark's destination, kind, and the destination of the Hidden Portal row it points at.
fn kinds_and_rows(state: &PanelState) -> Vec<(&str, PortalKind, Option<&str>)> {
    let MapDataStatus::Ready(data) = &state.data else { panic!("no map data: {:?}", state.data) };
    let row = |i: usize| data.hidden_portals[i].to_map_name.as_str();
    marks(state).iter().map(|m| (m.to_map_name.as_str(), m.kind, m.hidden_portal.map(row))).collect()
}

#[test]
fn hidden_and_quest_portals_are_marked_as_such_each_pointing_at_its_row() {
    let state = on_field(|m| {
        m.portal("east00", 2, KERNING_CITY).portal("hide00", 10, NORTHERN_TOP).portal("quest00", 11, CONSTRUCTION_SITE)
    });

    assert_eq!(
        kinds_and_rows(&state),
        [
            ("Kerning City", PortalKind::Visible, None),
            ("Northern Top of Construction Site", PortalKind::Hidden, Some("Northern Top of Construction Site")),
            ("Construction Site North of Kerning City", PortalKind::Quest, Some("Construction Site North of Kerning City")),
        ]
    );
}

/// Each mark's destination, where it leads and its kind.
fn destinations(state: &PanelState) -> Vec<(&str, Leads, PortalKind)> {
    marks(state).iter().map(|m| (m.to_map_name.as_str(), m.leads, m.kind)).collect()
}

/// Kerning City is a Town: its invisible exits lead into its buildings, Doors drawn into
/// the scenery, marked like the visible exits to Sunset Sky and the Construction Site.
#[test]
fn a_town_has_a_map_preview_with_its_doors_marked_as_doors_to_other_maps() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = arrive(&mut minerva, "victoria-road__kerning-city--window", 0);

    assert_eq!(preview(&state).picture, "picture010003000");
    assert_eq!(
        destinations(&state),
        [
            ("Kerning City Self-Defence Item Store", Leads::AnotherMap, PortalKind::Door),
            ("Kerning City Pharmacy", Leads::AnotherMap, PortalKind::Door),
            ("Sunset Sky", Leads::AnotherMap, PortalKind::Visible),
            ("Construction Site North of Kerning City", Leads::AnotherMap, PortalKind::Visible),
        ]
    );
}

/// Spawn points and the like lead to the dead end: not portals anyone can take.
#[test]
fn portals_to_the_dead_end_are_not_marked() {
    let state = on_field(|m| m.portal("sp", 0, DEAD_END).portal("west00", 2, SUNSET_SKY).portal("tp", 6, DEAD_END));

    assert_eq!(destinations(&state), [("Sunset Sky", Leads::AnotherMap, PortalKind::Visible)]);
}

/// OSMS colours a portal to a map it doesn't list grey: its name comes from
/// `lookups.json`, with no street, or failing that it is its ID.
#[test]
fn a_portal_to_a_map_not_in_the_data_is_marked_as_such_with_the_name_osms_gives_it() {
    let world = osms::game().without(88_001_000).without(88_000_000).naming(88_001_000, "Backalley Semi-Basement");

    let state = on_field_in(world, |m| m.portal("in10", 2, 88_001_000).portal("b", 10, 88_000_000));

    let named: Vec<_> = marks(&state).iter().map(|m| (m.to_map_name.as_str(), m.to_street_name.as_str(), m.leads, m.kind)).collect();
    assert_eq!(
        named,
        [
            ("Backalley Semi-Basement", "", Leads::MapNotInTheData, PortalKind::Visible),
            ("088000000", "", Leads::MapNotInTheData, PortalKind::Hidden),
        ]
    );
}

/// OSMS lists one map with no name or street, which isn't in the map list: a portal to it
/// leads to a map not in the data, named as OSMS's lookups name it, or else by its ID.
#[test]
fn a_portal_to_a_map_with_no_name_or_street_is_marked_as_to_a_map_not_in_the_data() {
    let world = osms::game().with(map(100_000_001, "", "")).with(map(100_000_002, " ", " ")).naming(100_000_002, "Somewhere");

    let state = on_field_in(world, |m| m.portal("in00", 2, 100_000_001).portal("in01", 2, 100_000_002));

    let named: Vec<_> = marks(&state).iter().map(|m| (m.to_map_name.as_str(), m.to_street_name.as_str(), m.leads)).collect();
    assert_eq!(named, [("100000001", "", Leads::MapNotInTheData), ("Somewhere", "", Leads::MapNotInTheData)]);
}

/// Each teleport within the map says where it lands, and at which mark: a pair lands at
/// each other, both ways, and a teleport may land at another's mark without being landed at.
#[test]
fn a_teleport_within_the_map_lands_at_the_mark_of_the_portal_it_names() {
    let state = on_field(|m| {
        m.teleport("hide01", "hide02").at(300, 1200).teleport("hide02", "hide01").at(340, 640).teleport("up00", "hide02").at(900, 1210)
    });

    let landings: Vec<_> = marks(&state).iter().map(|m| (m.leads, m.lands_at)).collect();
    assert_eq!(
        landings,
        [
            (Leads::WithinTheMap, Some(Landing { at: PicturePoint { x: 340, y: 640 }, mark: Some(1), both_ways: true })),
            (Leads::WithinTheMap, Some(Landing { at: PicturePoint { x: 300, y: 1200 }, mark: Some(0), both_ways: true })),
            (Leads::WithinTheMap, Some(Landing { at: PicturePoint { x: 340, y: 640 }, mark: Some(1), both_ways: false })),
        ]
    );
}

/// The overlay gets the panel state as JSON: each mark's spot beside the rest, and each
/// landing's spot, its mark, and whether the teleport is one of a pair.
#[test]
fn the_overlay_receives_where_a_teleport_lands_and_whether_it_is_one_of_a_pair() {
    let state = on_field(|m| m.teleport("hide01", "hide02").at(300, 1200).teleport("hide02", "hide01").at(340, 640));

    let json = serde_json::to_value(&state).unwrap();

    let mark = &json["mapPreview"]["marks"][0];
    assert_eq!((&mark["x"], &mark["y"]), (&serde_json::json!(300), &serde_json::json!(1200)));
    assert_eq!(mark["landsAt"], serde_json::json!({ "x": 340, "y": 640, "mark": 1, "bothWays": true }));
}

/// Many teleports land on a spawn point, which leads to the dead end and so has no mark:
/// the landing is where that point is. One whose landing OSMS doesn't list lands nowhere
/// the preview can show.
#[test]
fn a_teleport_landing_on_an_unmarked_point_lands_where_it_is_and_one_landing_nowhere_listed_has_no_landing() {
    let state = on_field(|m| {
        m.portal_at("tohide03", 1, DEAD_END, 520, 940).teleport("hide03", "tohide03").at(80, 940).teleport("hide04", "nowhere")
    });

    let landings: Vec<_> = marks(&state).iter().map(|m| m.lands_at).collect();
    assert_eq!(landings, [Some(Landing { at: PicturePoint { x: 520, y: 940 }, mark: None, both_ways: false }), None]);
}

/// A teleport within the map is to the map itself, named as it is; never a Hidden Portal
/// row, though an invisible one is hidden and marked so. On a Town too: a Door leads into
/// a building, another map. A portal into the map itself that OSMS doesn't flag as a
/// teleport is one all the same.
#[test]
fn a_hidden_teleport_is_marked_hidden_within_the_map_but_has_no_row_even_on_a_town() {
    for town in [false, true] {
        let state = on_field(|m| {
            let m = if town { m.town() } else { m };
            m.teleport("hide01", "hide02").portal("back00", 10, FIELD)
        });

        let MapDataStatus::Ready(data) = &state.data else { panic!("no map data") };
        assert_eq!(data.hidden_portals, [], "town: {town}");
        let named: Vec<_> = marks(&state)
            .iter()
            .map(|m| (m.to_map_name.as_str(), m.to_street_name.as_str(), m.leads, m.kind, m.hidden_portal))
            .collect();
        assert_eq!(
            named,
            [
                ("Field", "Made-up Street", Leads::WithinTheMap, PortalKind::Hidden, None),
                ("Field", "Made-up Street", Leads::WithinTheMap, PortalKind::Hidden, None),
            ],
            "town: {town}"
        );
    }
}

/// OSMS gives a position as it is in the picture, even just off its edge.
#[test]
fn a_portal_off_the_pictures_edge_is_marked_where_osms_puts_it() {
    let state = on_field(|m| m.portal_at("west00", 2, SUNSET_SKY, -12, 1810));

    assert_eq!(marks(&state)[0].at, PicturePoint { x: -12, y: 1810 });
}

#[test]
fn a_map_without_a_picture_in_osmss_manifest_has_no_map_preview() {
    let world = osms::game().with(map(FIELD, "Made-up Street", "Field").portal("west00", 2, SUNSET_SKY));
    let mut files = world.files();
    let mut manifest: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&files.map_manifest).unwrap();
    manifest.remove("100000000");
    files.map_manifest = serde_json::Value::Object(manifest).to_string();
    let mut minerva = Minerva::new(FakeSource::answering_osms(vec![fresh_files(files)]));
    minerva.on_reading(read_as("Made-up Street", "Field", 0));

    let state = minerva.on_reading(read_as("Made-up Street", "Field", 333));

    assert!(matches!(state.data, MapDataStatus::Ready(_)));
    assert_eq!(state.map_preview, None);
}

#[test]
fn no_map_preview_shows_before_a_map_is() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.tick(0);

    assert_eq!(state.map_preview, None);
}

/// The panel on Kerning City, from `source`, at 0 ms.
fn in_kerning_city(source: FakeSource) -> (Minerva, PanelState) {
    let mut minerva = Minerva::new(source);
    let state = arrive(&mut minerva, "victoria-road__kerning-city--window", 0);
    (minerva, state)
}

fn image(state: &PanelState) -> &PreviewImage {
    &preview(state).image
}

/// The maps' pictures asked for, in order, leaving out the Mob Cards' animations.
fn map_pictures(requests: &std::sync::Mutex<Vec<String>>) -> Vec<String> {
    requests.lock().unwrap().iter().filter(|path| path.starts_with("maps/")).cloned().collect()
}

/// The picture is asked for by its path under OSMS's data; the overlay loads it from where
/// the source says it is.
#[test]
fn the_shown_maps_picture_is_asked_for_by_its_path_and_loads_from_where_the_source_says() {
    let source = FakeSource::osms().pictures_taking(1);
    let requests = source.picture_requests.clone();
    let (mut minerva, state) = in_kerning_city(source);
    assert_eq!(*image(&state), PreviewImage::Loading);

    let state = minerva.tick(400).clone();
    assert_eq!(*image(&state), PreviewImage::Loading);
    let state = minerva.tick(500).clone();

    assert_eq!(*image(&state), PreviewImage::Ready { url: "picture:maps/picture010003000.webp".into() });
    assert_eq!(*requests.lock().unwrap(), ["maps/picture010003000.webp"]);
}

#[test]
fn a_picture_that_cant_be_had_is_unavailable() {
    for error in [SourceError::Unreachable("offline".into()), SourceError::FormatChanged("404".into())] {
        let source = FakeSource::osms().with_pictures(move |_| Err(error.clone()));
        let (mut minerva, _) = in_kerning_city(source);

        let state = minerva.tick(400);

        assert_eq!(*image(state), PreviewImage::Unavailable);
    }
}

/// OSMS couldn't be reached: the picture is asked for again five minutes on, while the
/// map is still shown.
#[test]
fn an_unreachable_picture_is_asked_for_again_five_minutes_on() {
    let mut reachable = false;
    let source = FakeSource::osms().with_pictures(move |path| {
        let answer = if reachable { Ok(format!("picture:{path}")) } else { Err(SourceError::Unreachable("offline".into())) };
        reachable = true;
        answer
    });
    let requests = source.picture_requests.clone();
    let (mut minerva, _) = in_kerning_city(source);
    minerva.tick(400);

    let state = minerva.tick(400 + 4 * 60_000).clone();
    assert_eq!(*image(&state), PreviewImage::Unavailable);
    let state = minerva.tick(400 + 5 * 60_000).clone();

    assert_eq!(*image(&state), PreviewImage::Ready { url: "picture:maps/picture010003000.webp".into() });
    assert_eq!(map_pictures(&requests).len(), 2);
}

/// A picture is kept for the run: coming back to a map shows it at once, asking nothing.
#[test]
fn a_map_shown_again_shows_its_picture_without_asking_again() {
    let source = FakeSource::osms();
    let requests = source.picture_requests.clone();
    let (mut minerva, _) = in_kerning_city(source);
    minerva.tick(400);
    arrive(&mut minerva, "victoria-road__sunset-sky--window", 1000);
    minerva.tick(1400);

    let state = arrive(&mut minerva, "victoria-road__kerning-city--window", 2000);

    assert_eq!(*image(&state), PreviewImage::Ready { url: "picture:maps/picture010003000.webp".into() });
    assert_eq!(map_pictures(&requests), ["maps/picture010003000.webp", "maps/picture010003070.webp"]);
}

/// While it isn't settled which of the maps sharing a title the player is on, no picture
/// is asked for.
#[test]
fn no_picture_is_asked_for_while_no_map_preview_shows() {
    let source = common::sharing_a_title();
    let requests = source.picture_requests.clone();
    let (mut minerva, state) = in_kerning_city(source);
    assert!(state.candidates.is_some());

    minerva.tick(400);

    assert_eq!(map_pictures(&requests), Vec::<String>::new());
}

/// The map comes first: its picture is asked for ahead of its mobs' animations.
#[test]
fn the_maps_picture_is_asked_for_before_the_mob_cards_animations() {
    let source = FakeSource::osms().pictures_taking(1);
    let requests = source.picture_requests.clone();
    let mut minerva = Minerva::new(source);
    arrive(&mut minerva, "victoria-road__sunset-sky--window", 0);

    minerva.tick(400);

    assert_eq!(requests.lock().unwrap().first().map(String::as_str), Some("maps/picture010003070.webp"));
}
