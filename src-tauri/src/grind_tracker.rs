//! The Grind Tracker's window, beside the panel: the Session's EXP figures. Clickable while
//! it shows, and dragged by its bar. One of our windows over the game
//! ([`crate::panel_window`]); this says what its page renders.

use crate::panel_window::{Heights, PanelWindow, Shows, WindowSpec};
use minerva_core::{Action, Character, ExpTracking, Minerva, PanelState};
use serde::Serialize;

pub const GRIND_TRACKER: &str = "grind-tracker";

/// Its width in CSS px (logical); `src/grind-tracker.css` lays out to fit. Its height
/// follows its content, as the page reports it.
pub const SPEC: WindowSpec = WindowSpec {
    label: GRIND_TRACKER,
    event: "grind-tracker-state",
    watcher: "on-top-grind-tracker",
    width: 280.0,
    height: Heights { default: 140.0, min: 40.0, max: 400.0 },
    sized: Minerva::set_grind_tracker_size,
    moved: Some(|x, y| Action::GrindTrackerMoved { x, y }),
    beneath: &[],
};

/// What the Grind Tracker's page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackerView<'a> {
    /// The window shows: otherwise the page draws nothing.
    shown: bool,
    tracking: &'a ExpTracking,
    /// The Character playing, as the Status Bar names them, beside the EXP bar's reading.
    character: Option<&'a Character>,
    /// Some map has a Personal Best: the window bar's Personal Bests button shows.
    personal_bests: bool,
    /// The Personal Bests window is open: that button shows as on.
    personal_bests_open: bool,
    /// One of our windows has been over the Pickup Log for a while: the window says so.
    pickup_log_covered: bool,
}

/// Show the window where the core says, or nothing; and send its page the Grind Tracker's
/// figures.
pub fn sync(window: &mut PanelWindow, state: &PanelState) {
    let placement = state.grind_tracker.placement;
    let view = TrackerView {
        shown: placement.is_some(),
        tracking: &state.exp_tracking,
        character: state.character.as_ref(),
        personal_bests: state.grind_tracker.personal_bests,
        personal_bests_open: state.personal_bests.open,
        pickup_log_covered: state.pickup_log_covered,
    };
    window.sync(&view, Shows { placement, clickable: placement.is_some(), takes_keyboard: false });
}
