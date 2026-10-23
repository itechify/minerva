//! The player drives Minerva with global hotkeys: Ctrl+Shift+letter, because
//! Windows and other apps already hold many Win+Alt letters (Win+Alt+F among them).
//! They are live only while the game or Minerva is in front, so other apps keep
//! their own Ctrl+Shift shortcuts. The shell registers what the panel state lists while
//! it says they are live, and hands every press to the core, which decides what it does.

mod common;

use common::{FakeSource, arrive, SUNSET_SAMPLE};
use minerva_core::{
    Minerva, ExpBar, ExpBarReading, Foreground, Hotkeys, PanelControl,
    PlacementMode, Request, Settings, SetupReason, TrackingState,
};

fn ctrl_shift(letter: &str) -> String {
    format!("ctrl+shift+{letter}")
}

#[test]
fn the_hotkeys_are_ctrl_shift_letters() {
    let minerva = Minerva::new(FakeSource::osms());

    assert_eq!(
        minerva.panel_state().hotkeys,
        Hotkeys {
            toggle_panel: ctrl_shift("c"),
            cycle_placement: ctrl_shift("p"),
            open_meowdb_page: ctrl_shift("m"),
            setup: ctrl_shift("s"),
            choose_candidate: ctrl_shift("n"),
            search: ctrl_shift("f"),
            grind_tracker: ctrl_shift("g"),
            item_card: ctrl_shift("i"),
            item_lookup: ctrl_shift("u"),
            deal_finder: ctrl_shift("d"),
        }
    );
}

#[test]
fn ctrl_shift_i_shows_the_item_card_and_ctrl_shift_u_looks_the_item_up_even_with_a_settings_file_from_before_them() {
    let before_item_lookup = r#"{"version":4,"hotkeys":{"togglePanel":"ctrl+shift+x","grindTracker":"ctrl+shift+g"}}"#;
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(before_item_lookup).unwrap()).clone();

    assert_eq!((state.hotkeys.item_card.as_str(), state.hotkeys.item_lookup.as_str()), ("ctrl+shift+i", "ctrl+shift+u"));
    assert_eq!(state.hotkeys.toggle_panel, ctrl_shift("x"), "the player's own bindings are kept");
    let saved = minerva.settings();
    assert_eq!((saved.hotkeys.item_card.as_str(), saved.hotkeys.item_lookup.as_str()), ("ctrl+shift+i", "ctrl+shift+u"));
    assert_eq!(saved.version, 8, "and saved with the rest");
}

/// The Item Card took Item Lookup's key: the key the player presses over an item now
/// shows its card, and Item Lookup, which opens the MeowDB page, moved to Ctrl+Shift+U.
#[test]
fn a_settings_file_from_before_the_item_card_gives_its_item_lookup_key_to_the_card() {
    let rebound = r#"{"version":4,"hotkeys":{"itemLookup":"ctrl+shift+k"}}"#;
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(rebound).unwrap()).clone();

    assert_eq!(state.hotkeys.item_card, ctrl_shift("k"));
    assert_eq!(state.hotkeys.item_lookup, ctrl_shift("u"));
}

#[test]
fn a_settings_file_from_before_the_item_card_with_item_lookup_off_has_the_card_off() {
    let off = r#"{"version":4,"hotkeys":{"itemLookup":""}}"#;
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(off).unwrap()).clone();

    assert_eq!(state.hotkeys.item_card, "");
    assert_eq!(state.hotkeys.item_lookup, ctrl_shift("u"));
}

#[test]
fn a_settings_file_from_before_the_item_card_with_ctrl_shift_u_in_use_has_item_lookup_off() {
    let taken = r#"{"version":4,"hotkeys":{"grindTracker":"ctrl+shift+u","itemLookup":"ctrl+shift+i"}}"#;
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(taken).unwrap()).clone();

    assert_eq!(state.hotkeys.grind_tracker, ctrl_shift("u"));
    assert_eq!(state.hotkeys.item_card, ctrl_shift("i"));
    assert_eq!(state.hotkeys.item_lookup, "", "off rather than share the Grind Tracker's key");
}

#[test]
fn a_settings_file_from_version_5_keeps_both_item_hotkeys_as_saved() {
    let saved = r#"{"version":5,"hotkeys":{"itemCard":"ctrl+shift+j","itemLookup":"ctrl+shift+k"}}"#;
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(saved).unwrap()).clone();

    assert_eq!((state.hotkeys.item_card.as_str(), state.hotkeys.item_lookup.as_str()), ("ctrl+shift+j", "ctrl+shift+k"));
}

#[test]
fn rebound_item_hotkeys_are_the_ones_that_show_the_card_and_look_the_item_up() {
    let mut minerva = Minerva::new(FakeSource::osms());
    let hotkeys = Hotkeys { item_card: "ctrl+shift+j".into(), item_lookup: "ctrl+shift+k".into(), ..Hotkeys::default() };
    minerva.restore(Settings { hotkeys, ..Settings::default() });
    minerva.on_game_window(Some(common::window_at_origin(1920, 1200)));

    assert_eq!(minerva.on_hotkey("ctrl+shift+i"), None);
    assert_eq!(minerva.on_hotkey("ctrl+shift+u"), None);
    assert_eq!(minerva.on_hotkey("ctrl+shift+k"), Some(Request::CaptureItemTooltip));
    minerva.on_item_tooltip_reading(None);
    assert_eq!(minerva.on_hotkey("ctrl+shift+j"), Some(Request::CaptureItemTooltip));
}

/// Settings saved by version 2, with its Win+Alt hotkeys.
const WIN_ALT_SETTINGS: &str = r#"{
  "version": 2,
  "expanded": true,
  "placementMode": "dockedLeft",
  "hotkeys": {
    "togglePanel": "super+alt+c",
    "editMode": "super+alt+e",
    "cyclePlacement": "super+alt+p",
    "openMeowdbPage": "super+alt+o",
    "setup": "super+alt+s",
    "chooseCandidate": "super+alt+n",
    "search": "super+alt+f"
  }
}"#;

#[test]
fn win_alt_hotkeys_from_version_2_are_replaced_by_the_ctrl_shift_ones_and_the_rest_is_kept() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(WIN_ALT_SETTINGS).unwrap());

    assert_eq!(state.hotkeys, Hotkeys::default());
    assert_eq!(state.placement_mode, PlacementMode::DockedLeft);
    assert!(minerva.on_hotkey("super+alt+c").is_none());
    assert!(minerva.panel_state().expanded);
    minerva.on_hotkey("ctrl+shift+c");
    assert!(!minerva.panel_state().expanded);
    assert_eq!(minerva.settings().hotkeys, Hotkeys::default(), "and saved as Ctrl+Shift");
}

/// Settings saved as version 3 by a build of the EXP tracking branch, which kept the
/// Win+Alt hotkeys (and has fields this build doesn't know).
const VERSION_3_WIN_ALT_SETTINGS: &str = r#"{
  "version": 3,
  "expanded": true,
  "placementMode": "inside",
  "hotkeys": {
    "togglePanel": "super+alt+c",
    "editMode": "super+alt+e",
    "cyclePlacement": "super+alt+p",
    "openMeowdbPage": "super+alt+o",
    "setup": "super+alt+s",
    "chooseCandidate": "super+alt+n",
    "resetSession": "super+alt+x"
  },
  "expBarRegion": { "marked": { "x": 1016, "y": 1145, "width": 207, "height": 54 }, "client": { "width": 1920, "height": 1200 } }
}"#;

#[test]
fn win_alt_hotkeys_saved_as_version_3_are_replaced_by_the_ctrl_shift_ones_too() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(VERSION_3_WIN_ALT_SETTINGS).unwrap());

    assert_eq!(state.hotkeys, Hotkeys::default());
    assert!(state.hotkeys.all().iter().all(|hotkey| hotkey.starts_with("ctrl+shift+")));
    assert_eq!(minerva.settings().hotkeys, Hotkeys::default(), "and saved as Ctrl+Shift");
}

/// Settings saved by version 1, with its own Ctrl+Shift letters.
const VERSION_1_SETTINGS: &str = r#"{
  "version": 1,
  "expanded": true,
  "placement": "dockedLeft",
  "hotkeys": {
    "togglePanel": "ctrl+shift+m",
    "editMode": "ctrl+shift+e",
    "cyclePlacement": "ctrl+shift+d",
    "openMeowdbPage": "ctrl+shift+o",
    "setup": "ctrl+shift+s"
  }
}"#;

#[test]
fn version_1_hotkeys_are_replaced_by_the_defaults_and_the_rest_is_kept() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(VERSION_1_SETTINGS).unwrap());

    assert_eq!(state.hotkeys, Hotkeys::default());
    assert_eq!(state.placement_mode, PlacementMode::DockedLeft);
    assert!(minerva.on_hotkey("ctrl+shift+m").is_none());
    assert!(minerva.panel_state().expanded);
}

#[test]
fn ctrl_shift_c_collapses_the_panel_and_expands_it_again() {
    let mut minerva = Minerva::new(FakeSource::osms());

    minerva.on_hotkey("ctrl+shift+c");
    assert!(!minerva.panel_state().expanded);
    minerva.on_hotkey("ctrl+shift+c");
    assert!(minerva.panel_state().expanded);
}

#[test]
fn ctrl_shift_p_cycles_the_placement() {
    let mut minerva = Minerva::new(FakeSource::osms());

    minerva.on_hotkey("ctrl+shift+p");

    assert_eq!(minerva.panel_state().placement_mode, PlacementMode::DockedRight);
}

/// Settings saved before edit mode was retired, with its hotkey.
const WITH_EDIT_MODE: &str = r#"{
  "version": 4,
  "placementMode": "detached",
  "hotkeys": {
    "togglePanel": "ctrl+shift+x",
    "editMode": "ctrl+shift+e",
    "cyclePlacement": "ctrl+shift+p",
    "openMeowdbPage": "ctrl+shift+m",
    "setup": "ctrl+shift+s",
    "chooseCandidate": "ctrl+shift+n",
    "search": "ctrl+shift+f"
  }
}"#;

#[test]
fn settings_with_the_retired_edit_mode_hotkey_still_load_and_it_is_neither_registered_nor_saved() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(WITH_EDIT_MODE).unwrap()).clone();

    assert_eq!(state.placement_mode, PlacementMode::Detached);
    assert_eq!(state.hotkeys.toggle_panel, "ctrl+shift+x", "the rest is kept");
    assert!(!state.hotkeys.all().contains(&"ctrl+shift+e"));
    let saved = serde_json::to_string(&minerva.settings()).unwrap();
    assert!(!saved.contains("editMode"), "{saved}");
}

#[test]
fn ctrl_shift_e_does_nothing_any_more() {
    let mut minerva = Minerva::new(FakeSource::osms());
    let before = minerva.panel_state().clone();

    assert_eq!(minerva.on_hotkey("ctrl+shift+e"), None);

    assert_eq!(*minerva.panel_state(), before);
}

#[test]
fn ctrl_shift_m_opens_the_shown_maps_meowdb_page_and_nothing_without_a_map() {
    let mut minerva = Minerva::new(FakeSource::osms());
    assert_eq!(minerva.on_hotkey("ctrl+shift+m"), None);

    arrive(&mut minerva, SUNSET_SAMPLE, 0);

    assert_eq!(
        minerva.on_hotkey("ctrl+shift+m"),
        Some(Request::OpenInBrowser { url: "https://meowdb.com/msclassic/maps/010003070".into() })
    );
}

#[test]
fn ctrl_shift_s_opens_setup_again() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(common::set_up());
    assert!(!minerva.panel_state().setup.window_open);

    minerva.on_hotkey("ctrl+shift+s");

    let state = minerva.panel_state();
    assert_eq!(state.setup.needed, Some(SetupReason::Requested));
    assert!(state.setup.window_open);
}

#[test]
fn hotkeys_the_player_rebound_are_the_ones_that_act() {
    let mut minerva = Minerva::new(FakeSource::osms());
    let hotkeys = Hotkeys { toggle_panel: "ctrl+shift+x".into(), ..Hotkeys::default() };
    minerva.restore(Settings { hotkeys, ..Settings::default() });

    minerva.on_hotkey("ctrl+shift+c");
    assert!(minerva.panel_state().expanded);
    minerva.on_hotkey("Ctrl+Shift+X");
    assert!(!minerva.panel_state().expanded);
}

#[test]
fn a_hotkey_another_app_has_taken_is_shown_as_unavailable() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.hotkeys_unavailable(vec!["ctrl+shift+n".into()], vec![]);

    assert_eq!(state.unavailable_hotkeys, ["ctrl+shift+n"]);
    assert!(state.invalid_hotkeys.is_empty());
}

#[test]
fn a_hotkey_that_names_no_key_combination_is_shown_as_unavailable_and_invalid() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.hotkeys_unavailable(vec!["ctrl+shift+n".into()], vec!["ctrl+shift+banana".into()]);

    // Both do nothing; only the second is the player's own typo to fix.
    assert_eq!(state.unavailable_hotkeys, ["ctrl+shift+n", "ctrl+shift+banana"]);
    assert_eq!(state.invalid_hotkeys, ["ctrl+shift+banana"]);
}

#[test]
fn ctrl_shift_f_is_the_search_hotkey_even_with_a_settings_file_from_before_search() {
    let before_search = r#"{"version":4,"hotkeys":{"togglePanel":"ctrl+shift+x","chooseCandidate":"ctrl+shift+n"}}"#;
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(before_search).unwrap()).clone();

    assert_eq!(state.hotkeys.search, ctrl_shift("f"));
    assert_eq!(state.hotkeys.toggle_panel, ctrl_shift("x"), "the player's own bindings are kept");
    assert_eq!(minerva.settings().hotkeys.search, ctrl_shift("f"), "and saved with the rest");
}

#[test]
fn every_hotkey_the_panel_lists_is_one_to_register_the_search_one_included() {
    let minerva = Minerva::new(FakeSource::osms());

    let to_register = minerva.panel_state().hotkeys.all();

    assert_eq!(
        to_register,
        [
            "ctrl+shift+c",
            "ctrl+shift+p",
            "ctrl+shift+m",
            "ctrl+shift+s",
            "ctrl+shift+n",
            "ctrl+shift+g",
            "ctrl+shift+i",
            "ctrl+shift+u",
            "ctrl+shift+f",
            "ctrl+shift+d"
        ]
    );
}

#[test]
fn a_rebound_search_hotkey_another_app_has_taken_is_shown_as_unavailable() {
    let mut minerva = Minerva::new(FakeSource::osms());
    let hotkeys = Hotkeys { search: "ctrl+shift+q".into(), ..Hotkeys::default() };
    minerva.restore(Settings { hotkeys, ..Settings::default() });

    let state = minerva.hotkeys_unavailable(vec!["ctrl+shift+q".into()], vec![]);

    assert_eq!(state.hotkeys.search, "ctrl+shift+q");
    assert!(state.hotkeys.all().contains(&"ctrl+shift+q"));
    assert_eq!(state.unavailable_hotkeys, ["ctrl+shift+q"]);
}

#[test]
fn the_hotkeys_are_live_only_while_the_game_or_minerva_is_in_front() {
    let mut minerva = Minerva::new(FakeSource::osms());

    assert!(minerva.on_foreground(Foreground::Game).hotkeys_live);
    assert!(!minerva.on_foreground(Foreground::OtherApp).hotkeys_live, "other apps keep their Ctrl+Shift keys");
    assert!(minerva.on_foreground(Foreground::Minerva).hotkeys_live, "e.g. the overlay with the search view open, or setup");
}

#[test]
fn before_the_foreground_is_known_the_hotkeys_are_not_live() {
    let minerva = Minerva::new(FakeSource::osms());

    assert!(!minerva.panel_state().hotkeys_live);
}

#[test]
fn a_hotkey_reported_unavailable_stays_listed_while_the_hotkeys_are_not_live() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_foreground(Foreground::Game);
    minerva.hotkeys_unavailable(vec!["ctrl+shift+n".into()], vec![]);

    let state = minerva.on_foreground(Foreground::OtherApp);

    assert_eq!(state.unavailable_hotkeys, ["ctrl+shift+n"]);
}

/// A core with EXP tracking on (both regions kept, on a 1920x1200 game window) and
/// these hotkeys.
fn tracking_with(hotkeys: Hotkeys) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(Settings { hotkeys, ..common::set_up() });
    minerva.on_game_window(Some(common::window_at_origin(1920, 1200)));
    minerva
}

/// Start a Session: the EXP bar reads `160 [3.81%]`, then `266 [6.33%]`.
fn start_session(minerva: &mut Minerva) {
    for (at_ms, exp, percent_hundredths) in [(0, 160, 381), (10_000, 266, 633)] {
        minerva.on_exp_bar_reading(ExpBarReading { bar: Some(ExpBar { exp, percent_hundredths }), at_ms, place: None });
    }
    assert_eq!(minerva.panel_state().exp_tracking.state, TrackingState::WarmingUp);
}

#[test]
fn ctrl_shift_r_no_longer_resets_the_session() {
    let mut minerva = tracking_with(Hotkeys::default());
    start_session(&mut minerva);
    let before = minerva.panel_state().exp_tracking.clone();

    // Ctrl attacks in MapleStory: Shift and R pressed while attacking reset the Session
    // without asking. Only the Grind Tracker window's reset icon does now.
    assert_eq!(minerva.on_hotkey("ctrl+shift+r"), None);

    assert_eq!(minerva.panel_state().exp_tracking, before);
}

#[test]
fn the_reset_icon_on_the_grind_tracker_window_resets_the_session() {
    let mut minerva = tracking_with(Hotkeys::default());
    start_session(&mut minerva);

    assert_eq!(minerva.press(PanelControl::ResetSession), None);

    let tracking = &minerva.panel_state().exp_tracking;
    assert_eq!(tracking.state, TrackingState::Waiting);
    assert_eq!(tracking.rolling, None);
    assert!(tracking.reset_undoable);
}

/// Settings saved before the Grind Tracker hotkey existed, with a hotkey the player
/// rebound and the reset hotkey there was then, rebound too.
const SETTINGS_WITH_RESET_SESSION: &str = r#"{
  "version": 4,
  "hotkeys": {
    "togglePanel": "ctrl+shift+x",
    "cyclePlacement": "ctrl+shift+p",
    "openMeowdbPage": "ctrl+shift+m",
    "setup": "ctrl+shift+s",
    "chooseCandidate": "ctrl+shift+n",
    "search": "ctrl+shift+f",
    "resetSession": "ctrl+shift+z"
  }
}"#;

#[test]
fn settings_from_before_the_grind_tracker_hotkey_get_ctrl_shift_g_and_keep_the_rest() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = minerva.restore(serde_json::from_str(SETTINGS_WITH_RESET_SESSION).unwrap());

    assert_eq!(state.hotkeys.grind_tracker, ctrl_shift("g"));
    assert_eq!(state.hotkeys.toggle_panel, ctrl_shift("x"));
}

#[test]
fn a_saved_reset_hotkey_is_ignored_and_not_saved_again() {
    let saved: Settings = serde_json::from_str(SETTINGS_WITH_RESET_SESSION).unwrap();
    let mut minerva = tracking_with(saved.hotkeys);
    start_session(&mut minerva);

    minerva.on_hotkey("ctrl+shift+z");

    assert_eq!(minerva.panel_state().exp_tracking.state, TrackingState::WarmingUp);
    let json = serde_json::to_string(&minerva.settings()).unwrap();
    assert!(!json.contains("resetSession"), "{json}");
}
