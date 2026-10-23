//! OSMS's file set also gives what mob search needs about the world: each mob's ID, each
//! map's Mob Rate, every exit to another map with its portal kind, and each map's nearest
//! town. They show in Density, Distance and routes from the first launch, with the set in
//! hand.
//!
//! In the made-up game world Kerning City leads to Sunset Sky and the Construction Site,
//! and the Construction Site's invisible exit (type 1, off a Town, so a Hidden Portal) is
//! the only way up to its Northern Top.

mod common;

use common::osms::{self, CONSTRUCTION_SITE, DEAD_END, HENESYS_HUNTING_GROUND_I, NORTHERN_TOP};
use common::world::search_for;
use common::{FakeSource, arrive, fresh_files};
use minerva_core::osms::OsmsFiles;
use minerva_core::{Action, Minerva, Distance, PanelState, Recognition, SearchResult, Step, TitleLines, TitleReading};

const KERNING: &str = "victoria-road__kerning-city--window";

/// OSMS serving `world`, the player arrived on Kerning City as the set comes, and `mob`
/// searched for.
fn searched_from_kerning(world: osms::World, mob: &str) -> (Minerva, PanelState) {
    let mut minerva = Minerva::new(FakeSource::with_world(world));
    arrive(&mut minerva, KERNING, 0);
    let state = search_for(&mut minerva, mob);
    (minerva, state)
}

/// The result on the map named `map_name`.
fn result<'a>(state: &'a PanelState, map_name: &str) -> &'a SearchResult {
    let found = state.search.results.iter().find(|r| r.map_name == map_name);
    found.unwrap_or_else(|| panic!("no result on {map_name}: {:?}", state.search.results))
}

fn read_as(street: &str, map: &str, at_ms: u64) -> TitleReading {
    TitleReading { lines: Some(TitleLines { street: street.into(), map: map.into() }), found: Some(common::TITLE_FOUND), at_ms }
}

#[test]
fn each_mob_comes_with_its_id() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_reading(read_as("Victoria Road", "Henesys Hunting Ground I", 0));
    let state = minerva.on_reading(read_as("Victoria Road", "Henesys Hunting Ground I", 333));

    let mut ids: Vec<_> = state.data.data().unwrap().mobs.iter().map(|m| (m.name.as_str(), m.id.as_str())).collect();
    ids.sort();
    assert_eq!(
        ids,
        [
            ("Blue Snail", "3"),
            ("Green Mushroom", "13"),
            ("Orange Mushroom", "9"),
            ("Red Snail", "5"),
            ("Shroom", "4"),
            ("Slime", "7"),
            ("Stump", "6"),
        ]
    );
}

/// Orange Mushrooms: 5 on Henesys Hunting Ground I (Mob Rate ×1.5), 8 on Sunset Sky
/// (×0.6), and 10 on the Construction Site, for which OSMS gives no Mob Rate: ×1.
#[test]
fn each_maps_mob_rate_counts_in_density_to_the_hundredth_and_none_is_x1() {
    let (_, state) = searched_from_kerning(osms::game(), "orange");

    for (map, density) in [
        ("Henesys Hunting Ground I", 750),
        ("Sunset Sky", 480),
        ("Construction Site North of Kerning City", 1_000),
    ] {
        assert_eq!(result(&state, map).density_hundredths, density, "{map}");
    }
}

/// Octopuses: 6 on the Construction Site, 4 on the Northern Top above it.
#[test]
fn every_exit_counts_in_distance_and_a_hidden_portal_too() {
    let (_, state) = searched_from_kerning(osms::game(), "octopus");

    for (map, changes) in [("Construction Site North of Kerning City", 1), ("Northern Top of Construction Site", 2)] {
        assert_eq!(result(&state, map).distance, Distance::MapChanges(changes), "{map}");
    }
}

/// The game world, with the Construction Site's invisible exit up to the Northern Top
/// changed to this type and destination.
fn with_hidden_exit(kind: u64, to: u64) -> osms::World {
    osms::game().changing(CONSTRUCTION_SITE, |mut m| {
        let exit = m.portals.iter_mut().find(|p| p.name == "hiden00").unwrap();
        (exit.kind, exit.to) = (kind, to);
        m.exits = None;
        m
    })
}

#[test]
fn a_map_only_a_type_11_exit_leads_to_is_quest_only() {
    let (_, state) = searched_from_kerning(with_hidden_exit(11, NORTHERN_TOP), "octopus");

    assert_eq!(result(&state, "Northern Top of Construction Site").distance, Distance::QuestOnly);
}

#[test]
fn an_exit_into_the_same_map_or_to_the_dead_end_leads_nowhere() {
    for to in [CONSTRUCTION_SITE, DEAD_END] {
        let (_, state) = searched_from_kerning(with_hidden_exit(1, to), "octopus");

        assert_eq!(result(&state, "Northern Top of Construction Site").distance, Distance::NoRoute, "{to}");
        assert_eq!(result(&state, "Construction Site North of Kerning City").distance, Distance::MapChanges(1));
    }
}

/// The Construction Site's nearest town made up as Henesys Hunting Ground I, a map no
/// named scroll goes to.
#[test]
fn a_nearest_town_teleport_scroll_goes_to_the_nearest_town_osms_gives() {
    let world = osms::game().changing(CONSTRUCTION_SITE, |m| m.returning_to(HENESYS_HUNTING_GROUND_I));
    let (mut minerva, state) = searched_from_kerning(world, "orange");
    assert_eq!(result(&state, "Henesys Hunting Ground I").distance, Distance::NoRoute, "scrolls off");

    let state = minerva.act(Action::ToggleTeleportScrolls);

    let hunting_ground = result(state, "Henesys Hunting Ground I");
    assert_eq!(hunting_ground.distance, Distance::MapChanges(2));
    assert_eq!(hunting_ground.route.last().unwrap().by, Some(Step::TeleportScroll));
}

#[test]
fn a_mob_rate_nearest_town_or_exits_not_as_expected_is_a_format_change() {
    let files = osms::game().files();
    let altered = [
        ("a Mob Rate in words", files.maps.replace("\"mob_rate\":1.5", "\"mob_rate\":\"fast\"")),
        ("a negative Mob Rate", files.maps.replace("\"mob_rate\":1.5", "\"mob_rate\":-1.5")),
        ("a nearest town in words", files.maps.replace("\"return_map_id\":10003000", "\"return_map_id\":\"Kerning City\"")),
        ("exits in words", files.maps.replace("\"exits\":[", "\"exits\":[\"north\",")),
    ];
    for (how, maps) in altered {
        assert_ne!(maps, files.maps, "{how}");
        let mut minerva = Minerva::new(FakeSource::answering_osms(vec![fresh_files(OsmsFiles { maps, ..files.clone() })]));

        let state = arrive(&mut minerva, KERNING, 0);

        assert_eq!(state.recognition, Recognition::FormatChanged, "{how}");
    }
}
