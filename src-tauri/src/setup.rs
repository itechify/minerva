//! The setup window. The overlay never takes focus on a click and has no room for setup, so
//! setup has its own normal window, declared hidden in `tauri.conf.json`: creating windows
//! inside commands can deadlock on Windows. Every decision is the core's; this only shows
//! the window and relays. Nothing is marked in it: the Minimap Title is found wherever it
//! is (ADR 0021).

use crate::gamewin;
use crate::view::{LastView, ViewSender};
use crate::Command;
use minerva_core::{Action, Hotkeys, MapMark, PanelControl, PanelState, Recognition, ScreenRect, SetupState, Updates};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Manager, State, WebviewWindow, WindowEvent};

pub const SETUP: &str = "setup";

/// What the setup commands share with the core loop.
pub struct Bridge {
    pub commands: Sender<Command>,
    /// Titles of game windows the player picked by hand.
    pub picks: Sender<String>,
}

/// The last view sent to the setup window, for when it loads after it was sent.
#[derive(Default)]
pub struct SetupPageView(pub LastView);

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
    pub fn new(app: &AppHandle) -> tauri::Result<Self> {
        let setup = app.get_webview_window(SETUP).expect("setup window in tauri.conf.json");
        let handle = app.clone();
        setup.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide(&handle, SETUP);
                let _ = handle.state::<Bridge>().commands.send(Command::Act(Action::CancelSetup));
            }
        });
        Ok(Self { app: app.clone(), setup, shown: false, view: ViewSender::new(SETUP, "setup-state") })
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
        self.view.send(&self.app, &view, &self.app.state::<SetupPageView>().0);
    }
}

fn hide(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.hide();
    }
}

/// What the setup window renders, as last sent (`None` before the first).
#[tauri::command]
pub async fn setup_view(view: State<'_, SetupPageView>) -> Result<Option<serde_json::Value>, String> {
    Ok(view.0.get())
}

/// Take the window under the mouse pointer as the game window.
#[tauri::command]
pub async fn pick_game_window(bridge: State<'_, Bridge>) -> Result<String, String> {
    let title = gamewin::window_under_cursor()?;
    bridge.picks.send(title.clone()).map_err(|e| e.to_string())?;
    Ok(title)
}

/// The Overlay Guide's Change hotkeys: open the overlay's settings view, as its gear does.
/// It comes in front with the keyboard, which our setup window, in front now, may give it.
#[tauri::command]
pub async fn open_hotkey_settings(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Press(PanelControl::Settings)).map_err(|e| e.to_string())
}

/// About & Credits' link: open the Minerva site in the browser, as the panel opens
/// MeowDB's pages.
#[tauri::command]
pub async fn open_site(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Press(PanelControl::OpenSite)).map_err(|e| e.to_string())
}

/// About & Credits' GitHub link: open Minerva's source in the browser.
#[tauri::command]
pub async fn open_github(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Press(PanelControl::OpenGithub)).map_err(|e| e.to_string())
}

/// About & Credits' Discord link: open Minerva's Discord in the browser.
#[tauri::command]
pub async fn open_discord(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Press(PanelControl::OpenDiscord)).map_err(|e| e.to_string())
}

/// About & Credits' Check for updates: check for a newer version now, the setting on or off.
#[tauri::command]
pub async fn check_for_updates(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::CheckForUpdates)).map_err(|e| e.to_string())
}

/// About & Credits' "What's new": open the newer version's release page in the browser.
#[tauri::command]
pub async fn open_release(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Press(PanelControl::OpenRelease)).map_err(|e| e.to_string())
}

/// About & Credits' Update (or Try again): choose to update. The core asks first while a
/// Session is running.
#[tauri::command]
pub async fn update(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Press(PanelControl::Update)).map_err(|e| e.to_string())
}

/// About & Credits' Update now, Update when I quit or Later, when it asked before updating.
#[tauri::command]
pub async fn update_choice(choice: UpdateChoice, bridge: State<'_, Bridge>) -> Result<(), String> {
    let action = match choice {
        UpdateChoice::Now => Action::UpdateNow,
        UpdateChoice::WhenIQuit => Action::UpdateWhenIQuit,
        UpdateChoice::Later => Action::UpdateLater,
    };
    bridge.commands.send(Command::Act(action)).map_err(|e| e.to_string())
}

/// What the player chose when About & Credits asked before updating.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateChoice {
    Now,
    WhenIQuit,
    Later,
}

/// Finish setup: nothing has to be found first (ADR 0021).
#[tauri::command]
pub async fn finish_setup(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::FinishSetup)).map_err(|e| e.to_string())
}
