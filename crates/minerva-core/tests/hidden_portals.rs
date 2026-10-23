//! The panel lists the shown map's Hidden Portals: the exits the player can't see, each
//! with where it leads, from OSMS's portals. Which portals are hidden follows the Hidden
//! Portal rule and the portal override list. Where on the map each is goes to the Map
//! Preview (#60), so the rows hold no position. Served from made-up worlds in OSMS's
//! shape.

mod common;

use common::osms::{self, CONSTRUCTION_SITE, DEAD_END, KERNING_CITY, NORTHERN_TOP, map};
use common::{FakeSource, arrive, fresh_files, CONSTRUCTION_SAMPLE, KERNING_SAMPLE, SUNSET_SAMPLE, read_as, FIELD};
use minerva_core::osms::OsmsFiles;
use minerva_core::{Minerva, HiddenPortal, MapDataStatus, PanelState, PortalOverride, Recognition};

const NORTHERN: &str = "hidden-street__northern-top-of-construction-site--window";

/// The panel on the made-up Field, with the game's maps around it.
fn on_field(change: impl FnOnce(osms::Map) -> osms::Map) -> PanelState {
    on_field_with(osms::game(), change, Vec::new())
}

fn on_field_with(
    world: osms::World,
    change: impl FnOnce(osms::Map) -> osms::Map,
    overrides: Vec<PortalOverride>,
) -> PanelState {
    let world = world.with(change(map(FIELD, "Made-up Street", "Field")));
    let mut minerva = Minerva::new(FakeSource::with_world(world)).with_portal_overrides(overrides);
    minerva.on_reading(read_as("Made-up Street", "Field", 0));
    minerva.on_reading(read_as("Made-up Street", "Field", 333)).clone()
}

fn hidden_portals(state: &PanelState) -> &[HiddenPortal] {
    match &state.data {
        MapDataStatus::Ready(data) => &data.hidden_portals,
        other => panic!("no map data: {other:?}"),
    }
}

/// Where each Hidden Portal leads, and whether it is a Quest Portal.
fn destinations(state: &PanelState) -> Vec<(&str, bool)> {
    hidden_portals(state).iter().map(|p| (p.to_map_name.as_str(), p.quest)).collect()
}

#[test]
fn the_construction_sites_invisible_exit_is_a_hidden_portal_up_to_a_hidden_street() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = arrive(&mut minerva, CONSTRUCTION_SAMPLE, 0);

    assert_eq!(
        hidden_portals(&state),
        [HiddenPortal {
            name: "hiden00".into(),
            to_map_id: "010003081".into(),
            to_map_name: "Northern Top of Construction Site".into(),
            to_street_name: "Hidden Street".into(),
            to_mark: None,
            to_hidden_street: true,
            quest: false,
        }]
    );
}

/// Kerning City's invisible exits lead into its buildings: on a Town they are Doors,
/// drawn into the scenery, not hidden.
#[test]
fn the_doors_of_a_town_are_not_hidden_portals() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

    assert_eq!(hidden_portals(&state), []);
}

/// Sunset Sky's invisible portals are teleports within the map; the Northern Top's only
/// exit is visible (type 2); a walk-into exit (type 3) counts as visible.
#[test]
fn visible_exits_walk_into_exits_and_teleports_within_the_map_are_not_hidden_portals() {
    for sample in [SUNSET_SAMPLE, NORTHERN] {
        let mut minerva = Minerva::new(FakeSource::osms());
        let state = arrive(&mut minerva, sample, 0);
        assert_eq!(hidden_portals(&state), [], "{sample}");
    }

    let state = on_field(|m| m.portal("east00", 2, KERNING_CITY).portal("in00", 3, CONSTRUCTION_SITE));
    assert_eq!(hidden_portals(&state), []);
}

#[test]
fn hidden_portals_of_type_10_and_quest_portals_of_type_11_are_hidden_portals() {
    let state = on_field(|m| m.portal("hide00", 10, KERNING_CITY).portal("quest00", 11, CONSTRUCTION_SITE));

    assert_eq!(destinations(&state), [("Kerning City", false), ("Construction Site North of Kerning City", true)]);
}

/// Only invisible exits are Doors: a Town's hidden and Quest Portals are still hidden.
#[test]
fn a_towns_hidden_and_quest_portals_are_still_hidden_portals() {
    let state = on_field(|m| m.town().portal("in00", 1, KERNING_CITY).portal("hide00", 10, NORTHERN_TOP).portal("q00", 11, CONSTRUCTION_SITE));

    assert_eq!(
        destinations(&state),
        [("Northern Top of Construction Site", false), ("Construction Site North of Kerning City", true)]
    );
}

/// OSMS lists portals into the map itself and to its dead end (spawn points); neither
/// goes anywhere, whether or not OSMS marks it as within the map. Nor does a hidden
/// teleport within the map.
#[test]
fn an_exit_into_the_same_map_or_to_the_dead_end_or_a_hidden_teleport_is_not_a_hidden_portal() {
    let state = on_field(|m| m.portal("sp", 1, DEAD_END).portal("back00", 10, FIELD).teleport("hide01", "hide02"));

    assert_eq!(hidden_portals(&state), []);

    let hidden_teleport = on_field(|m| {
        let mut m = m.teleport("hide01", "hide02");
        m.portals[0].kind = 10;
        m
    });
    assert_eq!(hidden_portals(&hidden_teleport), []);
}

#[test]
fn only_destinations_on_a_hidden_street_are_labelled_hidden_street() {
    let state = on_field(|m| m.portal("a", 10, NORTHERN_TOP).portal("b", 10, KERNING_CITY));

    let labelled: Vec<_> = hidden_portals(&state).iter().map(|p| (p.to_map_name.as_str(), p.to_hidden_street)).collect();
    assert_eq!(labelled, [("Northern Top of Construction Site", true), ("Kerning City", false)]);
}

/// A destination outside OSMS's map list takes its name from `lookups.json`, with no
/// street; one that isn't there either is named by its ID.
#[test]
fn a_destination_outside_the_map_list_is_named_from_osms_lookups_or_else_by_its_id() {
    let world = osms::game().without(88_001_000).without(88_000_000).naming(88_001_000, "Backalley Semi-Basement");

    let state = on_field_with(world, |m| m.portal("a", 10, 88_001_000).portal("b", 10, 88_000_000), Vec::new());

    let named: Vec<_> = hidden_portals(&state)
        .iter()
        .map(|p| (p.to_map_id.as_str(), p.to_map_name.as_str(), p.to_street_name.as_str(), p.to_hidden_street))
        .collect();
    assert_eq!(
        named,
        [("088001000", "Backalley Semi-Basement", "", false), ("088000000", "088000000", "", false)]
    );
}

/// Exceptions found in play go on the override list, keyed by map ID and portal name.
/// Here the Field's visible west exit is listed as if it were found hidden; the
/// Construction Site's override doesn't touch the Field's portal of the same name.
#[test]
fn an_exit_the_override_list_marks_hidden_is_a_hidden_portal() {
    let overrides = vec![
        PortalOverride { map_id: "100000000", portal: "west00", hidden: true },
        PortalOverride { map_id: "010003080", portal: "east00", hidden: true },
    ];

    let state = on_field_with(
        osms::game(),
        |m| m.portal("west00", 2, KERNING_CITY).portal("east00", 2, CONSTRUCTION_SITE),
        overrides,
    );

    assert_eq!(destinations(&state), [("Kerning City", false)]);
}

#[test]
fn an_exit_the_override_list_marks_not_hidden_is_left_out() {
    let overrides = vec![PortalOverride { map_id: "100000000", portal: "in00", hidden: false }];

    let state = on_field_with(osms::game(), |m| m.portal("in00", 10, KERNING_CITY), overrides);

    assert_eq!(hidden_portals(&state), []);
}

/// Never a partial list: portals, or the names they're shown by, that aren't as the
/// reader expects are a changed format, with no map shown.
#[test]
fn a_file_set_whose_portals_are_not_as_expected_is_a_format_change() {
    let files = osms::game().files();
    let altered = [
        ("portals.json isn't JSON", OsmsFiles { portals: "[".into(), ..files.clone() }),
        ("a portal's type in words", OsmsFiles { portals: files.portals.replace("\"type\":1,", "\"type\":\"invisible\","), ..files.clone() }),
        ("a destination renamed", OsmsFiles { portals: files.portals.replace("\"dest_map\"", "\"to_map\""), ..files.clone() }),
        ("a destination that isn't a map ID", OsmsFiles { portals: files.portals.replace("\"dest_map\":\"10003081\"", "\"dest_map\":\"north\""), ..files.clone() }),
        ("a portal's position in words", OsmsFiles { portals: files.portals.replace("\"x\":1400", "\"x\":\"far right\""), ..files.clone() }),
        ("within the map in words", OsmsFiles { portals: files.portals.replace("\"intra_map\":true", "\"intra_map\":\"yes\""), ..files.clone() }),
        ("no map names in lookups.json", OsmsFiles { lookups: files.lookups.replace("\"map_names\"", "\"maps\""), ..files.clone() }),
        ("map_manifest.json isn't a list of pictures", OsmsFiles { map_manifest: "[]".into(), ..files.clone() }),
        ("a Town flag in words", OsmsFiles { maps: files.maps.replace("\"is_town\":true", "\"is_town\":\"yes\""), ..files.clone() }),
    ];
    for (how, files) in altered {
        let mut minerva = Minerva::new(FakeSource::answering_osms(vec![fresh_files(files)]));

        let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

        assert_eq!(state.recognition, Recognition::FormatChanged, "{how}");
        assert_eq!(state.map, None, "{how}");
    }
}
