//! Whether one of our windows has been over the Pickup Log for a while (ADR 0026): the
//! Grind Tracker says so quietly, so a dip in mesos an hour has its reason in sight.
//!
//! Our windows are painted out of every capture, so a window over the log hides its lines
//! from the reader, which sees the capture as covered, or finds nothing. Where the log is
//! is where it was last seen: a capture with nothing in it leaves that as it was.

use crate::pickup_log::{PickupLogReading, PickupLogSeen};
use crate::ScreenRect;

/// One of our windows over the log for longer than this is the player's doing, not the
/// pointer passing (ms): the log is covered.
pub(crate) const COVERED_AFTER_MS: u64 = 3_000;

/// Where the Pickup Log was last seen and since when our windows have been over it.
#[derive(Debug, Default)]
pub(crate) struct PickupCover {
    /// Where the log was last seen on screen.
    area: Option<ScreenRect>,
    /// The capture time of the first reading that found one of our windows over it, of those
    /// since they were last off it.
    under_since: Option<u64>,
    /// The latest reading's capture time.
    now: u64,
}

impl PickupCover {
    /// Take in a reading, with our windows as they are.
    pub fn on_reading(&mut self, reading: &PickupLogReading, ours: &[ScreenRect]) {
        if let PickupLogSeen::Clear { area } | PickupLogSeen::Covered { area } = reading.seen {
            self.area = Some(area);
        }
        self.now = self.now.max(reading.at_ms);
        if self.under(ours) {
            self.under_since.get_or_insert(reading.at_ms);
        } else {
            self.under_since = None;
        }
    }

    /// Whether the log is covered, our windows being `ours` now: they are over where it was
    /// seen, and have been for more than [`COVERED_AFTER_MS`] of readings.
    pub fn covered(&mut self, ours: &[ScreenRect]) -> bool {
        if !self.under(ours) {
            self.under_since = None;
        }
        self.under_since.is_some_and(|since| self.now.saturating_sub(since) > COVERED_AFTER_MS)
    }

    fn under(&self, ours: &[ScreenRect]) -> bool {
        self.area.is_some_and(|area| ours.iter().any(|window| window.overlaps(area)))
    }
}
