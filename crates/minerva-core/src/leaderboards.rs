//! Uploading to the Map Leaderboards (#110, ADR 0015): the player shows a Character there from
//! the Personal Bests window, and the core says what the site should hold of them. The shell
//! does the HTTP, with the Leaderboard Key it keeps (never the core: a Bug Report writes down
//! everything the core takes in), and tells the core each answer.
//!
//! - **Opting in** is per Character, never for the Unassigned bests. The player confirms with
//!   the count of maps there is something to upload for. Opting out deletes all of the
//!   Character's entries from the site. Which Characters are shown, and when each last upload
//!   went through, are kept with the settings.
//! - **What is uploaded**: each shown Character's whole set of entries, which replaces the set
//!   before it on the site, so a Visit removed, merged or moved in the window goes from the
//!   site too. An entry is the Character's best Visit of a kind (Solo Best or Party Best,
//!   ADR 0026) on a map in a Level Band, of those that have ended (the Visit under way's best
//!   goes up when it ends), with the level and job that set it and its date: a band's Solo Best
//!   and its Party Best are entries of their own (ADR 0015). A Visit with no level, job or date can't be an entry: the
//!   next best dated one in its band stands in. Nor can one dated over 23 hours ahead of the
//!   wall clock (the site refuses one over a day ahead of its own: see [`AHEAD_MS`]), until it isn't.
//! - **When**: once per shown Character at each launch (what the site holds may have changed
//!   since), then whenever their set changes, one upload at a time each. Only on a tick, so
//!   the bests are back and the wall clock known first.
//! - **The answers**: taken in, with the over-cap and removed entries listed, but neither is a
//!   failure; a name claimed by another key says so (and the player asks on Discord), and is
//!   tried again now and then and at each change, since the maintainer can only hand the name
//!   to a key it refused; blocked by the maintainer (ADR 0031) says so, and sends nothing more
//!   of theirs this launch, unless the player sends them again; rate-limited waits as long as the
//!   site says; anything else (offline, say) is tried again quietly, later each time.
//! - **Removing a Character**, or merging one into another, takes the one gone off the site.
//! - **Pulled first** (#206): since an upload replaces what the site holds, a Character whose
//!   bests here may be far fewer than the site's has the site's entries pulled (`GET`) before
//!   anything of theirs goes up: one held (below), or one never uploaded from this install (a new
//!   PC, or a Character met for the first time, whose name no key may hold yet). Each entry
//!   joins their Visits on its map as any Visit does, unless one there has its EXP/h and date
//!   already, and the toggle says how many maps had one come back. Then what is here goes up. A pull that fails is tried again as an upload is, and nothing of theirs goes up
//!   meanwhile.
//! - **Bests lost**: a launch whose bests file didn't read starts with none, and sending what is
//!   here then would take every shown Character's entries off the site. So each Character
//!   shown then is held, at that launch and any after (the file saved from then on holds only
//!   the bests set since), until their entries are pulled from the site and merged in. While
//!   that fails, the toggle says why, as a problem, and sending them again pulls at once.
//!   Turning them off and on sends what is here, deleting first.
//! - **Problems** (held with the pull failing, blocked, a name claimed, an upload tried again) the
//!   window says as problems, and the player can send them again at once rather than wait (held,
//!   that pulls at once).

use crate::personal_best::{LevelBand, SavedBests, VisitBest, band_bests};
use crate::SITE_URL;
use crate::status_bar::MAX_LEVEL;
use serde::{Deserialize, Serialize};

/// How long to wait after the first failed upload; each failure after doubles it, up to
/// [`MOST_RETRY_MS`].
const FIRST_RETRY_MS: u64 = 30_000;
const MOST_RETRY_MS: u64 = 30 * 60_000;
/// How long a Character whose name another key holds waits to try again, unless their set
/// changes first.
const CLAIMED_RETRY_MS: u64 = 30 * 60_000;
/// How far ahead of the wall clock an entry's date may be: 23 hours. The site takes up to a day
/// ahead of its own clock (ADR 0015) and refuses the whole set for one later, so an hour less
/// here keeps a set from a PC whose clock runs up to an hour ahead of the site's from that.
const AHEAD_MS: u64 = 23 * 60 * 60_000;
/// The longest job name the site takes.
const MOST_JOB_CHARS: usize = 32;

/// One entry on a Map Leaderboard, as the site's API takes it: a Character's best Visit of a
/// kind (solo or party) on a map in a Level Band (the site works the band out from the level).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntry {
    /// The 9-digit map ID.
    pub map: String,
    pub level: u32,
    pub job: String,
    pub exp_per_hour: u64,
    /// When the Visit set it, in ms since the Unix epoch.
    pub set_at: u64,
    /// It is the map's Party Best, not its Solo Best (ADR 0026): the site ranks the two apart
    /// and marks a party entry. Always in the body: an entry that says nothing of it is a
    /// Party Best to the site, as a best of no known kind is everywhere.
    #[serde(default)]
    pub party: bool,
}

/// An upload the shell is to make, from when the core asks for it until the shell tells it
/// the answer: a Character's whole set of entries (`PUT`), or, with `entries` `None`, taking
/// them all off the site (`DELETE`, opting out), or, with `pull`, getting the set the site
/// holds of them (`GET`), to merge in before sending (see the module's "Pulled first").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardUpload {
    /// Which upload it is: one more with each the core asks for. The answer names it.
    pub id: u64,
    /// The Character's name.
    pub name: String,
    pub entries: Option<Vec<LeaderboardEntry>>,
    /// It gets their entries from the site, sending nothing: `entries` is `None`. Its answer
    /// is read by [`read_leaderboard_pull`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pull: bool,
}

impl LeaderboardUpload {
    /// Where on the site it goes, from [`SITE_URL`]: the API's path for the Character's entries.
    pub fn path(&self) -> String {
        format!("api/characters/{}/entries", encoded(&self.name))
    }

    /// What to `PUT` there: the API's body. `None` for a `DELETE` or a pull's `GET`.
    pub fn body(&self) -> Option<String> {
        #[derive(Serialize)]
        struct Body<'a> {
            entries: &'a [LeaderboardEntry],
        }
        let entries = self.entries.as_deref()?;
        Some(serde_json::to_string(&Body { entries }).unwrap_or_default())
    }
}

/// What the Map Leaderboards' answer to an upload means, from its HTTP `status`, its
/// `Retry-After` header and its `body` (ADR 0015).
pub fn read_leaderboard_answer(status: u16, retry_after: Option<&str>, body: &str) -> LeaderboardAnswer {
    #[derive(Deserialize)]
    struct Taken {
        #[serde(default)]
        rejected: Vec<Rejection>,
    }
    #[derive(Deserialize)]
    struct Rejection {
        map: String,
        band: u32,
        #[serde(default)]
        party: bool,
        reason: String,
        cap: Option<u64>,
    }
    match status {
        // Taken: it went through, whatever the body says of what was kept off.
        200 => {
            let rejected = serde_json::from_str::<Taken>(body).map_or_else(|_| Vec::new(), |taken| taken.rejected);
            let rejected = rejected
                .into_iter()
                .filter_map(|r| {
                    let reason = match r.reason.as_str() {
                        "over-cap" => RejectedReason::OverCap { cap: r.cap.unwrap_or(0) },
                        "removed" => RejectedReason::Removed {},
                        _ => return None,
                    };
                    Some(RejectedEntry { map: r.map, band: r.band, party: r.party, reason })
                })
                .collect();
            LeaderboardAnswer::Accepted { rejected }
        }
        204 => LeaderboardAnswer::Deleted {},
        _ => refused(status, retry_after, body),
    }
}

/// What the Map Leaderboards' answer to a pull means, from its HTTP `status`, its `Retry-After`
/// header and its `body`: the entries the site holds of the Character (none for a name no key
/// holds), or why there are none to have.
pub fn read_leaderboard_pull(status: u16, retry_after: Option<&str>, body: &str) -> LeaderboardAnswer {
    #[derive(Deserialize)]
    struct Held {
        entries: Vec<LeaderboardEntry>,
    }
    match (status, serde_json::from_str::<Held>(body)) {
        (200, Ok(held)) => LeaderboardAnswer::Pulled { entries: held.entries },
        (200, Err(e)) => LeaderboardAnswer::Failed { why: format!("HTTP 200, but not entries ({e})") },
        _ => refused(status, retry_after, body),
    }
}

/// What an answer that isn't the site doing as asked means: too many lately, a name another key
/// holds, blocked, or anything else as words.
fn refused(status: u16, retry_after: Option<&str>, body: &str) -> LeaderboardAnswer {
    #[derive(Deserialize, Default)]
    struct Refusal {
        #[serde(default)]
        error: String,
        detail: Option<String>,
    }
    if status == 429 {
        let retry_after_secs = retry_after.and_then(|s| s.trim().parse().ok()).unwrap_or(60);
        return LeaderboardAnswer::RateLimited { retry_after_secs };
    }
    let refusal: Refusal = serde_json::from_str(body).unwrap_or_default();
    if status == 409 && refusal.error == "claimed" {
        return LeaderboardAnswer::Claimed {};
    }
    if status == 403 && refusal.error == "blocked" {
        return LeaderboardAnswer::Blocked {};
    }
    let mut why = format!("HTTP {status}");
    if !refusal.error.is_empty() {
        why += &format!(": {}", refusal.error);
    }
    if let Some(detail) = refusal.detail {
        why += &format!(" ({detail})");
    }
    LeaderboardAnswer::Failed { why }
}

/// What the Map Leaderboards answered an upload, as the shell tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LeaderboardAnswer {
    /// The set was taken (200), but for the entries `rejected`.
    Accepted { rejected: Vec<RejectedEntry> },
    /// The entries were deleted (204).
    Deleted {},
    /// A pull's: the entries the site holds of the Character (200), none for a name no key holds.
    Pulled { entries: Vec<LeaderboardEntry> },
    /// Another Leaderboard Key holds the name (409 `claimed`).
    Claimed {},
    /// The maintainer blocked uploads from this key, its Account or the name (403 `blocked`,
    /// ADR 0031).
    Blocked {},
    /// Too many uploads lately (429): try again in `retry_after_secs`.
    RateLimited { retry_after_secs: u64 },
    /// The site couldn't be reached, or answered otherwise: why, in words.
    Failed { why: String },
}

/// An entry an upload held that the Map Leaderboards didn't take, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedEntry {
    /// The 9-digit map ID.
    pub map: String,
    /// The Level Band, by its first level (1, 10, 20…), as the site names it.
    pub band: u32,
    /// It was a Party Best's entry, not a Solo Best's.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub party: bool,
    pub reason: RejectedReason,
}

/// An entry the Map Leaderboards didn't take, as the Personal Bests window lists it: with its
/// Level Band's levels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedListing {
    /// The 9-digit map ID.
    pub map: String,
    /// The Level Band's first level and last: 1 and 9, 10 and 19, and so on.
    pub lowest_level: u32,
    pub highest_level: u32,
    /// It was a Party Best's entry, not a Solo Best's.
    pub party: bool,
    pub reason: RejectedReason,
}

impl From<&RejectedEntry> for RejectedListing {
    fn from(entry: &RejectedEntry) -> Self {
        let band = LevelBand::of(entry.band);
        Self {
            map: entry.map.clone(),
            lowest_level: band.lowest(),
            highest_level: band.highest(),
            party: entry.party,
            reason: entry.reason.clone(),
        }
    }
}

/// Why the Map Leaderboards didn't take an entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RejectedReason {
    /// Its EXP/h is above 5× the map's solo ceiling: `cap` is the most an entry there may have.
    OverCap { cap: u64 },
    /// The maintainer removed it: the same Visit stays off.
    Removed {},
}

/// A Character the player has shown on the Map Leaderboards, or taken off while the site
/// hasn't yet heard: kept with the settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardCharacter {
    pub name: String,
    /// Shown; `false`: taken off, and their entries not yet deleted from the site.
    pub shown: bool,
    /// When an upload of theirs last went through, by the wall clock (ms since the Unix epoch).
    /// `None`: none has from this install, so the site's are pulled first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uploaded_at_unix_ms: Option<u64>,
    /// Their uploads are held: the bests file didn't read at a launch since they were shown, so
    /// what is here may be far less than the site holds. Until the site's are pulled and merged
    /// in, or the player turns them off and on.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub held: bool,
}

/// The Personal Bests window's "Share" for a Character, under their name: on for one met for
/// the first time with an entry, until the player turns it off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardToggle {
    /// The player shows them there.
    pub on: bool,
    /// How many maps they have an entry for: what turning it on says it shares ("Share
    /// Egbert's bests on 14 maps").
    pub maps: usize,
    /// The window offers the switch: they're shown, so it can turn them off, or they have an
    /// entry to upload. With none (bests with no level, job or date read), there is nothing to
    /// show there yet.
    pub offered: bool,
    pub status: LeaderboardStatus,
    /// Where they stand is a problem, which the window says as one, and which the player can
    /// send again: [`LeaderboardStatus::Retrying`], [`LeaderboardStatus::Claimed`],
    /// [`LeaderboardStatus::Blocked`] or [`LeaderboardStatus::Held`].
    pub problem: bool,
    /// When an upload of theirs last went through (ms since the Unix epoch); `None` if none
    /// has yet.
    pub uploaded_at_unix_ms: Option<u64>,
    /// The entries the site didn't take at the last upload, while they're shown there.
    pub rejected: Vec<RejectedListing>,
    /// How many maps had a Visit come back from the site this launch, when their entries were
    /// pulled: 0 if none did.
    pub maps_from_site: usize,
}

/// Where a Character stands with the Map Leaderboards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LeaderboardStatus {
    /// Not shown there, and nothing of theirs left to delete.
    Off,
    /// An upload (or, turned off, the deletion) is under way or about to be.
    Sending,
    /// The site has what it should: their entries, as last uploaded.
    UpToDate,
    /// The last upload didn't go through: it is tried again quietly.
    Retrying,
    /// Another Leaderboard Key holds the name: the player asks on Discord.
    Claimed,
    /// The maintainer blocked their uploads: nothing more is sent this launch, and the player
    /// asks on Discord if it was a mistake.
    Blocked,
    /// Nothing is sent: the bests file didn't read at a launch since they were shown, so the
    /// site may hold more of theirs than is here, and getting those from the site failed. It is
    /// tried again quietly; sending them again tries at once.
    Held,
}

/// One Character's upload standing: where they stand with the Map Leaderboards, this launch.
#[derive(Debug, Clone)]
struct UploadStanding {
    saved: LeaderboardCharacter,
    /// What the site was last told to hold of theirs and took; `None` until it has this launch.
    confirmed: Option<Vec<LeaderboardEntry>>,
    /// The upload under way.
    in_flight: Option<LeaderboardUpload>,
    /// The upload last tried didn't go through: not again until then, unless what is to be
    /// sent changes.
    wait: Option<Wait>,
    /// How many uploads in a row haven't gone through.
    failures: u32,
    /// Another key holds the name, as the last answer said.
    claimed: bool,
    /// The site refused their upload as blocked this launch: nothing more goes up but the
    /// deletion, until the player sends them again or Minerva starts again.
    blocked: bool,
    rejected: Vec<RejectedEntry>,
    /// Their entries were pulled from the site this launch.
    pulled: bool,
    /// How many maps had a Visit come back from the site when they were.
    maps_from_site: usize,
}

#[derive(Debug, Clone)]
struct Wait {
    until_ms: u64,
    /// What was to be sent then: nothing, for a pull.
    wanted: Option<Vec<LeaderboardEntry>>,
    /// The site said how long: a change doesn't cut it short.
    rate_limited: bool,
    /// It was a pull that didn't go through: only the next pull waits, never an upload or the
    /// deletion (turned off meanwhile, say).
    pull: bool,
}

/// Every Character shown on the Map Leaderboards, or being taken off, and every one met.
#[derive(Debug, Default)]
pub(crate) struct Leaderboards {
    standings: Vec<UploadStanding>,
    /// Every Character met with an entry, or shown or taken off by the player: one met for the
    /// first time is shown. `None` while it isn't known who was met before (settings from
    /// before version 7, until the bests read at a launch).
    met: Option<Vec<String>>,
    /// [`Leaderboards::met`] was just taken from the bests restored at this launch, which may
    /// yet turn out not to have been read.
    met_from_bests: bool,
    /// The id of the last upload asked for.
    last_id: u64,
    /// The latest capture time seen in a tick.
    now_ms: u64,
}

impl Leaderboards {
    /// Take back the Characters an earlier launch kept, and those it met (`None`: not known), in
    /// place of any: nothing is under way.
    pub fn restore(&mut self, saved: Vec<LeaderboardCharacter>, met: Option<Vec<String>>) {
        self.standings = saved.into_iter().map(UploadStanding::new).collect();
        self.met = met;
        self.met_from_bests = false;
    }

    /// The Characters to keep with the settings.
    pub fn saved(&self) -> Vec<LeaderboardCharacter> {
        self.standings.iter().map(|standing| standing.saved.clone()).collect()
    }

    /// The Characters met, to keep with the settings.
    pub fn met(&self) -> Option<Vec<String>> {
        self.met.clone()
    }

    /// The bests earlier launches saved are back, `history`: when it isn't known who was met
    /// before, everyone in them was, so none of them starts shown.
    pub fn bests_restored(&mut self, history: &SavedBests) {
        if self.met.is_none() {
            self.met = Some(history.characters.iter().map(|c| c.name.clone()).collect());
            self.met_from_bests = true;
        }
    }

    /// The player showed the Character named `name` on the Map Leaderboards (`on`), or took them
    /// off: met, so that stands.
    pub fn show(&mut self, name: &str, on: bool) {
        if let Some(met) = &mut self.met
            && !met.iter().any(|m| m == name)
        {
            met.push(name.to_string());
        }
        match self.standings.iter_mut().find(|standing| standing.saved.name == name) {
            Some(standing) if standing.saved.shown != on => {
                standing.saved.shown = on;
                // Off and on again, the player sends what is here.
                standing.saved.held = false;
                // Shown again, the site may have deleted them meanwhile; taken off, nothing
                // held stands.
                standing.confirmed = None;
                standing.wait = None;
                standing.failures = 0;
            }
            Some(_) => {}
            None if on => self.standings.push(UploadStanding::new(LeaderboardCharacter {
                name: name.to_string(),
                shown: true,
                uploaded_at_unix_ms: None,
                held: false,
            })),
            None => {}
        }
    }

    /// The player sent the Character named `name` again: whatever stood in the way (a wait
    /// after a failure, a name another key held), what is here goes up at the next tick,
    /// replacing what the site holds of them. Held, their entries are pulled from the site at
    /// the next tick instead, and sent with what is here once merged in. Taken off, their
    /// deletion goes again the same way. Nothing changes while an upload of theirs is under
    /// way.
    pub fn send_again(&mut self, name: &str) {
        let Some(standing) = self.standings.iter_mut().find(|standing| standing.saved.name == name) else { return };
        if standing.in_flight.is_some() {
            return;
        }
        standing.confirmed = None;
        standing.wait = None;
        standing.failures = 0;
        standing.claimed = false;
        standing.blocked = false;
    }

    /// Linked to an Account just now: each Character whose name another key held goes up again
    /// at the next tick, since the name may be theirs through the Account. Uploads held stay
    /// held.
    pub fn retry_claimed(&mut self) {
        for standing in self.standings.iter_mut().filter(|standing| standing.claimed && standing.in_flight.is_none()) {
            standing.claimed = false;
            standing.confirmed = None;
            standing.wait = None;
            standing.failures = 0;
        }
    }

    /// Whether any Character is shown on the Map Leaderboards.
    pub fn any_shown(&self) -> bool {
        self.standings.iter().any(|standing| standing.saved.shown)
    }

    /// The Personal Bests saved at earlier launches couldn't be read at this one: hold the
    /// uploads of every Character shown, until their entries are pulled from the site. Who was
    /// met can't be told from them either.
    pub fn bests_lost(&mut self) {
        if self.met_from_bests {
            self.met = None;
        }
        for standing in self.standings.iter_mut().filter(|standing| standing.saved.shown) {
            standing.saved.held = true;
        }
    }

    /// Time passed, to `at_ms`: show each Character met for the first time with an entry, and
    /// ask for each upload due, with `history` the bests of ended Visits and `wall_clock` the
    /// time now. None before the wall clock is known.
    pub fn tick(&mut self, at_ms: u64, history: &SavedBests, wall_clock: Option<u64>) {
        self.now_ms = self.now_ms.max(at_ms);
        let Some(now_unix_ms) = wall_clock else { return };
        let new: Vec<String> = match &self.met {
            Some(met) => history
                .characters
                .iter()
                .filter(|c| !met.contains(&c.name) && !entries(history, &c.name, now_unix_ms).is_empty())
                .map(|c| c.name.clone())
                .collect(),
            None => Vec::new(),
        };
        for name in new {
            self.show(&name, true);
        }
        for standing in &mut self.standings {
            // Blocked, nothing goes but the deletion the player asks for by turning them off.
            if standing.in_flight.is_some() || (standing.saved.shown && standing.blocked) {
                continue;
            }
            // Their entries pulled from the site first, held or not, then what is here goes up.
            if standing.to_pull() {
                if !standing.wait.as_ref().is_some_and(|wait| self.now_ms < wait.until_ms) {
                    self.last_id += 1;
                    standing.in_flight = Some(LeaderboardUpload { id: self.last_id, name: standing.saved.name.clone(), entries: None, pull: true });
                }
                continue;
            }
            let wanted = standing.saved.shown.then(|| entries(history, &standing.saved.name, now_unix_ms));
            let due = match &wanted {
                Some(entries) => standing.confirmed.as_ref() != Some(entries),
                None => true,
            };
            // A pull that failed holds up nothing but the next pull, unless the site said how long.
            let waiting = standing.wait.as_ref().is_some_and(|wait| {
                (!wait.pull || wait.rate_limited) && self.now_ms < wait.until_ms && (wait.rate_limited || wait.wanted == wanted)
            });
            if due && !waiting {
                self.last_id += 1;
                standing.in_flight =
                    Some(LeaderboardUpload { id: self.last_id, name: standing.saved.name.clone(), entries: wanted, pull: false });
            }
        }
    }

    /// The site answered upload `id` with `answer`, the wall clock at `wall_clock`. A pull's
    /// entries, to merge into the Character's bests: whose, and what. Merged, what is here then
    /// goes up, held or not.
    pub fn answer(&mut self, id: u64, answer: LeaderboardAnswer, wall_clock: Option<u64>) -> Option<(String, Vec<LeaderboardEntry>)> {
        let at = self.standings.iter().position(|standing| standing.in_flight.as_ref().is_some_and(|u| u.id == id))?;
        let standing = &mut self.standings[at];
        let LeaderboardUpload { name, entries, pull, .. } = standing.in_flight.take()?;
        match (answer, entries) {
            (LeaderboardAnswer::Pulled { entries }, None) if pull => {
                standing.pulled = true;
                standing.saved.held = false;
                standing.wait = None;
                standing.failures = 0;
                standing.claimed = false;
                return Some((name, entries));
            }
            (LeaderboardAnswer::Accepted { rejected }, Some(entries)) => {
                standing.confirmed = Some(entries);
                standing.saved.uploaded_at_unix_ms = wall_clock.or(standing.saved.uploaded_at_unix_ms);
                standing.rejected = rejected;
                standing.wait = None;
                standing.failures = 0;
                standing.claimed = false;
            }
            // Deleted: nothing of theirs is left there. Shown again meanwhile, their set goes up
            // at the next tick.
            (LeaderboardAnswer::Deleted {}, None) if !pull => {
                if standing.saved.shown {
                    standing.confirmed = None;
                } else {
                    self.standings.remove(at);
                }
            }
            // Another key holds the name, so nothing of theirs is there to delete.
            (LeaderboardAnswer::Claimed {}, None) if !pull && !standing.saved.shown => {
                self.standings.remove(at);
            }
            (LeaderboardAnswer::Claimed {}, wanted) => {
                standing.confirmed = None;
                standing.claimed = true;
                standing.failures = 0;
                standing.wait = Some(Wait { until_ms: self.now_ms + CLAIMED_RETRY_MS, wanted, rate_limited: false, pull });
            }
            (LeaderboardAnswer::Blocked {}, Some(_)) => {
                standing.confirmed = None;
                standing.blocked = true;
                standing.failures = 0;
                standing.wait = None;
            }
            (LeaderboardAnswer::RateLimited { retry_after_secs }, wanted) => {
                standing.failures += 1;
                let until_ms = self.now_ms + retry_after_secs.max(1).saturating_mul(1000);
                standing.wait = Some(Wait { until_ms, wanted, rate_limited: true, pull });
            }
            // Failed, or an answer that doesn't fit the upload: tried again, later each time.
            (_, wanted) => {
                standing.failures += 1;
                let backoff = FIRST_RETRY_MS.saturating_mul(1 << (standing.failures - 1).min(16)).min(MOST_RETRY_MS);
                standing.wait = Some(Wait { until_ms: self.now_ms + backoff, wanted, rate_limited: false, pull });
            }
        }
        None
    }

    /// `maps` maps had a Visit come back from the site when the Character named `name`'s entries
    /// were pulled and merged in, as the toggle says.
    pub fn came_back(&mut self, name: &str, maps: usize) {
        if let Some(standing) = self.standings.iter_mut().find(|standing| standing.saved.name == name) {
            standing.maps_from_site = maps;
        }
    }

    /// The uploads under way, for the shell to make.
    pub fn uploads(&self) -> Vec<LeaderboardUpload> {
        self.standings.iter().filter_map(|standing| standing.in_flight.clone()).collect()
    }

    /// The toggle for the Character named `name`, with `history` the bests of ended Visits.
    pub fn toggle(&self, name: &str, history: &SavedBests, wall_clock: Option<u64>) -> LeaderboardToggle {
        let mut maps: Vec<String> = Vec::new();
        for entry in entries(history, name, wall_clock.unwrap_or(u64::MAX - AHEAD_MS)) {
            if !maps.contains(&entry.map) {
                maps.push(entry.map);
            }
        }
        let standing = self.standings.iter().find(|standing| standing.saved.name == name);
        let status = match standing {
            None => LeaderboardStatus::Off,
            // Held while their entries are being pulled is only sending; it is a problem once
            // pulling fails. A name another key holds is said as that.
            Some(standing) if standing.saved.shown && standing.saved.held && standing.failures > 0 => LeaderboardStatus::Held,
            Some(standing) if standing.saved.shown && standing.blocked => LeaderboardStatus::Blocked,
            Some(standing) if standing.claimed => LeaderboardStatus::Claimed,
            Some(standing) if standing.failures > 0 => LeaderboardStatus::Retrying,
            Some(standing) if standing.in_flight.is_some() || !standing.saved.shown || standing.confirmed.is_none() => {
                LeaderboardStatus::Sending
            }
            Some(_) => LeaderboardStatus::UpToDate,
        };
        let on = standing.is_some_and(|standing| standing.saved.shown);
        let rejected = standing.filter(|_| on).map_or_else(Vec::new, |standing| standing.rejected.iter().map(RejectedListing::from).collect());
        LeaderboardToggle {
            on,
            maps: maps.len(),
            offered: on || !maps.is_empty(),
            status,
            problem: matches!(status, LeaderboardStatus::Retrying | LeaderboardStatus::Claimed | LeaderboardStatus::Blocked | LeaderboardStatus::Held),
            uploaded_at_unix_ms: standing.and_then(|standing| standing.saved.uploaded_at_unix_ms),
            rejected,
            maps_from_site: standing.map_or(0, |standing| standing.maps_from_site),
        }
    }
}

impl UploadStanding {
    fn new(saved: LeaderboardCharacter) -> Self {
        Self {
            saved,
            confirmed: None,
            in_flight: None,
            wait: None,
            failures: 0,
            claimed: false,
            blocked: false,
            rejected: Vec::new(),
            pulled: false,
            maps_from_site: 0,
        }
    }

    /// Their entries are to be pulled from the site before anything of theirs goes up: shown,
    /// and held, or never uploaded from this install (a new PC, say, whose first upload would
    /// otherwise replace what the site holds of them with what little is here).
    fn to_pull(&self) -> bool {
        self.saved.shown && !self.pulled && (self.saved.held || self.saved.uploaded_at_unix_ms.is_none())
    }
}

/// Map `map`'s Map Leaderboard on the site (ADR 0015's deep link), opened at Level Band `band`
/// with the Character named `name`'s entry marked, as much as is known of them.
pub(crate) fn map_url(map: &str, band: Option<LevelBand>, name: Option<&str>) -> String {
    let mut url = format!("{SITE_URL}leaderboards/map/?map={map}");
    if let Some(band) = band {
        url += &format!("&band={}", band.lowest());
    }
    if let Some(name) = name {
        url += &format!("&name={}", encoded(name));
    }
    url
}

/// `name` for a URL: its letters and digits as they are, any other byte percent-encoded.
fn encoded(name: &str) -> String {
    let mut out = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() {
            out.push(char::from(byte));
        } else {
            out += &format!("%{byte:02X}");
        }
    }
    out
}

/// The entries of the Character named `name` in `history`, the bests of ended Visits: their best
/// Visit of each kind (Solo and Party Bests apart, ADR 0026) on each map in each Level Band that
/// has a level, a job and a date, and isn't dated over [`AHEAD_MS`] after `now_unix_ms`. By map in
/// the order they are saved, then by band, lowest first, a band's Solo Best before its Party Best.
fn entries(history: &SavedBests, name: &str, now_unix_ms: u64) -> Vec<LeaderboardEntry> {
    let mut entries = Vec::new();
    for map in history.maps_of(Some(name)) {
        let entry = |visit: &VisitBest| {
            let (Some(level), Some(job), Some(set_at)) = (visit.level, &visit.job, visit.set_at_unix_ms) else { return None };
            let job_fits = (1..=MOST_JOB_CHARS).contains(&job.chars().count());
            let fits = (1..=MAX_LEVEL).contains(&level) && job_fits && set_at <= now_unix_ms.saturating_add(AHEAD_MS);
            fits.then(|| LeaderboardEntry {
                map: map.id.clone(),
                level,
                job: job.clone(),
                exp_per_hour: visit.exp_per_hour,
                set_at,
                party: visit.kind.is_party(),
            })
        };
        // One that can't be an entry leaves the next best of its kind in its band to stand in.
        let mut of_map: Vec<(LevelBand, crate::BestKind, LeaderboardEntry)> = Vec::new();
        for kind in [crate::BestKind::Solo, crate::BestKind::Party] {
            let can_be_entries = map.visits.iter().filter(|visit| visit.kind == kind && entry(visit).is_some());
            of_map.extend(band_bests(can_be_entries).into_iter().filter_map(|(band, visit)| Some((band, kind, entry(visit)?))));
        }
        of_map.sort_by_key(|(band, party, _)| (*band, *party));
        entries.extend(of_map.into_iter().map(|(_, _, entry)| entry));
    }
    entries
}
