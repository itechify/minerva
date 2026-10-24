//! The Settings window (GLOSSARY): the hotkeys, the Deal Margin, the Grind Tracker's switch,
//! Game Data and Updates, in a window of its own over the game. The gear at the top of the
//! panel opens it and closes it again, as its own × does, and setup's Change hotkeys opens
//! it. Each launch starts with it closed.
//!
//! - It has the keyboard as it opens, and again when the player clicks a Change or the Deal
//!   Margin field; clicking back into the game gives the keyboard back and the window stays
//!   open. Opening the search view takes the keyboard from it, and it from the search view,
//!   which closes. Never while it doesn't show: it would take the keyboard from the game.
//! - Where it goes: beside the panel, the tops level, on the side towards the middle of the
//!   game, clear of the captured regions and our other windows, until the player drags it;
//!   then where they left it relative to the client area, until they drag it back beside
//!   the panel. No wider or taller than the client area, always wholly on a monitor and
//!   never over a captured region (it shows in screen captures, ours included).
//! - It shows while setup is open, which opens it, and only while the overlay can show
//!   (see [`crate::Layer`]).

use crate::grind_tracker::ClientOffset;
use crate::hotkey_settings::{RebindProblem, SettingsNotice};
use crate::item_card::beside_panel;
use crate::panel::{GameWindow, ScreenRect, Size, in_view};
use crate::settings::Hotkey;
use serde::Serialize;

/// The Settings window while it is open, as the shell shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsWindow {
    /// Where it goes on screen; `None` while it is hidden: with no game window, or while
    /// another app is in front (see [`crate::Layer`]).
    pub placement: Option<ScreenRect>,
    /// It has the keyboard: it opened, or the player clicked a Change or the Deal Margin
    /// field, and hasn't clicked back into the game since.
    pub takes_keyboard: bool,
    /// The hotkey waiting for the player to press its new combination; `None` while none is.
    pub rebinding: Option<Hotkey>,
    /// Why the last combination pressed for it wasn't taken.
    pub problem: Option<RebindProblem>,
    /// What the player's last change came to, until they start another or close the window.
    pub notice: Option<SettingsNotice>,
    /// Every hotkey has its default: Restore defaults would change nothing.
    pub at_defaults: bool,
}

/// Where the Settings window goes on screen.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SettingsPlace {
    /// Where the player dragged it; `None` until they do, and again once they drag it back
    /// beside the panel: it goes beside the panel.
    pub at: Option<ClientOffset>,
    /// The size its page lays it out at, in physical px for the current display, as the
    /// shell measures it.
    pub size: Size,
}

impl Default for SettingsPlace {
    fn default() -> Self {
        Self { at: None, size: Size { width: 0, height: 0 } }
    }
}

impl SettingsPlace {
    /// The player dragged the window so its top-left corner is at (x, y), physical screen
    /// px: it keeps that spot relative to the `game` window. Let go within an eighth of its
    /// width of its spot beside the `panel` (placed off `avoid` and our `others`), it goes
    /// back beside the panel.
    pub fn moved_to(&mut self, x: i32, y: i32, game: &GameWindow, panel: ScreenRect, avoid: &[ScreenRect], others: &[ScreenRect]) {
        let home = Self { at: None, ..*self }.placement(game, panel, avoid, others);
        let snap = home.width / 8;
        let near = home.x.abs_diff(x) <= snap && home.y.abs_diff(y) <= snap;
        self.at = (!near).then_some(ClientOffset { x: x - game.client.x, y: y - game.client.y });
    }

    /// Where the window goes over the `game`, with the panel (expanded or collapsed) at
    /// `panel`: where the player dragged it, moved the shortest way off any of `avoid`; or
    /// else beside the panel, off `avoid` and our `others` windows too.
    pub fn placement(&self, game: &GameWindow, panel: ScreenRect, avoid: &[ScreenRect], others: &[ScreenRect]) -> ScreenRect {
        let client = game.client;
        let size = Size { width: self.size.width.min(client.width), height: self.size.height.min(client.height) };
        match self.at {
            Some(at) => {
                let spot = ScreenRect { x: client.x + at.x, y: client.y + at.y, width: size.width, height: size.height };
                in_view(game, spot, avoid)
            }
            None => {
                let avoid: Vec<ScreenRect> = avoid.iter().chain(others).chain([&panel]).copied().collect();
                beside_panel(game, panel, size, &avoid)
            }
        }
    }
}
