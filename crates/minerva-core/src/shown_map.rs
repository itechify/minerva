//! Which map the panel shows for the confirmed Minimap Title, and that map's data.
//!
//! Usually one map has the title. When several share it, the panel shows them without
//! asking if they would all show the same; otherwise it shows the first, outlines each
//! of them so the player can tell them apart, and lets the player choose. A choice
//! holds until the title changes.

use crate::CurrentMap;
use crate::map_data::{MapData, MapDataStatus, for_panel};
use crate::map_preview::{MapPreview, PreviewImage};
use crate::pictures::Pictures;
use crate::osms::MapSet;
use serde::Serialize;

/// Maps that share one Minimap Title and would show different content, with neither
/// the map the player came from nor the player having settled which it is. The panel
/// shows one of them; the choose-candidate hotkey shows the next.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidates {
    /// In map list order.
    pub maps: Vec<Candidate>,
    /// Which of them the panel shows (an index into `maps`).
    pub shown: usize,
    /// The player chose the shown one with the hotkey. The choice holds until the
    /// map changes.
    pub chosen: bool,
}

/// One of the maps sharing the title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// The map's 9-digit ID.
    pub id: String,
    /// What sets it apart from the others.
    pub outline: Outline,
}

/// A map at a glance: enough to tell apart maps that share a title.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outline {
    /// The lowest and highest mob level; `None` on a map without mobs.
    pub mob_levels: Option<LevelRange>,
    /// The names of its lowest-level mobs, up to [`FIRST_MOBS`], lowest first.
    pub first_mobs: Vec<String>,
    /// How many Hidden Portals it has.
    pub hidden_portals: usize,
}

/// A range of mob levels, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelRange {
    pub lowest: u32,
    pub highest: u32,
}

/// How many mobs an [`Outline`] names.
pub const FIRST_MOBS: usize = 3;

impl Outline {
    fn of(data: &MapData) -> Self {
        // The panel's mobs are sorted by level.
        let mob_levels = match (data.mobs.first(), data.mobs.last()) {
            (Some(lowest), Some(highest)) => Some(LevelRange { lowest: lowest.level, highest: highest.level }),
            _ => None,
        };
        Self {
            mob_levels,
            first_mobs: data.mobs.iter().take(FIRST_MOBS).map(|m| m.name.clone()).collect(),
            hidden_portals: data.hidden_portals.len(),
        }
    }
}

#[derive(Default)]
pub(crate) struct ShownMap {
    /// The maps with the confirmed title, in map list order; empty before there is one.
    maps: Vec<CurrentMap>,
    settling: Settling,
}

/// Whether the core settled which of the maps sharing the title to show.
#[derive(Clone, Copy)]
enum Settling {
    /// The core knows which map it is (`maps[i]`).
    Settled(usize),
    /// They would all show the same, so it shows the first without asking; but which of
    /// them the player is on is still open.
    Alike,
    /// They show different things and nothing settled which: the player may choose.
    Choosing { shown: usize, chosen: bool },
}

impl Default for Settling {
    fn default() -> Self {
        Self::Settled(0)
    }
}

impl ShownMap {
    /// The title confirmed on the latest frame belongs to `maps` (none: no title yet),
    /// whose data is in `set`. Whether it is another title than before: the player
    /// arrived on another map, a new Visit.
    pub fn show(&mut self, maps: Vec<CurrentMap>, set: &MapSet) -> bool {
        let arrived = maps != self.maps;
        if arrived {
            let came_from = self.connected_maps(set);
            // The map the player came from settles it when it leads to just one of
            // them; otherwise it doesn't matter if they all show the same.
            let mut connected = maps.iter().enumerate().filter(|(_, m)| came_from.contains(&m.id));
            self.settling = match (maps.len(), connected.next(), connected.next()) {
                (0 | 1, _, _) => Settling::Settled(0),
                (_, Some((i, _)), None) => Settling::Settled(i),
                _ if all_show_the_same(&maps, set) => Settling::Alike,
                _ => Settling::Choosing { shown: 0, chosen: false },
            };
            self.maps = maps;
        }
        arrived
    }

    /// The maps the shown map leads to, if the core is sure it's the player's;
    /// otherwise none.
    fn connected_maps(&self, set: &MapSet) -> Vec<String> {
        let sure = match self.settling {
            Settling::Settled(_) | Settling::Alike | Settling::Choosing { chosen: true, .. } => true,
            Settling::Choosing { chosen: false, .. } => false,
        };
        match (sure, self.map().and_then(|m| set.data(&m.id))) {
            (true, Some(data)) => data.connected_maps.clone(),
            _ => Vec::new(),
        }
    }

    /// The maps the player could be on now, going by the map shown: that map, and
    /// those it leads to (when the core is sure it's the player's).
    pub fn within_reach(&self, set: &MapSet) -> Vec<String> {
        let mut maps = self.connected_maps(set);
        maps.extend(self.map().map(|m| m.id.clone()));
        maps
    }

    /// Show the next of the maps sharing the title, if the player may choose.
    pub fn choose_next(&mut self) {
        if let Settling::Choosing { shown, .. } = self.settling {
            self.settling = Settling::Choosing { shown: (shown + 1) % self.maps.len(), chosen: true };
        }
    }

    pub fn map(&self) -> Option<&CurrentMap> {
        self.maps.get(self.shown())
    }

    /// The shown map's Map Mark. While it isn't settled which of the maps sharing the title
    /// the player is on, only one they all share: the mark says where the player is.
    pub fn mark(&self) -> Option<String> {
        let mark = self.map()?.mark.clone();
        match self.settling {
            Settling::Settled(_) | Settling::Choosing { chosen: true, .. } => mark,
            Settling::Alike | Settling::Choosing { chosen: false, .. } => {
                mark.filter(|mark| self.maps.iter().all(|m| m.mark.as_ref() == Some(mark)))
            }
        }
    }

    /// The shown map's data from `set`, as the panel shows it: a Saved Copy, saved at
    /// `saved_copy_from`, when the set is one the panel must say is old.
    pub fn data(&self, set: &MapSet, saved_copy_from: Option<u64>) -> MapDataStatus {
        let Some(data) = self.map().and_then(|m| set.data(&m.id)) else { return MapDataStatus::NoMap };
        let data = for_panel(data.clone());
        match saved_copy_from {
            Some(saved_at_unix_ms) => MapDataStatus::SavedCopy { saved_at_unix_ms, data },
            None => MapDataStatus::Ready(data),
        }
    }

    /// The shown map's Map Preview, from `set`, its picture as `pictures` has it: `None`
    /// when it has no picture, and unless it's settled which map the player is on (each
    /// has its own layout).
    pub fn preview(&self, set: &MapSet, pictures: &Pictures) -> Option<MapPreview> {
        if !matches!(self.settling, Settling::Settled(_) | Settling::Choosing { chosen: true, .. }) {
            return None;
        }
        let data = set.data(&self.map()?.id)?;
        let picture = data.picture.clone()?;
        let image = PreviewImage::of(&picture, pictures);
        Some(MapPreview { picture, image, marks: data.marks.clone(), spawns: data.spawns.clone(), npc_spots: Vec::new() })
    }

    pub fn candidates(&self, set: &MapSet) -> Option<Candidates> {
        let Settling::Choosing { shown, chosen } = self.settling else { return None };
        let outline = |id: &str| set.data(id).map(|data| Outline::of(&for_panel(data.clone())));
        let maps = self
            .maps
            .iter()
            .map(|m| Candidate { id: m.id.clone(), outline: outline(&m.id).unwrap_or_default() })
            .collect();
        Some(Candidates { maps, shown, chosen })
    }

    fn shown(&self) -> usize {
        match self.settling {
            Settling::Settled(i) | Settling::Choosing { shown: i, .. } => i,
            Settling::Alike => 0,
        }
    }
}

/// Whether every one of `maps` would show the same as the first.
fn all_show_the_same(maps: &[CurrentMap], set: &MapSet) -> bool {
    let data = |m: &CurrentMap| set.data(&m.id).map(|data| for_panel(data.clone()));
    let first = maps.first().and_then(data);
    maps.iter().all(|m| match (&first, data(m)) {
        (Some(first), Some(this)) => first.shows_same_as(&this),
        _ => false,
    })
}
