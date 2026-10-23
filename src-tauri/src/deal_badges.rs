//! The Deal Badges' window (ADR 0014): a badge beside each listing of the Player Shop the
//! Deal Finder read, left of the shop's window where the core puts it. Without badges its
//! page draws nothing and clicks pass through it; with them it takes clicks: one on a badge
//! shows its item's card. Before a capture for an Item Card or Item Lookup press it moves off
//! the screen, as the Item Card does. One of our windows over the game
//! ([`crate::panel_window`]); this says what its page renders.

use crate::panel_window::{Heights, PanelWindow, Shows, WindowSpec};
use minerva_core::{DealBadge, Minerva, PanelState};
use serde::Serialize;

pub const DEAL_BADGES: &str = "deal-badges";

/// Its width in CSS px (logical); `src/deal-badges.css` right-aligns the badges in it,
/// towards the shop. Its height is the shop's list's, as the core places it: only its width
/// counts.
pub const SPEC: WindowSpec = WindowSpec {
    label: DEAL_BADGES,
    event: "deal-badges-state",
    watcher: "on-top-deal-badges",
    width: 116.0,
    height: Heights { default: 0.0, min: 0.0, max: 0.0 },
    sized: Minerva::set_deal_badges_size,
    moved: None,
    beneath: &[],
};

/// What the badges' page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BadgesView<'a> {
    /// The badges, each level with its listing (physical px from the window's top);
    /// empty while the window draws nothing.
    badges: &'a [DealBadge],
    /// The Deal Line's name, for each badge's words.
    deal_line: Option<&'a str>,
}

/// Show the badges where the core says, or nothing; and take clicks only while there are
/// badges.
pub fn sync(window: &mut PanelWindow, state: &PanelState) {
    let shown = state.deal_badges.as_ref();
    let view = BadgesView {
        badges: shown.map_or(&[], |shown| &shown.badges),
        deal_line: state.deal_finder.deal_line.as_deref(),
    };
    let shows = Shows { placement: shown.map(|shown| shown.placement), clickable: state.deal_badges_clickable, takes_keyboard: false };
    window.sync(&view, shows);
}
