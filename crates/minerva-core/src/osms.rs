//! OSMS Data Explorer's file set (ADR 0008): the raw texts the shell fetches from
//! [`DATA_URL`], and the reader that turns them into every map's data and every mob's
//! Mob Card figures and animation. Credit "OSMS Data Explorer (osmsdataexplorer.com)".

use crate::loader::Saved;
use crate::map_data::{MapData, MapSummary, Mob, PerParty};
use crate::map_mark;
use crate::map_index::{MapEntry, SourceError};
use crate::map_preview::{PicturePoint, SpawnMark};
use crate::mob_card::{DamageRange, Elements, HpPerExp, MobDetails};
use crate::portal_overrides::{PORTAL_OVERRIDES, PortalOverride};
use crate::portals::{self, Destination, Exit, MapFacts};
use crate::shown_map::LevelRange;
use serde::{Deserialize, Serialize};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};

/// Where OSMS Data Explorer publishes its current data set: each of [`OsmsFiles::NAMES`]
/// is a file under it.
pub const DATA_URL: &str = "https://osmsdataexplorer.com/data/current/";

/// Where OSMS Data Explorer keeps its pictures, each named by its content: a picture's
/// path is below it ([`crate::MapDataSource::picture`]).
pub const PICTURES_URL: &str = "https://osmsdataexplorer.com/data/";

/// OSMS's file set as fetched: each file's text, unread. Saved and swapped whole.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OsmsFiles {
    /// `maps.json`: every map by region, with its names, Town flag, nearest town, Mob
    /// Rate, exits and spawn points.
    pub maps: String,
    /// `portals.json`: every map's portals.
    pub portals: String,
    /// `monsters.json`: every mob's figures and the maps it spawns on.
    pub monsters: String,
    /// `map_manifest.json`: each map's picture, by content hash.
    pub map_manifest: String,
    /// `lookups.json`: names, among them those of maps outside `maps.json`.
    pub lookups: String,
}

impl OsmsFiles {
    /// The files' names under [`DATA_URL`], in the order of the fields.
    pub const NAMES: [&str; 5] = ["maps.json", "portals.json", "monsters.json", "map_manifest.json", "lookups.json"];
}

/// OSMS's file set as the source saved it, at an earlier launch or earlier in this one,
/// and when: its `files`, `saved_at_unix_ms` and the Minerva version it was `saved_by`
/// ([`crate::MINERVA_VERSION`]; `None` for a set saved before sets said so).
pub type SavedFiles = Saved<OsmsFiles>;

/// A map's row in the ceilings table ([`MapSet::solo_ceilings`]): its solo EXP/h ceiling,
/// the levels of the middle half of its spawns, and its Map Mark, by OSMS's name for it
/// (`None` for a map without one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoloCeiling {
    pub exp_per_hour: u64,
    pub mob_levels: LevelRange,
    pub mark: Option<String>,
}

/// A Map Mark as the site shows it beside a map's name: the area it stands for, as players
/// know it, and where OSMS keeps its picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteMark {
    pub name: String,
    pub picture_url: String,
}

/// An NPC standing on a map in the map list, as Search finds it (ADR 0028).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Npc {
    /// Its ID, unpadded, as OSMS gives it.
    pub id: String,
    pub name: String,
    /// Its picture's path ([`crate::MapDataSource::picture`]); `None` when OSMS has none.
    pub picture: Option<String>,
    /// Each map in the map list it stands on, in OSMS's order.
    pub maps: Vec<NpcOnMap>,
}

/// A map an NPC stands on, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcOnMap {
    pub map_id: String,
    /// Each spot it stands at in the map's picture, in OSMS's order: more than one when it
    /// stands in more than one, none when OSMS doesn't place it.
    pub spots: Vec<PicturePoint>,
}

impl Npc {
    /// Where it stands on map `map_id`; `None` when it doesn't stand there.
    pub(crate) fn spots_on(&self, map_id: &str) -> Option<&[PicturePoint]> {
        self.maps.iter().find(|on| on.map_id == map_id).map(|on| on.spots.as_slice())
    }
}

/// Every map's data from OSMS's file set: the map list, each listed map's figures, and
/// every mob's Mob Card figures.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapSet {
    /// In OSMS's order: by region, then as `maps.json` lists them.
    maps: Vec<(MapEntry, MapData)>,
    /// Every mob in `monsters.json`, by its ID.
    mobs: HashMap<String, MobDetails>,
    /// The path of each mob's default animation, by its ID; a mob OSMS names none for
    /// isn't here.
    animations: HashMap<String, String>,
    /// The path of each Map Mark's picture, by OSMS's name for the mark.
    marks: BTreeMap<String, String>,
    /// Every NPC that stands on a map in the map list, by ID.
    npcs: Vec<Npc>,
}

impl MapSet {
    /// Every map in the map list, with its data as read (mobs in any order).
    pub fn maps(&self) -> impl Iterator<Item = (&MapEntry, &MapData)> {
        self.maps.iter().map(|(entry, data)| (entry, data))
    }

    /// Each map's solo EXP-per-hour ceiling from its Map Summary, the levels of the middle
    /// half of its spawns and its Map Mark, by its 9-digit ID: the table the Map Leaderboards
    /// cap entries with (ADR 0015), and Where to train matches maps to a Level Band and an
    /// area by (ADR 0024). A map with no Map Summary, or a ceiling of 0, isn't in it.
    pub fn solo_ceilings(&self) -> BTreeMap<String, SoloCeiling> {
        let ceiling = |entry: &MapEntry, data: &MapData| {
            let exp_per_hour = data.summary.map(|s| s.exp_per_hour.solo).filter(|&solo| solo > 0)?;
            Some(SoloCeiling { exp_per_hour, mob_levels: data.middle_mob_levels()?, mark: entry.mark.clone() })
        };
        self.maps().filter_map(|(entry, data)| Some((entry.id.clone(), ceiling(entry, data)?))).collect()
    }

    /// Every Map Mark OSMS has a picture of, by OSMS's name for it, as the site shows it:
    /// the marks the ceilings table's maps name.
    pub fn site_marks(&self) -> BTreeMap<String, SiteMark> {
        self.marks()
            .map(|(mark, path)| {
                let site = SiteMark { name: map_mark::area_name(mark), picture_url: format!("{PICTURES_URL}{path}") };
                (mark.to_string(), site)
            })
            .collect()
    }

    /// The map list.
    pub(crate) fn entries(&self) -> Vec<MapEntry> {
        self.maps.iter().map(|(entry, _)| entry.clone()).collect()
    }

    /// Map `id`'s data, if it is in the map list.
    pub(crate) fn data(&self, id: &str) -> Option<&MapData> {
        self.maps.iter().find(|(entry, _)| entry.id == id).map(|(_, data)| data)
    }

    /// Map `id`'s entry in the map list, if it is in it.
    pub(crate) fn entry(&self, id: &str) -> Option<&MapEntry> {
        self.maps.iter().find(|(entry, _)| entry.id == id).map(|(entry, _)| entry)
    }

    /// Mob `id`'s Mob Card figures. Every mob a map lists has them.
    pub(crate) fn mob(&self, id: &str) -> Option<&MobDetails> {
        self.mobs.get(id)
    }

    /// The path of OSMS's default animation of mob `id` ([`crate::MapDataSource::picture`]),
    /// if OSMS names one.
    pub(crate) fn animation(&self, id: &str) -> Option<&str> {
        self.animations.get(id).map(String::as_str)
    }

    /// Every NPC that stands on a map in the map list.
    pub fn npcs(&self) -> &[Npc] {
        &self.npcs
    }

    /// NPC `id`, if it stands on a map in the map list.
    pub(crate) fn npc(&self, id: &str) -> Option<&Npc> {
        self.npcs.iter().find(|npc| npc.id == id)
    }

    /// Every Map Mark OSMS has a picture of: its name, and its picture's path
    /// ([`crate::MapDataSource::picture`]), by name.
    pub fn marks(&self) -> impl Iterator<Item = (&str, &str)> {
        self.marks.iter().map(|(name, path)| (name.as_str(), path.as_str()))
    }
}

/// Read OSMS's file set. All or nothing: a file that isn't JSON, or isn't shaped as the
/// reader expects, is [`SourceError::FormatChanged`], never partial figures.
///
/// Map IDs are zero-padded to 9 digits, and names trimmed. A map with no name or no
/// street can't be told by its title, so it is left out, and a portal to it leads to a
/// map not in the data.
///
/// A map's portals to other maps, with their kinds, and its Hidden Portals follow the
/// Hidden Portal rule and the app's hand-kept [`PORTAL_OVERRIDES`]. Teleports within the
/// map (`intra_map`) are neither, but they are marked on the Map Preview with every other
/// portal not to the dead end, at its position in the map's picture (named in
/// `map_manifest.json`), and so is every spawn point with a position. A destination
/// outside the map list is named from `lookups.json`, with no street, or else by its ID.
///
/// The NPCs are those `npc_lookup` names that stand on a map in the map list, with where
/// they stand there; an NPC it doesn't name can't be searched for, so it is left out.
pub fn read(files: &OsmsFiles) -> Result<MapSet, SourceError> {
    read_with(files, PORTAL_OVERRIDES)
}

/// [`read`], with these portal overrides in place of [`PORTAL_OVERRIDES`].
pub(crate) fn read_with(files: &OsmsFiles, overrides: &[PortalOverride]) -> Result<MapSet, SourceError> {
    read_set(files, overrides).map_err(|why| SourceError::FormatChanged(format!("OSMS: {why}")))
}

fn read_set(files: &OsmsFiles, overrides: &[PortalOverride]) -> Result<MapSet, String> {
    let maps: MapsFile = parse(&files.maps, "maps.json")?;
    let portals: HashMap<String, Vec<OsmsPortal>> = parse(&files.portals, "portals.json")?;
    let monsters: MonstersFile = parse(&files.monsters, "monsters.json")?;
    let pictures: HashMap<String, String> = parse(&files.map_manifest, "map_manifest.json")?;
    let lookups: LookupsFile = parse(&files.lookups, "lookups.json")?;
    let mobs: HashMap<u64, &OsmsMob> = monsters.monsters.iter().map(|mob| (mob.id, mob)).collect();
    let mut marks = BTreeMap::new();
    for (name, path) in &maps.map_mark_icons {
        marks.insert(name.clone(), map_mark::picture_path(path).map_err(|why| format!("map mark {name}: {why}"))?);
    }

    let mut listed = Vec::new();
    for map in maps.regions.iter().flat_map(|region| &region.maps) {
        let entry = MapEntry {
            id: map_id(map.id)?,
            street_name: map.street_name.trim().to_string(),
            map_name: map.name.trim().to_string(),
            region: map.region.clone(),
            // "None", or a mark without a picture: nothing to show.
            mark: map.map_mark.clone().filter(|mark| marks.contains_key(mark)),
        };
        listed.push((entry, map));
    }
    // A map with no name or no street can't be told by its title.
    let titled = |entry: &MapEntry| !entry.street_name.is_empty() && !entry.map_name.is_empty();
    let names: HashMap<&str, &MapEntry> =
        listed.iter().filter(|(entry, _)| titled(entry)).map(|(entry, _)| (entry.id.as_str(), entry)).collect();
    // A map's name and street, and whether it is in the map list; from the map list,
    // else lookups.json, else its ID.
    let looked_up = |id: &str| lookups.map_names.get(id).map(|name| name.trim()).filter(|name| !name.is_empty());
    let name_of = |id: &str| match (names.get(id), looked_up(id)) {
        (Some(entry), _) => Destination {
            map_name: entry.map_name.clone(),
            street_name: entry.street_name.clone(),
            mark: entry.mark.clone(),
            known: true,
        },
        (None, name) => Destination {
            map_name: name.unwrap_or(id).to_string(),
            street_name: String::new(),
            mark: None,
            known: false,
        },
    };

    let mut set = Vec::new();
    let mut npcs: BTreeMap<u64, Npc> = BTreeMap::new();
    for (entry, map) in &listed {
        let mut data = map_data(entry, map, portals.get(&entry.id), &mobs, &lookups.mob_names, &name_of, overrides)
            .map_err(|why| format!("map {}: {why}", entry.id))?;
        data.picture = pictures.get(&entry.id).cloned();
        if titled(entry) {
            for (id, spots) in npcs_on(map)? {
                let Some(named) = maps.npc_lookup.get(&id.to_string()) else { continue };
                let npc = match npcs.entry(id) {
                    Entry::Occupied(npc) => npc.into_mut(),
                    Entry::Vacant(at) => {
                        let picture = named.thumbnail.as_deref().map(map_mark::picture_path).transpose();
                        let picture = picture.map_err(|why| format!("npc {id}: {why}"))?;
                        at.insert(Npc { id: id.to_string(), name: named.name.trim().to_string(), picture, maps: Vec::new() })
                    }
                };
                npc.maps.push(NpcOnMap { map_id: entry.id.clone(), spots });
            }
            set.push((entry.clone(), data));
        }
    }
    let npcs = npcs.into_values().filter(|npc| !npc.name.is_empty()).collect();
    let mobs = monsters.monsters.iter().map(|mob| (mob.id.to_string(), mob_details(mob, &monsters.monsters))).collect();
    let mut animations = HashMap::new();
    for mob in &monsters.monsters {
        if let Some(hash) = &mob.gif {
            animations.insert(mob.id.to_string(), animation_path(hash).map_err(|why| format!("mob {}: {why}", mob.id))?);
        }
    }
    Ok(MapSet { maps: set, mobs, animations, marks, npcs })
}

/// The NPCs standing on `map`, each once in the order `npcs` lists them, with where it
/// stands in the map's picture (none, when `npc_positions` doesn't place it).
fn npcs_on(map: &OsmsMap) -> Result<Vec<(u64, Vec<PicturePoint>)>, String> {
    let mut on = Vec::new();
    for &id in &map.npcs {
        if on.iter().any(|&(listed, _)| listed == id) {
            continue;
        }
        let spots = map
            .npc_positions
            .iter()
            .filter(|spot| spot.id == id)
            .map(|spot| match (pixel(spot.x), pixel(spot.y)) {
                (Some(x), Some(y)) => Ok(PicturePoint { x, y }),
                _ => Err(format!("npc {id} at {}, {}", spot.x, spot.y)),
            })
            .collect::<Result<_, _>>()?;
        on.push((id, spots));
    }
    Ok(on)
}

/// Where OSMS keeps the mob animation with content hash `hash`, below [`PICTURES_URL`].
fn animation_path(hash: &str) -> Result<String, String> {
    if hash.is_empty() || !hash.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(format!("gif {hash:?} isn't a hash"));
    }
    Ok(format!("images/monsters/{hash}.webp"))
}

/// A mob's Mob Card figures, from its record among `all` of `monsters.json`.
///
/// Touch DMG is `floor(1.1 × P.ATK)` to `ceil(1.5 × P.ATK) − 1`, and Speed is "Slow" below
/// 0, "Fast" above 0 and otherwise "Normal": both rules fitted to MeowDB's pages. A figure
/// that is absent or 0 is `None`.
///
/// HP per EXP is ranked among all the set's mobs, the mobs in the game now: tied mobs
/// share a rank, and a mob without EXP has no HP per EXP and ranks below every other.
fn mob_details(mob: &OsmsMob, all: &[OsmsMob]) -> MobDetails {
    let p_atk = mob.p_atk.filter(|&p| p > 0).map(u64::from);
    let speed = match mob.speed {
        Some(speed) if speed < 0 => "Slow",
        Some(speed) if speed > 0 => "Fast",
        _ => "Normal",
    };
    let hp_per_exp = (mob.exp > 0).then(|| {
        let (hp, exp) = (u128::from(mob.hp), u128::from(mob.exp));
        // Compared exactly: a / b < c / d is a·d < c·b.
        let lower = all.iter().filter(|other| other.exp > 0 && u128::from(other.hp) * exp < hp * u128::from(other.exp));
        HpPerExp {
            // Rounded half up.
            hundredths: u64::try_from((hp * 200 + exp) / (2 * exp)).unwrap_or(u64::MAX),
            rank: u32::try_from(lower.count() + 1).unwrap_or(u32::MAX),
            ranked: u32::try_from(all.len()).unwrap_or(u32::MAX),
        }
    });
    let mut elements = Elements::default();
    for (element, affinity) in &mob.elements {
        let list = match affinity {
            Affinity::Weak => &mut elements.weak,
            Affinity::Strong => &mut elements.strong,
            Affinity::Immune => &mut elements.immune,
        };
        list.push(element.trim().to_string());
    }
    MobDetails {
        elements,
        touch_damage: p_atk.map(|p| DamageRange { min: 11 * p / 10, max: (3 * p).div_ceil(2) - 1 }),
        mp: mob.mp.filter(|&mp| mp > 0),
        accuracy: mob.acc.filter(|&acc| acc > 0),
        avoid: mob.eva.filter(|&eva| eva > 0),
        physical_defense: mob.p_def.filter(|&def| def > 0),
        magic_defense: mob.m_def.filter(|&def| def > 0),
        speed: speed.to_string(),
        hp_per_exp,
    }
}

/// One map's data, from its entry in `maps.json`, its `portals` and the `mobs`, with
/// `mob_names` from `lookups.json`.
fn map_data(
    entry: &MapEntry,
    map: &OsmsMap,
    portals: Option<&Vec<OsmsPortal>>,
    mobs: &HashMap<u64, &OsmsMob>,
    mob_names: &HashMap<String, String>,
    name_of: &impl Fn(&str) -> Destination,
    overrides: &[PortalOverride],
) -> Result<MapData, String> {
    let spawning = spawn_mobs(map, mobs, mob_names)?;
    let (summary, mobs) = spawns(map, &spawning, mobs)?;
    let exits = portals
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|p| {
            let to_map_id = map_id(p.dest_map.trim().parse().map_err(|_| format!("dest_map {:?}", p.dest_map))?)?;
            Ok(Exit {
                name: p.name.trim().to_string(),
                kind: p.kind,
                to: name_of(&to_map_id),
                to_map_id,
                within_map: p.intra_map,
                landing: p.dest_portal.trim().to_string(),
                at: PicturePoint {
                    x: pixel(p.x).ok_or_else(|| format!("portal {} x {}", p.name, p.x))?,
                    y: pixel(p.y).ok_or_else(|| format!("portal {} y {}", p.name, p.y))?,
                },
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let facts = MapFacts { id: &entry.id, is_town: map.is_town };
    let sorted = portals::portals(&facts, exits, overrides);
    let mob_rate_percent = match map.mob_rate {
        None => 100,
        Some(rate) if rate >= 0.0 && rate.is_finite() => (rate * 100.0).round() as u32,
        Some(rate) => return Err(format!("mob_rate {rate} isn't a rate")),
    };
    let nearest_town = match map.return_map_id.map(map_id).transpose()? {
        Some(town) if town == portals::DEAD_END => None,
        Some(town) => Some(town),
        None if map.is_town => Some(entry.id.clone()),
        None => None,
    };
    let connected_maps = map.exits.iter().map(|&id| map_id(id)).collect::<Result<_, _>>()?;
    let spawns = map
        .mob_positions
        .iter()
        .zip(&spawning)
        .filter_map(|(point, mob)| {
            let at = PicturePoint { x: pixel(point.x?)?, y: pixel(point.y?)? };
            Some(SpawnMark { at, mob: mob.to_string() })
        })
        .collect();
    Ok(MapData {
        summary,
        mobs,
        hidden_portals: sorted.hidden_portals,
        connected_maps,
        portals: sorted.portals,
        mob_rate_percent,
        nearest_town,
        marks: sorted.marks,
        spawns,
        // The panel picks these out when it shows the data; the reader adds the picture.
        ..MapData::default()
    })
}

/// How long the game takes to respawn a mob on an untimed spawn point, in seconds: a
/// full party kills each as soon as it spawns.
const RESPAWN_SECONDS: f64 = 7.56;

/// The share of a full party's kills a player alone gets on untimed spawn points.
const SOLO_SHARE: f64 = 0.75;

/// The mob each of `map`'s spawn points spawns, in their order: the ID of its record in
/// `monsters.json`.
///
/// OSMS folds some mobs into another of the same name: a spawn point may name a mob with
/// no record of its own, whose points the record of a mob of its name (`lookups.json`'s
/// `mob_names`) counts on the map with its own. That record is the one of its name on the
/// map whose `count` there is more than its own spawn points; with none, or more than
/// one, the file set isn't as the reader expects.
fn spawn_mobs(map: &OsmsMap, mobs: &HashMap<u64, &OsmsMob>, mob_names: &HashMap<String, String>) -> Result<Vec<u64>, String> {
    let points_of = |id: u64| map.mob_positions.iter().filter(|point| point.id == id).count() as u64;
    let folds_more = |mob: &OsmsMob| mob.maps.iter().any(|on| on.id == map.id && on.count > points_of(mob.id));
    map.mob_positions
        .iter()
        .map(|point| {
            if mobs.contains_key(&point.id) {
                return Ok(point.id);
            }
            let unlisted = || format!("mob {} isn't in monsters.json", point.id);
            let name = mob_names.get(&point.id.to_string()).map(|name| name.trim()).ok_or_else(unlisted)?;
            let folding: Vec<u64> =
                mobs.values().filter(|mob| mob.name.trim() == name && folds_more(mob)).map(|mob| mob.id).collect();
            match folding[..] {
                [id] => Ok(id),
                _ => Err(format!("{}, nor folded into one {name} on the map", unlisted())),
            }
        })
        .collect()
}

/// A map's mobs, from its spawn points grouped by mob (`spawning`, as [`spawn_mobs`]
/// gives them), with their figures from the mobs' records; and its Map Summary, worked out
/// as MeowDB works it out (`None` without mobs).
///
/// An untimed spawn point gives a full party `3600 / 7.56` mobs an hour, and a player
/// alone 0.75 of that. A timed one gives `3600 / timer`, alone or in a party; a ranged
/// timer, the average OSMS gives. Each mob's EXP an hour is its EXP times its mobs an
/// hour. Each mob's figures are rounded, the map's are their sums, and the map's Mob
/// Rate isn't applied.
fn spawns(map: &OsmsMap, spawning: &[u64], mobs: &HashMap<u64, &OsmsMob>) -> Result<(Option<MapSummary>, Vec<Mob>), String> {
    let mut points: Vec<(u64, u64)> = Vec::new();
    for &id in spawning {
        match points.iter_mut().find(|(mob, _)| *mob == id) {
            Some((_, count)) => *count += 1,
            None => points.push((id, 1)),
        }
    }
    let total: u64 = points.iter().map(|(_, count)| count).sum();
    let mut sums = [0u64; 4];
    let mut listed = Vec::new();
    for (id, count) in points {
        let mob = mobs.get(&id).ok_or_else(|| format!("mob {id} isn't in monsters.json"))?;
        let timer = mob.maps.iter().find(|on| on.id == map.id).and_then(|on| on.mob_time.as_deref());
        let (solo, full_party) = match timer.map(seconds).transpose()? {
            Some(timer) => (3600.0 / timer, 3600.0 / timer),
            None => (SOLO_SHARE * 3600.0 / RESPAWN_SECONDS, 3600.0 / RESPAWN_SECONDS),
        };
        let (solo, full_party) = (count as f64 * solo, count as f64 * full_party);
        let exp = mob.exp as f64;
        for (sum, figure) in sums.iter_mut().zip([solo, full_party, solo * exp, full_party * exp]) {
            *sum += figure.round() as u64;
        }
        listed.push(Mob {
            id: id.to_string(),
            name: mob.name.trim().to_string(),
            level: mob.level,
            hp: mob.hp,
            exp: mob.exp,
            spawn_count: u32::try_from(count).map_err(|e| e.to_string())?,
            // Rounded half up, as MeowDB shows it.
            spawn_share_percent: ((count * 200 + total) / (2 * total)) as u32,
        });
    }
    let [mobs_solo, mobs_party, exp_solo, exp_party] = sums;
    let summary = (!listed.is_empty()).then_some(MapSummary {
        mobs_per_hour: PerParty { solo: mobs_solo, full_party: mobs_party },
        exp_per_hour: PerParty { solo: exp_solo, full_party: exp_party },
    });
    Ok((summary, listed))
}

/// A spawn timer in seconds: `"300.00"`, or for a ranged timer the average in brackets,
/// `"180.00-1800.00 [720.00]"`.
fn seconds(timer: &str) -> Result<f64, String> {
    let number = |s: &str| s.trim().parse::<f64>().ok().filter(|n| n.is_finite() && *n > 0.0);
    let average = match timer.split_once('[') {
        Some((range, average)) => {
            let bounds = range.split_once('-').and_then(|(low, high)| Some((number(low)?, number(high)?)));
            bounds.and(average.strip_suffix(']')).and_then(number)
        }
        None => number(timer),
    };
    average.ok_or_else(|| format!("mob_time {timer:?} isn't a timer"))
}

/// One of the set's files, read into `T`.
fn parse<T: serde::de::DeserializeOwned>(text: &str, file: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|e| format!("{file}: {e}"))
}

/// A map ID as Minerva writes it: 9 digits, zero padding kept.
fn map_id(id: u64) -> Result<String, String> {
    let padded = format!("{id:09}");
    if padded.len() > 9 {
        return Err(format!("map ID {id} has more than 9 digits"));
    }
    Ok(padded)
}

/// `maps.json`.
#[derive(Deserialize)]
struct MapsFile {
    regions: Vec<Region>,
    /// Each Map Mark's picture, by the mark's name, below [`DATA_URL`]
    /// (`"images/map_marks/Henesys.png"`). Without it, no map has a mark.
    #[serde(default)]
    map_mark_icons: BTreeMap<String, String>,
    /// Every NPC, by its ID unpadded.
    #[serde(default)]
    npc_lookup: HashMap<String, OsmsNpc>,
}

/// An NPC in `maps.json`'s `npc_lookup`.
#[derive(Deserialize)]
struct OsmsNpc {
    name: String,
    /// Its picture, below [`DATA_URL`] (`"images/npcs/0000001.png"`); `null` for none.
    thumbnail: Option<String>,
}

/// Where an NPC stands on a map, in the pixels of the map's picture.
#[derive(Deserialize)]
struct NpcSpot {
    id: u64,
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
struct Region {
    maps: Vec<OsmsMap>,
}

/// A map in `maps.json`. Optional fields are absent, never null.
#[derive(Deserialize)]
struct OsmsMap {
    id: u64,
    name: String,
    street_name: String,
    region: String,
    #[serde(default)]
    is_town: bool,
    /// Its nearest town; absent on some Towns (meaning the Town itself) and a few other
    /// maps (meaning none).
    return_map_id: Option<u64>,
    /// Absent: ×1.
    mob_rate: Option<f64>,
    /// The maps its portals lead to.
    #[serde(default)]
    exits: Vec<u64>,
    /// Its Map Mark's name, a key of `map_mark_icons`; `"None"` for none.
    map_mark: Option<String>,
    /// One per spawn point.
    #[serde(default)]
    mob_positions: Vec<SpawnPoint>,
    /// The NPCs standing on it, by ID; absent for none.
    #[serde(default)]
    npcs: Vec<u64>,
    /// Where each of them stands; one NPC may stand in more than one spot.
    #[serde(default)]
    npc_positions: Vec<NpcSpot>,
}

/// A pixel of a map's picture, as OSMS gives a portal's position: whole, though read
/// as any number.
fn pixel(at: f64) -> Option<i32> {
    (at.is_finite() && at.abs() < f64::from(i32::MAX)).then(|| at.round() as i32)
}

/// A portal in `portals.json`.
#[derive(Deserialize)]
struct OsmsPortal {
    name: String,
    #[serde(rename = "type")]
    kind: u64,
    /// The destination's ID as text, without zero padding.
    dest_map: String,
    /// The portal it lands at on its destination, by name; empty to the dead end.
    #[serde(default)]
    dest_portal: String,
    /// Where it is, in the pixels of the map's picture.
    x: f64,
    y: f64,
    /// A teleport within the map.
    #[serde(default)]
    intra_map: bool,
}

/// `lookups.json`.
#[derive(Deserialize)]
struct LookupsFile {
    /// By 9-digit map ID.
    map_names: HashMap<String, String>,
    /// By mob ID, unpadded: the mobs `monsters.json` folds into another among them.
    #[serde(default)]
    mob_names: HashMap<String, String>,
}

#[derive(Deserialize)]
struct SpawnPoint {
    /// The mob's ID.
    id: u64,
    /// Where it is, in the pixels of the map's picture; a point without both still counts.
    x: Option<f64>,
    y: Option<f64>,
}

/// `monsters.json`.
#[derive(Deserialize)]
struct MonstersFile {
    monsters: Vec<OsmsMob>,
}

/// A mob in `monsters.json`. Optional fields are absent, never null.
#[derive(Deserialize)]
struct OsmsMob {
    id: u64,
    name: String,
    level: u32,
    hp: u64,
    exp: u64,
    /// P.ATK, what Touch DMG is worked out from.
    #[serde(rename = "PADamage")]
    p_atk: Option<u32>,
    #[serde(rename = "PDDamage")]
    p_def: Option<u32>,
    #[serde(rename = "MDDamage")]
    m_def: Option<u32>,
    mp: Option<u64>,
    acc: Option<u32>,
    /// AVOID.
    eva: Option<u32>,
    /// Below 0 slow, above 0 fast.
    speed: Option<i64>,
    /// Each element it has an affinity to ("Fire"), in name order; absent for none.
    #[serde(default)]
    elements: BTreeMap<String, Affinity>,
    /// The content hash of OSMS's default animation of it, one of its `gifs`.
    gif: Option<String>,
    /// The maps it spawns on; absent for a mob on none.
    #[serde(default)]
    maps: Vec<MobOnMap>,
}

/// A mob's affinity to an element. Any other is a changed format.
#[derive(Deserialize)]
enum Affinity {
    Weak,
    Strong,
    Immune,
}

#[derive(Deserialize)]
struct MobOnMap {
    id: u64,
    /// Its spawn points on the map, those of the mobs folded into it among them.
    #[serde(default)]
    count: u64,
    /// Its spawn points' timer on the map, if they're timed.
    mob_time: Option<String>,
}
