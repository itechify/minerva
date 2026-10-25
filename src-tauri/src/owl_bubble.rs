//! The Owl Bubble's window (ADR 0025): the owl at the top centre of the game window where
//! the core puts it, saying Item Lookup's line (ADR 0007) or that a newer version is out
//! (ADR 0019). Clicks pass through it to the game unless the core says it takes them: a list
//! of items to pick from, or the update's line. One of our windows over the game
//! ([`crate::panel_window`]); this says what its page renders.

use crate::panel_window::{Heights, PanelWindow, Shows, WindowSpec};
use minerva_core::{Minerva, OwlBubble, PanelState};
use serde::Serialize;

pub const OWL_BUBBLE: &str = "owl-bubble";

/// Its width in CSS px (logical); `src/owl-bubble.css` centres the owl and its bubble in it.
/// Its height follows them, as the page reports it: never less than the owl.
pub const SPEC: WindowSpec = WindowSpec {
    label: OWL_BUBBLE,
    event: "owl-bubble-state",
    watcher: "on-top-owl-bubble",
    width: 520.0,
    height: Heights { default: 56.0, min: 48.0, max: 400.0 },
    sized: Minerva::set_owl_bubble_size,
    moved: None,
    beneath: &[],
};

/// What the owl's page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OwlView<'a> {
    /// What the owl says, and how; `None` while the window draws nothing.
    bubble: Option<&'a OwlBubble>,
    /// The hotkey the player pressed (Item Lookup's or the Item Card's), for Item Lookup's
    /// line to name.
    hotkey: &'a str,
}

/// Show the owl where the core says, or nothing; and take clicks only where the core says.
pub fn sync(window: &mut PanelWindow, state: &PanelState) {
    let bubble = state.owl_bubble.as_ref();
    let view = OwlView { bubble, hotkey: &state.item_lookup_hotkey };
    let shows = Shows {
        placement: bubble.map(|owl| owl.placement),
        clickable: bubble.is_some_and(|owl| owl.clickable),
        takes_keyboard: false,
    };
    window.sync(&view, shows);
}
