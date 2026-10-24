//! One map's hunting information, and how the panel shows it.

use crate::map_preview::{PortalMark, SpawnMark};
use crate::shown_map::LevelRange;
use serde::{Deserialize, Serialize};

/// One map's data, as read from OSMS's file set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapData {
    /// `None` on a map without mobs, such as a town.
    pub summary: Option<MapSummary>,
    /// In the panel, sorted by level (then name); from a source, in any order.
    pub mobs: Vec<Mob>,
    /// The exits the player can't see, in the source's order. Empty when there are none.
    pub hidden_portals: Vec<HiddenPortal>,
    /// The IDs of the maps its portals lead to. They tell apart maps sharing a title
    /// (the one the player came from is connected to the right one); not rendered.
    #[serde(skip_serializing)]
    pub connected_maps: Vec<String>,
    /// Every portal to another map (not to the map itself or the dead end), with its
    /// kind, in the source's order: the map's edges in the world's portal graph. Not
    /// rendered.
    #[serde(skip_serializing)]
    pub portals: Vec<Portal>,
    /// The map's Mob Rate in hundredths: 150 is a 1.5x respawn rate.
    #[serde(skip_serializing)]
    pub mob_rate_percent: u32,
    /// The map's nearest town, where a nearest-town Teleport Scroll goes; a town's is
    /// itself. `None` when there is none. Not
    /// rendered.
    #[serde(skip_serializing)]
    pub nearest_town: Option<String>,
    /// The content-hash name of OSMS's picture of the map, from `map_manifest.json`;
    /// `None` when it has none. Rendered in the Map Preview, not here.
    #[serde(skip_serializing)]
    pub picture: Option<String>,
    /// Every portal on the map but those to the dead end, marked as the Map Preview marks
    /// them, in the source's order. Rendered in the Map Preview, not here.
    #[serde(skip_serializing)]
    pub marks: Vec<PortalMark>,
    /// Every Spawn Point with a position, dotted as the Map Preview dots them, in the
    /// source's order. Rendered in the Map Preview, not here.
    #[serde(skip_serializing)]
    pub spawns: Vec<SpawnMark>,
}

/// A portal from a map to another map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Portal {
    /// The portal's name on its map (e.g. `"in00"`).
    pub name: String,
    pub to_map_id: String,
    pub kind: PortalKind,
}

/// What kind of portal it is, as the Hidden Portal rule and the portal overrides tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PortalKind {
    /// The player can see it: on the minimap, or a walk-into exit.
    Visible,
    /// A Hidden Portal that works for anyone.
    Hidden,
    /// A town's portal into a building, drawn into the scenery.
    Door,
    /// A Quest Portal: it only works while the player meets a quest condition.
    Quest,
}

/// A portal to another map that is drawn neither on the minimap nor in the world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenPortal {
    /// The portal's name on its map (e.g. `"in00"`); with the map ID, what
    /// the portal override list is keyed by.
    pub name: String,
    pub to_map_id: String,
    pub to_map_name: String,
    pub to_street_name: String,
    /// The destination's Map Mark, by OSMS's name for it (see [`crate::PanelState::map_marks`]);
    /// `None` for a map without one.
    pub to_mark: Option<String>,
    /// The destination is a Hidden Street.
    pub to_hidden_street: bool,
    /// A Quest Portal: it only works while the player meets a quest condition.
    pub quest: bool,
}

/// The Map Summary: the ceiling figures (every mob killed as soon as it spawns), worked
/// out as MeowDB works them out, with the map's mob rate not applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapSummary {
    pub mobs_per_hour: PerParty,
    /// The EXP-per-hour ceiling.
    pub exp_per_hour: PerParty,
}

/// A figure for a player alone on the map, and for a party of 6 or more: MeowDB's two;
/// sizes in between are not derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerParty {
    pub solo: u64,
    pub full_party: u64,
}

/// A mob that spawns on the map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mob {
    /// The mob's ID (e.g. `"3"`, Blue Snail), MeowDB's and OSMS's: the same mob on every map.
    pub id: String,
    pub name: String,
    pub level: u32,
    pub hp: u64,
    pub exp: u64,
    /// How many of this mob the map holds at once.
    pub spawn_count: u32,
    /// Its spawn count as a share of all the map's spawns, in whole percent.
    pub spawn_share_percent: u32,
}

/// The shown map's data, from OSMS's file set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum MapDataStatus {
    /// No map is shown.
    #[default]
    NoMap,
    Ready(MapData),
    /// From a copy of the file set saved at `saved_at_unix_ms` (ms since the Unix epoch),
    /// over 7 days ago, because OSMS couldn't be reached since: its figures, to be shown
    /// as such.
    SavedCopy {
        #[serde(rename = "savedAtUnixMs")]
        saved_at_unix_ms: u64,
        #[serde(flatten)]
        data: MapData,
    },
}

impl MapDataStatus {
    /// The map's data, fresh or from a saved copy.
    pub fn data(&self) -> Option<&MapData> {
        match self {
            Self::Ready(data) | Self::SavedCopy { data, .. } => Some(data),
            _ => None,
        }
    }

    /// The map's solo EXP-per-hour ceiling, from its Map Summary; `None` without data
    /// or a Map Summary (a town).
    pub fn solo_exp_ceiling(&self) -> Option<u64> {
        self.data()?.summary.map(|s| s.exp_per_hour.solo)
    }
}

impl MapData {
    /// Whether the panel would show the same for both maps: the same Map Summary, mobs
    /// and Hidden Portals, those by destination and kind. (Mobs sorted as the panel sorts
    /// them.)
    pub(crate) fn shows_same_as(&self, other: &MapData) -> bool {
        let portals = |data: &MapData| -> Vec<(String, bool)> {
            data.hidden_portals.iter().map(|p| (p.to_map_id.clone(), p.quest)).collect()
        };
        self.summary == other.summary && self.mobs == other.mobs && portals(self) == portals(other)
    }

    /// The lowest and highest of its mobs' levels, its mobs in any order; `None` with no
    /// mobs.
    pub(crate) fn mob_levels(&self) -> Option<LevelRange> {
        let lowest = self.mobs.iter().map(|m| m.level).min()?;
        let highest = self.mobs.iter().map(|m| m.level).max()?;
        Some(LevelRange { lowest, highest })
    }

    /// The levels of the middle half of its spawns: from the level a quarter of them are at
    /// or under to the level three quarters are, each mob counted as often as the map holds
    /// it at once. A mob a few of its spawns are, much lower or higher than the rest, leaves
    /// it as it is. With no spawns counted, all its mobs' levels; `None` with no mobs.
    pub(crate) fn middle_mob_levels(&self) -> Option<LevelRange> {
        let total: u64 = self.mobs.iter().map(|m| u64::from(m.spawn_count)).sum();
        if total == 0 {
            return self.mob_levels();
        }
        let mut by_level: Vec<&Mob> = self.mobs.iter().collect();
        by_level.sort_by_key(|m| m.level);
        // The level the spawns reach `quarters` quarters of the total at.
        let at = |quarters: u64| {
            let mut counted = 0;
            by_level.iter().find_map(|m| {
                counted += u64::from(m.spawn_count);
                (counted * 4 >= total * quarters).then_some(m.level)
            })
        };
        Some(LevelRange { lowest: at(1)?, highest: at(3)? })
    }
}

/// A source's data as the panel shows it: every mob, sorted by level (then name).
pub(crate) fn for_panel(mut data: MapData) -> MapData {
    data.mobs.sort_by(|a, b| a.level.cmp(&b.level).then_with(|| a.name.cmp(&b.name)));
    data
}
