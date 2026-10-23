//! The setup window. The overlay never takes focus on a click and has no room for setup, so
//! setup has its own normal window, declared hidden in `tauri.conf.json`: creating windows
//! inside commands can deadlock on Windows. Every decision is the core's; this only shows
//! the window and relays. Nothing is marked in it: the Minimap Title is found wherever it
//! is (ADR 0021).

use crate::gamewin;
use crate::view::ViewSender;
use crate::Command;
use minerva_core::{Action, Hotkeys, MapMark, PanelState, Recognition, ScreenRect, SetupState, Updates};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Manager, State, WebviewWindow, WindowEvent};

pub const SETUP: &str = "setup";

/// What the setup window renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupView<'a> {
    setup: &'a SetupState,
    recognition: Recognition,
    /// The game client area's size, when the game window was found.
    game: Option<GameView>,
    /// The window title looked for.
    game_title: &'a str,
    /// No Minimap Title has been found for a while: the minimap may be closed.
    title_lost: bool,
    hotkeys: &'a Hotkeys,
    /// Hotkeys that do nothing: another app has them, or they are invalid.
    unavailable_hotkeys: &'a [String],
    /// Those of them that don't name a key combination.
    invalid_hotkeys: &'a [String],
    /// Every Map Mark, which the map found names its own by.
    map_marks: &'a BTreeMap<String, MapMark>,
    /// The Grind Tracker is on (the settings view's switch): setup says how to change it.
    grind_tracker_on: bool,
    /// Whether a newer version is out, for About & Credits.
    updates: &'a Updates,
}

#[derive(Serialize)]
struct GameView {
    width: u32,
    height: u32,
}

/// Shows and hides the setup window as the core says.
pub struct SetupWindows {
    app: AppHandle,
    setup: WebviewWindow,
    shown: bool,
    /// Sends the setup window its view.
    view: ViewSender,
}

impl SetupWindows {
    /// Closing setup tells the core, which decides what that means (on the first run,
    /// quitting).
    pub fn new(app: &AppHandle) -> Self {
        let setup = app.get_webview_window(SETUP).expect("setup window in tauri.conf.json");
        let handle = app.clone();
        setup.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide(&handle, SETUP);
                let _ = handle.state::<Sender<Command>>().send(Command::Act(Action::CancelSetup));
            }
        });
        Self { app: app.clone(), setup, shown: false, view: ViewSender::new(SETUP, "setup-state") }
    }

    /// The player started Minerva again: bring an open setup window in front, where
    /// they can see it. Right after their launch, so Windows lets us take the foreground.
    pub fn bring_to_front(&self) {
        if self.shown {
            let _ = self.setup.unminimize();
            let _ = self.setup.set_focus();
        }
    }

    /// Follow the core's setup state, and send the setup window what it renders.
    pub fn sync(&mut self, state: &PanelState, game: Option<ScreenRect>) {
        match (self.shown, state.setup.window_open) {
            (false, true) => {
                let _ = self.setup.show();
                let _ = self.setup.unminimize();
                let _ = self.setup.set_focus();
            }
            (true, false) => hide(&self.app, SETUP),
            _ => {}
        }
        self.shown = state.setup.window_open;
        let view = SetupView {
            setup: &state.setup,
            recognition: state.recognition,
            game: game.map(|c| GameView { width: c.width, height: c.height }),
            game_title: &state.game_window_title,
            title_lost: state.title_lost,
            hotkeys: &state.hotkeys,
            unavailable_hotkeys: &state.unavailable_hotkeys,
            invalid_hotkeys: &state.invalid_hotkeys,
            map_marks: &state.map_marks,
            grind_tracker_on: state.grind_tracker.on,
            updates: &state.updates,
        };
        self.view.send(&self.app, &view);
    }
}

fn hide(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.hide();
    }
}

/// Take the window under the mouse pointer as the game window.
#[tauri::command]
pub async fn pick_game_window(commands: State<'_, Sender<Command>>) -> Result<String, String> {
    let title = gamewin::window_under_cursor()?;
    commands.send(Command::PickGameWindow(title.clone())).map_err(|e| e.to_string())?;
    Ok(title)
}
