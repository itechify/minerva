//! Recognition rests on OSMS Data Explorer's map list, in its file set: asked for at
//! startup, and without it the panel never passes raw readings off as real maps.

mod common;

use common::osms::{self, KERNING_CITY, SUNSET_SKY, map};
use common::{FakeSource, black_frame, sample_frame};
use minerva_core::{Minerva, PanelState, Recognition, TitleLines, TitleReading};
use std::sync::atomic::Ordering;
use std::task::Poll;

const KERNING: &str = "victoria-road__kerning-city--window";
const SUNSET: &str = "victoria-road__sunset-sky--window";

fn shown(state: &PanelState) -> Option<(&str, &str, &str)> {
    state.map.as_ref().map(|m| (m.id.as_str(), m.street_name.as_str(), m.map_name.as_str()))
}

#[test]
fn a_title_is_recognised_as_the_map_osms_lists_under_its_zero_padded_id() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_frame(sample_frame(KERNING, 0));

    let state = minerva.on_frame(sample_frame(KERNING, 333));

    assert_eq!(state.recognition, Recognition::Ready);
    assert_eq!(shown(state), Some(("010003000", "Victoria Road", "Kerning City")));
}

#[test]
fn names_osms_gives_with_spaces_around_them_are_trimmed() {
    let world = osms::game().changing(SUNSET_SKY, |m| map(m.id, "  Victoria Road ", "Sunset Sky   "));
    let mut minerva = Minerva::new(FakeSource::with_world(world));
    minerva.on_frame(sample_frame(SUNSET, 0));

    let state = minerva.on_frame(sample_frame(SUNSET, 333));

    assert_eq!(shown(state), Some(("010003070", "Victoria Road", "Sunset Sky")));
}

fn read_as(street: &str, map: &str, at_ms: u64) -> TitleReading {
    TitleReading { lines: Some(TitleLines { street: street.into(), map: map.into() }), found: Some(common::TITLE_FOUND), at_ms }
}

/// OSMS lists one map with neither a name nor a street. A map missing either can't be
/// told by its title, so it isn't in the map list: a reading of just its other line
/// isn't taken for it.
#[test]
fn a_map_without_a_name_or_a_street_is_not_in_the_map_list() {
    let world = osms::game()
        .with(map(100_000_000, "Victoria Road", " "))
        .with(map(100_000_001, "", "Lonely Field"));
    for (street, name) in [("Victoria Road", ""), ("", "Lonely Field")] {
        let mut minerva = Minerva::new(FakeSource::with_world(world.clone()));

        minerva.on_reading(read_as(street, name, 0));
        let state = minerva.on_reading(read_as(street, name, 333));

        assert_eq!(state.map, None, "{street:?} / {name:?}");
    }
}

#[test]
fn a_title_that_matches_no_known_map_is_not_shown_as_a_map() {
    let mut minerva = Minerva::new(FakeSource::with_world(osms::game().without(KERNING_CITY)));
    minerva.on_frame(sample_frame(KERNING, 0));

    let state = minerva.on_frame(sample_frame(KERNING, 333));

    assert_eq!(state.recognition, Recognition::Ready);
    assert_eq!(state.map, None);
    assert!(state.not_sure);
}

#[test]
fn when_osms_cannot_be_reached_and_nothing_is_saved_recognition_is_unavailable_and_no_map_is_shown() {
    let mut minerva = Minerva::new(FakeSource::osms_offline());
    minerva.on_frame(sample_frame(KERNING, 0));

    let state = minerva.on_frame(sample_frame(KERNING, 333));

    assert_eq!(state.recognition, Recognition::Unavailable);
    assert_eq!(state.map, None);
}

/// Setup's Finding your map needs the map list before the first frame, to name the map the
/// first frames show.
#[test]
fn the_file_set_is_asked_for_at_startup_before_any_frame() {
    let source = FakeSource::osms();
    let requests = source.osms_requests.clone();
    let mut minerva = Minerva::new(source);

    let state = minerva.tick(0);

    assert_eq!(requests.load(Ordering::SeqCst), 1);
    assert_eq!(state.recognition, Recognition::Ready);
}

#[test]
fn the_file_set_is_asked_for_once_not_on_every_frame() {
    let source = FakeSource::osms();
    let requests = source.osms_requests.clone();
    let mut minerva = Minerva::new(source);

    for i in 0..5 {
        minerva.on_frame(sample_frame(KERNING, 333 * i));
    }
    minerva.on_frame(black_frame(2000));

    assert_eq!(requests.load(Ordering::SeqCst), 1);
}

#[test]
fn with_nothing_saved_an_unreachable_osms_is_asked_again_after_a_minute_not_on_every_frame() {
    let source = FakeSource::osms_offline();
    let requests = source.osms_requests.clone();
    let mut minerva = Minerva::new(source);

    for i in 0..10 {
        minerva.on_frame(black_frame(333 * i));
    }
    assert_eq!(requests.load(Ordering::SeqCst), 1);

    minerva.on_frame(black_frame(60_000));
    assert_eq!(requests.load(Ordering::SeqCst), 2);
}

#[test]
fn recognition_starts_working_once_asking_again_finds_the_file_set() {
    let unreachable = Poll::Ready(Err(minerva_core::SourceError::Unreachable("offline".into())));
    let mut minerva = Minerva::new(FakeSource::answering_osms(vec![unreachable, common::fresh(osms::game())]));
    minerva.on_frame(sample_frame(KERNING, 0));

    minerva.on_frame(sample_frame(KERNING, 60_000));
    let state = minerva.on_frame(sample_frame(KERNING, 60_333));

    assert_eq!(state.recognition, Recognition::Ready);
    assert_eq!(shown(state).map(|(id, _, _)| id), Some("010003000"));
}

/// The set arrives on the third time the core checks on it. Nothing waits for it
/// meanwhile, and frames read before it arrives aren't recognised.
#[test]
fn a_slow_file_set_never_holds_up_the_panel_and_recognition_starts_once_it_arrives() {
    let mut minerva =
        Minerva::new(FakeSource::answering_osms(vec![Poll::Pending, Poll::Pending, common::fresh(osms::game())]));

    assert_eq!(minerva.tick(0).recognition, Recognition::Starting);
    let state = minerva.on_frame(sample_frame(KERNING, 333));
    assert_eq!(state.recognition, Recognition::Starting);
    assert_eq!(state.map, None);
    assert_eq!(minerva.tick(400).recognition, Recognition::Ready);

    minerva.on_frame(sample_frame(KERNING, 666));
    let state = minerva.on_frame(sample_frame(KERNING, 1000));
    assert_eq!(shown(state).map(|(id, _, _)| id), Some("010003000"));
}
