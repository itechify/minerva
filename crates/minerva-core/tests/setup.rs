//! First-run setup, and setup again when the player asks. Nothing is marked: the Minimap
//! Title is found wherever it is (ADR 0021), so setup shows the map it found, and is never
//! held up waiting for one.
//!
//! Worked example used throughout: the Kerning City window sample, whose client area
//! (1916x1197 physical px) sits at (812,110) on screen.

mod common;

use common::FakeSource;
use minerva_core::{Action, Foreground, GameWindow, Minerva, PanelControl, Request, ScreenPoint, ScreenRect, SetupReason, TitleCapture};

const KERNING: &str = "victoria-road__kerning-city--window";

/// The client area of a window sample shown at (812,110).
fn game(sample: &str) -> ScreenRect {
    let client = common::client_of(&common::load(sample));
    ScreenRect { x: 812, y: 110, width: client.width(), height: client.height() }
}

/// The game window as the shell reports it, with this client area on a 3440x1440 screen.
fn window(client: ScreenRect) -> Option<GameWindow> {
    let frame = ScreenRect { x: client.x - 1, y: client.y - 31, width: client.width + 2, height: client.height + 32 };
    Some(GameWindow { client, frame, screens: vec![ScreenRect { x: 0, y: 0, width: 3440, height: 1440 }] })
}

/// The capture the core asks for, taken of `sample`'s client area shown at `game(sample)`.
fn capture(minerva: &Minerva, sample: &str, at_ms: u64) -> TitleCapture {
    let client = common::client_of(&common::load(sample));
    let region = minerva.panel_state().title_capture.expect("a title capture with the game in front");
    let frame = common::capture(&client, game(sample), region, at_ms);
    TitleCapture { frame, origin: ScreenPoint { x: region.x, y: region.y }, ours: minerva.panel_state().our_windows.clone() }
}

/// A fresh install with the game in front, as the player first sees setup.
fn first_run() -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));
    minerva.on_foreground(Foreground::Game);
    minerva
}

#[test]
fn a_fresh_install_asks_for_setup_and_looks_for_the_title_all_the_same() {
    let minerva = first_run();
    let state = minerva.panel_state();

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
    assert_eq!(state.title_capture, Some(game(KERNING)), "the whole client area");
    assert!(state.title_searching);
}

#[test]
fn setup_shows_the_map_the_minimap_title_shows_once_it_is_found() {
    let mut minerva = first_run();
    assert_eq!(minerva.panel_state().setup.map, None);
    assert_eq!(minerva.panel_state().setup.credit, None);

    let first = capture(&minerva, KERNING, 0);
    minerva.on_title_capture(first);
    let second = capture(&minerva, KERNING, 333);
    let state = minerva.on_title_capture(second);

    let shown = state.setup.map.as_ref().map(|m| (m.street_name.as_str(), m.map_name.as_str()));
    assert_eq!(shown, Some(("Victoria Road", "Kerning City")));
    assert_eq!(state.setup.credit, Some("Map names from OSMS Data Explorer"));
}

#[test]
fn setup_can_be_finished_before_any_title_is_found() {
    let mut minerva = first_run();

    let state = minerva.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, None);
    assert!(!state.setup.window_open);
    assert!(state.title_searching, "the title is still looked for");
}

/// What the shell does on restart: save the settings as JSON, load them into a new core.
fn restart(minerva: &Minerva) -> Minerva {
    let json = serde_json::to_string(&minerva.settings()).unwrap();
    let mut restarted = Minerva::new(FakeSource::osms());
    restarted.restore(serde_json::from_str(&json).unwrap());
    restarted
}

/// A core whose setup was finished.
fn set_up() -> Minerva {
    let mut minerva = first_run();
    minerva.act(Action::FinishSetup);
    minerva
}

#[test]
fn finished_setup_survives_a_restart() {
    let mut restarted = restart(&set_up());

    let state = restarted.on_game_window(window(game(KERNING)));

    assert_eq!(state.setup.needed, None);
    assert!(!state.setup.window_open);
}

/// A settings file from before ADR 0021 has the Title Region the player marked at setup: setup
/// was finished, and the region is dropped, the title found wherever it is.
#[test]
fn a_settings_file_with_a_marked_title_region_has_finished_setup_and_looks_for_the_title() {
    let json = r#"{"version":7,"titleRegion":{"marked":{"x":70,"y":40,"width":130,"height":48},"client":{"width":1916,"height":1197}}}"#;
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(serde_json::from_str(json).unwrap());
    minerva.on_game_window(window(game(KERNING)));
    let state = minerva.on_foreground(Foreground::Game);

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.title_capture, Some(game(KERNING)));
    let saved = serde_json::to_value(minerva.settings()).unwrap();
    assert_eq!(saved.get("titleRegion"), None);
    assert_eq!(saved.get("setupDone"), Some(&serde_json::Value::Bool(true)));
}

#[test]
fn a_settings_file_from_before_without_a_title_region_has_not_finished_setup() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(serde_json::from_str(r#"{"version":7,"titleRegion":null}"#).unwrap());

    let state = minerva.on_game_window(window(game(KERNING)));

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
}

#[test]
fn a_new_game_resolution_doesnt_ask_for_setup_again() {
    let mut minerva = set_up();

    let resized = ScreenRect { width: 1280, height: 720, ..game(KERNING) };
    let state = minerva.on_game_window(window(resized));

    assert_eq!(state.setup.needed, None);
    assert!(!state.setup.window_open);
    assert_eq!(state.title_capture, Some(resized));
}

#[test]
fn asking_to_set_up_again_opens_setup_and_closing_it_just_closes_it() {
    let mut minerva = set_up();
    let state = minerva.act(Action::RequestSetup);
    assert!(state.setup.window_open);
    assert_eq!(state.setup.needed, Some(SetupReason::Requested));

    let state = minerva.act(Action::CancelSetup);

    assert!(!state.setup.window_open);
    assert_eq!(state.setup.needed, None);
    assert!(!state.quit);
}

#[test]
fn cancelling_first_run_setup_leaves_it_still_needed() {
    let mut minerva = first_run();

    let state = minerva.act(Action::CancelSetup);

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
}

#[test]
fn a_game_window_picked_by_hand_is_looked_for_from_then_on_even_after_a_restart() {
    let mut minerva = Minerva::new(FakeSource::osms());
    assert_eq!(minerva.panel_state().game_window_title, "MapleStory");

    let state = minerva.pick_game_window("MapleStory Classic".into());
    assert_eq!(state.game_window_title, "MapleStory Classic");

    assert_eq!(restart(&minerva).panel_state().game_window_title, "MapleStory Classic");
}

#[test]
fn an_empty_settings_file_has_a_re_setup_hotkey_and_needs_setup() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(serde_json::from_str("{}").unwrap());

    let state = minerva.on_game_window(window(game(KERNING)));

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
    assert_eq!(state.hotkeys.setup, "ctrl+shift+s");
}

// The setup window: the core says when it shows, and what closing it does.

#[test]
fn the_setup_window_opens_on_first_run_and_closing_it_then_quits() {
    let mut minerva = Minerva::new(FakeSource::osms());
    let state = minerva.on_game_window(window(game(KERNING)));
    assert!(state.setup.window_open);
    assert!(!state.quit);

    // The player hasn't been through its warning.
    let state = minerva.act(Action::CancelSetup);
    assert!(state.quit);
}

#[test]
fn finishing_setup_closes_its_window() {
    let minerva = set_up();

    assert!(!minerva.panel_state().setup.window_open);
}

#[test]
fn about_and_credits_opens_the_minerva_site_in_the_browser_and_changes_nothing_else() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));
    let before = minerva.panel_state().clone();

    assert_eq!(
        minerva.press(PanelControl::OpenSite),
        Some(Request::OpenInBrowser { url: "https://owlofminerva.app/".into() })
    );
    assert_eq!(minerva.panel_state(), &before);
}

#[test]
fn about_and_credits_opens_minervas_github_and_discord_in_the_browser_and_changes_nothing_else() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));
    let before = minerva.panel_state().clone();

    assert_eq!(
        minerva.press(PanelControl::OpenGithub),
        Some(Request::OpenInBrowser { url: "https://github.com/itechify/minerva".into() })
    );
    assert_eq!(
        minerva.press(PanelControl::OpenDiscord),
        Some(Request::OpenInBrowser { url: "https://discord.gg/NnbnVWkRrF".into() })
    );
    assert_eq!(minerva.panel_state(), &before);
}

