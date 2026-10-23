//! Only one Minerva runs at a time. Starting it again while it runs shows the one
//! that is running: its panel, expanded, while there is a game window to show it on;
//! otherwise setup, so the player sees it is there.

mod common;

use common::FakeSource;
use minerva_core::{
    Action, Minerva, Foreground, GameWindow, PanelSizes, PlacementMode, ScreenRect, Settings, Size,
};

const GAME: ScreenRect = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };

/// A core set up before, so no
/// first-run setup is waiting, with or without the game window found.
fn set_up(game_found: bool) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms_offline());
    minerva.set_panel_sizes(PanelSizes {
        expanded: Size { width: 380, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    minerva.restore(Settings {
        setup_done: true,
        ..Settings::default()
    });
    let game = game_found.then(|| GameWindow { client: GAME, frame: GAME, screens: vec![GAME] });
    minerva.on_game_window(game);
    minerva
}

#[test]
fn launched_again_with_the_game_open_the_collapsed_panel_expands() {
    let mut minerva = set_up(true);
    assert!(!minerva.act(Action::ToggleExpanded).expanded);

    let state = minerva.act(Action::LaunchedAgain);

    assert!(state.expanded);
    assert!(!state.setup.window_open, "the panel is enough to show it runs");
}

#[test]
fn launched_again_from_another_app_with_the_game_open_the_panel_expands_for_the_game() {
    let mut minerva = set_up(true);
    minerva.act(Action::ToggleExpanded);
    // Started from Explorer or the Start menu, say: the overlay hides behind it.
    minerva.on_foreground(Foreground::OtherApp);

    let state = minerva.act(Action::LaunchedAgain);

    assert!(state.expanded);
    assert!(!state.setup.window_open, "it shows once the player is back in the game");
}

#[test]
fn launched_again_without_the_game_opens_setup() {
    let mut minerva = set_up(false);
    assert!(!minerva.panel_state().setup.window_open);

    let state = minerva.act(Action::LaunchedAgain);

    assert!(state.setup.window_open);
}

#[test]
fn launched_again_detached_without_the_game_opens_setup_too() {
    let mut minerva = set_up(false);
    minerva.restore(Settings {
        placement_mode: PlacementMode::Detached,
        setup_done: minerva.settings().setup_done,
        ..Settings::default()
    });

    let state = minerva.act(Action::LaunchedAgain);

    // A detached panel shows only with the game, like the others.
    assert!(state.setup.window_open);
}

#[test]
fn launched_again_during_first_run_setup_leaves_setup_as_it_is() {
    let mut minerva = Minerva::new(FakeSource::osms_offline());
    minerva.on_game_window(Some(GameWindow { client: GAME, frame: GAME, screens: vec![GAME] }));
    let before = minerva.panel_state().setup.clone();
    assert!(before.window_open);

    let state = minerva.act(Action::LaunchedAgain);

    assert_eq!(state.setup, before);
}
