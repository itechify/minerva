//! Map tracker: decides when the current map changes.

use crate::matcher::Snap;

/// Consecutive confident reads of a map needed before the panel switches to it. At
/// the shell's ~3 Hz capture this switches within about a second of arriving.
const READS_TO_SWITCH: u32 = 2;

#[derive(Default)]
pub struct Tracker {
    /// Index (into the map list) of the last map we were confident about.
    current: Option<usize>,
    /// A different map read confidently on the latest frames, and how many in a row.
    pending: Option<(usize, u32)>,
}

/// What the latest frame means for the panel.
pub struct Tracked {
    pub current: Option<usize>,
    pub not_sure: bool,
}

impl Tracker {
    /// Take the latest frame's match: `None` when it was unreadable or not confident.
    pub fn update(&mut self, snap: Option<Snap>) -> Tracked {
        let Some(snap) = snap else {
            self.pending = None;
            return self.unconfirmed();
        };
        if self.current == Some(snap.index) {
            self.pending = None;
            return Tracked { current: self.current, not_sure: false };
        }
        let reads = match self.pending {
            Some((index, n)) if index == snap.index => n + 1,
            _ => 1,
        };
        if reads >= READS_TO_SWITCH {
            self.current = Some(snap.index);
            self.pending = None;
            return Tracked { current: self.current, not_sure: false };
        }
        self.pending = Some((snap.index, reads));
        self.unconfirmed()
    }

    /// The map list changed: each map known by its index in the old list, by its index
    /// in the new one (`None`: no longer listed).
    pub fn remap(&mut self, index: impl Fn(usize) -> Option<usize>) {
        self.current = self.current.and_then(&index);
        self.pending = self.pending.and_then(|(at, reads)| Some((index(at)?, reads)));
    }

    fn unconfirmed(&self) -> Tracked {
        Tracked { current: self.current, not_sure: true }
    }
}
