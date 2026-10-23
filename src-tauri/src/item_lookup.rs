//! The Item Lookup alert's window (ADR 0007): what the latest lookup came to, at the top
//! centre of the game window where the core puts it. Clicks pass through it to the game,
//! except while it lists items to pick from. One of our windows over the game
//! ([`crate::panel_window`]); this says what its page renders.

use crate::panel_window::{Heights, PanelWindow, Shows, WindowSpec};
use minerva_core::{ItemLookupLine, Minerva, PanelState};
use serde::Serialize;

pub const ITEM_LOOKUP: &str = "item-lookup";

/// Its width in CSS px (logical); `src/item-lookup.css` centres the alert in it. Its height
/// follows the alert, as the page reports it.
pub const SPEC: WindowSpec = WindowSpec {
    label: ITEM_LOOKUP,
    event: "item-lookup-state",
    watcher: "on-top-item-lookup",
    width: 440.0,
    height: Heights { default: 40.0, min: 24.0, max: 400.0 },
    sized: Minerva::set_item_lookup_size,
    moved: None,
    beneath: &[],
};

/// What the alert's page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AlertView<'a> {
    /// The window shows: otherwise the page draws nothing.
    shown: bool,
    line: Option<&'a ItemLookupLine>,
    /// The hotkey the player pressed (Item Lookup's or the Item Card's), for the alert to
    /// name.
    hotkey: &'a str,
}

/// Show the alert where the core says, or nothing; and let clicks through unless it lists
/// items to pick from.
pub fn sync(window: &mut PanelWindow, state: &PanelState) {
    let placement = state.item_lookup_placement;
    let line = state.item_lookup.as_ref().filter(|_| placement.is_some());
    let view = AlertView { shown: line.is_some(), line, hotkey: &state.item_lookup_hotkey };
    window.sync(&view, Shows { placement, clickable: state.item_lookup_clickable, takes_keyboard: false });
}
