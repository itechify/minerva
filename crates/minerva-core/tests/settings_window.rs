//! The Settings window: a window of its own over the game, which the gear at the top of the
//! panel opens and closes, as its own × does. It opens beside the panel, clear of the
//! captured regions and our other windows, until the player drags it somewhere; then it keeps
//! that spot relative to the game window, across restarts too, until they drag it back. It
//! has the keyboard as it opens, and again when the player clicks a Change or the Deal Margin
//! field; clicking back into the game gives the keyboard back, and the window stays open.
//!
//! Worked example used throughout, as in `grind_tracker_window.rs`: the game's client area
//! at (812,110), 1920x1200 physical px, on one 3440x1440 monitor; an expanded panel of 340x72
//! (so at (2376,510)) and a Settings window of 420x500.

mod common;

use common::{FakeSource, GAME};
use minerva_core::{
    Action, ClientOffset, Foreground, GameWindow, Hotkey, KeyPress, Minerva, PanelControl, PanelSizes, PanelState, ScreenRect, Settings,
    SettingsNotice, Size,
};

fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect { x, y, width, height }
}

/// The game window with this client area, framed as measured, on `SCREEN`.
fn game(client: ScreenRect) -> Option<GameWindow> {
    Some(common::window_on_wide_screen(client))
}

const WINDOW: Size = Size { width: 420, height: 500 };

/// Set up, with the Grind Tracker off so its window isn't beside the panel.
fn no_tracker() -> Settings {
    Settings { grind_tracker_on: false, ..common::set_up() }
}

/// A core restored from `settings`, with the game window found, its Minimap Title found, the
/// game in front and the windows' sizes known.
fn started(settings: Settings) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms_offline());
    minerva.restore(settings);
    minerva.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    minerva.set_grind_tracker_size(Size { width: 300, height: 150 });
    minerva.set_settings_size(WINDOW);
    minerva.on_game_window(game(GAME));
    minerva.on_foreground(Foreground::Game);
    common::found_title(&mut minerva, common::title_on(GAME), 0);
    minerva
}

/// As [`started`], with the Settings window opened by the gear.
fn opened(settings: Settings) -> Minerva {
    let mut minerva = started(settings);
    minerva.press(PanelControl::Settings);
    minerva
}

fn placed(state: &PanelState) -> Option<ScreenRect> {
    state.settings_window.as_ref().and_then(|window| window.placement)
}

/// What the shell does on restart: save the settings as JSON, load them into a new core.
fn restart(minerva: &Minerva) -> Minerva {
    started(common::settings_after_restart(minerva))
}

/// Pressing a control, then the panel state after it.
fn pressed(minerva: &mut Minerva, control: PanelControl) -> PanelState {
    minerva.press(control);
    minerva.panel_state().clone()
}

fn has_keyboard(state: &PanelState) -> bool {
    state.settings_window.as_ref().is_some_and(|window| window.takes_keyboard)
}

#[test]
fn the_gear_opens_it_beside_the_panel_with_the_keyboard_and_closes_it_again() {
    let mut minerva = started(no_tracker());
    assert_eq!(minerva.panel_state().settings_window, None);

    assert_eq!(minerva.press(PanelControl::Settings), None);
    let state = minerva.panel_state();
    // The panel is in the game's right half: the window goes towards its middle, 8 px left
    // of the panel (2376), its top level with the panel's (510).
    assert_eq!(placed(state), Some(rect(1948, 510, 420, 500)));
    assert!(has_keyboard(state));
    assert!(!state.overlay_takes_keyboard, "the panel leaves the keyboard alone");

    minerva.press(PanelControl::Settings);
    assert_eq!(minerva.panel_state().settings_window, None);
}

#[test]
fn clicking_back_into_the_game_gives_the_keyboard_back_and_the_window_stays_open() {
    let mut minerva = opened(no_tracker());
    minerva.act(Action::BeginRebind(Hotkey::Search));

    let state = minerva.act(Action::LeaveSettings);

    let window = state.settings_window.as_ref().expect("still open");
    assert!(!window.takes_keyboard);
    assert_eq!(window.rebinding, None, "the keys pressed for it would reach the game");
    assert!(state.hotkeys_live);
    assert_eq!(placed(state), Some(rect(1948, 510, 420, 500)));
}

#[test]
fn a_change_or_the_deal_margin_field_takes_the_keyboard_again() {
    let mut minerva = opened(no_tracker());
    minerva.act(Action::LeaveSettings);

    assert!(has_keyboard(minerva.act(Action::BeginRebind(Hotkey::Search))));

    minerva.act(Action::LeaveSettings);
    assert!(has_keyboard(minerva.act(Action::FocusSettings)));
}

#[test]
fn opening_it_again_from_setup_keeps_what_it_shows_and_takes_the_keyboard() {
    let mut minerva = opened(no_tracker());
    minerva.act(Action::BeginRebind(Hotkey::Search));
    minerva.rebind_key(KeyPress { code: "KeyQ".into(), ctrl: true, alt: false, shift: false, win: false });
    minerva.act(Action::LeaveSettings);

    let state = minerva.act(Action::OpenSettings);

    let window = state.settings_window.as_ref().expect("still open");
    assert!(window.takes_keyboard);
    assert_eq!(window.notice, Some(SettingsNotice::Saved { hotkey: Hotkey::Search, swapped_with: None }));
}

#[test]
fn the_search_view_and_the_settings_window_take_the_keyboard_from_each_other() {
    let mut minerva = opened(no_tracker());
    minerva.act(Action::BeginRebind(Hotkey::Search));

    let state = pressed(&mut minerva, PanelControl::Search);
    assert!(state.search.view.is_some());
    assert!(state.overlay_takes_keyboard);
    let window = state.settings_window.as_ref().expect("still open");
    assert!(!window.takes_keyboard);
    assert_eq!(window.rebinding, None);

    let state = minerva.act(Action::BeginRebind(Hotkey::Search));
    assert!(has_keyboard(state));
    assert_eq!(state.search.view, None, "the search view closes as it loses the keyboard");
    assert!(!state.overlay_takes_keyboard);
}

#[test]
fn the_gear_leaves_the_panel_as_it_is_and_collapsing_the_panel_leaves_the_window_open() {
    let mut minerva = started(no_tracker());
    minerva.press(PanelControl::TogglePanel);

    let state = pressed(&mut minerva, PanelControl::Settings);
    assert!(!state.expanded);
    // Beside the collapsed icon (2672,510), towards the middle of the game.
    assert_eq!(placed(&state), Some(rect(2244, 510, 420, 500)));

    minerva.press(PanelControl::TogglePanel);
    let state = pressed(&mut minerva, PanelControl::TogglePanel);
    assert!(state.settings_window.is_some());
}

#[test]
fn beside_the_panel_it_keeps_clear_of_the_grind_tracker_there() {
    let minerva = opened(common::set_up());
    let state = minerva.panel_state();

    // The Grind Tracker is left of the panel (2068..2368, 510..660), where the window would
    // go, and right of the panel leaves the game: it goes below the Grind Tracker.
    assert_eq!(state.grind_tracker.placement, Some(rect(2068, 510, 300, 150)));
    assert_eq!(placed(state), Some(rect(1948, 668, 420, 500)));
}

#[test]
fn dragged_it_keeps_its_spot_relative_to_the_game_window_after_a_restart_too() {
    let mut minerva = opened(no_tracker());

    let state = minerva.act(Action::SettingsMoved { x: 1000, y: 300 });
    assert_eq!(placed(state), Some(rect(1000, 300, 420, 500)));

    // The game window moving takes it along.
    let moved = rect(GAME.x + 100, GAME.y + 50, GAME.width, GAME.height);
    assert_eq!(placed(minerva.on_game_window(game(moved))), Some(rect(1100, 350, 420, 500)));

    assert_eq!(minerva.settings().settings_at, Some(ClientOffset { x: 188, y: 190 }));
    let mut restarted = restart(&minerva);
    restarted.press(PanelControl::Settings);
    assert_eq!(placed(restarted.panel_state()), Some(rect(1000, 300, 420, 500)));
}

#[test]
fn dragged_back_beside_the_panel_it_follows_the_panel_again() {
    let mut minerva = opened(no_tracker());
    minerva.act(Action::SettingsMoved { x: 1000, y: 300 });

    // Let go within an eighth of its width (52 px) of its spot beside the panel (1948,510).
    let state = minerva.act(Action::SettingsMoved { x: 1948 + 40, y: 510 - 30 });
    assert_eq!(placed(state), Some(rect(1948, 510, 420, 500)));
    assert_eq!(minerva.settings().settings_at, None);
}

#[test]
fn dragged_onto_a_captured_region_it_moves_off_it() {
    let mut minerva = opened(no_tracker());

    // Over the Minimap Title found at (882..1502, 150..198): our windows show in the
    // capture, so it moves the shortest way off it, here below.
    let state = minerva.act(Action::SettingsMoved { x: 1000, y: 160 });

    assert_eq!(placed(state), Some(rect(1000, 198, 420, 500)));
}

#[test]
fn it_hides_without_the_game_and_while_another_app_is_in_front_but_shows_with_setup() {
    let mut minerva = opened(no_tracker());
    let shown = placed(minerva.panel_state());

    let state = minerva.on_game_window(None);
    assert_eq!(placed(state), None);
    assert!(state.settings_window.is_some(), "hidden, not closed");
    assert!(!has_keyboard(state));
    let state = minerva.on_game_window(game(GAME));
    assert_eq!(placed(state), shown);
    assert!(!has_keyboard(state), "it never takes the keyboard from the game by coming back");

    minerva.press(PanelControl::Setup);
    assert_eq!(placed(minerva.on_foreground(Foreground::Minerva)), shown, "setup's Change hotkeys opens it");

    assert_eq!(placed(minerva.on_foreground(Foreground::OtherApp)), None);
    assert_eq!(placed(minerva.on_foreground(Foreground::Game)), shown);
}

#[test]
fn each_launch_starts_with_it_closed() {
    let minerva = opened(no_tracker());

    let restarted = restart(&minerva);

    assert_eq!(restarted.panel_state().settings_window, None);
}
