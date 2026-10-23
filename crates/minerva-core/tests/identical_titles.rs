//! Some maps share one Minimap Title, so the title alone can't say which of them the
//! player is on. When every such map would show the same Map Summary, mobs and Hidden
//! Portals (by destination and kind), the panel just shows it. Otherwise it prefers the
//! one connected to the map the player came from, and failing that lets the player
//! choose with a hotkey.
//!
//! There are no samples of these titles, so the Kerning City sample stands in for
//! them: in each test's world its title belongs to the maps under test.

mod common;

use common::osms::{self, CONSTRUCTION_SITE, KERNING_CITY, SUNSET_SKY, mob};
use common::{FakeSource, arrive, black_frame, sample_frame, KERNING_SAMPLE, SUNSET_SAMPLE, shown_id};
use minerva_core::{Action, Minerva, LevelRange, MapDataStatus, Outline, PanelState};

/// The titles OSMS's map list gives to more than one map: Maple Road / Mushroom Town and
/// Hidden Street / Coconut Harvest, and no others (checked by hand against the map names
/// fixture; a fact about OSMS's data, not something the core does).
const MUSHROOM_TOWNS: [&str; 2] = ["000000010", "000000020"];
const COCONUT_HARVESTS: [&str; 3] = ["090040200", "090040201", "090040202"];

fn id(id: &str) -> u64 {
    id.parse().unwrap()
}

/// `world`, with the Kerning City sample's title given to the maps `ids` and to no other.
fn titled_like_kerning_city(world: osms::World, ids: &[&str]) -> osms::World {
    let world = if ids.contains(&"010003000") { world } else { world.without(KERNING_CITY) };
    ids.iter().fold(world, |world, &id| {
        world.changing(self::id(id), |mut m| {
            m.street_name = "Victoria Road".into();
            m.name = "Kerning City".into();
            m
        })
    })
}

/// The candidates' IDs, which of them the panel shows, and whether the player chose it.
fn candidates(state: &PanelState) -> Option<(Vec<&str>, usize, bool)> {
    let c = state.candidates.as_ref()?;
    Some((c.maps.iter().map(|m| m.id.as_str()).collect(), c.shown, c.chosen))
}

/// The maps OSMS's map list titles `street` / `map`, under the Kerning City sample's
/// title, each given a different made-up number of Blue Snails so that none shows the
/// same as another.
fn real_shared_title_reading_as_kerning_city(street: &str, name: &str) -> osms::World {
    let world = osms::game();
    let ids: Vec<String> = world
        .maps
        .iter()
        .filter(|m| m.street_name.trim() == street && m.name.trim() == name)
        .map(|m| format!("{:09}", m.id))
        .collect();
    let world = ids.iter().enumerate().fold(world, |world, (i, id)| world.changing(self::id(id), |m| m.spawns(3, i as u32 + 1)));
    titled_like_kerning_city(world, &ids.iter().map(String::as_str).collect::<Vec<_>>())
}

#[test]
fn osmss_map_list_gives_the_mushroom_town_title_to_two_maps_and_coconut_harvest_to_three() {
    for (street, map, ids) in [
        ("Maple Road", "Mushroom Town", MUSHROOM_TOWNS.to_vec()),
        ("Hidden Street", "Coconut Harvest", COCONUT_HARVESTS.to_vec()),
    ] {
        let mut minerva = Minerva::new(FakeSource::with_world(real_shared_title_reading_as_kerning_city(street, map)));

        let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

        assert_eq!(candidates(&state), Some((ids, 0, false)), "{street} / {map}");
    }
}

#[test]
fn arriving_in_mushroom_town_shows_it_without_asking_since_both_mushroom_towns_show_the_same() {
    let mut minerva = Minerva::new(FakeSource::with_world(titled_like_kerning_city(osms::game(), &MUSHROOM_TOWNS)));
    minerva.on_frame(sample_frame(KERNING_SAMPLE, 0));

    let state = minerva.on_frame(sample_frame(KERNING_SAMPLE, 333));

    assert!(MUSHROOM_TOWNS.contains(&shown_id(state).unwrap()));
    assert_eq!(state.candidates, None);
    assert!(matches!(&state.data, MapDataStatus::Ready(data) if data.mobs.is_empty()));
}

#[test]
fn maps_alike_but_for_their_hidden_portals_are_offered_as_candidates() {
    // Neither has mobs, but the second Mushroom Town has a Hidden Portal here and the
    // first none.
    let world = osms::game().changing(id(MUSHROOM_TOWNS[1]), |m| m.portal("hide00", 10, 1_000_000));
    let mut minerva = Minerva::new(FakeSource::with_world(titled_like_kerning_city(world, &MUSHROOM_TOWNS)));

    let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

    assert_eq!(candidates(&state), Some((MUSHROOM_TOWNS.to_vec(), 0, false)));
    // What sets them apart: no mobs either way, but one Hidden Portal against none.
    let outlines: Vec<_> = state.candidates.as_ref().unwrap().maps.iter().map(|m| m.outline.clone()).collect();
    let none_mobs_and = |hidden_portals| Outline { mob_levels: None, first_mobs: vec![], hidden_portals };
    assert_eq!(outlines, [none_mobs_and(0), none_mobs_and(1)]);
}

/// Each Mushroom Town with one Hidden Portal: the same portal name or not, they show the
/// same if the portals lead to the same map and are the same kind.
#[test]
fn hidden_portals_are_compared_by_destination_and_kind() {
    let cases = [
        ("the same destination and kind, named apart", (10, 1_000_000, "a"), (10, 1_000_000, "b"), false),
        ("another destination", (10, 1_000_000, "a"), (10, 1_000_100, "a"), true),
        ("a Quest Portal against a hidden one", (10, 1_000_000, "a"), (11, 1_000_000, "a"), true),
    ];
    for (how, (kind_a, to_a, name_a), (kind_b, to_b, name_b), offered) in cases {
        let world = osms::game()
            .changing(id(MUSHROOM_TOWNS[0]), |m| m.portal(name_a, kind_a, to_a))
            .changing(id(MUSHROOM_TOWNS[1]), |m| m.portal(name_b, kind_b, to_b));
        let mut minerva = Minerva::new(FakeSource::with_world(titled_like_kerning_city(world, &MUSHROOM_TOWNS)));

        let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

        assert_eq!(state.candidates.is_some(), offered, "{how}");
    }
}

/// The Coconut Harvests, if they differed: the first with Sunset Sky's 5 mobs, the second
/// with the Construction Site's 6, the third with 2 mobs of its own.
fn coconut_harvests_that_differ() -> FakeSource {
    let game = osms::game();
    let spawns_of = |id: u64| game.maps.iter().find(|m| m.id == id).unwrap().spawns.clone();
    let (sunset, site) = (spawns_of(SUNSET_SKY), spawns_of(CONSTRUCTION_SITE));
    let world = osms::game()
        .with_mob(mob(1032, "Flyeye", 41, 1419, 99))
        .with_mob(mob(1059, "Minor Zombie", 58, 3619, 197))
        .changing(id(COCONUT_HARVESTS[0]), |m| osms::Map { spawns: sunset, ..m })
        .changing(id(COCONUT_HARVESTS[1]), |m| osms::Map { spawns: site, ..m })
        .changing(id(COCONUT_HARVESTS[2]), |m| m.spawns(1032, 18).spawns(1059, 10));
    FakeSource::with_world(titled_like_kerning_city(world, &COCONUT_HARVESTS))
}

fn mob_count(state: &PanelState) -> Option<usize> {
    match &state.data {
        MapDataStatus::Ready(data) => Some(data.mobs.len()),
        _ => None,
    }
}

fn coconut_candidates(shown: usize, chosen: bool) -> Option<(Vec<&'static str>, usize, bool)> {
    Some((COCONUT_HARVESTS.to_vec(), shown, chosen))
}

/// Each candidate's mob levels and first (lowest-level) mobs.
fn mobs_at_a_glance(state: &PanelState) -> Vec<Option<(LevelRange, Vec<String>)>> {
    let maps = &state.candidates.as_ref().expect("candidates").maps;
    maps.iter().map(|m| Some((m.outline.mob_levels?, m.outline.first_mobs.clone()))).collect()
}

#[test]
fn each_candidate_is_listed_with_its_mob_levels_and_first_mobs_to_tell_them_apart() {
    let mut minerva = Minerva::new(coconut_harvests_that_differ());

    let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

    let names = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
    assert_eq!(
        mobs_at_a_glance(&state),
        [
            Some((LevelRange { lowest: 2, highest: 14 }, names(&["Blue Snail", "Stump", "Orange Mushroom"]))),
            Some((LevelRange { lowest: 4, highest: 19 }, names(&["Red Snail", "Orange Mushroom", "Dark Stump"]))),
            Some((LevelRange { lowest: 41, highest: 58 }, names(&["Flyeye", "Minor Zombie"]))),
        ]
    );
}

#[test]
fn arriving_with_nothing_to_go_on_in_one_of_maps_that_differ_shows_the_first_and_offers_the_others() {
    let mut minerva = Minerva::new(coconut_harvests_that_differ());

    let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

    assert_eq!(shown_id(&state), Some("090040200"));
    assert_eq!(candidates(&state), coconut_candidates(0, false));
    assert_eq!(mob_count(&state), Some(5));
}

/// Kerning City shares its title with the Northern Top of Construction Site here. Sunset
/// Sky leads to Kerning City (and the Construction Site), not to the Northern Top.
fn kerning_city_and_northern_top() -> osms::World {
    titled_like_kerning_city(osms::game(), &["010003000", "010003081"])
}

#[test]
fn coming_from_a_map_connected_to_just_one_of_them_shows_that_one() {
    let mut minerva = Minerva::new(FakeSource::with_world(kerning_city_and_northern_top()));
    arrive(&mut minerva, SUNSET_SAMPLE, 0);

    minerva.on_frame(sample_frame(KERNING_SAMPLE, 666));
    let state = minerva.on_frame(sample_frame(KERNING_SAMPLE, 1000));

    assert_eq!(shown_id(state), Some("010003000"));
    assert_eq!(state.candidates, None);
    assert!(matches!(state.data, MapDataStatus::Ready(_)));
}

/// OSMS unreachable, with a saved copy over a week old: the map the player came from
/// still settles which map it is.
#[test]
fn coming_from_a_connected_map_settles_it_from_a_saved_copy_too() {
    const SAVED_AT: u64 = 1_790_812_800_000;
    let source = FakeSource::osms_offline().with_saved_set(kerning_city_and_northern_top(), SAVED_AT);
    let mut minerva = Minerva::new(source);
    minerva.on_wall_clock(SAVED_AT + 8 * 24 * 60 * 60 * 1000);
    arrive(&mut minerva, SUNSET_SAMPLE, 0);

    minerva.on_frame(sample_frame(KERNING_SAMPLE, 666));
    let state = minerva.on_frame(sample_frame(KERNING_SAMPLE, 1000));

    assert_eq!(shown_id(state), Some("010003000"));
    assert_eq!(state.candidates, None);
    assert!(matches!(state.data, MapDataStatus::SavedCopy { .. }));
}

#[test]
fn the_choose_candidate_hotkey_shows_each_candidate_in_turn_with_its_own_data() {
    let mut minerva = Minerva::new(coconut_harvests_that_differ());
    arrive(&mut minerva, KERNING_SAMPLE, 0);

    let state = minerva.act(Action::ChooseCandidate);
    assert_eq!(shown_id(state), Some("090040201"));
    assert_eq!(candidates(state), coconut_candidates(1, true));
    assert_eq!(mob_count(state), Some(6));

    let state = minerva.act(Action::ChooseCandidate);
    assert_eq!(shown_id(state), Some("090040202"));
    assert_eq!(mob_count(state), Some(2));

    let state = minerva.act(Action::ChooseCandidate);
    assert_eq!(shown_id(state), Some("090040200"));
    assert_eq!(candidates(state), coconut_candidates(0, true));
}

#[test]
fn a_chosen_candidate_holds_through_unreadable_frames_until_the_map_changes() {
    let mut minerva = Minerva::new(coconut_harvests_that_differ());
    arrive(&mut minerva, KERNING_SAMPLE, 0);
    minerva.act(Action::ChooseCandidate);

    minerva.on_frame(black_frame(666));
    let state = minerva.on_frame(sample_frame(KERNING_SAMPLE, 1000));
    assert_eq!(shown_id(state), Some("090040201"));
    assert_eq!(candidates(state), coconut_candidates(1, true));

    // Off to Sunset Sky and back: the choice is gone, and the first is shown again.
    minerva.on_frame(sample_frame(SUNSET_SAMPLE, 1333));
    let state = minerva.on_frame(sample_frame(SUNSET_SAMPLE, 1666));
    assert_eq!(shown_id(state), Some("010003070"));
    assert_eq!(state.candidates, None);
    minerva.on_frame(sample_frame(KERNING_SAMPLE, 2000));
    let state = minerva.on_frame(sample_frame(KERNING_SAMPLE, 2333));
    assert_eq!(shown_id(state), Some("090040200"));
    assert_eq!(candidates(state), coconut_candidates(0, false));
}

/// Settings from before this hotkey existed (or from before Ctrl+Shift) get Ctrl+Shift+N.
#[test]
fn ctrl_shift_n_shows_the_next_candidate_even_with_settings_from_before_it_existed() {
    let older = r#"{"version":1,"expanded":true,"hotkeys":{"togglePanel":"ctrl+shift+m","openMeowdbPage":"ctrl+shift+o"}}"#;
    let mut minerva = Minerva::new(coconut_harvests_that_differ());
    minerva.restore(serde_json::from_str(older).unwrap());
    arrive(&mut minerva, KERNING_SAMPLE, 0);

    minerva.on_hotkey("ctrl+shift+n");

    assert_eq!(candidates(minerva.panel_state()), coconut_candidates(1, true));
}

#[test]
fn the_choose_candidate_hotkey_does_nothing_when_there_is_nothing_to_choose() {
    let mut minerva = Minerva::new(FakeSource::with_world(titled_like_kerning_city(osms::game(), &MUSHROOM_TOWNS)));
    minerva.on_frame(sample_frame(KERNING_SAMPLE, 0));
    let before = minerva.on_frame(sample_frame(KERNING_SAMPLE, 333)).clone();

    let state = minerva.act(Action::ChooseCandidate);

    assert_eq!(*state, before);
}

/// Which map's picture the Map Preview shows, if any.
fn previewed(state: &PanelState) -> Option<&str> {
    state.map_preview.as_ref().map(|p| p.picture.as_str())
}

/// Each map has its own layout, so the Map Preview waits until it's settled which map the
/// player is on: shown the first of maps that differ, they see its figures but no picture,
/// until they choose; then the picture is the chosen one's.
#[test]
fn no_map_preview_shows_while_candidates_wait_for_a_choice_and_the_chosen_ones_shows_after() {
    let mut minerva = Minerva::new(coconut_harvests_that_differ());

    let state = arrive(&mut minerva, KERNING_SAMPLE, 0);
    assert_eq!(mob_count(&state), Some(5));
    assert_eq!(previewed(&state), None);

    let state = minerva.act(Action::ChooseCandidate);
    assert_eq!(previewed(state), Some("picture090040201"));
}

/// Maps that would show the same figures are shown without asking, but which of them the
/// player is on is still open: the layout may differ, so there is no picture.
#[test]
fn maps_alike_show_their_figures_without_asking_but_no_map_preview() {
    let mut minerva = Minerva::new(FakeSource::with_world(titled_like_kerning_city(osms::game(), &MUSHROOM_TOWNS)));

    let state = arrive(&mut minerva, KERNING_SAMPLE, 0);

    assert_eq!(state.candidates, None);
    assert!(matches!(state.data, MapDataStatus::Ready(_)));
    assert_eq!(previewed(&state), None);
}

#[test]
fn coming_from_a_map_connected_to_just_one_of_them_shows_its_map_preview() {
    let mut minerva = Minerva::new(FakeSource::with_world(kerning_city_and_northern_top()));
    arrive(&mut minerva, SUNSET_SAMPLE, 0);

    minerva.on_frame(sample_frame(KERNING_SAMPLE, 666));
    let state = minerva.on_frame(sample_frame(KERNING_SAMPLE, 1000));

    assert_eq!(previewed(state), Some("picture010003000"));
}
