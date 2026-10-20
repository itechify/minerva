//! First-run setup windows. The overlay never takes focus on a click and has no room
//! for setup, so setup has its own normal window, plus a "region" window laid over the
//! game's client area where the player marks the Title Region.
//! Both are declared hidden in `tauri.conf.json`: creating windows inside commands can
//! deadlock on Windows. Every decision is the core's; this only shows windows and
//! relays.
//!
//! Setup's window is an ordinary desktop window. The region window is always-on-top,
//! like the overlay: it is our own, shown only while the player marks a region, and
//! with it in front the overlay still shows (it is one of ours).

use crate::gamewin;
use crate::view::{LastView, ViewSender};
use crate::Command;
use minerva_core::{
    Action, Hotkeys, MapMark, MarkTarget, PanelControl, PanelState, Recognition, Rect, ScreenRect, SelectionDirection,
    SetupState,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow, WindowEvent};

pub const SETUP: &str = "setup";
pub const REGION: &str = "region";

/// What the setup commands share with the core loop.
pub struct Bridge {
    pub commands: Sender<Command>,
    /// Titles of game windows the player picked by hand.
    pub picks: Sender<String>,
    /// The game's client area as last found, where the region window goes.
    pub game: Mutex<Option<ScreenRect>>,
}

/// The last view sent to the setup window, for when it (or the region window) loads
/// after it was sent.
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
    hotkeys: &'a Hotkeys,
    /// Hotkeys that do nothing: another app has them, or they are invalid.
    unavailable_hotkeys: &'a [String],
    /// Those of them that don't name a key combination.
    invalid_hotkeys: &'a [String],
    /// Every Map Mark, which a recognised test read's map names its own by.
    map_marks: &'a BTreeMap<String, MapMark>,
    /// The Grind Tracker is on (the settings view's switch): setup says how to change it.
    grind_tracker_on: bool,
}

#[derive(Serialize)]
struct GameView {
    width: u32,
    height: u32,
}

/// What the region window renders: the selection, and what it is being marked around.
#[derive(Serialize)]
struct SelectionView {
    selection: Option<Rect>,
    target: Option<MarkTarget>,
}

/// Shows and hides the setup window as the core says.
pub struct SetupWindows {
    app: AppHandle,
    setup: WebviewWindow,
    shown: bool,
    region_client: Option<ScreenRect>,
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
                hide(&handle, REGION);
                let _ = handle.state::<Bridge>().commands.send(Command::Act(Action::CancelSetup));
            }
        });
        Ok(Self {
            app: app.clone(), setup, shown: false, region_client: None,
            view: ViewSender::new(SETUP, "setup-state"),
        })
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
        *self.app.state::<Bridge>().game.lock().unwrap() = game;
        match (self.shown, state.setup.window_open) {
            (false, true) => {
                let _ = self.setup.show();
                let _ = self.setup.unminimize();
                let _ = self.setup.set_focus();
            }
            (true, false) => {
                hide(&self.app, SETUP);
                hide(&self.app, REGION);
            }
            _ => {}
        }
        self.shown = state.setup.window_open;
        let mut client = game.filter(|_| state.setup.selection.is_some() && self.shown);
        if let Some(region) = self.app.get_webview_window(REGION) {
            if let Some(area) = client {
                let shown = (|| -> tauri::Result<()> {
                    if self.region_client != Some(area) {
                        region.set_position(PhysicalPosition::new(area.x, area.y))?;
                        region.set_size(PhysicalSize::new(area.width, area.height))?;
                    }
                    if self.region_client.is_none() {
                        region.show()?;
                        region.set_focus()?;
                    }
                    Ok(())
                })();
                if let Err(error) = shown {
                    let _ = region.hide();
                    focus_setup(&self.app);
                    let _ = self.app.state::<Bridge>().commands.send(Command::Act(Action::CancelSelection));
                    let _ = self.app.emit_to(SETUP, "marking-error", error.to_string());
                    client = None;
                }
            } else if self.region_client.is_some() {
                let _ = region.hide();
                if self.shown { focus_setup(&self.app); }
            }
        }
        self.region_client = client;
        let view = SetupView {
            setup: &state.setup,
            recognition: state.recognition,
            game: game.map(|c| GameView { width: c.width, height: c.height }),
            game_title: &state.game_window_title,
            hotkeys: &state.hotkeys,
            unavailable_hotkeys: &state.unavailable_hotkeys,
            invalid_hotkeys: &state.invalid_hotkeys,
            map_marks: &state.map_marks,
            grind_tracker_on: state.grind_tracker.on,
        };
        if self.view.send(&self.app, &view, &self.app.state::<SetupPageView>().0) {
            let selection = SelectionView { selection: state.setup.selection, target: state.setup.selection_target };
            let _ = self.app.emit_to(REGION, "selection-state", &selection);
        }
    }
}

fn hide(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.hide();
    }
}

fn focus_setup(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETUP) {
        let _ = window.set_focus();
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

/// Begin marking the Title Region; sync lays the window over the game client area. Its
/// CSS px times the device pixel ratio are then client px.
#[tauri::command]
pub async fn start_marking(bridge: State<'_, Bridge>, target: MarkTarget) -> Result<(), String> {
    (*bridge.game.lock().unwrap()).ok_or("The game window isn't showing. Bring it back, then try again.")?;
    bridge.commands.send(Command::Act(Action::BeginSelection(target))).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn adjust_selection(
    bridge: State<'_, Bridge>, direction: SelectionDirection, resize: bool, coarse: bool,
) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::AdjustSelection { direction, resize, coarse }))
        .map_err(|e| e.to_string())
}

/// Enter on the region window: test-read the selection, around `target`.
#[tauri::command]
pub async fn confirm_selection(app: AppHandle, bridge: State<'_, Bridge>, target: MarkTarget) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::ConfirmSelection)).map_err(|e| e.to_string())?;
    app.emit_to(SETUP, marked_event(target), ()).map_err(|e| e.to_string())
}

/// The player dragged the Title Region (physical px, relative to the client area).
#[tauri::command]
pub async fn mark_region(
    app: AppHandle,
    bridge: State<'_, Bridge>,
    target: MarkTarget,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    hide(&app, REGION);
    focus_setup(&app);
    let marked = Rect { x, y, width, height };
    let action = match target {
        MarkTarget::Title => Action::MarkTitleRegion(marked),
    };
    bridge.commands.send(Command::Act(action)).map_err(|e| e.to_string())?;
    // The setup window moves on to the test read.
    app.emit_to(SETUP, marked_event(target), ()).map_err(|e| e.to_string())
}

/// What tells the setup window a region was marked, and so its test read is on.
fn marked_event(target: MarkTarget) -> &'static str {
    match target {
        MarkTarget::Title => "title-marked",
    }
}

/// The player pressed Esc on the region window.
#[tauri::command]
pub async fn cancel_marking(app: AppHandle, bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::CancelSelection)).map_err(|e| e.to_string())?;
    hide(&app, REGION);
    focus_setup(&app);
    app.emit_to(SETUP, "marking-cancelled", ()).map_err(|e| e.to_string())
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

/// Finish setup (the core only does once the test reads of what was marked succeeded).
#[tauri::command]
pub async fn finish_setup(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::FinishSetup)).map_err(|e| e.to_string())
}
