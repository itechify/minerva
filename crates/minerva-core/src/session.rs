//! The Session (ADR 0005): EXP bar readings, with their capture times, turned into Time
//! to Level and EXP rates. There is no clock here and no EXP table.
//!
//! - The level's total EXP comes from the bar itself: the game rounds the percentage to
//!   hundredths, so each reading allows a range of totals ("149 [0.89%]": 16,649 to
//!   16,836). The range learned is what every reading in the level allows; it narrows
//!   as the percentage grows.
//! - A reading that fits the range is the same level: less EXP than before is a death.
//!   One that doesn't fit is a level-up if it allows only bigger totals and the bar fell
//!   from near 100%, its EXP and its percentage both (one figure misread near the end
//!   of a level can allow bigger totals, but the other hasn't fallen); a misread
//!   otherwise, unless [`SWITCH_READINGS`] in a row fit another level. If their EXP
//!   carries on from the last reading taken (see [`carries_on`]), and their level is
//!   within [`RELEARN_PERCENT`] of the one learned, the level was learned wrong (a
//!   misread that fitted it narrowed the range past the true total), or they are one
//!   misread repeated on still frames: it is learned anew from them, and the Session
//!   goes on. Otherwise it is a character switch, which starts a new Session, if the
//!   bar was out of sight between the last reading taken and the first of them: the
//!   character select screen has no EXP bar. If it wasn't, they are one misread
//!   repeated, however often (a still bar shows the same frame again and again), and
//!   are set aside. A reading barely into the next level may fit the range too (2 [0.01%]
//!   allows a wide range of totals); when the bar went from near 100% to as near 0%, it
//!   is a level-up all the same.
//! - The Session starts on the first EXP change, from the EXP the bar shows then.
//! - Active time runs while the EXP keeps changing. A gap of up to [`IDLE_MS`] between
//!   changes counts; a longer one is a break, which pauses the Session and counts for
//!   nothing, from the last change to the next, so the rates hold while paused. Once
//!   the Session has warmed up it stays so, breaks or not.
//! - A bar that can't be read isn't a break: nothing is known of the EXP then. Once it
//!   has gone [`UNREADABLE_MS`] without reading whole the Session says so, its figures
//!   held. If it reads again within [`IDLE_MS`] of the last whole reading, the stretch
//!   was a gap like any other; after longer, the stretch counts for nothing, and neither
//!   does whatever the EXP did over it: the next reading is taken without a change. (No
//!   frames at all, as while the game window is minimised, is no evidence either way.)
//! - The player's reset can be undone until the EXP next changes, which starts the new
//!   Session; a character switch ends the old one for good.
//! - Each map the panel confirms starts a Visit (GLOSSARY), which Sessions don't end:
//!   the share of the map's solo ceiling counts only the play since. The title going
//!   unread for a while doesn't end it, nor does the map's page coming later, nor the
//!   player choosing among Candidates (they haven't moved).
//! - The figures shown hold still for each [`REFRESH_MS`] of capture time, rather than
//!   jumping with every kill. Events show at once: the state changing (the Session
//!   starting, warming up, pausing, resuming, or the bar going unread), the first figures
//!   there are, a death, a level-up, a reset or its undoing, a character switch, a new
//!   Visit, a map with another ceiling or Personal Best, and a best removed (this
//!   Visit's or one from the history). The bar's last reading shows as it comes, and so
//!   do what moves with it: the Pace Trace (the 5 minutes of active play up to the last
//!   EXP change, a step at each, against the pace of the 5 minutes before them, and
//!   against the Best row's pace from arriving on the map once that is within reach), the
//!   forecast (where the bar will be at the Last 5 min pace shown) and the ground a death
//!   took back.
//! - The Rolling Rate as shown afresh, over a full 5 minutes of the Visit's play, counts
//!   towards the Visit's Personal Best on the map (see [`crate::personal_best`]), unless
//!   the bar can't be read. The core says which map that is, if any: one it is certain
//!   of, with a Map Summary. Maps that share a title but would show the same count as the
//!   first of them, the one the panel shows. Bests outlast Sessions, as Visits do. Each
//!   is stamped with the Character the Session is for, and the level and job the Status
//!   Bar showed, and the Best row races that Character's (the Unassigned one while none
//!   is named).

use crate::ExpBar;
use crate::bug_report::{Level, Note};
use crate::personal_best::{Bests, SavedBests, Stamp, VisitBest};
use crate::status_bar::StatusBar;
use serde::{Deserialize, Serialize};

/// What EXP tracking shows: everything the panel's tracking section and the collapsed
/// icon's badge render.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpTracking {
    /// Off, waiting, warming up, active or paused.
    pub state: TrackingState,
    /// Time to Level in seconds at the net Rolling Rate, rounded more coarsely the
    /// further off it is (see [`rounded_time_to_level`]); `None` ("—") while there is
    /// no rate yet, the net rate is zero or negative, or the level's total isn't known.
    pub time_to_level_s: Option<u64>,
    /// Time to Level, short, for the headline and the collapsed icon's badge: "47m",
    /// "3h 10m", "14h".
    pub time_to_level_badge: Option<String>,
    /// The Rolling Rate: over the last 5 minutes of active play. `None` until the
    /// Session has some active time.
    pub rolling: Option<ExpRate>,
    /// The average over the whole Session's active time. `None` as for `rolling`.
    pub session_average: Option<ExpRate>,
    /// The Rolling Rate's EXP/h on the shown map, as a share of its solo EXP/h ceiling
    /// (its Map Summary), in hundredths of a percent: 7300 for "73%". It counts
    /// only the play since arriving on the map (the Visit's), once there has been
    /// [`WARM_UP_MS`] of it, so one map's EXP is never measured against another's
    /// ceiling, the same or not. `None` without a recognised map with a Map Summary,
    /// until then, or while the rate there is zero or negative.
    pub ceiling_share_hundredths: Option<i64>,
    /// The ceiling the share is of: the shown map's solo EXP/h (its Map Summary), while
    /// there is a share.
    pub solo_exp_ceiling: Option<u64>,
    /// Deaths this Session and what they cost: "deaths: 2 (−1.4%)".
    pub deaths: Deaths,
    /// The Session started afresh by itself, on a switch to another character, and
    /// hasn't warmed up since: the panel says why the figures went. Never after a reset
    /// the player made.
    pub character_switched: bool,
    /// The Session's active time so far, in whole seconds; 0 while waiting.
    pub active_s: u64,
    /// The active time the Rolling Rate covers, in whole seconds: the Session's, up to 5
    /// minutes. Once `active_s` is more, the Session average is over a longer stretch and
    /// says something the Rolling Rate doesn't.
    pub rolling_s: u64,
    /// The player reset the Session and the EXP hasn't changed since: the reset can be
    /// undone, bringing the old Session back.
    pub reset_undoable: bool,
    /// The last EXP bar reading taken, as the bar shows it; it changes with every
    /// reading rather than holding with the figures, so the player sees it being read.
    /// `None` until one is.
    pub bar: Option<ExpBar>,
    /// The Best row: the shown map's Personal Best, in EXP/h, as it stands: the best of the
    /// earlier Visits from arriving, and this Visit's best from the refresh that beats it
    /// (as the Grind Tracker writes both, 3 significant figures; a tie stays the earlier
    /// Visits'). `None` until the map has a best, and where no bests are kept: a map without
    /// a Map Summary, or Candidates waiting for a choice.
    pub personal_best: Option<u64>,
    /// The Best row's figure was set on this Visit: the row says so.
    pub personal_best_this_visit: bool,
    /// The star on the Rolling Rate's row: the Best row is this Visit's, and the Rolling Rate
    /// is it, as the Grind Tracker writes both (3 significant figures), over a full 5 minutes
    /// of active play on the map with the bar read, as a best counts. It goes as soon as the
    /// rate drops back. It is of the Rolling Rate shown, so it holds with it between
    /// refreshes.
    pub new_best: bool,
    /// The Pace Trace: the last 5 minutes raced against the 5 before them, and against
    /// the Best row's pace once it is within reach. `None` until there has been a minute
    /// of play before the 5 minutes to race. It changes with the EXP, as `bar` does,
    /// rather than holding with the figures.
    pub pace: Option<PaceTrace>,
    /// Where the bar will be at the Last 5 min pace shown, after each of [`FORECAST_S`]
    /// short of the level-up. Measured from the bar's last reading, it moves with it.
    /// Empty without a gain to go on.
    pub forecast: Vec<Forecast>,
    /// The bar's percentage before the last death in this level, while it reads less:
    /// the ground a death took back. It goes once the bar is past it again, the level is
    /// up, or another character's bar is read.
    pub lost_from_hundredths: Option<u32>,
    /// Unreadable because the EXP figures had nowhere to be read: the Status Bar isn't found,
    /// and wasn't since the client area last changed size (ADR 0017).
    pub no_status_bar: bool,
}

/// The Pace Trace: the Session's EXP over the 5 minutes of active play up to its last
/// change, a step at each change, against the pace of the 5 minutes before them, and
/// once it is within reach, the pace of the map's Personal Best. It moves only as the EXP
/// does, never with the clock: between changes it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaceTrace {
    /// The active time it spans: the Rolling Rate's 5 minutes.
    pub span_ms: u64,
    /// Where the 5 minutes begin, then a point at each EXP change since, the last change
    /// last. Each holds the EXP gained since the first.
    pub points: Vec<PacePoint>,
    /// The pace of the 5 minutes of active play before them (or of as many as there
    /// were, a minute at least), a straight line from the first point to the last change.
    pub earlier: [PacePoint; 2],
    /// The Best row's EXP/h as a pace, a straight line: from where the race on the map
    /// begins (the first point, or arriving on the map if later), at the EXP gained by
    /// then, to the last change. Only once it is within reach: the play since arriving, a
    /// minute of it at least, has come within [`IN_REACH_PERCENT`] of it on this Visit.
    pub best: Option<[PacePoint; 2]>,
    /// Arriving on the map, if it was within the 5 minutes: how long before the last
    /// change, 0 if there has been none since.
    pub arrived_ago_ms: Option<u64>,
}

/// A point on the Pace Trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PacePoint {
    /// How long before the last EXP change, in active time.
    pub ago_ms: u64,
    /// The EXP gained since the trace's first point: less after a death, below 0 if
    /// deaths took more.
    pub exp: i64,
}

/// Where the bar will be some time from now at the Last 5 min pace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    /// How far ahead, in seconds.
    pub after_s: u64,
    /// The bar's percentage then, in hundredths: always short of 100%.
    pub percent_hundredths: u32,
}

/// The deaths in a Session: drops in EXP within the level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deaths {
    /// How many deaths.
    pub count: u32,
    /// The EXP they cost, all told.
    pub exp_lost: u64,
    /// The EXP lost in hundredths of a percent of the level, as the bar showed it.
    pub percent_lost_hundredths: u32,
}

/// A net EXP rate: gains less what deaths cost. Negative when deaths cost more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpRate {
    /// Net EXP per hour of active time.
    pub exp_per_hour: i64,
    /// The same rate in hundredths of a percent of the current level per hour: 125 for
    /// "1.25%/h". `None` while the level's total isn't known (only 0.00% read yet).
    pub percent_per_hour_hundredths: Option<i64>,
}

/// Where the Session is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackingState {
    /// The Grind Tracker is off.
    #[default]
    Off,
    /// Tracking is on, but the EXP hasn't changed yet: the Session starts when it does.
    Waiting,
    /// The first [`WARM_UP_MS`] of active time: the rates are still jumpy.
    WarmingUp,
    /// Warmed up (once is enough), and the EXP changed within the last [`IDLE_MS`].
    Active,
    /// No EXP change for more than [`IDLE_MS`]: the rates hold until the next change.
    Paused,
    /// The EXP bar hasn't read whole for [`UNREADABLE_MS`]: something covers it, or the
    /// Status Bar isn't found. Whatever the Session was doing, its figures hold until the
    /// bar reads again.
    Unreadable,
}

/// A gap between EXP changes longer than this is a break: it pauses the Session.
const IDLE_MS: u64 = 2 * 60_000;
/// The EXP bar going this long without reading whole is a problem to tell the player
/// about, not a moment's cover (the pointer passing over it).
const UNREADABLE_MS: u64 = 10_000;
/// A Session warms up for this much active time.
const WARM_UP_MS: u64 = 60_000;
/// The Rolling Rate covers this much active time.
const ROLLING_MS: u64 = 5 * 60_000;
/// An hour, which the rates are per.
const HOUR_MS: u64 = 3_600_000;
/// 100% in hundredths of a percent, the unit of the bar and of every share here.
const HUNDRED_PERCENT: u32 = 10_000;
/// Readings in a row that fit a level of their own, and not the one learned, that make
/// a character switch (or show the level was learned wrong). A misread has been seen to
/// repeat on four frames of a still bar, which is why a switch also needs the bar to
/// have been out of sight.
const SWITCH_READINGS: u32 = 3;
/// No frame of the EXP bar for this long is the bar out of sight, as a frame that
/// doesn't read whole is: it isn't being captured (the game window gone, or its size
/// changed). Frames come every second.
const OUT_OF_SIGHT_MS: u64 = 5_000;
/// Readings that fit a level of their own, carrying on from the last reading taken,
/// are the same level learned anew if their level is within this many percent of the
/// one learned. A misread narrows the range by far less, and another character's level
/// is apart by more unless it is the next one up or down; at that, its bar would have
/// to be where the last reading left off.
const RELEARN_PERCENT: u128 = 10;
/// A level-up needs the bar within at least this many hundredths of 100% before it.
const NEAR_LEVEL_UP_HUNDREDTHS: u32 = 1000;
/// The figures shown are measured afresh once in each stretch of this much capture time
/// (counted from capture time 0), unless an event shows them sooner.
const REFRESH_MS: u64 = 30_000;
/// The Pace Trace shows the Best's pace once the play since arriving on the map comes to
/// this share of it, in percent: within reach.
const IN_REACH_PERCENT: i64 = 90;
/// How far ahead the forecast looks, in seconds: 5, 10, 15 and 30 minutes, then 1, 2, 3,
/// 6, 12 and 24 hours. The Grind Tracker marks as many as have room on its gauge.
const FORECAST_S: [u64; 10] = [300, 600, 900, 1_800, 3_600, 7_200, 10_800, 21_600, 43_200, 86_400];

/// The level's total EXP as far as readings tell: `lo..=hi`, `hi` unbounded
/// (`u64::MAX`) while only 0.00% has been read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Total {
    lo: u64,
    hi: u64,
}

impl Total {
    /// The totals a reading allows: `exp / total` rounds to the percentage shown, and
    /// the level holds more than `exp`. `None` if no total does (a misread).
    fn of(bar: ExpBar) -> Option<Total> {
        // `exp * HUNDRED_PERCENT / total` is within 0.5 of `percent`: doubled, to stay
        // whole.
        let doubled_exp = u128::from(bar.exp) * 2 * u128::from(HUNDRED_PERCENT);
        let percent = u128::from(bar.percent_hundredths);
        let lo = doubled_exp.div_ceil(2 * percent + 1).max(u128::from(bar.exp) + 1);
        let hi = if percent == 0 { u128::from(u64::MAX) } else { doubled_exp / (2 * percent - 1) };
        (lo <= hi).then(|| Total { lo: lo as u64, hi: hi.min(u128::from(u64::MAX)) as u64 })
    }

    /// The totals both allow, if any.
    fn meet(self, other: Total) -> Option<Total> {
        let t = Total { lo: self.lo.max(other.lo), hi: self.hi.min(other.hi) };
        (t.lo <= t.hi).then_some(t)
    }

    /// Twice the best estimate of the total, the middle of the range, so that it stays
    /// whole; `None` while unbounded.
    fn doubled_midpoint(self) -> Option<u64> {
        (self.hi != u64::MAX).then(|| self.lo + self.hi)
    }

    /// As a Bug Report writes it down.
    fn level(self) -> Level {
        Level { lo: self.lo, hi: (self.hi != u64::MAX).then_some(self.hi) }
    }

    /// Whether `other`, which shares no total with this, is within [`RELEARN_PERCENT`]
    /// of it.
    fn close_to(self, other: Total) -> bool {
        let (lower, upper) = if other.lo > self.hi { (self, other) } else { (other, self) };
        u128::from(upper.lo) * 100 <= u128::from(lower.hi) * (100 + RELEARN_PERCENT)
    }
}

/// Readings in a row that don't fit the level learned.
#[derive(Debug, Clone, Copy)]
struct Misfits {
    /// The totals they all allow.
    total: Total,
    /// How many there are.
    count: u32,
    /// The bar was out of sight between the last reading taken and the first of them,
    /// as it is when another character's readings come.
    after_out_of_sight: bool,
}

/// The Session and what it has learned about the level from the EXP bar.
#[derive(Debug, Default)]
pub(crate) struct Session {
    /// The level's total, learned from the readings accepted in this level.
    total: Option<Total>,
    /// The last reading accepted.
    last: Option<ExpBar>,
    /// The readings rejected since the last one accepted, which may be another
    /// character's level.
    misfits: Option<Misfits>,
    /// The bar has been out of sight since the last reading accepted: a frame didn't
    /// read whole, or none came for [`OUT_OF_SIGHT_MS`].
    out_of_sight: bool,
    /// The largest gain between two readings in this Session, in hundredths of a
    /// percent: how near 100% the bar must have been for a level-up.
    largest_gain_hundredths: u32,
    /// The latest time seen, from a reading or a tick.
    now: u64,
    /// The figures since the Session started; `None` while waiting.
    figures: Option<Figures>,
    /// Level-ups this Session.
    level_ups: u32,
    /// The bar's percentage before the last death in this level, until it reads that
    /// much again. It is the bar's, not the Session's: a reset leaves it.
    lost_from: Option<u32>,
    /// The Visit (its id) on which the Best's pace came within reach: the Pace Trace
    /// shows it for the rest of that Visit.
    best_in_reach: Option<u64>,
    /// This Session started on a character switch, not the player's reset.
    character_switched: bool,
    /// What the panel shows, and what it was measured for; `None` until it is first
    /// measured, and after a reset.
    shown: Option<Shown>,
    /// When the EXP bar last read whole; `None` before it ever has.
    read_at: Option<u64>,
    /// The first frame since then that didn't read whole; `None` while the bar reads.
    unread_since: Option<u64>,
    /// The Session the player's reset ended, until the EXP next changes.
    undo: Option<Undo>,
    /// The Visit under way. It outlasts Sessions: a reset or a character switch doesn't
    /// move the player.
    visit: Visit,
    /// Every map's Personal Bests, which outlast Sessions too.
    bests: Bests,
    /// What it made of the readings since a Bug Report last took them.
    notes: Vec<Note>,
    /// Who the Session is for, as the Status Bar names them: `None` until a name is read,
    /// and after a switch to a Character whose name wasn't read.
    character: Option<Character>,
    /// The Status Bar has shown the Character's name since it was last out of sight: another
    /// name read now is a misread, and readings that fit another level aren't a switch.
    named: bool,
    /// The Status Bar has been out of sight since its level was last read: another level
    /// read now, with no name, is another Character.
    level_unseen: bool,
    /// When a Status Bar capture was last taken in, found or not.
    status_bar_at: Option<u64>,
    /// The level the level's total is learned in, as the Status Bar showed it (one more after
    /// a level-up the EXP bar showed first); `None` until it is read, and after a switch.
    level: Option<u32>,
    /// The Status Bar showed the next level, in a frame captured then, before the EXP bar
    /// did: the next reading captured since is the new level's.
    level_up_at: Option<u64>,
}

/// A Character (GLOSSARY): who is playing, as the Status Bar names them. The name is the
/// Character; the job and the level are as the Status Bar last showed them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub name: String,
    pub job: Option<String>,
    pub level: Option<u32>,
}

/// A Visit (GLOSSARY): from the panel confirming a map until it confirms another.
#[derive(Debug, Default, Clone, Copy)]
struct Visit {
    /// Which Visit it is, to tell it from the others: one more with each map the panel
    /// confirms, 0 before the first.
    id: u64,
    /// When it started: the latest capture time seen then.
    at: u64,
}

/// What the player's reset set aside, to bring back if they undo it.
#[derive(Debug)]
struct Undo {
    figures: Figures,
    largest_gain_hundredths: u32,
    level_ups: u32,
    character_switched: bool,
    /// The Visit then (its id): on another map since, the share counts from arriving there.
    visit: u64,
}

/// The figures the panel shows, held for the rest of the [`REFRESH_MS`] stretch they
/// were measured in.
#[derive(Debug)]
struct Shown {
    /// Which stretch of capture time they were measured in.
    stretch: u64,
    against: Against,
    /// The Session's level-ups by then.
    level_ups: u32,
    tracking: ExpTracking,
}

/// Where figures were measured, and against what: on another Visit, or against another
/// ceiling or Personal Best, they are measured afresh.
#[derive(Debug, PartialEq)]
struct Against {
    /// The Visit (its id).
    visit: u64,
    /// The shown map's solo ceiling.
    ceiling: Option<u64>,
    /// The map whose Personal Best the Best row shows.
    map: Option<String>,
    /// The Character whose Personal Best it is: the Unassigned one for `None`.
    character: Option<String>,
}

/// The Best row: the map's Personal Best as it stands, in EXP/h, and whether this Visit set it.
#[derive(Debug, Clone, Copy)]
struct BestRow {
    exp_per_hour: u64,
    this_visit: bool,
}

/// Where a started Session stood at an EXP change.
#[derive(Debug, Clone, Copy)]
struct Mark {
    /// Active time then.
    active: u64,
    /// Net EXP gained by then.
    gained: i64,
}

/// Net EXP gained over a stretch of active time.
#[derive(Debug, Clone, Copy)]
struct Gained {
    /// The net EXP: negative if deaths cost more than was gained.
    exp: i64,
    /// The active time it took.
    over_ms: u64,
}

impl Gained {
    /// The EXP per hour as `parts` for every `whole` EXP, rounded: EXP/h with `(1, 1)`,
    /// hundredths of a percent of a ceiling `c` with `(HUNDRED_PERCENT, c)`. `over_ms`
    /// and `whole` are more than 0.
    fn per_hour(self, parts: u64, whole: u64) -> i64 {
        let scaled = i128::from(self.exp) * i128::from(HOUR_MS) * i128::from(parts);
        div_round(scaled, i128::from(self.over_ms) * i128::from(whole)) as i64
    }

    /// As an [`ExpRate`] on a level whose doubled midpoint is `doubled_total` (if
    /// known); `None` without any active time.
    fn rate(self, doubled_total: Option<u64>) -> Option<ExpRate> {
        (self.over_ms > 0).then(|| ExpRate {
            exp_per_hour: self.per_hour(1, 1),
            // The total is doubled, so the parts are too.
            percent_per_hour_hundredths: doubled_total
                .map(|t| self.per_hour(2 * u64::from(HUNDRED_PERCENT), t)),
        })
    }
}

/// A started Session's figures.
#[derive(Debug)]
struct Figures {
    /// When the EXP last changed.
    last_change_at: u64,
    /// Active time up to `last_change_at`.
    active_at_change: u64,
    /// Net EXP gained since the start.
    gained: i64,
    /// The start and each change since, back to the last one before the Pace Trace's
    /// earlier 5 minutes, which come before the Rolling Rate's window.
    marks: Vec<Mark>,
    /// The deaths so far.
    deaths: Deaths,
    /// The Session has had [`WARM_UP_MS`] of active time at some point. A break later
    /// takes active time back, but not this.
    warmed_up: bool,
    /// Active time when the Visit under way started: the share of the map's ceiling, and
    /// the Visit's Personal Best, count only the play since.
    visit_from: u64,
    /// Active time from which the Visit's Personal Best counts: when the Visit started, or
    /// when the player last removed this Visit's best, if later, so the play that set it
    /// isn't counted again.
    best_from: u64,
}

impl Figures {
    /// A Session started at `at`.
    fn new(at: u64) -> Self {
        Figures {
            last_change_at: at,
            active_at_change: 0,
            gained: 0,
            marks: vec![Mark { active: 0, gained: 0 }],
            deaths: Deaths::default(),
            warmed_up: false,
            visit_from: 0,
            best_from: 0,
        }
    }

    /// A Visit started at active time `at`: its share and its best count from then.
    fn start_visit(&mut self, at: u64) {
        self.visit_from = at;
        self.best_from = at;
    }

    /// Whether the Session has warmed up by `now`.
    fn warm(&self, now: u64) -> bool {
        self.warmed_up || self.active(now) >= WARM_UP_MS
    }

    /// How long the EXP has gone unchanged at `now`.
    fn idle(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_change_at)
    }

    /// Whether the player is on a break at `now`.
    fn paused(&self, now: u64) -> bool {
        self.idle(now) > IDLE_MS
    }

    /// Active time at `now`: the idle stretch since the last change counts while it is
    /// a gap in play, and not at all once it is a break.
    fn active(&self, now: u64) -> u64 {
        self.active_at_change + if self.paused(now) { 0 } else { self.idle(now) }
    }

    /// Count a change of `gain` EXP at `at`.
    fn change(&mut self, gain: i64, at: u64) {
        self.active_at_change = self.active(at);
        self.last_change_at = at;
        self.gained += gain;
        self.marks.push(Mark { active: self.active_at_change, gained: self.gained });
        let from = self.active_at_change.saturating_sub(2 * ROLLING_MS);
        let before = self.marks.iter().rposition(|m| m.active <= from).unwrap_or(0);
        self.marks.drain(..before);
    }

    /// The net EXP gained over the Rolling Rate's window at `now`.
    fn rolling(&self, now: u64) -> Gained {
        self.rolling_since(now, 0)
    }

    /// The net EXP gained over the Rolling Rate's window at `now`, or over the part of
    /// it since active time `since`, if that is later.
    fn rolling_since(&self, now: u64, since: u64) -> Gained {
        let active = self.active(now);
        let from = active.saturating_sub(ROLLING_MS).max(since).min(active);
        let then = self.marks.iter().rev().find(|m| m.active <= from).map_or(0, |m| m.gained);
        Gained { exp: self.gained - then, over_ms: active - from }
    }

    /// The net EXP gained over the whole Session at `now`.
    fn average(&self, now: u64) -> Gained {
        Gained { exp: self.gained, over_ms: self.active(now) }
    }
}

impl Session {
    /// Take a whole EXP bar reading captured at `at_ms`. After frames that didn't read
    /// whole, for longer than [`IDLE_MS`] since the last that did, a reading of the same
    /// level, or of the next, is taken as where the Session stands now, without counting
    /// the change since the last one: no one saw how it came about. One that fits neither
    /// is weighed as ever, towards a misread or a character switch.
    pub fn on_bar(&mut self, bar: ExpBar, at_ms: u64) {
        self.tick(at_ms);
        let unseen = self.unread_since.is_some()
            && self.read_at.is_some_and(|read_at| at_ms.saturating_sub(read_at) > IDLE_MS);
        if self.read_at.is_some_and(|read_at| at_ms.saturating_sub(read_at) > OUT_OF_SIGHT_MS) {
            self.out_of_sight = true;
        }
        // Even a reading no level allows (a misread) shows the bar is there to read.
        self.read_at = Some(at_ms);
        self.unread_since = None;
        let Some(allowed) = Total::of(bar) else {
            self.notes.push(Note::NoLevelAllows);
            return;
        };
        let level_up_shown = self.level_up_at.is_some_and(|at| at_ms >= at);
        let (Some(total), Some(last)) = (self.total, self.last) else {
            if level_up_shown {
                self.level_up_at = None;
            }
            self.accept(allowed, bar);
            return;
        };
        if level_up_shown {
            // The Status Bar showed the next level: this reading is in it, though it may fit
            // the level before (2 [0.04%] allows a wide range of totals), or the bar was
            // nowhere near 100% when last read (it went unread through the level-up).
            self.level_up_at = None;
            if bar.exp < last.exp || total.meet(allowed).is_none() {
                self.level_up(total, last, bar, allowed, unseen, at_ms);
            } else {
                // More EXP than before, as the level before could hold: what came about
                // can't be told, so it isn't counted, and the level is learned afresh.
                self.notes.push(Note::LevelUpUncounted);
                self.accept(allowed, bar);
            }
            return;
        }
        let wrapped = self.wrapped(total, last, bar, allowed);
        if !wrapped && let Some(narrowed) = total.meet(allowed) {
            if unseen {
                self.notes.push(Note::Unseen);
                self.accept(narrowed, bar);
            } else {
                self.same_level(narrowed, last, bar, at_ms);
            }
        } else if total.doubled_midpoint().is_some() && (wrapped || self.levelled_up(total, last, bar, allowed)) {
            self.level_up(total, last, bar, allowed, unseen, at_ms);
            // Ahead of the Status Bar, which shows it within a capture or two, unless it
            // showed it already in a frame captured after this one.
            if self.level_up_at.take().is_none() {
                self.level = self.level.map(|level| level + 1);
            }
        } else {
            // A misread, or another character's level: that takes readings in a row
            // that fit it, after the bar was out of sight.
            let more = self.misfits.and_then(|m| Some(Misfits { total: m.total.meet(allowed)?, count: m.count + 1, ..m }));
            let after_out_of_sight = self.out_of_sight;
            let misfits = more.unwrap_or(Misfits { total: allowed, count: 1, after_out_of_sight });
            self.misfits = Some(misfits);
            if misfits.count < SWITCH_READINGS {
                // Not yet.
                self.notes.push(Note::Misfit { count: misfits.count, level: total.level() });
            } else if total.close_to(misfits.total) && carries_on(last, bar, self.near(), misfits.total) {
                // The level was learned wrong: the Session goes on.
                self.notes.push(Note::Relearned { from: total.level(), to: misfits.total.level() });
                self.same_level(misfits.total, last, bar, at_ms);
            } else if !misfits.after_out_of_sight {
                // The bar read all along: one misread, repeated.
                self.notes.push(Note::Misfit { count: misfits.count, level: total.level() });
            } else if self.named {
                // The Status Bar names the same Character: the level was learned wrong, and
                // what the EXP did while the bar was out of sight isn't known.
                self.notes.push(Note::Relearned { from: total.level(), to: misfits.total.level() });
                self.accept(misfits.total, bar);
            } else {
                // No name read: the readings alone tell another Character.
                self.notes.push(Note::CharacterSwitch { level: misfits.total.level() });
                self.switch(None);
                self.accept(misfits.total, bar);
            }
        }
    }

    /// Take `bar`, the first reading in the level after `last`'s (learned as `total`), which
    /// allows the totals `allowed`: what was left of the old level and what is in the new one
    /// are counted, unless the bar went `unseen`, or the old level's total isn't known.
    fn level_up(&mut self, total: Total, last: ExpBar, bar: ExpBar, allowed: Total, unseen: bool, at_ms: u64) {
        // The new level is bigger than the old: of the totals the reading allows, only those
        // count, if any do.
        let bigger = Total { lo: allowed.lo.max(total.hi.saturating_add(1)), ..allowed };
        self.accept(if bigger.lo <= bigger.hi { bigger } else { allowed }, bar);
        self.lost_from = None;
        self.level_ups += 1;
        let Some(doubled_total) = total.doubled_midpoint() else {
            self.notes.push(Note::LevelUpUncounted);
            return;
        };
        let gain = (doubled_total as i64 - 2 * last.exp as i64 + 1) / 2 + bar.exp as i64;
        self.notes.push(Note::LevelUp { gain, level: total.level() });
        if unseen {
            self.notes.push(Note::Unseen);
        } else {
            self.change(gain, at_ms);
        }
    }

    /// A Status Bar capture taken at `at_ms` found `bar`, or none (the character select
    /// screen, say). A name other than the Character's, after the bar was out of sight, is
    /// another Character: a new Session at once. With no name read, so is another level than
    /// the one before, other than the next. The next level is a level-up the next EXP bar
    /// reading is in; any other is learned afresh from it.
    pub fn on_status_bar(&mut self, bar: Option<&StatusBar>, at_ms: u64) {
        self.tick(at_ms);
        // No capture for a while is the bar out of sight, as with the EXP bar.
        if self.status_bar_at.is_some_and(|at| at_ms.saturating_sub(at) > OUT_OF_SIGHT_MS) {
            self.named = false;
            self.level_unseen = true;
        }
        self.status_bar_at = Some(at_ms);
        let Some(bar) = bar else {
            self.named = false;
            self.level_unseen = true;
            return;
        };
        match (&bar.name, &self.character) {
            (Some(name), Some(character)) if *name == character.name => self.named = true,
            (Some(name), Some(_)) if self.named => {
                // Two names on a bar that never went out of sight: one is a misread.
                self.notes.push(Note::NameMisread { name: name.clone() });
                return;
            }
            (Some(name), Some(_)) => {
                self.notes.push(Note::NameSwitch { name: name.clone() });
                self.switch(Some(name.clone()));
            }
            (Some(name), None) => {
                self.character = Some(Character { name: name.clone(), job: None, level: None });
                self.named = true;
            }
            (None, _) => {}
        }
        if let Some(level) = bar.level {
            match self.level {
                Some(known) if known == level => {}
                Some(known) if level == known + 1 => {
                    self.notes.push(Note::LevelShown { level });
                    self.level_up_at = Some(at_ms);
                }
                Some(_) if bar.name.is_none() && self.level_unseen => {
                    self.notes.push(Note::LevelSwitch { level });
                    self.switch(None);
                }
                Some(known) => {
                    // A level the bar couldn't show next: learned afresh from the next reading,
                    // and the change since isn't counted.
                    self.notes.push(Note::LevelChanged { from: known, to: level });
                    self.total = None;
                    self.misfits = None;
                    self.level_up_at = None;
                }
                None => {}
            }
            self.level = Some(level);
            self.level_unseen = false;
        }
        if self.named
            && let Some(character) = &mut self.character
        {
            character.job = bar.job.clone().or(character.job.take());
            character.level = bar.level.or(character.level);
        }
    }

    /// Another Character, named `name` if the Status Bar showed it: a new Session, with
    /// nothing learned of the level.
    fn switch(&mut self, name: Option<String>) {
        self.reset();
        self.lost_from = None;
        self.character_switched = true;
        self.total = None;
        self.last = None;
        self.level = None;
        self.level_up_at = None;
        self.named = name.is_some();
        self.character = name.map(|name| Character { name, job: None, level: None });
    }

    /// Who the Session is for, as the Status Bar named them.
    pub fn character(&self) -> Option<&Character> {
        self.character.as_ref()
    }

    /// The name of who the Session is for: whose bests are raced (`None`: the Unassigned ones).
    fn character_name(&self) -> Option<&str> {
        self.character.as_ref().map(|c| c.name.as_str())
    }

    /// Who a best set now is stamped with: the Character the Status Bar named, and the level
    /// and job it showed. With no Character named, it is Unassigned, with the level if one
    /// was read.
    fn stamp(&self) -> Stamp {
        match &self.character {
            Some(character) => Stamp {
                character: Some(character.name.clone()),
                level: character.level.or(self.level),
                job: character.job.clone(),
            },
            None => Stamp { character: None, level: self.level, job: None },
        }
    }

    /// Take `bar`, a reading after `last` in the same level, whose total it narrows to
    /// `total`: a gain, or with less EXP, a death.
    fn same_level(&mut self, total: Total, last: ExpBar, bar: ExpBar, at_ms: u64) {
        self.accept(total, bar);
        if bar.percent_hundredths > last.percent_hundredths {
            let gain = bar.percent_hundredths - last.percent_hundredths;
            self.largest_gain_hundredths = self.largest_gain_hundredths.max(gain);
        }
        if let Some(figures) = self.change(bar.exp as i64 - last.exp as i64, at_ms)
            && bar.exp < last.exp
        {
            // Less EXP in the same level: a death.
            figures.deaths.count += 1;
            figures.deaths.exp_lost += last.exp - bar.exp;
            figures.deaths.percent_lost_hundredths += last.percent_hundredths.saturating_sub(bar.percent_hundredths);
            self.notes.push(Note::Death { exp_lost: last.exp - bar.exp });
            // Where the bar was: after deaths in a row, the highest it fell from.
            self.lost_from = Some(self.lost_from.map_or(last.percent_hundredths, |p| p.max(last.percent_hundredths)));
        }
    }

    /// A reading `bar` that doesn't fit the level learned (`total`), and allows the totals
    /// `allowed`, is the next level's after `last`: its total is higher, and the bar fell
    /// from near 100%, both its EXP and its percentage. "Near" is within 10 points, or
    /// twice the largest gain seen in the Session if that is more. One figure misread
    /// near the end of a level can make a higher total, but the other, read right,
    /// hasn't fallen.
    fn levelled_up(&self, total: Total, last: ExpBar, bar: ExpBar, allowed: Total) -> bool {
        last.percent_hundredths + self.near() >= HUNDRED_PERCENT
            && bar.exp < last.exp
            && bar.percent_hundredths < last.percent_hundredths
            && allowed.lo > total.hi
    }

    /// A reading `bar` barely into the next level after `last`: the bar was near 100%
    /// and now is as near 0%, with less EXP, and it allows a bigger total than the level
    /// learned (`total`). It may allow the old level's total too (2 [0.01%] allows
    /// 13,334 to 40,000 EXP), but a death costs far less than a whole level.
    fn wrapped(&self, total: Total, last: ExpBar, bar: ExpBar, allowed: Total) -> bool {
        let near = self.near();
        last.percent_hundredths + near >= HUNDRED_PERCENT
            && bar.percent_hundredths <= near
            && bar.exp < last.exp
            && allowed.hi > total.hi
    }

    /// How near 100% (or 0%) the bar must be for a level-up: within 10 points, or twice
    /// the largest gain seen in the Session if that is more.
    fn near(&self) -> u32 {
        (2 * self.largest_gain_hundredths).max(NEAR_LEVEL_UP_HUNDREDTHS)
    }

    /// Take `bar` as the last reading, in a level of `total`.
    fn accept(&mut self, total: Total, bar: ExpBar) {
        self.total = Some(total);
        self.last = Some(bar);
        self.misfits = None;
        self.out_of_sight = false;
        // The ground a death took back, made good.
        if self.lost_from.is_some_and(|p| bar.percent_hundredths >= p) {
            self.lost_from = None;
        }
    }

    /// Back to waiting for an EXP change; what was learned about the level stays, but
    /// readings rejected so far no longer count towards a character switch. Nothing is
    /// left to undo.
    pub fn reset(&mut self) {
        self.figures = None;
        self.misfits = None;
        self.largest_gain_hundredths = 0;
        self.level_ups = 0;
        self.character_switched = false;
        self.shown = None;
        self.undo = None;
    }

    /// The Session has started: it has figures, since the first EXP gain.
    pub fn running(&self) -> bool {
        self.figures.is_some()
    }

    /// The player's reset: as [`Session::reset`], but the Session it ends is set aside
    /// until the EXP next changes, for [`Session::undo_reset`]. Pressed again while
    /// waiting, the one set aside stays.
    pub fn reset_by_player(&mut self) {
        let undo = match self.figures.take() {
            Some(figures) => Some(Undo {
                figures,
                largest_gain_hundredths: self.largest_gain_hundredths,
                level_ups: self.level_ups,
                character_switched: self.character_switched,
                visit: self.visit.id,
            }),
            None => self.undo.take(),
        };
        self.reset();
        self.undo = undo;
    }

    /// Bring back the Session the player's reset ended, if the EXP hasn't changed since.
    /// The time since counts as it would have: a gap in play, or a break.
    pub fn undo_reset(&mut self) {
        let Some(undo) = self.undo.take() else { return };
        let mut figures = undo.figures;
        if undo.visit != self.visit.id {
            // Reset on another map: the EXP hasn't changed since, so the time from then
            // to arriving here counts as it would have.
            figures.start_visit(figures.active(self.visit.at));
        }
        self.figures = Some(figures);
        self.largest_gain_hundredths = undo.largest_gain_hundredths;
        self.level_ups = undo.level_ups;
        self.character_switched = undo.character_switched;
        self.misfits = None;
        self.shown = None;
    }

    /// The player removed `map`'s Best row: this Visit's best if the row is it, otherwise the
    /// best from its history. What is shown is measured afresh against the next best, at once.
    pub fn remove_best(&mut self, map: &str) {
        let character = self.character_name().map(String::from);
        if self.best_row(map).is_some_and(|row| row.this_visit) {
            self.remove_this_visit(character.as_deref(), map);
        } else {
            self.bests.remove(character.as_deref(), map);
        }
        self.shown = None;
    }

    /// The player removed the Visit whose best was `visit` from `character`'s Visits on `map`
    /// (the Unassigned one for `None`), this Visit's included: if it was there, what is
    /// shown is measured afresh, at once, as the Best row may name another.
    pub fn remove_visit(&mut self, character: Option<&str>, map: &str, visit: &VisitBest) {
        let removed = if self.bests.this_visits_best(character, map) == Some(visit) {
            self.remove_this_visit(character, map)
        } else {
            self.bests.remove_visit(character, map, visit)
        };
        if removed {
            self.shown = None;
        }
    }

    /// Remove this Visit's best on `map`, if it is `character`'s: only the play from now on
    /// counts towards it again, so what set it (a misread, say) isn't counted again. So too
    /// for the Session a reset set aside, should the player undo it. Whether there was one.
    fn remove_this_visit(&mut self, character: Option<&str>, map: &str) -> bool {
        let removed = self.bests.remove_this_visit(character, map);
        if removed {
            let now = self.now;
            let undone = self.undo.as_mut().map(|undo| &mut undo.figures);
            for figures in self.figures.iter_mut().chain(undone) {
                figures.best_from = figures.active(now);
            }
        }
        removed
    }

    /// The player moved the Visit whose best was `visit` on `map`, of the Character named
    /// `from` (the Unassigned one for `None`), to the Character named `to`: if it was there,
    /// what is shown is measured afresh, at once.
    pub fn move_visit(&mut self, from: Option<&str>, map: &str, visit: &VisitBest, to: &str) {
        if self.bests.move_visit(from, map, visit, to) {
            self.shown = None;
        }
    }

    /// The player gave the Character named `to` every Unassigned Visit: if there was one,
    /// what is shown is measured afresh, at once.
    pub fn assign_unassigned(&mut self, to: &str) {
        if self.bests.assign_unassigned(to) {
            self.shown = None;
        }
    }

    /// The player removed the Character named `name`, with their bests: if they had any,
    /// what is shown is measured afresh, at once.
    pub fn remove_character(&mut self, name: &str) {
        if self.bests.remove_character(name) {
            self.shown = None;
        }
    }

    /// The player merged the Character named `from` into the one named `into`: if `from`
    /// had bests, what is shown is measured afresh, at once.
    pub fn merge_characters(&mut self, from: &str, into: &str) {
        if self.bests.merge(from, into) {
            self.shown = None;
        }
    }

    /// The panel confirmed another map: a new Visit starts, as of the latest time seen.
    /// The share of the map's ceiling counts only the play since.
    pub fn arrive(&mut self) {
        let now = self.now;
        self.notes.push(Note::Visit);
        self.visit = Visit { id: self.visit.id + 1, at: now };
        self.bests.end_visit();
        if let Some(figures) = &mut self.figures {
            figures.start_visit(figures.active(now));
        }
    }

    /// What it made of the readings since this was last asked, for a Bug Report.
    pub fn take_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    /// Every map's Personal Bests.
    pub fn bests(&self) -> &Bests {
        &self.bests
    }

    /// Take back the Personal Bests an earlier launch saved.
    pub fn restore_bests(&mut self, saved: SavedBests) {
        self.bests.restore(saved);
    }

    /// Time passed without a reading.
    pub fn tick(&mut self, at_ms: u64) {
        self.now = self.now.max(at_ms);
        if let Some(figures) = &mut self.figures {
            figures.warmed_up = figures.warm(self.now);
        }
    }

    /// A frame of the EXP bar, captured at `at_ms`, whose figures didn't read whole.
    pub fn on_unread(&mut self, at_ms: u64) {
        self.tick(at_ms);
        self.unread_since.get_or_insert(at_ms);
        self.out_of_sight = true;
    }

    /// Whether the bar has gone [`UNREADABLE_MS`] without reading whole.
    fn unreadable(&self) -> bool {
        self.unread_since.is_some_and(|since| self.now.saturating_sub(since) >= UNREADABLE_MS)
    }

    /// The EXP changed by `gain` at `at`: the Session starts (from the EXP the bar shows
    /// now), or counts it. The Session's figures, if the EXP changed.
    fn change(&mut self, gain: i64, at: u64) -> Option<&mut Figures> {
        if gain == 0 {
            return None;
        }
        match &mut self.figures {
            Some(figures) => figures.change(gain, at),
            None => {
                // A new Session: the one a reset ended is gone for good.
                self.notes.push(Note::SessionStarted);
                self.undo = None;
                self.figures = Some(Figures::new(at));
            }
        }
        self.figures.as_mut()
    }

    /// What the panel shows, with tracking on, on a map whose solo EXP/h ceiling is
    /// `ceiling` (if it is known), and which is `map` (its 9-digit ID) if Personal Bests
    /// are kept there: the figures measured earlier in this [`REFRESH_MS`] stretch, unless
    /// an event since asks for them afresh; and the bar's last reading. A best is dated
    /// by `now_unix_ms`, the wall clock, if the shell has told it.
    pub fn tracking(&mut self, ceiling: Option<u64>, map: Option<&str>, now_unix_ms: Option<u64>) -> ExpTracking {
        let mut fresh = self.measure(ceiling);
        let stretch = self.now / REFRESH_MS;
        // While the bar can't be read nothing new is known: the figures shown stay as
        // they were, unless the player arrives on another map or the share would be of
        // another ceiling or best (the map's page come, or another Candidate chosen).
        let unreadable = fresh.state == TrackingState::Unreadable;
        let character = self.character_name().map(String::from);
        let against = Against { visit: self.visit.id, ceiling, map: map.map(String::from), character };
        let holds = self.shown.as_ref().is_some_and(|shown| {
            shown.against == against
                && (unreadable
                    || shown.stretch == stretch
                        && shown.level_ups == self.level_ups
                        && shown.tracking.state == fresh.state
                        && shown.tracking.deaths == fresh.deaths
                        && shown.tracking.character_switched == fresh.character_switched
                        && shown.tracking.reset_undoable == fresh.reset_undoable
                        // The first figures show as soon as there are any, and the first
                        // share.
                        && shown.tracking.rolling.is_some() == fresh.rolling.is_some()
                        && shown.tracking.ceiling_share_hundredths.is_some()
                            == fresh.ceiling_share_hundredths.is_some())
        });
        match &mut self.shown {
            Some(shown) if holds => shown.tracking.state = fresh.state,
            _ => {
                if let Some(map) = map {
                    // The Rolling Rate shown over a full 5 minutes of active play here
                    // counts towards this Visit's best, unless nothing is known of the EXP.
                    let here = self.figures.as_ref().map(|f| f.rolling_since(self.now, f.best_from));
                    let full = here.filter(|h| h.over_ms >= ROLLING_MS && h.exp > 0);
                    let counted = full.filter(|_| !unreadable).map(|here| here.per_hour(1, 1) as u64);
                    self.bests.on_shown(map, counted, now_unix_ms, &self.stamp());
                    // Over a full 5 minutes here, the Rolling Rate shown is the one counted,
                    // against the Character's best (the Unassigned one while none is named).
                    let row = self.best_row(map);
                    let best = row.map(|row| row.exp_per_hour);
                    fresh.personal_best = best;
                    fresh.personal_best_this_visit = row.is_some_and(|row| row.this_visit);
                    // The star: Last 5 min is the best this Visit set on the row, as written.
                    fresh.new_best = fresh.personal_best_this_visit
                        && counted.zip(best).is_some_and(|(rate, best)| three_figures(rate) == three_figures(best));
                }
                self.shown = Some(Shown { stretch, against, level_ups: self.level_ups, tracking: fresh });
            }
        }
        let held = self.shown.as_ref().map(|shown| shown.tracking.clone()).unwrap_or_default();
        // The bar, the race and what lies ahead move with the EXP; the figures hold.
        ExpTracking {
            bar: self.last,
            pace: self.pace(held.personal_best),
            forecast: forecast(self.last, held.rolling),
            lost_from_hundredths: self.lost_from.filter(|&p| self.last.is_some_and(|bar| bar.percent_hundredths < p)),
            ..held
        }
    }

    /// `map`'s Best row for the Character the Session is for (the Unassigned one while none
    /// is named), if the map has a best. This Visit's takes it from the history's only once
    /// it reads higher, as the Grind Tracker writes them.
    fn best_row(&self, map: &str) -> Option<BestRow> {
        let character = self.character_name();
        let earlier = self.bests.history_best(character, map).map(|best| best.exp_per_hour);
        let this_visit = self.bests.this_visits_best(character, map).map(|best| best.exp_per_hour);
        let row = |exp_per_hour, this_visit| BestRow { exp_per_hour, this_visit };
        match (earlier, this_visit) {
            (Some(earlier), Some(visits)) if three_figures(visits) > three_figures(earlier) => Some(row(visits, true)),
            (Some(earlier), _) => Some(row(earlier, false)),
            (None, visits) => visits.map(|visits| row(visits, true)),
        }
    }

    /// The Pace Trace up to the last EXP change, against the 5 minutes before and the Best
    /// row's `best` EXP/h; `None` without a minute of play before the 5 minutes.
    fn pace(&mut self, best: Option<u64>) -> Option<PaceTrace> {
        let figures = self.figures.as_ref()?;
        let end = figures.active_at_change;
        let from = end.saturating_sub(ROLLING_MS);
        let before = from.saturating_sub(ROLLING_MS);
        if from - before < WARM_UP_MS {
            return None;
        }
        // The EXP gained by active time `at`, as the last change by then left it. The
        // marks reach back to the last one before `before`.
        let gained = |at: u64| figures.marks.iter().rev().find(|m| m.active <= at).map_or(0, |m| m.gained);
        let base = gained(from);
        let point = |at: u64, gained: i64| PacePoint { ago_ms: end - at, exp: gained - base };
        let points: Vec<PacePoint> = std::iter::once(point(from, base))
            .chain(figures.marks.iter().filter(|m| m.active > from).map(|m| point(m.active, m.gained)))
            .collect();
        let at_earlier = div_round(i128::from(base - gained(before)) * i128::from(end - from), i128::from(from - before));
        let earlier = [point(from, base), PacePoint { ago_ms: 0, exp: at_earlier as i64 }];
        // The Best counts only the play on the map, so its race starts on arriving.
        let race_from = from.max(figures.visit_from);
        let race = best.filter(|_| race_from < end).map(|best| {
            let at_best = div_round(i128::from(best) * i128::from(end - race_from), i128::from(HOUR_MS)) as i64;
            let start = gained(race_from);
            [point(race_from, start), point(end, start + at_best)]
        });
        let arrived_ago_ms = (figures.visit_from > from).then(|| end.saturating_sub(figures.visit_from));
        let gained_since = |p: PacePoint| points.last().map_or(0, |last| last.exp - p.exp);
        let in_reach = race.is_some_and(|[start, at_best]| {
            end - race_from >= WARM_UP_MS && gained_since(start) * 100 >= IN_REACH_PERCENT * (at_best.exp - start.exp)
        });
        if in_reach {
            self.best_in_reach = Some(self.visit.id);
        }
        let best = race.filter(|_| self.best_in_reach == Some(self.visit.id));
        Some(PaceTrace { span_ms: ROLLING_MS, points, earlier, best, arrived_ago_ms })
    }

    /// The figures as they stand now.
    fn measure(&self, ceiling: Option<u64>) -> ExpTracking {
        let unreadable = self.unreadable();
        let Some(figures) = &self.figures else {
            return ExpTracking {
                state: if unreadable { TrackingState::Unreadable } else { TrackingState::Waiting },
                character_switched: self.character_switched,
                reset_undoable: self.undo.is_some(),
                ..ExpTracking::default()
            };
        };
        let now = self.now;
        let warm = figures.warm(now);
        let state = if unreadable {
            TrackingState::Unreadable
        } else if figures.paused(now) {
            TrackingState::Paused
        } else if warm {
            TrackingState::Active
        } else {
            TrackingState::WarmingUp
        };
        let doubled_total = self.total.and_then(Total::doubled_midpoint);
        let rolling = figures.rolling(now);
        let gaining = rolling.exp > 0 && rolling.over_ms > 0;
        let time_to_level_s = match (doubled_total, self.last) {
            (Some(t), Some(last)) if gaining => {
                let doubled_to_go = i128::from(t.saturating_sub(2 * last.exp));
                let doubled_ms = doubled_to_go * i128::from(rolling.over_ms);
                Some(rounded_time_to_level(div_round(doubled_ms, 2 * i128::from(rolling.exp) * 1000) as u64))
            }
            _ => None,
        };
        // The share counts only the play on the shown map, once there is enough of it.
        let here = figures.rolling_since(now, figures.visit_from);
        let ceiling = ceiling.filter(|&c| c > 0 && here.exp > 0 && here.over_ms >= WARM_UP_MS);
        let average = figures.average(now);
        ExpTracking {
            state,
            time_to_level_s,
            time_to_level_badge: time_to_level_s.map(badge),
            rolling: rolling.rate(doubled_total),
            session_average: average.rate(doubled_total),
            ceiling_share_hundredths: ceiling.map(|c| here.per_hour(u64::from(HUNDRED_PERCENT), c)),
            solo_exp_ceiling: ceiling,
            deaths: figures.deaths,
            character_switched: self.character_switched && !warm,
            active_s: average.over_ms / 1000,
            rolling_s: rolling.over_ms / 1000,
            reset_undoable: false,
            bar: None,
            personal_best: None,
            personal_best_this_visit: false,
            new_best: false,
            pace: None,
            forecast: Vec::new(),
            lost_from_hundredths: None,
            no_status_bar: false,
        }
    }
}

/// Where `bar` will be after each of [`FORECAST_S`] at the `rolling` rate's %/h, as long
/// as it falls short of the level-up. None without a gain to go on, or while the level's
/// total isn't known.
fn forecast(bar: Option<ExpBar>, rolling: Option<ExpRate>) -> Vec<Forecast> {
    let (Some(bar), Some(per_hour)) = (bar, rolling.and_then(|r| r.percent_per_hour_hundredths)) else {
        return Vec::new();
    };
    if per_hour <= 0 {
        return Vec::new();
    }
    FORECAST_S
        .iter()
        .map(|&after_s| (after_s, i128::from(bar.percent_hundredths) + div_round(i128::from(per_hour) * i128::from(after_s), 3_600)))
        .take_while(|&(_, percent)| percent < i128::from(HUNDRED_PERCENT))
        .map(|(after_s, percent)| Forecast { after_s, percent_hundredths: percent as u32 })
        .collect()
}

/// An EXP rate as the Grind Tracker writes it, to 3 significant figures, halves up:
/// 66,523 is 66,500, and 66,550 is 66,600. It must agree with `rate()` in
/// `src/grind-tracker.ts`, which writes Last 5 min and the Best row: this Visit's best
/// takes the row only once it reads higher than the earlier Visits', and the star shows
/// while Last 5 min reads the same as it
/// (`this_visits_best_takes_the_row_once_it_reads_higher_as_the_grind_tracker_writes_them`).
fn three_figures(exp_per_hour: u64) -> u64 {
    let mut step = 1;
    while exp_per_hour / step >= 1000 {
        step *= 10;
    }
    (exp_per_hour + step / 2) / step * step
}

/// Whether `bar`, which fits a level of `total`, carries on from `last` as the same
/// level could: the EXP moved no more than `near` points of it, either way. Not the
/// percentage: `bar`'s may be a misread repeated on still frames, which can even fall
/// as the EXP grows.
fn carries_on(last: ExpBar, bar: ExpBar, near: u32, total: Total) -> bool {
    u128::from(bar.exp.abs_diff(last.exp)) * u128::from(HUNDRED_PERCENT) <= u128::from(near) * u128::from(total.lo)
}

/// Time to Level rounded no finer than it can be trusted: to the minute under an hour,
/// to 5 minutes under 10 hours, and to the hour beyond.
fn rounded_time_to_level(seconds: u64) -> u64 {
    let step = if seconds < 3_600 {
        60
    } else if seconds < 36_000 {
        300
    } else {
        3_600
    };
    div_round(i128::from(seconds), step) as u64 * step as u64
}

/// Time to Level (already rounded) for the badge: "<1m", "47m", "3h 10m", "14h".
fn badge(seconds: u64) -> String {
    let minutes = seconds / 60;
    match (minutes / 60, minutes % 60) {
        (0, 0) => "<1m".into(),
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// `n / d` rounded to the nearest whole number, halves away from zero (`d` > 0).
fn div_round(n: i128, d: i128) -> i128 {
    if n >= 0 { (n + d / 2) / d } else { -((-n + d / 2) / d) }
}
