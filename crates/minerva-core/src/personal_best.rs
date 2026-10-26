//! Personal Bests (GLOSSARY): the best Rolling Rate each Character has had on each map, kept
//! Visit by Visit.
//!
//! - A Visit's best on a map is the highest Rolling Rate, in EXP/h, the Grind Tracker
//!   showed at a refresh whose window was a full 5 minutes of active play, all on the map,
//!   with the EXP bar read: the Session decides what counts. Only while the map is
//!   certain and has a Map Summary: the core says which map that is. Maps that share a
//!   title but would show the same count as certain, as the first of them, which the
//!   panel shows: their bests are kept under its ID.
//! - A best is stamped with the Character the Status Bar named, and the level and job it
//!   showed, at the refresh that set it, and kept with that Character's bests. One set while
//!   no Character could be named is Unassigned, as are the bests saved before Characters
//!   existed (version 1). The player can move an Unassigned Visit to a Character, remove a
//!   Character with its bests, and merge one Character's into another's.
//! - A map has two Personal Bests, a Solo Best and a Party Best (GLOSSARY). A Visit's best is
//!   of the kind of the window it was measured over: a Party Best if the 5 minutes had Party
//!   EXP in them, otherwise a Solo Best (the Session decides). A Visit has one of each at most
//!   on a map. Bests saved before party play was told apart (versions 1 and 2) are Solo Bests.
//!   The player can move a Visit to the other kind, after confirming. Both kinds go up as entries
//!   of their own in each Level Band.
//! - A map's Personal Best, for a Character (or Unassigned), is the best of its Visits'.
//!   Each keeps a history of its best Visits on each map, the ones that have ended. The
//!   Grind Tracker's Best row shows the current Character's (or the Unassigned one's while no
//!   Character is named) as it stands (ADR 0018): the best in the history from arriving, and
//!   this Visit's from the refresh that beats it. This Visit's best joins the history when
//!   the Visit ends, or another Character is named. No rate is capped or rejected.
//! - A Visit's best is for the map it was last certain on with a rate to count. The
//!   player may choose among Candidates, and the hotkey goes past the others on the way.
//!   One shown with nothing to count yet (just after a reset, say) doesn't take the
//!   Visit's best, even if it had one of its own earlier in the Visit, so closing
//!   Minerva then keeps it for the map it came from. A Candidate's own best from
//!   earlier in the Visit counts again once it has a rate to count again. Another
//!   Character named ends the Visit's best for the one before: what came before is theirs.
//! - A best is dated by the wall clock at the refresh that first showed it, and keeps
//!   the date in the history.
//! - Each Character's history of a map (its 9-digit ID) keeps its [`VISITS_KEPT`] best
//!   Visits of each kind, and the best Visit of each kind in each [`LevelBand`] even if it
//!   isn't among them, so a Character that levels up keeps its lower bands' bests, and a party
//!   run never pushes out what they do alone. One that is neither is dropped.
//! - They outlive the launch, as the settings do: the core hands them to the shell
//!   as [`SavedBests`], with a revision so it saves them only when they change, and takes
//!   them back at startup. What it hands out includes the Visit under way's best, as if
//!   it had ended: closing Minerva ends it.
//! - The Personal Bests window shows every map's, as handed out (see [`crate::bests_window`]),
//!   one Character at a time, with where the Visit under way's best is among its map's.
//! - The player can remove a Visit that came from a bug: the best the Best row shows, or
//!   any Visit on any map from the Personal Bests window, this Visit's included. Removing
//!   the best, the map's next-best Visit's takes over the row. There is no undo. Once this
//!   Visit's best is removed, only the play since counts towards it again (the Session sees
//!   to that), so the play that set it isn't counted again.

use serde::{Deserialize, Serialize};

/// The shape [`SavedBests`] has now. Bumped whenever it changes. Version 1 had no
/// Characters: its bests read in as Unassigned. Version 2 had no Party Bests: its bests read in
/// as Solo Bests.
pub const BESTS_VERSION: u32 = 3;

/// How many Visits' bests each Character's history of a map keeps: the best of them, besides
/// the best in each [`LevelBand`].
const VISITS_KEPT: usize = 10;

/// Every Character's Visit bests on each map, and the Unassigned ones, as plain data the
/// shell can keep between launches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedBests {
    /// The [`BESTS_VERSION`] it was saved by.
    pub version: u32,
    /// The Unassigned bests: set while no Character could be named, or saved before
    /// Characters existed (version 1 called them `maps`). The maps with at least one.
    #[serde(default, alias = "maps")]
    pub unassigned: Vec<MapBests>,
    /// Each Character with at least one best, in the order they first had one.
    #[serde(default)]
    pub characters: Vec<CharacterBests>,
}

impl Default for SavedBests {
    /// No bests anywhere.
    fn default() -> Self {
        Self { version: BESTS_VERSION, unassigned: Vec::new(), characters: Vec::new() }
    }
}

impl SavedBests {
    /// Whether they were saved in a shape this version reads: this one, or an earlier one.
    /// The shell sets a file in any other aside, and the core leaves them.
    pub fn in_a_known_shape(&self) -> bool {
        (1..=BESTS_VERSION).contains(&self.version)
    }

    /// The maps' bests of the Character named `character`, or the Unassigned ones for
    /// `None`: none if they have none.
    pub fn maps_of(&self, character: Option<&str>) -> &[MapBests] {
        match character {
            None => &self.unassigned,
            Some(name) => self.characters.iter().find(|c| c.name == name).map_or(&[], |c| &c.maps),
        }
    }
}

/// The best of `visits` (a map's, highest first) in each Level Band they were set in, lowest
/// band first. A Visit with no level read has no band, and isn't among them.
pub(crate) fn band_bests<'a>(visits: impl IntoIterator<Item = &'a VisitBest>) -> Vec<(LevelBand, &'a VisitBest)> {
    let mut bands: Vec<(LevelBand, &VisitBest)> = Vec::new();
    // Highest first: the first in each band is its best.
    for visit in visits {
        let Some(band) = visit.level.map(LevelBand::of) else { continue };
        if bands.iter().all(|(b, _)| *b != band) {
            bands.push((band, visit));
        }
    }
    bands.sort_by_key(|(band, _)| *band);
    bands
}

/// One Character's bests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterBests {
    /// The Character's name, as the Status Bar shows it: the Character.
    pub name: String,
    /// The maps with at least one of their Visits' bests.
    pub maps: Vec<MapBests>,
}

/// One map's bests, a Character's or the Unassigned ones: the best of each Visit to it that
/// had one, and that the history keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapBests {
    /// The 9-digit map ID.
    pub id: String,
    /// Highest first: the first is the map's Personal Best.
    pub visits: Vec<VisitBest>,
}

/// The kind of play in a Personal Best's five-minute window.
/// Saved files and uploads keep the existing boolean `party` field.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "bool", into = "bool")]
pub enum BestKind {
    #[default]
    Solo,
    Party,
}

impl BestKind {
    pub fn is_party(self) -> bool { self == Self::Party }
    pub fn is_solo(&self) -> bool { *self == Self::Solo }
    pub(crate) fn index(self) -> usize { usize::from(self.is_party()) }
}

impl From<bool> for BestKind {
    fn from(party: bool) -> Self { if party { Self::Party } else { Self::Solo } }
}

impl From<BestKind> for bool {
    fn from(kind: BestKind) -> Self { kind.is_party() }
}

/// The best one Visit had on a map, stamped with the level and job the Status Bar showed at
/// the refresh that set it. The player removes it, or moves it, by all of it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisitBest {
    /// The Rolling Rate's EXP/h shown, over a full 5 minutes of active play on the map.
    pub exp_per_hour: u64,
    /// When it was first shown, by the wall clock (ms since the Unix epoch); `None` if
    /// the shell hadn't told the wall clock by then. The Personal Bests window shows it (the
    /// Best row has no date).
    pub set_at_unix_ms: Option<u64>,
    /// The Character's level then, as the Status Bar showed it; `None` if it wasn't read
    /// (and for the bests saved before Characters existed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    /// The Character's job then, as the game writes it ("Magician"); `None` if it wasn't read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    /// Party if its window had Party EXP (or the player moved it); otherwise Solo,
    /// as every best saved before party play was told apart is.
    #[serde(rename = "party", default, skip_serializing_if = "BestKind::is_solo")]
    pub kind: BestKind,
}

/// A Level Band (GLOSSARY): 10 levels, 1–9, 10–19, 20–29, and so on. Band `n` holds the
/// levels from `10n` to `10n + 9`; band 0 starts at level 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LevelBand(pub u32);

impl LevelBand {
    /// The band `level` is in.
    pub fn of(level: u32) -> Self {
        Self(level / 10)
    }

    /// Its lowest level: 1 for the first band, 10, 20 and so on for the rest.
    pub fn lowest(self) -> u32 {
        (self.0 * 10).max(1)
    }

    /// Its highest level: 9, 19, 29 and so on.
    pub fn highest(self) -> u32 {
        self.0 * 10 + 9
    }
}

/// Who a best is stamped with, at the refresh that sets it: the Character the Status Bar
/// named (`None`: Unassigned), and the level and job it showed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Stamp {
    pub character: Option<String>,
    pub level: Option<u32>,
    pub job: Option<String>,
}

/// The Visit under way's best on one map: whose it is, and what.
#[derive(Debug, Clone)]
struct UnderWay {
    map: String,
    character: Option<String>,
    best: VisitBest,
}

/// Every Character's map bests, the Unassigned ones, and the Visit under way's.
#[derive(Debug, Default)]
pub(crate) struct Bests {
    /// The history: each Character's (and the Unassigned) best Visits on each map that have
    /// ended, highest first. The Best row shows the first, and the player can remove it.
    history: SavedBests,
    /// The Visit under way's best on each map it was certain on with a rate to count, of each
    /// kind (Solo and Party): one of a kind, unless the player chose among Candidates. Only
    /// `on`'s are the Visit's bests.
    visit: Vec<UnderWay>,
    /// The map the Visit under way was last certain on with a rate to count: the one its
    /// best is for, and the only one [`Bests::saved`] hands out a best from this Visit
    /// for. The hotkey goes past the other Candidates on the way to the one the player
    /// chooses; one it shows with nothing to count doesn't take the best, even if it had
    /// one of its own earlier in the Visit.
    on: Option<String>,
    /// Changes whenever what [`Bests::saved`] hands out does.
    revision: u64,
}

impl Bests {
    /// The Rolling Rate was shown afresh with the player certain to be on `map`, at
    /// `now_unix_ms` by the wall clock, the Status Bar showing `stamp`. `rate` is its EXP/h
    /// if it counts towards this Visit's best there: over a full 5 minutes of active play
    /// all on the map, with the EXP bar read. It counts towards the Visit's Party Best if
    /// the window had Party EXP in it (`kind`), otherwise towards its Solo Best. A best is set
    /// when it is first shown: the same rate again keeps it, and who set it.
    pub fn on_shown(&mut self, map: &str, rate: Option<u64>, kind: BestKind, now_unix_ms: Option<u64>, stamp: &Stamp) {
        self.changing(|bests| bests.record(map, rate, kind, now_unix_ms, stamp));
    }

    /// As [`Bests::on_shown`], leaving the revision to it.
    fn record(&mut self, map: &str, rate: Option<u64>, kind: BestKind, now_unix_ms: Option<u64>, stamp: &Stamp) {
        // Another Character named (or none, after one was): the Visit's best so far is the
        // one before's, and joins their history now.
        if self.visit.iter().any(|under_way| under_way.character != stamp.character) {
            self.end_visit();
        }
        // A Candidate shown with nothing to count doesn't take the Visit's best, even one
        // with a best of its own from earlier in the Visit.
        let Some(rate) = rate else { return };
        if self.on.as_deref() != Some(map) {
            self.on = Some(map.to_string());
        }
        let shown = UnderWay {
            map: map.to_string(),
            character: stamp.character.clone(),
            best: VisitBest { exp_per_hour: rate, set_at_unix_ms: now_unix_ms, level: stamp.level, job: stamp.job.clone(), kind },
        };
        match self.visit.iter_mut().find(|under_way| under_way.map == map && under_way.best.kind == kind) {
            Some(under_way) if under_way.best.exp_per_hour >= rate => {}
            Some(under_way) => *under_way = shown,
            None => self.visit.push(shown),
        }
    }

    /// The Visit under way ended, or another Character was named: its bests join its map's.
    pub fn end_visit(&mut self) {
        let on = self.on.take();
        let ended: Vec<UnderWay> = self.visit.drain(..).filter(|under_way| Some(&under_way.map) == on.as_ref()).collect();
        for UnderWay { map, character, best } in ended {
            join(&mut self.history, character.as_deref(), map, best);
        }
    }

    /// The Visit under way's bests, a Solo Best and a Party Best at most, on the map it is for.
    fn this_visit(&self) -> impl Iterator<Item = &UnderWay> {
        let on = self.on.as_deref();
        self.visit.iter().filter(move |under_way| on == Some(under_way.map.as_str()))
    }

    /// The Visit under way's best of `kind`, on `map`, if it
    /// is `character`'s (the Unassigned one's for `None`) and for that map: the one the
    /// Visit's best is for.
    pub fn this_visits_best(&self, character: Option<&str>, map: &str, kind: BestKind) -> Option<&VisitBest> {
        self.this_visit()
            .find(|under_way| under_way.map == map && under_way.character.as_deref() == character && under_way.best.kind == kind)
            .map(|under_way| &under_way.best)
    }

    /// Do `change`, moving the revision on if it changed what [`Bests::saved`] hands out.
    fn changing(&mut self, change: impl FnOnce(&mut Self)) {
        let before = self.saved();
        change(self);
        if self.saved() != before {
            self.revision += 1;
        }
    }

    /// `map`'s best of `kind`, from the history of the Character
    /// named `character` (the Unassigned one for `None`), the best of the earlier Visits, if
    /// it has one: what the Best row shows until this Visit beats it.
    pub fn history_best(&self, character: Option<&str>, map: &str, kind: BestKind) -> Option<&VisitBest> {
        self.history.maps_of(character).iter().find(|m| m.id == map).and_then(|m| m.visits.iter().find(|v| v.kind == kind))
    }

    /// Remove `map`'s best of `kind`, from the history of `character`
    /// (the Unassigned one for `None`), the best of the earlier Visits. The map's next-best
    /// Visit's of that kind takes over. This Visit's best isn't in the history until the Visit
    /// ends: see [`Bests::remove_this_visit`].
    pub fn remove(&mut self, character: Option<&str>, map: &str, kind: BestKind) {
        if let Some(best) = self.history_best(character, map, kind).cloned() {
            self.remove_visit(character, map, &best);
        }
    }

    /// Remove the Visit under way's best of `kind`, on `map`, if it
    /// is `character`'s (the Unassigned one's for `None`); whether there was one. Nothing of
    /// it joins the history when the Visit ends, unless a best is set again.
    pub fn remove_this_visit(&mut self, character: Option<&str>, map: &str, kind: BestKind) -> bool {
        if self.this_visits_best(character, map, kind).is_none() {
            return false;
        }
        self.changing(|bests| bests.visit.retain(|under_way| !(under_way.map == map && under_way.best.kind == kind)));
        true
    }

    /// Remove the Visit whose best was `visit` from `character`'s history of `map` (the
    /// Unassigned one for `None`); whether there was one. The Visits under it move up. This
    /// Visit's best isn't in the history until the Visit ends: see [`Bests::remove_this_visit`].
    pub fn remove_visit(&mut self, character: Option<&str>, map: &str, visit: &VisitBest) -> bool {
        let mut removed = false;
        self.changing(|bests| removed = take(&mut bests.history, character, map, visit));
        removed
    }

    /// Move the Visit whose best was `visit` from `map`'s history of the Character named
    /// `from` (the Unassigned one for `None`) to the history of the Character named `to`,
    /// where it joins their Visits there as any Visit does; whether there was one to move.
    pub fn move_visit(&mut self, from: Option<&str>, map: &str, visit: &VisitBest, to: &str) -> bool {
        if from == Some(to) {
            return false;
        }
        let mut moved = false;
        self.changing(|bests| {
            moved = take(&mut bests.history, from, map, visit);
            if moved {
                join(&mut bests.history, Some(to), map.to_string(), visit.clone());
            }
        });
        moved
    }

    /// Move the Visit whose best was `visit` on `map`, of the Character named `character`
    /// (the Unassigned one for `None`), to the bests of `kind`, where it joins their Visits
    /// as any Visit does; whether there was one to move.
    pub fn set_visit_party(&mut self, character: Option<&str>, map: &str, visit: &VisitBest, kind: BestKind) -> bool {
        if visit.kind == kind {
            return false;
        }
        let mut moved = false;
        self.changing(|bests| {
            moved = take(&mut bests.history, character, map, visit);
            if moved {
                join(&mut bests.history, character, map.to_string(), VisitBest { kind, ..visit.clone() });
            }
        });
        moved
    }

    /// Move every Unassigned Visit to the history of the Character named `to`, where each
    /// joins their Visits on its map as any Visit does; whether there was one.
    pub fn assign_unassigned(&mut self, to: &str) -> bool {
        let mut moved = false;
        self.changing(|bests| {
            let unassigned = std::mem::take(&mut bests.history.unassigned);
            moved = !unassigned.is_empty();
            for map in unassigned {
                for visit in map.visits {
                    join(&mut bests.history, Some(to), map.id.clone(), visit);
                }
            }
        });
        moved
    }

    /// Remove the Character named `name`, with every best in their history; whether they had
    /// one. The Visit under way's best is left: it isn't in the history yet.
    pub fn remove_character(&mut self, name: &str) -> bool {
        let mut removed = false;
        self.changing(|bests| {
            let characters = &mut bests.history.characters;
            let before = characters.len();
            characters.retain(|c| c.name != name);
            removed = characters.len() < before;
        });
        removed
    }

    /// Merge the Character named `from` into the one named `into`: each of `from`'s Visits
    /// joins `into`'s on its map as any Visit does, so the best in each Level Band stays, and
    /// `from` has none left. Whether `from` had any to merge.
    pub fn merge(&mut self, from: &str, into: &str) -> bool {
        if from == into {
            return false;
        }
        let mut merged = false;
        self.changing(|bests| {
            let characters = &mut bests.history.characters;
            let Some(at) = characters.iter().position(|c| c.name == from) else { return };
            let moving = characters.remove(at);
            for map in moving.maps {
                for visit in map.visits {
                    join(&mut bests.history, Some(into), map.id.clone(), visit);
                }
            }
            merged = true;
        });
        merged
    }

    /// Whether some map has a best: in any history, or the Visit under way's. Whether
    /// [`Bests::saved`] hands out any, without making them.
    pub fn any(&self) -> bool {
        !self.history.unassigned.is_empty() || !self.history.characters.is_empty() || self.this_visit().next().is_some()
    }

    /// The history: every Character's (and the Unassigned) bests of the Visits that have
    /// ended, without the Visit under way's.
    pub fn history(&self) -> &SavedBests {
        &self.history
    }

    /// Every map's Visit bests, for the shell to save: the Visit under way's as if it
    /// ended now, since closing Minerva ends it.
    pub fn saved(&self) -> SavedBests {
        self.standing().0
    }

    /// Every map's Visit bests as [`Bests::saved`] hands them out, and where the Visit
    /// under way's bests (its Solo Best and its Party Best) are among them: whose history each
    /// is in (`None`: Unassigned), its map, and its place in the map's Visits. One the history
    /// doesn't keep is left out.
    pub fn standing(&self) -> (SavedBests, Vec<(Option<String>, String, usize)>) {
        let mut saved = self.history.clone();
        let joined: Vec<&UnderWay> = self
            .this_visit()
            .filter(|under_way| {
                join(&mut saved, under_way.character.as_deref(), under_way.map.clone(), under_way.best.clone()).is_some()
            })
            .collect();
        // Where each is once both have joined: the second may have gone above the first.
        let this_visit = joined
            .into_iter()
            .filter_map(|under_way| {
                let maps = saved.maps_of(under_way.character.as_deref());
                let visits = &maps.iter().find(|m| m.id == under_way.map)?.visits;
                let at = visits.iter().position(|v| *v == under_way.best)?;
                Some((under_way.character.clone(), under_way.map.clone(), at))
            })
            .collect();
        (saved, this_visit)
    }

    /// Take back the bests an earlier launch saved, in place of any there are, in this
    /// version's shape. Bests saved in a shape this version doesn't read are left, and so is
    /// a best of no EXP, which no Visit sets (the file was edited): nothing is measured
    /// against it.
    pub fn restore(&mut self, mut saved: SavedBests) {
        if !saved.in_a_known_shape() {
            return;
        }
        saved.version = BESTS_VERSION;
        drop_empty(&mut saved.unassigned);
        for character in &mut saved.characters {
            drop_empty(&mut character.maps);
        }
        saved.characters.retain(|c| !c.maps.is_empty());
        self.changing(|bests| bests.history = saved);
    }

    /// Changes whenever what [`Bests::saved`] hands out does: a best removed, moved or
    /// merged in the history, or a Character removed; the Visit under way's best set,
    /// raised, or moved to a Candidate chosen that counts a rate, where the history keeps
    /// it; bests restored. A Visit ending changes nothing handed out.
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

/// Leave out the bests of no EXP, and the maps with none left.
fn drop_empty(maps: &mut Vec<MapBests>) {
    for map in maps.iter_mut() {
        map.visits.retain(|best| best.exp_per_hour > 0);
    }
    maps.retain(|map| !map.visits.is_empty());
}

/// A Visit's `best` on `map` joins the map's in the history of `character` (the Unassigned
/// one for `None`), highest first: where it goes among them. The history keeps the
/// [`VISITS_KEPT`] best and the best in each Level Band: one that is neither is dropped
/// (`None`).
fn join(bests: &mut SavedBests, character: Option<&str>, map: String, best: VisitBest) -> Option<usize> {
    let visits = visits_of(bests, character, map);
    let below = visits.iter().position(|v| v.exp_per_hour < best.exp_per_hour).unwrap_or(visits.len());
    visits.insert(below, best.clone());
    keep(visits);
    // The Visits above it are kept as they were: only it, or those under it, can go.
    (visits.get(below) == Some(&best)).then_some(below)
}

/// Of a map's Visits, highest first, keep the [`VISITS_KEPT`] best and the best in each
/// Level Band, of each kind (Solo and Party) on its own: a party run never pushes out what the
/// Character does alone.
fn keep(visits: &mut Vec<VisitBest>) {
    // Per kind: Solo, then Party.
    let mut bands: [Vec<LevelBand>; 2] = [Vec::new(), Vec::new()];
    let mut places = [0usize; 2];
    visits.retain(|visit| {
        let kind = visit.kind.index();
        let top = places[kind] < VISITS_KEPT;
        places[kind] += 1;
        let band_best = visit.level.map(LevelBand::of).is_some_and(|band| {
            let first = !bands[kind].contains(&band);
            bands[kind].push(band);
            first
        });
        top || band_best
    });
}

/// Take the Visit whose best was `visit` out of `character`'s history of `map` (the
/// Unassigned one for `None`), dropping the map, and the Character, if none is left; whether
/// it was there.
fn take(bests: &mut SavedBests, character: Option<&str>, map: &str, visit: &VisitBest) -> bool {
    let maps = maps_mut(bests, character);
    let taken = maps.iter().position(|m| m.id == map).and_then(|at| {
        let listed = maps[at].visits.iter().position(|v| v == visit)?;
        maps[at].visits.remove(listed);
        if maps[at].visits.is_empty() {
            maps.remove(at);
        }
        Some(())
    });
    // A Character with none left goes (as does one only just made by looking).
    bests.characters.retain(|c| !c.maps.is_empty());
    taken.is_some()
}

/// The maps' bests in the history of `character` (the Unassigned one for `None`), to change:
/// a Character with none has an empty list made for them.
fn maps_mut<'a>(bests: &'a mut SavedBests, character: Option<&str>) -> &'a mut Vec<MapBests> {
    let Some(name) = character else { return &mut bests.unassigned };
    let characters = &mut bests.characters;
    let at = match characters.iter().position(|c| c.name == name) {
        Some(at) => at,
        None => {
            characters.push(CharacterBests { name: name.to_string(), maps: Vec::new() });
            characters.len() - 1
        }
    };
    &mut characters[at].maps
}

/// `map`'s Visit bests in the history of `character` (the Unassigned one for `None`),
/// highest first: none yet if it had none.
fn visits_of<'a>(bests: &'a mut SavedBests, character: Option<&str>, map: String) -> &'a mut Vec<VisitBest> {
    let maps = maps_mut(bests, character);
    let at = match maps.iter().position(|m| m.id == map) {
        Some(at) => at,
        None => {
            maps.push(MapBests { id: map, visits: Vec::new() });
            maps.len() - 1
        }
    };
    &mut maps[at].visits
}
