//! The Settings window, beside the panel: the hotkeys, the Deal Margin, the Grind Tracker's
//! switch, Game Data and Updates. Clickable while it shows, and dragged by its bar. One of
//! our windows over the game ([`crate::panel_window`]); this says what its page renders,
//! and relays the key presses and the keyboard leaving it.
//!
//! It takes the keyboard while the core says (it opened, or the player clicked a Change or
//! the Deal Margin field), and the core releases the hotkeys while the player rebinds one, so
//! the combination they press reaches the window. Every decision is the core's; the clicks
//! are [`minerva_core::Action`]s, through `act`.

use crate::Command;
use crate::bests_window::PERSONAL_BESTS;
use crate::gamewin;
use crate::grind_tracker::GRIND_TRACKER;
use crate::panel_window::{Heights, PanelWindow, Shows, WindowSpec};
use minerva_core::{Action, DealLineChoice, GameData, Hotkeys, KeyPress, Minerva, PanelState, SettingsWindow};
use serde::Serialize;
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Manager, State, WindowEvent};

pub const SETTINGS: &str = "settings";

/// The width its page lays out at, in CSS px (logical); the core narrows the window to a
/// narrower game, and `src/settings.css` lays out to whatever it gets. Its height follows
/// its content, as the page reports it: past the most, the settings scroll. Over the panel,
/// the Grind Tracker and the Personal Bests window where it covers them: the player opened
/// it to change something.
pub const SPEC: WindowSpec = WindowSpec {
    label: SETTINGS,
    event: "settings-state",
    watcher: "on-top-settings",
    width: 420.0,
    height: Heights { default: 500.0, min: 80.0, max: 600.0 },
    sized: Minerva::set_settings_size,
    moved: Some(|x, y| Action::SettingsMoved { x, y }),
    beneath: &["overlay", GRIND_TRACKER, PERSONAL_BESTS],
};

/// What the Settings page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPage<'a> {
    /// The window while it is open: otherwise the page draws nothing, as it does while it
    /// is hidden (no placement).
    window: Option<&'a SettingsWindow>,
    hotkeys: &'a Hotkeys,
    unavailable_hotkeys: &'a [String],
    invalid_hotkeys: &'a [String],
    /// The Deal Margin, in percent.
    margin_percent: u32,
    /// The Grind Tracker's switch.
    grind_tracker_on: bool,
    /// "Check for updates".
    checks_on: bool,
    /// "Prices from maplestory.market".
    market_prices: bool,
    /// What the Deal Line may be, the chosen one marked.
    deal_lines: &'a [DealLineChoice],
    game_data: &'a GameData,
}

/// Show the window where the core says, or nothing, with the keyboard while the core says;
/// and send its page the settings.
pub fn sync(window: &mut PanelWindow, state: &PanelState) {
    let settings = state.settings_window.as_ref();
    let placement = settings.and_then(|settings| settings.placement);
    let view = SettingsPage {
        window: settings,
        hotkeys: &state.hotkeys,
        unavailable_hotkeys: &state.unavailable_hotkeys,
        invalid_hotkeys: &state.invalid_hotkeys,
        margin_percent: state.deal_finder.margin_percent,
        grind_tracker_on: state.grind_tracker.on,
        checks_on: state.updates.checks_on,
        market_prices: state.deal_finder.market_prices,
        deal_lines: &state.deal_finder.choices,
        game_data: &state.game_data,
    };
    let takes_keyboard = settings.is_some_and(|settings| settings.takes_keyboard);
    window.sync(&view, Shows { placement, clickable: placement.is_some(), takes_keyboard });
}

/// The keyboard leaving the Settings window for another app's window (the player clicked
/// back into the game) tells the core: it stops waiting for a hotkey's keys, which would no
/// longer reach it, and the window stays open. Not when focus only moved within our windows,
/// as into the overlay's search view or the window's own webview.
pub fn watch_focus(app: &AppHandle, commands: Sender<Command>) {
    let Some(window) = app.get_webview_window(SETTINGS) else { return };
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event
            && !gamewin::foreground_is_ours()
        {
            let _ = commands.send(Command::Act(Action::LeaveSettings));
        }
    });
}

/// A key the player pressed on the Settings window.
/// The player chose this as the Deal Line, which the Deal Finder compares listings with: a
/// Market Line's ID, or one of maplestory.market's figures'.
#[tauri::command]
pub fn choose_deal_line(line: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::ChooseDealLine(line));
}

#[tauri::command]
pub fn rebind_key(press: KeyPress, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::RebindKey(press));
}
