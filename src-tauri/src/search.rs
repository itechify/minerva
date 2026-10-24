//! Search in the overlay. The overlay is always clickable but never activated by a
//! click; only while the search view is open does it take the keyboard (ADR 0001): it
//! becomes focusable and comes to the front when the view opens, and gives focus back when
//! it closes, so Windows returns it to the game. Nothing is sent
//! to the game: no input, no messages.
//!
//! Every decision is the core's; this only moves focus to and from our own overlay and
//! relays the player's typing, keys and clicks.

use crate::Command;
use minerva_core::PanelControl;
use std::sync::mpsc::Sender;
use tauri::{State, WebviewWindow};

/// Make our overlay the foreground window, so the player's typing goes to its query
/// box. Only `SetForegroundWindow` on our own window, which Windows allows us right
/// after the player pressed our hotkey or clicked the overlay (our process received the
/// last input). Not tao's `set_focus`: when that call is refused, it synthesises an Alt
/// key press (`SendInput`), which would go to the game.
pub fn take_focus(overlay: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = overlay.hwnd() {
        // SAFETY: a plain Win32 call on our own window handle.
        let taken = unsafe { windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd) };
        if !taken.as_bool() {
            crate::errors::error!("Windows refused the overlay the foreground: the game keeps the keyboard");
        }
    }
    #[cfg(not(windows))]
    let _ = overlay;
}

/// The search view closed: if our overlay still has the foreground, hide it and show it
/// again without activating it, so Windows activates the window below it (the game),
/// as when any window in front of the game closes. Only our own window is hidden and
/// shown; nothing is called on the game's.
pub fn give_focus_back(overlay: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = overlay.hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SW_HIDE, SW_SHOWNOACTIVATE, ShowWindow};
        // SAFETY: plain Win32 calls on our own window handle.
        unsafe {
            if GetForegroundWindow() == hwnd {
                let _ = ShowWindow(hwnd, SW_HIDE);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
    }
    #[cfg(not(windows))]
    let _ = overlay;
}

// The player's input with a payload of its own; what is only an [`minerva_core::Action`]
// goes through `act`. Plain (not async) commands run in the order they come, so a quick
// type-then-Enter reaches the core as typed.

/// One of the icons at the top of the panel (🔍, Help & Setup, placement and the rest):
/// the same as its hotkey. Or a control the setup window shares with the panel (its Change
/// hotkeys, About & Credits' links and Update).
#[tauri::command]
pub fn panel_press(control: PanelControl, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Press(control));
}

/// A click on a listed mob's name in the panel (its ID): open its MeowDB page.
#[tauri::command]
pub fn open_mob_page(id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::OpenMobPage(id));
}

/// A click on one of the items an Item Lookup found sharing the name it read (its
/// MeowDB ID): open its MeowDB page.
#[tauri::command]
pub fn pick_item(id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::PickItem(id));
}

/// The query box now holds `query`.
#[tauri::command]
pub fn search_query(query: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::SearchQuery(query));
}

/// A click on a matched mob, or on a map in a mob's map list (its ID): pick it.
#[tauri::command]
pub fn search_pick(id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::SearchPick(id));
}

/// A click on a matched map (its ID): make it the Destination.
#[tauri::command]
pub fn search_pick_destination(id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::SearchPickDestination(id));
}

/// A click on matched items (the first one's ID): show their Item Card.
#[tauri::command]
pub fn search_pick_item(id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::SearchPickItem(id));
}
