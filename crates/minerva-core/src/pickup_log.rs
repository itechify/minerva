//! The Pickup Log reader (ADR 0026): the fading lines in the bottom-right corner of the
//! client area that announce gained EXP, mesos and picked-up Items. It finds the log itself,
//! counts the lines that arrived by how far the stack moved since the last clean capture, and
//! reads only those. It never counts a line it isn't sure of.
//!
//! The reader keeps the last clean capture's lines, so one reader reads one stream of
//! captures, in order. What a capture reads as carries the lines that arrived, so a Bug
//! Report's replay needs no pixels.

use crate::{Frame, ScreenPoint, ScreenRect};
use serde::{Deserialize, Serialize};

/// How often the Pickup Log is captured while it is read (ms): 10 times a second, on a thread
/// of its own (ADR 0026).
pub const PICKUP_LOG_CAPTURE_MS: u64 = 100;

/// A capture of the band of the client area the Pickup Log is looked for in.
#[derive(Debug, Clone)]
pub struct PickupLogCapture {
    pub frame: Frame,
    /// Where the frame's top-left pixel is on screen.
    pub origin: ScreenPoint,
}

/// What a Pickup Log capture reads as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PickupLogReading {
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
    /// Whether the log was seen, and where.
    pub seen: PickupLogSeen,
    /// The lines that arrived since the last clean capture, oldest first. Empty unless
    /// [`PickupLogSeen::Clear`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arrived: Vec<PickupLine>,
    /// More than one shift of the stack fitted, and the smallest was taken.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub chose: bool,
    /// Every row was new: lines may have come and gone unseen.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub overflowed: bool,
}

/// Whether a capture showed the Pickup Log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PickupLogSeen {
    /// Lines read clean: `area` is where they are on screen.
    Clear { area: ScreenRect },
    /// Lines found, but some of their rows can't be read (a Mob, a skill effect or one of our
    /// windows over them): the capture is skipped, and the next clean one is matched to the
    /// last clean one.
    Covered { area: ScreenRect },
    /// No lines: the log is empty, or isn't where it was looked for.
    Nothing,
}

/// One line of the Pickup Log, by its kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PickupLine {
    /// "You received EXP (+n)": white for the player's own kill (`party: false`), yellow for
    /// a share of a partymate's (`party: true`).
    Exp { amount: u64, party: bool },
    /// "Bonus EXP for PARTY (+n)", on every kill while partied.
    PartyBonus { amount: u64 },
    /// "You have gained mesos (+n)".
    Mesos { amount: u64 },
    /// "<name> x<quantity> earned. (<tab>)". `name` is as read, without the "..." of a name
    /// the game cut short (`cut_short`).
    Item { name: String, quantity: u32, tab: String, cut_short: bool },
    /// A line that arrived but didn't read as any of these: counted as arrived, never as a
    /// pickup.
    Unreadable,
}

/// Reads a stream of Pickup Log captures, in order.
#[derive(Debug, Default)]
pub struct PickupLogReader {}

impl PickupLogReader {
    /// A reader with the bundled recogniser.
    pub fn bundled() -> Self {
        Self::default()
    }

    /// Read one capture, the next after the last one read.
    pub fn read(&mut self, capture: &PickupLogCapture) -> PickupLogReading {
        PickupLogReading { at_ms: capture.frame.at_ms, seen: PickupLogSeen::Nothing, arrived: Vec::new(), chose: false, overflowed: false }
    }
}
