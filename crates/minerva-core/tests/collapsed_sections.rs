//! The Map, Navigation and Mobs sections: a click on a section's heading collapses it to
//! that heading, and another opens it again. All start open. A collapsed section stays
//! collapsed from map to map and between sessions, until the player opens it. The Map
//! section keeps the Hidden Portals section's name (`hiddenPortals`) in the settings.

mod common;

use common::{arrive, SUNSET_SAMPLE, KERNING_SAMPLE, shown_map_name};
use common::world::{loaded, map, orange, world};
use minerva_core::{Action, CollapsedSections, Minerva, Section, Settings};

#[test]
fn every_section_starts_open() {
    let minerva = Minerva::new(world(vec![]));

    let state = minerva.panel_state();
    assert_eq!(state.collapsed_sections, CollapsedSections { hidden_portals: false, mobs: false, navigation: false });
    assert_eq!(minerva.settings().collapsed_sections, CollapsedSections::default());
}

#[test]
fn a_click_collapses_its_section_alone_and_another_opens_it_again() {
    let mut minerva = Minerva::new(world(vec![]));

    let state = minerva.act(Action::ToggleSection(Section::HiddenPortals)).clone();
    assert_eq!(state.collapsed_sections, CollapsedSections { hidden_portals: true, mobs: false, navigation: false });

    let state = minerva.act(Action::ToggleSection(Section::Mobs)).clone();
    assert_eq!(state.collapsed_sections, CollapsedSections { hidden_portals: true, mobs: true, navigation: false });

    let state = minerva.act(Action::ToggleSection(Section::HiddenPortals)).clone();
    assert_eq!(state.collapsed_sections, CollapsedSections { hidden_portals: false, mobs: true, navigation: false });
}

#[test]
fn the_navigation_section_collapses_and_opens_again_on_its_own() {
    let mut minerva = Minerva::new(world(vec![]));

    let state = minerva.act(Action::ToggleSection(Section::Navigation)).clone();
    assert_eq!(state.collapsed_sections, CollapsedSections { hidden_portals: false, mobs: false, navigation: true });

    let state = minerva.act(Action::ToggleSection(Section::Navigation)).clone();
    assert_eq!(state.collapsed_sections, CollapsedSections::default());
}

#[test]
fn a_collapsed_section_stays_collapsed_on_the_next_map() {
    let mut minerva = loaded(vec![
        map("100", "Sunset Sky", 100, vec![orange(5)]),
        map("103", "Kerning City", 100, vec![]),
    ]);
    arrive(&mut minerva, SUNSET_SAMPLE, 100);
    minerva.act(Action::ToggleSection(Section::Mobs));
    minerva.act(Action::ToggleSection(Section::Navigation));

    let state = arrive(&mut minerva, KERNING_SAMPLE, 2_000);

    assert_eq!(shown_map_name(&state), Some("Kerning City"));
    assert!(state.collapsed_sections.mobs, "still collapsed after the map changed");
    assert!(state.collapsed_sections.navigation, "still collapsed after the map changed");
    assert!(!state.collapsed_sections.hidden_portals);
}

#[test]
fn collapsed_sections_are_saved_and_restored_next_session() {
    let mut minerva = Minerva::new(world(vec![]));
    minerva.act(Action::ToggleSection(Section::HiddenPortals));
    minerva.act(Action::ToggleSection(Section::Navigation));
    let json = serde_json::to_string(&minerva.settings()).unwrap();
    assert!(json.contains(r#""collapsedSections":{"hiddenPortals":true,"mobs":false,"navigation":true}"#), "{json}");

    let mut restarted = Minerva::new(world(vec![]));
    let state = restarted.restore(serde_json::from_str::<Settings>(&json).unwrap()).clone();

    assert_eq!(state.collapsed_sections, CollapsedSections { hidden_portals: true, mobs: false, navigation: true });
    let state = restarted.act(Action::ToggleSection(Section::HiddenPortals));
    assert!(!state.collapsed_sections.hidden_portals, "opened again");
    assert!(!restarted.settings().collapsed_sections.hidden_portals);
}

#[test]
fn a_settings_file_from_before_collapsing_has_every_section_open() {
    let before = r#"{"version":4,"hotkeys":{"togglePanel":"ctrl+shift+x"},"taxis":true}"#;
    let mut minerva = Minerva::new(world(vec![]));

    let state = minerva.restore(serde_json::from_str(before).unwrap()).clone();

    assert_eq!(state.collapsed_sections, CollapsedSections::default());
    assert!(state.search.travel.taxis, "the rest of the file still reads");
}

/// Settings saved before there was a Navigation section keep their collapsed sections, and
/// it is open.
#[test]
fn a_settings_file_from_before_the_navigation_section_keeps_its_sections_with_navigation_open() {
    let before = r#"{"version":4,"collapsedSections":{"hiddenPortals":true,"mobs":true}}"#;
    let mut minerva = Minerva::new(world(vec![]));

    let state = minerva.restore(serde_json::from_str(before).unwrap()).clone();

    assert_eq!(state.collapsed_sections, CollapsedSections { hidden_portals: true, mobs: true, navigation: false });
}

#[test]
fn the_overlay_names_the_sections_as_it_sends_them() {
    let sections =
        [(r#""hiddenPortals""#, Section::HiddenPortals), (r#""mobs""#, Section::Mobs), (r#""navigation""#, Section::Navigation)];
    for (sent, section) in sections {
        assert_eq!(serde_json::from_str::<Section>(sent).unwrap(), section);
    }

    let minerva = Minerva::new(world(vec![]));
    let json = serde_json::to_value(minerva.panel_state()).unwrap();
    assert_eq!(json["collapsedSections"], serde_json::json!({ "hiddenPortals": false, "mobs": false, "navigation": false }));
}
