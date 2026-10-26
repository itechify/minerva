//! The Grind Tracker's window: whether the player wants it, and where it goes on screen.
//! It shows the Session's figures beside the panel, in a window of its own that the
//! player drags anywhere and closes with its ×.

use crate::panel::{GameWindow, PlacementMode, ScreenRect, Size, in_view, screen_at};
use serde::{Deserialize, Serialize};

/// A window's top-left corner relative to the game client's top-left corner (physical
/// px). It may lie outside the client: beside the game, or on another monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientOffset {
    pub x: i32,
    pub y: i32,
}

/// The Grind Tracker's window, as the shell shows it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrindTrackerWindow {
    /// The Grind Tracker is on (the Settings window's switch): the EXP figures are read.
    pub on: bool,
    /// The player wants it open: the panel's Grind Tracker button shows as on.
    pub open: bool,
    /// Where it goes on screen; `None` while it is hidden: closed, with the Grind Tracker
    /// off, with no game window, while setup is open, or while another app is in front
    /// (see [`crate::Layer`]). Never over the Minimap Title where it was found, the Status Bar's Character or
    /// its EXP figures: it shows in screen captures, ours included.
    pub placement: Option<ScreenRect>,
    /// Some map has a Personal Best, the Visit under way's included: the window bar's
    /// button that opens the Personal Bests window shows.
    pub personal_bests: bool,
    /// The Loot section is open, showing the Mesos row and the Items picked up (#171): as the
    /// player left it, across launches.
    pub loot_open: bool,
}

/// The gap between the panel and the Grind Tracker's window beside it.
const GAP: i32 = 8;

#[derive(Debug, Clone)]
pub(crate) struct GrindTracker {
    pub open: bool,
    /// Where the player dragged it; `None` until they do, and again once they drag it
    /// back beside the panel: it follows the panel.
    pub at: Option<ClientOffset>,
    /// Its size in physical px for the current display, as the shell measures it.
    pub size: Size,
}

impl Default for GrindTracker {
    fn default() -> Self {
        Self { open: true, at: None, size: Size { width: 0, height: 0 } }
    }
}

impl GrindTracker {
    /// The player dragged the window so its top-left corner is at (x, y), physical
    /// screen px: it keeps that spot relative to the `game` window. Let go within an
    /// eighth of its width of its spot beside the panel, it goes back to following the
    /// panel (placed at `panel` in `mode`).
    pub fn moved_to(&mut self, x: i32, y: i32, game: &GameWindow, panel: ScreenRect, mode: PlacementMode, avoid: &[ScreenRect]) {
        let home = Self { at: None, ..self.clone() }.placement(game, panel, mode, avoid);
        let snap = self.size.width / 8;
        let near = home.x.abs_diff(x) <= snap && home.y.abs_diff(y) <= snap;
        self.at = (!near).then_some(ClientOffset { x: x - game.client.x, y: y - game.client.y });
    }

    /// Where the window goes beside the `game`, with the panel (expanded or collapsed) at
    /// `panel` in `mode`: where the player dragged it, or else beside the panel. Always
    /// wholly on a monitor, and moved the shortest way off any of `avoid`.
    pub fn placement(&self, game: &GameWindow, panel: ScreenRect, mode: PlacementMode, avoid: &[ScreenRect]) -> ScreenRect {
        let Size { width, height } = self.size;
        let spot = match self.at {
            Some(at) => ScreenRect { x: game.client.x + at.x, y: game.client.y + at.y, width, height },
            None => self.beside(game, panel, mode),
        };
        in_view(game, spot, avoid)
    }

    /// Beside the panel, the tops level: the panel's top stays put as it grows and
    /// shrinks with each map, so this does too. Docked, on the side away from the game;
    /// otherwise on the side towards the middle of the game. The other side if that one
    /// leaves the panel's monitor.
    fn beside(&self, game: &GameWindow, panel: ScreenRect, mode: PlacementMode) -> ScreenRect {
        let Size { width, height } = self.size;
        let left = ScreenRect { x: panel.x - GAP - width as i32, y: panel.y, width, height };
        let right = ScreenRect { x: panel.x + panel.width as i32 + GAP, ..left };
        let middle = |r: ScreenRect| 2 * i64::from(r.x) + i64::from(r.width);
        let leftwards = match mode {
            PlacementMode::DockedLeft => true,
            PlacementMode::DockedRight => false,
            PlacementMode::Inside | PlacementMode::Detached => middle(panel) >= middle(game.client),
        };
        let (first, second) = if leftwards { (left, right) } else { (right, left) };
        let screen = screen_at(game, panel.x, panel.y);
        [first, second].into_iter().find(|spot| screen.contains(*spot)).unwrap_or(first)
    }
}
