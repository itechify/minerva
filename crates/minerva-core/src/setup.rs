//! First-run setup, and setup again when the player asks. Nothing is marked: the Minimap Title
//! is found wherever it is (ADR 0021), and the EXP figures in the Status Bar (ADR 0017).

use crate::CurrentMap;
use serde::Serialize;

/// The credit for the map setup shows: the names it shows are OSMS Data Explorer's.
const MAP_NAMES_CREDIT: &str = "Map names from OSMS Data Explorer";

/// Why the player needs to (re)do setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupReason {
    /// Setup has never been finished.
    FirstRun,
    /// The player asked to see setup again.
    Requested,
}

/// Setup, as the overlay and the setup window show it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    /// Why setup is needed; `None` once it is finished.
    pub needed: Option<SetupReason>,
    /// The map the Minimap Title shows, for "Finding your map"; `None` until one is found.
    pub map: Option<CurrentMap>,
    /// "Map names from OSMS Data Explorer" when `map` shows one.
    pub credit: Option<&'static str>,
    /// The setup window shows: when setup becomes needed, and when the player asks
    /// for it, until it is finished or closed.
    pub window_open: bool,
}

#[derive(Debug, Default)]
pub(crate) struct Setup {
    /// Setup has been finished, in this session or an earlier one.
    pub done: bool,
    /// The player asked to redo setup and hasn't finished or cancelled it.
    requested: bool,
    /// The setup window is showing.
    window_open: bool,
    /// Why setup was needed when the window was last opened for it (or closed).
    opened_for: Option<SetupReason>,
}

impl Setup {
    /// Finish setup. Nothing has to be found first: the title is looked for all along.
    pub fn finish(&mut self) {
        self.done = true;
        self.requested = false;
    }

    /// The re-setup hotkey: open setup, even if it was needed before and closed.
    pub fn request(&mut self) {
        self.requested = true;
        self.window_open = true;
    }

    /// The player closed setup without finishing it.
    pub fn cancel(&mut self) {
        self.requested = false;
        self.window_open = false;
    }

    /// Open the setup window when setup becomes needed (or needed for a new reason),
    /// and close it once it isn't.
    pub fn follow_needed(&mut self) {
        let needed = self.needed();
        if needed.is_none() {
            self.window_open = false;
        } else if needed != self.opened_for {
            self.window_open = true;
        }
        self.opened_for = needed;
    }

    /// Setup, showing `map`, the map the Minimap Title shows.
    pub fn state(&self, map: Option<CurrentMap>) -> SetupState {
        SetupState {
            needed: self.needed(),
            credit: map.is_some().then_some(MAP_NAMES_CREDIT),
            map,
            window_open: self.window_open,
        }
    }

    pub fn needed(&self) -> Option<SetupReason> {
        if !self.done {
            Some(SetupReason::FirstRun)
        } else if self.requested {
            Some(SetupReason::Requested)
        } else {
            None
        }
    }
}
