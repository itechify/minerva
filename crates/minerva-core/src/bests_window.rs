//! The Personal Bests window (GLOSSARY): every map's Personal Best side by side, over the
//! game, to compare maps. The Grind Tracker's star and its Best row's "Best" open it,
//! while some map has a best, and close it again; so does its own ×. Each launch starts
//! with it closed.
//!
//! - Live: the bests as saved now, the Visit under way's included, as closing Minerva
//!   would keep them, as the Best row has them.
//! - One Character's bests at a time: the core gives each Character's with a best, and the
//!   one playing's (with none, if they have none), by name, then the Unassigned ones while
//!   there are any or no Character is named. The page's picker shows the one playing until
//!   the player picks another, and keeps which they picked; the core never knows.
//! - One row a map, its Personal Best, highest EXP/h first. The window re-sorts as the
//!   player likes; the core gives the figures, never text.
//! - Each map's names, region, mob levels and solo EXP/h ceiling (its Map Summary, worked
//!   out as NiaMeowDB does) from OSMS's file set (ADR 0008), the same for every map: the
//!   levels and the share of the ceiling say what a best was worth. A best on a map not in
//!   the map list is kept, with none of these, and a map without mobs has no levels or
//!   ceiling.
//! - The map the player is on is marked: only while the panel is certain of it.
//! - Each map's row holds its Visit History, highest first, for the player to open: each
//!   Visit's EXP/h, its share of the ceiling, its date, and the level and job that set it.
//!   The Visit under way's best is among them, marked, where the history would keep it: it
//!   can be removed, but isn't in the history yet, so it can't be moved. The page keeps
//!   which maps are open; the core never knows.
//! - Where it goes: centred over the game's client area until the player drags it, then
//!   where they left it relative to the client area, until they drag it back to the
//!   middle. As wide as its page lays it out, unless the client area is narrower, and no
//!   taller than the client area. Always wholly on a monitor and never over a captured
//!   region (it shows in screen captures, ours included), but over the panel and the
//!   Grind Tracker if that's where it goes.

use crate::grind_tracker::ClientOffset;
use crate::leaderboards::{self, LeaderboardToggle};
use crate::map_data::MapData;
use crate::map_index::MapEntry;
use crate::panel::{GameWindow, ScreenRect, Size, in_view};
use crate::personal_best::{LevelBand, MapBests, SavedBests};
use crate::shown_map::LevelRange;
use crate::meowdb;
use serde::Serialize;

/// The Personal Bests window, as the shell shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalBestsWindow {
    /// The player opened it and hasn't closed it since: the Grind Tracker's star shows as
    /// on.
    pub open: bool,
    /// Where it goes on screen; `None` while it is hidden: closed, with no game window,
    /// while setup is open, or while another app is in front (see [`crate::Layer`]).
    pub placement: Option<ScreenRect>,
    /// Whose bests it can show, one at a time: each Character with a best, and the one
    /// playing, by name; then the Unassigned bests, if there are any or no Character is
    /// named. None while it is closed.
    pub characters: Vec<PersonalBestsCharacter>,
}

/// One Character's bests in the Personal Bests window, or the Unassigned ones: the choices
/// of its picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalBestsCharacter {
    /// The Character's name; `None` for the Unassigned bests.
    pub name: Option<String>,
    /// They are the Character playing, as the Status Bar names them (the Unassigned bests,
    /// while it names no one): the one the picker shows until the player picks another.
    pub current: bool,
    /// Every map they have a Personal Best on, highest EXP/h first.
    pub maps: Vec<PersonalBestsMap>,
    /// Their "Share to the Map Leaderboards"; `None` for the Unassigned bests, which are never
    /// uploaded.
    pub leaderboard: Option<LeaderboardToggle>,
}

/// One map's row in the Personal Bests window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalBestsMap {
    /// The 9-digit map ID.
    pub id: String,
    /// The map's names and region, as OSMS's map list gives them; empty while no file set
    /// has arrived, or if the map isn't in the list: the window goes by the ID.
    pub street_name: String,
    pub map_name: String,
    pub region: String,
    /// The map's Map Mark, by OSMS's name for it (see [`crate::PanelState::map_marks`]);
    /// `None` for a map without one, or not in the map list.
    pub mark: Option<String>,
    /// The map's MeowDB page.
    pub url: String,
    /// The map's Map Leaderboard on the site, opened at the Level Band of the Character playing
    /// (for another Character's row, the band of their best there) with this Character's entry
    /// marked. `None` for the Unassigned bests, which are no one's on the site, and for a map
    /// not in the map list, which has no page there.
    pub leaderboard_url: Option<String>,
    /// It is the Character's best of all maps, which the window stars: the first row, with
    /// more than one to compare. Never one of the Unassigned bests, which aren't one player's.
    pub best_of_all: bool,
    /// The lowest and highest of its mobs' levels; `None` if it isn't in the map list, or
    /// has no mobs.
    pub mob_levels: Option<LevelRange>,
    /// Its Personal Best: the Rolling Rate's EXP/h, as the Visit's best was set.
    pub exp_per_hour: u64,
    /// The map's solo EXP/h ceiling (its Map Summary), what the share is of; `None` if it
    /// isn't in the map list, or has no Map Summary.
    pub solo_exp_ceiling: Option<u64>,
    /// The best as a share of the map's solo EXP/h ceiling (its Map Summary), in
    /// hundredths of a percent, as the Grind Tracker writes its share: 322 for "3.2%".
    /// `None` with no ceiling.
    pub ceiling_share_hundredths: Option<u64>,
    /// When it was set, by the wall clock (ms since the Unix epoch); `None` if the shell
    /// hadn't told the wall clock by then.
    pub set_at_unix_ms: Option<u64>,
    /// The panel shows this map and is certain the player is on it.
    pub here: bool,
    /// Its Visit History, highest first, the Visit under way's best among them: the first
    /// is the one the row's figures are of.
    pub visits: Vec<PersonalBestsVisit>,
}

/// One of a map's Visits in the Personal Bests window: what the player removes, or moves, it
/// by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalBestsVisit {
    /// Its best: the Rolling Rate's EXP/h, when it was set, and the level and job that set
    /// it, as the Status Bar showed them (`None`: not read).
    pub exp_per_hour: u64,
    pub set_at_unix_ms: Option<u64>,
    pub level: Option<u32>,
    pub job: Option<String>,
    /// The best as a share of the map's solo EXP/h ceiling, in hundredths of a percent, as
    /// the row's; `None` with no ceiling.
    pub ceiling_share_hundredths: Option<u64>,
    /// It is the Visit under way's best: not in the Visit History until the Visit ends, so
    /// it can be removed but not moved yet.
    pub this_visit: bool,
}

/// The Personal Bests window: whether the player wants it, and where it goes on screen.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BestsWindow {
    pub open: bool,
    /// Where the player dragged it; `None` until they do, and again once they drag it
    /// back to the middle of the game: it stays centred over the game.
    pub at: Option<ClientOffset>,
    /// The size its page lays it out at, in physical px for the current display, as the
    /// shell measures it.
    pub size: Size,
}

impl Default for BestsWindow {
    fn default() -> Self {
        Self { open: false, at: None, size: Size { width: 0, height: 0 } }
    }
}

impl BestsWindow {
    /// The player dragged the window so its top-left corner is at (x, y), physical screen
    /// px: it keeps that spot relative to the `game` window. Let go within an eighth of its
    /// width of its spot in the middle of the game, it goes back to staying centred.
    pub fn moved_to(&mut self, x: i32, y: i32, game: &GameWindow, avoid: &[ScreenRect]) {
        let home = Self { at: None, ..*self }.placement(game, avoid);
        let snap = home.width / 8;
        let near = home.x.abs_diff(x) <= snap && home.y.abs_diff(y) <= snap;
        self.at = (!near).then_some(ClientOffset { x: x - game.client.x, y: y - game.client.y });
    }

    /// Where the window goes over the `game`: where the player dragged it, or else centred
    /// over the client area. No wider or taller than the client area, wholly on a monitor,
    /// and moved the shortest way off any of `avoid`.
    pub fn placement(&self, game: &GameWindow, avoid: &[ScreenRect]) -> ScreenRect {
        let client = game.client;
        let width = self.size.width.min(client.width);
        let height = self.size.height.min(client.height);
        let (x, y) = match self.at {
            Some(at) => (client.x + at.x, client.y + at.y),
            None => (client.x + ((client.width - width) / 2) as i32, client.y + ((client.height - height) / 2) as i32),
        };
        in_view(game, ScreenRect { x, y, width, height }, avoid)
    }
}

/// The Personal Bests window's choices for the bests `saved`, with `current` the Character
/// playing (`None`: none named) and `level` the level the Status Bar shows, and `this_visit` the Visit under way's best among them:
/// whose it is, its map, and its place in the map's Visits. `map` gives a map's entry in the
/// map list and its data, if it is in the list; `here` is the map the player is certainly
/// on, if any. `toggle` gives a Character's "Share to the Map Leaderboards".
pub(crate) fn characters<'a>(
    saved: &SavedBests,
    (current, level): (Option<&str>, Option<u32>),
    this_visit: Option<(Option<&str>, &str, usize)>,
    map: impl Fn(&str) -> Option<(&'a MapEntry, &'a MapData)> + Copy,
    here: Option<&str>,
    toggle: impl Fn(&str) -> LeaderboardToggle,
) -> Vec<PersonalBestsCharacter> {
    let mut names: Vec<&str> = saved.characters.iter().map(|c| c.name.as_str()).collect();
    names.extend(current.filter(|name| !names.contains(name)));
    names.sort_by_key(|name| (name.to_lowercase(), *name));
    let mut owners: Vec<Option<&str>> = names.into_iter().map(Some).collect();
    if !saved.unassigned.is_empty() || current.is_none() {
        owners.push(None);
    }
    owners
        .into_iter()
        .map(|owner| {
            let this_visit = this_visit.filter(|(whose, _, _)| *whose == owner).map(|(_, map, at)| (map, at));
            // The band of the Character playing, as the Status Bar shows it; another's, of their
            // best on the map.
            let playing = level.filter(|_| owner.is_none() || owner == current).map(LevelBand::of);
            // None for the Unassigned bests: they are no one's on the site.
            let link = |bests: &MapBests| {
                let name = owner?;
                let band = playing.or_else(|| bests.visits.iter().find_map(|v| v.level).map(LevelBand::of));
                Some(leaderboards::map_url(&bests.id, band, Some(name)))
            };
            let mut maps = rows(saved.maps_of(owner), this_visit, map, here, link);
            // The star on a Character's best of all maps, with another to compare it with.
            if owner.is_some() && maps.len() > 1 {
                maps[0].best_of_all = true;
            }
            PersonalBestsCharacter { name: owner.map(String::from), current: owner == current, maps, leaderboard: owner.map(&toggle) }
        })
        .collect()
}

/// The Personal Bests window's rows for one Character's bests on `saved` maps, with
/// `this_visit` the Visit under way's best among them: its map, and its place in the map's
/// Visits. `map` gives a map's entry in the map list and its data, if it is in the list;
/// `here` is the map the player is certainly on, if any.
fn rows<'a>(
    saved: &[MapBests],
    this_visit: Option<(&str, usize)>,
    map: impl Fn(&str) -> Option<(&'a MapEntry, &'a MapData)>,
    here: Option<&str>,
    link: impl Fn(&MapBests) -> Option<String>,
) -> Vec<PersonalBestsMap> {
    let mut maps: Vec<PersonalBestsMap> = saved
        .iter()
        .filter_map(|bests| {
            let best = bests.visits.first()?;
            let (entry, data) = map(&bests.id).unzip();
            let ceiling = data.and_then(|d| d.summary).map(|s| s.exp_per_hour.solo).filter(|&c| c > 0);
            let visits = bests
                .visits
                .iter()
                .enumerate()
                .map(|(at, visit)| PersonalBestsVisit {
                    exp_per_hour: visit.exp_per_hour,
                    set_at_unix_ms: visit.set_at_unix_ms,
                    level: visit.level,
                    job: visit.job.clone(),
                    ceiling_share_hundredths: ceiling.map(|c| share_hundredths(visit.exp_per_hour, c)),
                    this_visit: this_visit == Some((bests.id.as_str(), at)),
                })
                .collect();
            Some(PersonalBestsMap {
                id: bests.id.clone(),
                street_name: entry.map_or_else(String::new, |e| e.street_name.clone()),
                map_name: entry.map_or_else(String::new, |e| e.map_name.clone()),
                region: entry.map_or_else(String::new, |e| e.region.clone()),
                mark: entry.and_then(|e| e.mark.clone()),
                url: meowdb::map_url(&bests.id),
                // A map not in the map list has no page on the site.
                leaderboard_url: entry.and_then(|_| link(bests)),
                best_of_all: false,
                mob_levels: data.and_then(mob_levels),
                exp_per_hour: best.exp_per_hour,
                solo_exp_ceiling: ceiling,
                ceiling_share_hundredths: ceiling.map(|c| share_hundredths(best.exp_per_hour, c)),
                set_at_unix_ms: best.set_at_unix_ms,
                here: here == Some(bests.id.as_str()),
                visits,
            })
        })
        .collect();
    maps.sort_by(|a, b| b.exp_per_hour.cmp(&a.exp_per_hour).then_with(|| a.id.cmp(&b.id)));
    maps
}

/// The lowest and highest of the map's mobs' levels.
fn mob_levels(data: &MapData) -> Option<LevelRange> {
    let lowest = data.mobs.iter().map(|m| m.level).min()?;
    let highest = data.mobs.iter().map(|m| m.level).max()?;
    Some(LevelRange { lowest, highest })
}

/// `exp_per_hour` as a share of `ceiling` (> 0), in hundredths of a percent, rounded, halves
/// up.
fn share_hundredths(exp_per_hour: u64, ceiling: u64) -> u64 {
    let scaled = u128::from(exp_per_hour) * 10_000;
    ((scaled + u128::from(ceiling) / 2) / u128::from(ceiling)) as u64
}
