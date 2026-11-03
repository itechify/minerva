//! Map Loot (GLOSSARY, ADR 0033, #225): a Visit's Loot, sent to the site when the Visit ends, for
//! a map's page to work out mesos and Items an hour from. The Session tallies each Visit's Loot
//! as it does its own ([`VisitLoot`]); this says which Visits are worth sending, and keeps them
//! until the site has them. The shell does the HTTP, with the Leaderboard Key, as it does the
//! Map Leaderboards' uploads, and tells the core each answer.
//!
//! - **Which Visits**: one on a map the Personal Bests are kept on (certain, with a Map Summary),
//!   of a Character shown on the Map Leaderboards, with [`LEAST_ACTIVE_MS`] of active play,
//!   some mesos, and the Pickup Log readable for [`READABLE_TENTHS`] tenths of it. A Visit with
//!   no mesos is one whose drops went unpicked, and one with the log covered or unread for long
//!   undercounts by more than the log always does: either would pull a map's figures down.
//! - **What is sent**: the map, the kind (a Visit with the party bonus in it is a party one), the
//!   active time, the mesos and each real Item's count by its ID ([`ItemSet::loot_id`]). A name
//!   cut short that more than one Item's name starts with is tallied by the Grind Tracker, but
//!   can't be named here, so it isn't sent.
//! - **When**: as the Visit ends: the panel confirms another map, or another Character is named.
//!   Closing Minerva ends a Visit too, so what is waiting, and the Visit under way as if it had
//!   ended, are saved as [`SavedMapLoot`] and sent at the next launch. The Visit under way is
//!   saved afresh once a minute of its active time, not with every line, so the shell isn't
//!   always writing.
//! - **The answers**: one upload at a time, oldest first. Taken, it's done; refused (the
//!   Character turned off, blocked, a name another key holds, or one the site won't take), it's
//!   dropped; rate-limited, it waits as long as the site says; anything else (offline, say) is
//!   tried again later each time. A Character turned off meanwhile has theirs dropped unsent.

use crate::loot_items::{LootItem, Named};
use crate::osms_items::ItemSet;
use serde::{Deserialize, Serialize};

/// The shape [`SavedMapLoot`] has now. Bumped whenever it changes.
pub const MAP_LOOT_VERSION: u32 = 1;

/// The least active play a Visit has for its Loot to be sent: 10 minutes.
const LEAST_ACTIVE_MS: u64 = 10 * 60_000;
/// How much of a Visit's active time the Pickup Log must have been readable for, in tenths.
const READABLE_TENTHS: u64 = 9;
/// A Pickup Log reading counts as readable up to the next one, if that comes within this: the
/// reader reads ten times a second, so a longer gap is time it wasn't reading.
pub(crate) const READABLE_GAP_MS: u64 = 2_000;
/// How long to wait after the first upload that didn't go through; each after doubles it, up to
/// [`MOST_RETRY_MS`].
const FIRST_RETRY_MS: u64 = 30_000;
const MOST_RETRY_MS: u64 = 30 * 60_000;
/// How many Visits wait at most: a player offline for weeks keeps the latest.
const MOST_WAITING: usize = 200;

/// A Visit's Loot so far, as the Session tallies it alongside its own.
#[derive(Debug, Default, Clone)]
pub(crate) struct VisitLoot {
    /// The map the Personal Bests are kept on, as last told: `None` until one is.
    pub map: Option<String>,
    /// The party bonus came in during it.
    pub party: bool,
    /// Active time in Sessions of the Visit that ended (a reset, say), in ms.
    pub banked_ms: u64,
    /// How long the Pickup Log was readable while the Session ran, in ms.
    pub readable_ms: u64,
    pub mesos: u64,
    /// The real Items picked up, by OSMS's name.
    items: Vec<LootItem>,
}

impl VisitLoot {
    /// The Visit under way goes on, on `map`, for another Character: nothing of its Loot so far.
    pub fn after_switch(&self) -> Self {
        Self { map: self.map.clone(), ..Self::default() }
    }

    /// `quantity` of an Item line that `named`: counted if it's a real Item's name.
    pub fn add_item(&mut self, named: &Named, quantity: u32) {
        let Named::Item(name) = named else { return };
        let quantity = u64::from(quantity);
        match self.items.iter_mut().find(|item| item.name == *name) {
            Some(item) => item.count = item.count.saturating_add(quantity),
            None => self.items.push(LootItem { name: name.clone(), count: quantity }),
        }
    }

    /// As it stands, for `character` (`None`: no one named) after `active_ms` of active play in
    /// the Session under way.
    pub fn ended(&self, character: Option<&str>, active_ms: u64) -> VisitLootEnded {
        VisitLootEnded {
            character: character.map(String::from),
            map: self.map.clone(),
            party: self.party,
            active_ms: self.banked_ms + active_ms,
            readable_ms: self.readable_ms,
            mesos: self.mesos,
            items: self.items.clone(),
        }
    }
}

/// A Visit's Loot when it ended, or the Visit under way's as if it had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VisitLootEnded {
    character: Option<String>,
    map: Option<String>,
    party: bool,
    active_ms: u64,
    readable_ms: u64,
    mesos: u64,
    items: Vec<LootItem>,
}

impl VisitLootEnded {
    /// What is sent of it, if it's worth sending: see the module's "Which Visits" (but for
    /// whether its Character is shown, which the caller knows).
    pub fn worth_sending(self) -> Option<MapLootVisit> {
        let readable = self.readable_ms.saturating_mul(10) >= self.active_ms.saturating_mul(READABLE_TENTHS);
        if self.active_ms < LEAST_ACTIVE_MS || self.mesos == 0 || !readable {
            return None;
        }
        Some(MapLootVisit {
            name: self.character?,
            map: self.map?,
            party: self.party,
            active_ms: self.active_ms,
            mesos: self.mesos,
            items: self.items,
        })
    }
}

/// A Visit's Map Loot, waiting to be sent: the Items by name, named by ID as it goes, from the
/// item files then.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapLootVisit {
    /// The Character's name.
    pub name: String,
    /// The 9-digit map ID.
    pub map: String,
    pub party: bool,
    pub active_ms: u64,
    pub mesos: u64,
    pub items: Vec<LootItem>,
}

/// The Map Loot waiting to be sent, kept between launches: the Visit under way's among it, as
/// if it had ended, since closing Minerva ends it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedMapLoot {
    pub version: u32,
    #[serde(default)]
    pub visits: Vec<MapLootVisit>,
}

impl Default for SavedMapLoot {
    fn default() -> Self {
        Self { version: MAP_LOOT_VERSION, visits: Vec::new() }
    }
}

impl SavedMapLoot {
    /// Whether it was saved in a shape this version reads.
    pub fn in_a_known_shape(&self) -> bool {
        self.version <= MAP_LOOT_VERSION
    }
}

/// One Item of an upload: its ID, and how many.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapLootItem {
    pub id: u32,
    pub count: u64,
}

/// A Visit's Map Loot the shell is to send, from when the core asks for it until the shell tells
/// it the answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapLootUpload {
    /// Which upload it is: one more with each the core asks for. The answer names it.
    pub id: u64,
    /// The Character's name.
    pub name: String,
    /// The 9-digit map ID.
    pub map: String,
    pub party: bool,
    pub active_ms: u64,
    pub mesos: u64,
    pub items: Vec<MapLootItem>,
}

impl MapLootUpload {
    /// Where on the site it goes, from [`crate::SITE_URL`]: the API's path for the Character's
    /// Map Loot. It is `POST`ed.
    pub fn path(&self) -> String {
        format!("api/characters/{}/map-loot", crate::leaderboards::encoded(&self.name))
    }

    /// What to `POST` there: the API's body.
    pub fn body(&self) -> String {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            map: &'a str,
            party: bool,
            active_ms: u64,
            mesos: u64,
            items: &'a [MapLootItem],
        }
        let body = Body { map: &self.map, party: self.party, active_ms: self.active_ms, mesos: self.mesos, items: &self.items };
        serde_json::to_string(&body).unwrap_or_default()
    }
}

/// What the site answered a Map Loot upload, as the shell tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MapLootAnswer {
    /// Taken.
    Taken {},
    /// Refused for good, why in words: not sent again.
    Refused { why: String },
    /// Too many uploads lately (429): try again in `retry_after_secs`.
    RateLimited { retry_after_secs: u64 },
    /// The site couldn't be reached, or answered otherwise: why, in words. Tried again.
    Failed { why: String },
}

/// What the site's answer to a Map Loot upload means, from its HTTP `status`, its `Retry-After`
/// header and its `body`.
pub fn read_map_loot_answer(status: u16, retry_after: Option<&str>, body: &str) -> MapLootAnswer {
    #[derive(Deserialize, Default)]
    struct Refusal {
        #[serde(default)]
        error: String,
    }
    let why = || {
        let refusal: Refusal = serde_json::from_str(body).unwrap_or_default();
        if refusal.error.is_empty() { format!("HTTP {status}") } else { format!("HTTP {status}: {}", refusal.error) }
    };
    match status {
        200..=299 => MapLootAnswer::Taken {},
        429 => MapLootAnswer::RateLimited { retry_after_secs: retry_after.and_then(|s| s.trim().parse().ok()).unwrap_or(60) },
        400..=499 => MapLootAnswer::Refused { why: why() },
        _ => MapLootAnswer::Failed { why: why() },
    }
}

/// The Map Loot waiting to be sent, and the upload under way.
#[derive(Debug, Default)]
pub(crate) struct MapLoot {
    /// Oldest first: the first is the one under way, if any is.
    waiting: Vec<MapLootVisit>,
    /// The upload under way: of the first waiting.
    in_flight: Option<MapLootUpload>,
    /// The Visit under way's, as if it had ended, if it's worth sending yet.
    under_way: Option<MapLootVisit>,
    /// Not again until then.
    wait_until_ms: Option<u64>,
    /// How many uploads in a row haven't gone through.
    failures: u32,
    last_id: u64,
    now_ms: u64,
    revision: u64,
}

impl MapLoot {
    /// Take back what an earlier launch kept, in place of any: nothing is under way.
    pub fn restore(&mut self, saved: SavedMapLoot) {
        self.waiting = saved.visits;
        self.in_flight = None;
        self.revision += 1;
    }

    /// What to keep: what is waiting, and the Visit under way's.
    pub fn saved(&self) -> SavedMapLoot {
        let visits = self.waiting.iter().chain(&self.under_way).cloned().collect();
        SavedMapLoot { version: MAP_LOOT_VERSION, visits }
    }

    /// Changes whenever what [`MapLoot::saved`] keeps does, but for the Visit under way's within
    /// a minute of its active time.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// A Visit ended, worth sending.
    pub fn ended(&mut self, visit: MapLootVisit) {
        if self.waiting.len() >= MOST_WAITING && self.in_flight.is_none() {
            self.waiting.remove(0);
        }
        self.waiting.push(visit);
        self.revision += 1;
    }

    /// The Visit under way's, as if it had ended, if it's worth sending yet.
    pub fn under_way(&mut self, visit: Option<MapLootVisit>) {
        let minute = |visit: &Option<MapLootVisit>| visit.as_ref().map(|v| (v.name.clone(), v.map.clone(), v.active_ms / 60_000));
        if minute(&visit) != minute(&self.under_way) {
            self.revision += 1;
        }
        self.under_way = visit;
    }

    /// Time passed, to `at_ms`: ask for the next upload, if one is due, naming its Items from
    /// `items` (none until they are in), for a Character `sends` says is still shown.
    pub fn tick(&mut self, at_ms: u64, items: Option<&ItemSet>, sends: impl Fn(&str) -> bool) {
        self.now_ms = self.now_ms.max(at_ms);
        let before = self.waiting.len();
        // A Character turned off since has theirs dropped, never sent.
        let in_flight = self.in_flight.is_some();
        let mut first = true;
        self.waiting.retain(|visit| {
            let keep = (first && in_flight) || sends(&visit.name);
            first = false;
            keep
        });
        if self.waiting.len() != before {
            self.revision += 1;
        }
        if self.in_flight.is_some() || self.wait_until_ms.is_some_and(|until| self.now_ms < until) {
            return;
        }
        let (Some(visit), Some(items)) = (self.waiting.first(), items) else { return };
        self.last_id += 1;
        let named = visit.items.iter().filter_map(|item| Some(MapLootItem { id: items.loot_id(&item.name)?, count: item.count }));
        self.in_flight = Some(MapLootUpload {
            id: self.last_id,
            name: visit.name.clone(),
            map: visit.map.clone(),
            party: visit.party,
            active_ms: visit.active_ms,
            mesos: visit.mesos,
            items: named.collect(),
        });
    }

    /// The site answered upload `id` with `answer`. An answer to an upload no longer under way
    /// changes nothing.
    pub fn answer(&mut self, id: u64, answer: MapLootAnswer) {
        if self.in_flight.as_ref().is_none_or(|upload| upload.id != id) {
            return;
        }
        self.in_flight = None;
        match answer {
            MapLootAnswer::Taken {} | MapLootAnswer::Refused { .. } => {
                self.waiting.remove(0);
                self.revision += 1;
                self.failures = 0;
                self.wait_until_ms = None;
            }
            MapLootAnswer::RateLimited { retry_after_secs } => {
                self.failures += 1;
                self.wait_until_ms = Some(self.now_ms + retry_after_secs.max(1).saturating_mul(1000));
            }
            MapLootAnswer::Failed { .. } => {
                self.failures += 1;
                let backoff = FIRST_RETRY_MS.saturating_mul(1 << (self.failures - 1).min(16)).min(MOST_RETRY_MS);
                self.wait_until_ms = Some(self.now_ms + backoff);
            }
        }
    }

    /// The upload under way, for the shell to make.
    pub fn uploads(&self) -> Vec<MapLootUpload> {
        self.in_flight.iter().cloned().collect()
    }
}
