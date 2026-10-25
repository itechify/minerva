//! The Owl Bubble (ADR 0025): the owl at the top centre of the game window, saying one line
//! at a time in a speech bubble. Item Lookup's line goes first while it lasts (ADR 0007),
//! as the player just asked for it; otherwise the update's line, while Minerva found a newer
//! version by itself that the player hasn't been told of (ADR 0019). The page puts the line
//! into words; which line, its mood, where it goes and whether it takes clicks are decided
//! here.

use crate::item_lookup::{ItemLookup, ItemLookupLine};
use crate::panel::{GameWindow, ScreenRect, Size};
use serde::Serialize;

/// The Owl Bubble, as its window shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwlBubble {
    /// Where it goes on screen: at the top centre of the game window, clear of the captured
    /// regions.
    pub placement: ScreenRect,
    pub line: OwlLine,
    /// The bubble's look and the owl's pose.
    pub mood: OwlMood,
    /// It takes clicks: an Item Lookup's list to pick from, or the update's line (its × and
    /// the bubble). Otherwise every click goes through it to the game.
    pub clickable: bool,
}

/// What the owl says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "says", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum OwlLine {
    /// What the latest Item Lookup came to, or what kept an Item Card from showing.
    ItemLookup(ItemLookupLine),
    /// A newer version is out: "A newer version of Minerva is available! (X.Y.Z)". Its × is
    /// [`crate::Action::DismissUpdateLine`], and a click on it
    /// [`crate::Action::OpenUpdateLine`].
    Update { version: String },
}

/// How the owl says it: the bubble's look, and the owl's pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OwlMood {
    News,
    /// Something went wrong: the bubble wears the panel's problem note.
    Problem,
    /// Waiting on MeowDB or OSMS.
    Waiting,
}

impl OwlLine {
    pub(crate) fn mood(&self) -> OwlMood {
        match self {
            OwlLine::Update { .. } => OwlMood::News,
            OwlLine::ItemLookup(line) => match line.outcome {
                ItemLookup::LookingUp { .. } | ItemLookup::ItemsLoading => OwlMood::Waiting,
                ItemLookup::Opened { .. } | ItemLookup::Choose { .. } => OwlMood::News,
                ItemLookup::NoTooltip
                | ItemLookup::Unreadable
                | ItemLookup::Covered { .. }
                | ItemLookup::NotFound { .. }
                | ItemLookup::NoSuchItem { .. }
                | ItemLookup::ItemsUnavailable
                | ItemLookup::ItemsFormatChanged
                | ItemLookup::Unreachable
                | ItemLookup::FormatChanged => OwlMood::Problem,
            },
        }
    }

    fn clickable(&self) -> bool {
        match self {
            OwlLine::Update { .. } => true,
            OwlLine::ItemLookup(line) => matches!(line.outcome, ItemLookup::Choose { .. }),
        }
    }
}

/// What the owl says, of Item Lookup's `lookup` and the update's `update` (the newer
/// version's), at `size` over `game`, clear of `avoid`; `None` with nothing to say.
pub(crate) fn bubble(
    lookup: Option<ItemLookupLine>,
    update: Option<String>,
    game: &GameWindow,
    size: Size,
    avoid: &[ScreenRect],
) -> Option<OwlBubble> {
    let line = match (lookup, update) {
        (Some(lookup), _) => OwlLine::ItemLookup(lookup),
        (None, Some(version)) => OwlLine::Update { version },
        (None, None) => return None,
    };
    Some(OwlBubble { placement: placement(game, size, avoid), mood: line.mood(), clickable: line.clickable(), line })
}

/// The bubble's top edge sits this share of the game's client area down from its top.
const TOP_SHARE: f32 = 0.05;
/// Gap kept between the bubble and a captured region it moves clear of (physical px).
const GAP: i32 = 8;

/// Where the bubble of `size` goes: centred at the top of the game's client area, and below
/// any of `avoid` (the captured regions) it would cover, since our windows show in our
/// captures. Never wider than the client area.
fn placement(game: &GameWindow, size: Size, avoid: &[ScreenRect]) -> ScreenRect {
    let client = game.client;
    let width = size.width.min(client.width);
    let x = client.x + (client.width - width) as i32 / 2;
    let mut bubble = ScreenRect { x, y: client.y + (client.height as f32 * TOP_SHARE) as i32, width, height: size.height };
    // Each move only goes down, so going over the regions twice settles it.
    for _ in 0..2 {
        for region in avoid {
            if bubble.overlaps(*region) {
                bubble.y = region.y + region.height as i32 + GAP;
            }
        }
    }
    bubble
}
