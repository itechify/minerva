//! The settings view in the overlay, where the player rebinds the hotkeys and refreshes the
//! game data. It takes the keyboard as the search view does (see `search.rs`), and the core
//! releases the hotkeys while the player rebinds one, so the combination they press reaches
//! the overlay.
//!
//! Every decision is the core's; this only relays the key presses (the clicks are
//! [`minerva_core::Action`]s, through `act`).

use crate::Command;
use minerva_core::KeyPress;
use std::sync::mpsc::Sender;
use tauri::State;

/// A key the player pressed on the settings view.
#[tauri::command]
pub fn rebind_key(press: KeyPress, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::RebindKey(press));
}
