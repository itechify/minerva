//! The settings view in the overlay, where the player rebinds the hotkeys and refreshes the
//! game data. It takes the keyboard as the search view does (see `search.rs`), and the core
//! releases the hotkeys while the player rebinds one, so the combination they press reaches
//! the overlay.
//!
//! Every decision is the core's; this only relays the player's clicks and key presses.

use crate::Command;
use minerva_core::{Action, Hotkey, KeyPress};
use std::sync::mpsc::Sender;
use tauri::State;

fn act(commands: &Sender<Command>, action: Action) {
    let _ = commands.send(Command::Act(action));
}

/// Esc or the back arrow: back to the normal view.
#[tauri::command]
pub fn settings_close(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::CloseSettings);
}

/// A hotkey's Change button: wait for its new combination.
#[tauri::command]
pub fn rebind_begin(hotkey: Hotkey, commands: State<'_, Sender<Command>>) {
    act(&commands, Action::BeginRebind(hotkey));
}

/// A key the player pressed on the settings view.
#[tauri::command]
pub fn rebind_key(press: KeyPress, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::RebindKey(press));
}

/// The Cancel button beside the hotkey waiting for its new combination.
#[tauri::command]
pub fn rebind_cancel(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::CancelRebind);
}

/// The waiting row's Turn off: the hotkey has no combination any more.
#[tauri::command]
pub fn rebind_turn_off(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::TurnOffHotkey);
}

/// Swap, after the player pressed another hotkey's combination.
#[tauri::command]
pub fn rebind_swap(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::SwapHotkeys);
}

/// Restore defaults: every hotkey back to its default.
#[tauri::command]
pub fn hotkeys_reset(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::ResetHotkeys);
}

/// Undo, after Restore defaults.
#[tauri::command]
pub fn hotkeys_undo_reset(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::UndoResetHotkeys);
}

/// Game Data's Refresh: fetch OSMS's data afresh now.
#[tauri::command]
pub fn game_data_refresh(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::RefreshGameData);
}

/// The settings view's "Check for updates" switch: on, Minerva checks at launch and every
/// 6 hours; off, only when the player asks in Help & Setup.
#[tauri::command]
pub fn check_for_updates_set(on: bool, commands: State<'_, Sender<Command>>) {
    act(&commands, Action::SetCheckForUpdates { on });
}
