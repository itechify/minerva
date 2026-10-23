//! Reading a title takes a while, so the shell reads title captures with a
//! `TitleReader` on its own thread and hands the core each `TitleReading`: the window
//! tracking and the panel never wait for recognition. A reading counts exactly as the
//! frame it came from would.

mod common;

use common::osms;
use common::{FakeSource, sample_frame};
use minerva_core::{Minerva, PanelState, ScreenPoint, TitleCapture, TitleLines, TitleReader, TitleReading};

const KERNING: &str = "victoria-road__kerning-city--window";

fn shown(state: &PanelState) -> Option<(&str, &str)> {
    state.map.as_ref().map(|m| (m.id.as_str(), m.map_name.as_str()))
}

fn read_as(street: &str, map: &str, at_ms: u64) -> TitleReading {
    TitleReading { lines: Some(TitleLines { street: street.into(), map: map.into() }), found: Some(common::TITLE_FOUND), at_ms }
}

#[test]
fn frames_read_on_another_thread_switch_the_map_like_frames_fed_to_the_core() {
    let readings = std::thread::spawn(|| {
        let mut reader = TitleReader::bundled();
        let origin = ScreenPoint { x: 0, y: 0 };
        [0, 333].map(|at_ms| reader.read(&TitleCapture { frame: sample_frame(KERNING, at_ms), origin }))
    })
    .join()
    .unwrap();
    let mut minerva = Minerva::new(FakeSource::osms());

    let [first, second] = readings;
    minerva.on_reading(first);
    let state = minerva.on_reading(second);

    assert_eq!(shown(state), Some(("010003000", "Kerning City")));
    assert!(!state.not_sure);
}

/// Reported in play: the recogniser ran "Mountain" and "I" together.
#[test]
fn a_title_read_as_west_rocky_mountaini_snaps_to_west_rocky_mountain_i() {
    let mut minerva = Minerva::new(FakeSource::osms());

    minerva.on_reading(read_as("Victoria Road", "West Rocky MountainI", 0));
    let state = minerva.on_reading(read_as("Victoria Road", "West Rocky MountainI", 333));

    assert_eq!(shown(state), Some(("010004020", "West Rocky Mountain I")));
    assert!(!state.not_sure);
}

const KERNING_CITY: &str = "010003000";
const TREE_I: &str = "010002060";
const TREE_II: &str = "010002061";
const TREE_III: &str = "010002062";

/// Reported in play: ocrs read "The Tree That Grew II" with an unknown glyph for the
/// numeral, and the panel confidently showed The Tree That Grew I, the shortest name.
#[test]
fn a_title_whose_numeral_is_unreadable_is_not_sure_rather_than_the_shortest_name() {
    let mut minerva = Minerva::new(FakeSource::osms());

    minerva.on_reading(read_as("Victoria Road", "The Tree That Grew ?", 0));
    let state = minerva.on_reading(read_as("Victoria Road", "The Tree That Grew ?", 333));

    assert_eq!(shown(state), None);
    assert!(state.not_sure);
}

#[test]
fn an_unreadable_numeral_is_not_taken_for_the_name_without_one() {
    // OSMS lists both Pig Park and Pig Park II.
    let mut minerva = Minerva::new(FakeSource::osms());

    minerva.on_reading(read_as("Hidden Street", "Pig Park ?", 0));
    let state = minerva.on_reading(read_as("Hidden Street", "Pig Park ?", 333));

    assert_eq!(shown(state), None);
    assert!(state.not_sure);
}

/// The game font draws I, l, 1 and | alike, as one upright stroke.
#[test]
fn a_numeral_read_with_look_alike_strokes_snaps_to_its_map() {
    for numeral in ["ll", "|I", "1l", "Il"] {
        let mut minerva = Minerva::new(FakeSource::osms());
        let map = format!("The Tree That Grew {numeral}");

        minerva.on_reading(read_as("Victoria Road", &map, 0));
        let state = minerva.on_reading(read_as("Victoria Road", &map, 333));

        assert_eq!(shown(state), Some((TREE_II, "The Tree That Grew II")), "{map}");
        assert!(!state.not_sure, "{map}");
    }
}

/// Folding look-alikes and weighing numerals apart never confuses two real titles.
#[test]
fn every_title_in_osmss_map_list_read_exactly_shows_that_title() {
    let mut minerva = Minerva::new(FakeSource::osms());
    let mut wrong = Vec::new();
    let names = osms::map_names();
    // The nameless map isn't in the map list.
    let titled = names.maps.iter().filter(|m| !m.name.trim().is_empty() && !m.street_name.trim().is_empty());
    for (i, entry) in titled.enumerate() {
        let (street, map) = (entry.street_name.trim(), entry.name.trim());
        let at_ms = 1000 * i as u64;
        minerva.on_reading(read_as(street, map, at_ms));
        let state = minerva.on_reading(read_as(street, map, at_ms + 333));

        let shown = state.map.as_ref().map(|m| (m.street_name.as_str(), m.map_name.as_str()));
        if shown != Some((street, map)) || state.not_sure {
            wrong.push(format!("{street} / {map}: panel shows {shown:?} (not sure: {})", state.not_sure));
        }
    }
    assert!(wrong.is_empty(), "wrong maps:\n{}", wrong.join("\n"));
}

/// A source whose map list says map `from` leads to the maps `to`, and every other map
/// leads nowhere.
fn leading(from: &str, to: &[&str]) -> FakeSource {
    let id = |id: &str| id.parse::<u64>().unwrap();
    let to: Vec<u64> = to.iter().map(|to| id(to)).collect();
    let mut world = osms::map_names();
    world.maps = world.maps.into_iter().map(|m| osms::Map { exits: Some(vec![]), ..m }).collect();
    FakeSource::with_world(world.changing(id(from), |m| m.exits(&to)))
}

#[test]
fn an_unreadable_numeral_is_settled_by_the_map_the_player_came_from_when_it_leads_to_just_one() {
    let mut minerva = Minerva::new(leading(KERNING_CITY, &[TREE_II]));
    minerva.on_reading(read_as("Victoria Road", "Kerning City", 0));
    minerva.on_reading(read_as("Victoria Road", "Kerning City", 333));

    minerva.on_reading(read_as("Victoria Road", "The Tree That Grew ?", 666));
    let state = minerva.on_reading(read_as("Victoria Road", "The Tree That Grew ?", 1000));

    assert_eq!(shown(state), Some((TREE_II, "The Tree That Grew II")));
    assert!(!state.not_sure);
}

#[test]
fn an_unreadable_numeral_is_not_sure_when_the_map_the_player_came_from_leads_to_several() {
    let mut minerva = Minerva::new(leading(KERNING_CITY, &[TREE_II, TREE_III]));
    minerva.on_reading(read_as("Victoria Road", "Kerning City", 0));
    minerva.on_reading(read_as("Victoria Road", "Kerning City", 333));

    minerva.on_reading(read_as("Victoria Road", "The Tree That Grew ?", 666));
    let state = minerva.on_reading(read_as("Victoria Road", "The Tree That Grew ?", 1000));

    assert_eq!(shown(state), Some((KERNING_CITY, "Kerning City")));
    assert!(state.not_sure);
}

/// On The Tree That Grew I, which leads to II, an unreadable numeral could be either:
/// the player may not have moved.
#[test]
fn an_unreadable_numeral_on_one_of_the_maps_is_not_sure_when_it_leads_to_another() {
    let mut minerva = Minerva::new(leading(TREE_I, &[TREE_II]));
    minerva.on_reading(read_as("Victoria Road", "The Tree That Grew I", 0));
    minerva.on_reading(read_as("Victoria Road", "The Tree That Grew I", 333));

    minerva.on_reading(read_as("Victoria Road", "The Tree That Grew ?", 666));
    let state = minerva.on_reading(read_as("Victoria Road", "The Tree That Grew ?", 1000));

    assert_eq!(shown(state), Some((TREE_I, "The Tree That Grew I")));
    assert!(state.not_sure);
}

#[test]
fn an_unreadable_reading_keeps_the_last_map_marked_not_sure() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_reading(read_as("Victoria Road", "Kerning City", 0));
    minerva.on_reading(read_as("Victoria Road", "Kerning City", 333));

    let state = minerva.on_reading(TitleReading { lines: None, found: Some(common::TITLE_FOUND), at_ms: 666 });

    assert_eq!(shown(state), Some(("010003000", "Kerning City")));
    assert!(state.not_sure);
}
