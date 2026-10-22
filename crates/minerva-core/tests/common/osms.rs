//! Made-up worlds in OSMS Data Explorer's exact JSON shape (ADR 0008): invented maps,
//! portals and mobs, written out as the five files the shell fetches. No OSMS file is
//! committed; the one list taken from OSMS is the map names fixture, which [`game`]
//! builds on.

use super::fixtures_dir;
use minerva_core::osms::OsmsFiles;
use serde::Deserialize;
use serde_json::{Map as Object, Value, json};

/// A map in a made-up world, as `maps.json` and `portals.json` hold it.
#[derive(Debug, Clone)]
pub struct Map {
    pub id: u64,
    pub street_name: String,
    pub name: String,
    pub region: String,
    pub town: bool,
    /// `return_map_id`, left out when `None`.
    pub return_map: Option<u64>,
    /// `mob_rate`, left out when `None`.
    pub mob_rate: Option<f64>,
    /// `exits` as given; `None`: the maps its portals lead to.
    pub exits: Option<Vec<u64>>,
    pub spawns: Vec<Spawn>,
    pub portals: Vec<Portal>,
    /// `map_mark`; written as `"None"` when `None`, as OSMS writes a map without one.
    pub mark: Option<String>,
}

/// Some of a mob's spawn points on a map.
#[derive(Debug, Clone)]
pub struct Spawn {
    pub mob: u64,
    pub points: u32,
    /// The mob's `mob_time` on the map ("300.00", "180.00-1800.00 [720.00]"), if timed.
    pub timer: Option<String>,
    /// Where its points are in the map's picture.
    pub at: SpawnAt,
}

/// Where a [`Spawn`]'s points are in the map's picture.
#[derive(Debug, Clone, Copy, Default)]
pub enum SpawnAt {
    /// Each at a spot of its own along the map.
    #[default]
    Spread,
    /// At (`x`, `y`).
    Here(i64, i64),
    /// Nowhere: no `x` or `y`.
    Unplaced,
}

/// A portal, as `portals.json` holds it.
#[derive(Debug, Clone)]
pub struct Portal {
    pub name: String,
    /// OSMS's portal type: 1 invisible, 2 visible, 3 minimap-visible, 10 hidden, 11 script.
    pub kind: u64,
    pub to: u64,
    pub landing: String,
    pub x: i64,
    pub y: i64,
    /// `intra_map: true`: a teleport within the map.
    pub within_map: bool,
}

/// A mob, as `monsters.json` holds it. A figure that is `None` is left out.
#[derive(Debug, Clone)]
pub struct Mob {
    pub id: u64,
    pub name: String,
    pub level: u32,
    pub hp: u64,
    pub exp: u64,
    /// `PADamage`.
    pub p_atk: Option<u64>,
    /// `PDDamage`.
    pub p_def: Option<u64>,
    /// `MDDamage`.
    pub m_def: Option<u64>,
    pub mp: Option<u64>,
    pub acc: Option<u64>,
    /// `eva`.
    pub avoid: Option<u64>,
    pub speed: Option<i64>,
    /// `elements`: each element and its affinity ("Weak", "Strong", "Immune").
    pub elements: Vec<(String, String)>,
    /// `gif`, the hash of OSMS's default animation for it: `"gif<id>"` unless set.
    pub animation: Option<String>,
}

/// A map on Victoria Island with nothing on it.
pub fn map(id: u64, street_name: &str, name: &str) -> Map {
    Map {
        id,
        street_name: street_name.into(),
        name: name.into(),
        region: "Victoria Island".into(),
        town: false,
        return_map: None,
        mob_rate: None,
        exits: None,
        spawns: Vec::new(),
        portals: Vec::new(),
        mark: None,
    }
}

/// A mob with no figures but these, and its default animation `"gif<id>"`.
pub fn mob(id: u64, name: &str, level: u32, hp: u64, exp: u64) -> Mob {
    Mob {
        id,
        name: name.into(),
        level,
        hp,
        exp,
        p_atk: None,
        p_def: None,
        m_def: None,
        mp: None,
        acc: None,
        avoid: None,
        speed: None,
        elements: Vec::new(),
        animation: Some(format!("gif{id}")),
    }
}

impl Mob {
    pub fn p_atk(mut self, p_atk: u64) -> Self {
        self.p_atk = Some(p_atk);
        self
    }

    pub fn p_def(mut self, p_def: u64) -> Self {
        self.p_def = Some(p_def);
        self
    }

    pub fn m_def(mut self, m_def: u64) -> Self {
        self.m_def = Some(m_def);
        self
    }

    pub fn mp(mut self, mp: u64) -> Self {
        self.mp = Some(mp);
        self
    }

    pub fn acc(mut self, acc: u64) -> Self {
        self.acc = Some(acc);
        self
    }

    pub fn avoid(mut self, avoid: u64) -> Self {
        self.avoid = Some(avoid);
        self
    }

    pub fn speed(mut self, speed: i64) -> Self {
        self.speed = Some(speed);
        self
    }

    /// Its affinity (`"Weak"`, `"Strong"`, `"Immune"`) to `element` (`"Fire"`).
    pub fn element(mut self, element: &str, affinity: &str) -> Self {
        self.elements.push((element.into(), affinity.into()));
        self
    }

    /// The hash of its default animation (`gif`).
    pub fn animation(mut self, hash: &str) -> Self {
        self.animation = Some(hash.into());
        self
    }

    /// No default animation: `gif` left out.
    pub fn without_animation(mut self) -> Self {
        self.animation = None;
        self
    }
}

impl Map {
    pub fn in_region(mut self, region: &str) -> Self {
        self.region = region.into();
        self
    }

    /// A Town (`is_town: true`).
    pub fn town(mut self) -> Self {
        self.town = true;
        self
    }

    /// Its nearest town (`return_map_id`).
    pub fn returning_to(mut self, id: u64) -> Self {
        self.return_map = Some(id);
        self
    }

    pub fn mob_rate(mut self, rate: f64) -> Self {
        self.mob_rate = Some(rate);
        self
    }

    /// `points` spawn points of mob `mob`, untimed.
    pub fn spawns(mut self, mob: u64, points: u32) -> Self {
        self.spawns.push(Spawn { mob, points, timer: None, at: SpawnAt::Spread });
        self
    }

    /// A spawn point of mob `mob`, untimed, at (`x`, `y`) in the map's picture.
    pub fn spawn_at(mut self, mob: u64, x: i64, y: i64) -> Self {
        self.spawns.push(Spawn { mob, points: 1, timer: None, at: SpawnAt::Here(x, y) });
        self
    }

    /// A spawn point of mob `mob`, untimed, with no position.
    pub fn spawn_unplaced(mut self, mob: u64) -> Self {
        self.spawns.push(Spawn { mob, points: 1, timer: None, at: SpawnAt::Unplaced });
        self
    }

    /// `points` spawn points of mob `mob`, each on this timer.
    pub fn timed(mut self, mob: u64, points: u32, timer: &str) -> Self {
        self.spawns.push(Spawn { mob, points, timer: Some(timer.into()), at: SpawnAt::Spread });
        self
    }

    /// A portal of OSMS type `kind`, named `name`, to map `to`.
    pub fn portal(self, name: &str, kind: u64, to: u64) -> Self {
        let at = 100 + 50 * self.portals.len() as i64;
        self.portal_at(name, kind, to, at, 300)
    }

    /// As [`Map::portal`], at (`x`, `y`) in the map's picture.
    pub fn portal_at(mut self, name: &str, kind: u64, to: u64, x: i64, y: i64) -> Self {
        let landing = if to == DEAD_END { String::new() } else { "sp".into() };
        self.portals.push(Portal { name: name.into(), kind, to, landing, x, y, within_map: false });
        self
    }

    /// An invisible teleport within the map, named `name`, landing at portal `landing`.
    pub fn teleport(mut self, name: &str, landing: &str) -> Self {
        let at = 100 + 50 * self.portals.len() as i64;
        let id = self.id;
        self.portals.push(Portal {
            name: name.into(),
            kind: 1,
            to: id,
            landing: landing.into(),
            x: at,
            y: 200,
            within_map: true,
        });
        self
    }

    /// The portal added last, moved to (`x`, `y`) in the map's picture.
    pub fn at(mut self, x: i64, y: i64) -> Self {
        let portal = self.portals.last_mut().expect("a portal to move");
        (portal.x, portal.y) = (x, y);
        self
    }

    /// Its Map Mark, by the mark's name (`map_mark`).
    pub fn marked(mut self, mark: &str) -> Self {
        self.mark = Some(mark.into());
        self
    }

    /// `exits` as given, whatever its portals.
    pub fn exits(mut self, exits: &[u64]) -> Self {
        self.exits = Some(exits.to_vec());
        self
    }

    /// The maps its portals lead to, each once: OSMS's `exits`, unless given.
    fn exit_ids(&self) -> Vec<u64> {
        if let Some(exits) = &self.exits {
            return exits.clone();
        }
        let mut exits = Vec::new();
        for p in &self.portals {
            if p.to != self.id && p.to != DEAD_END && !exits.contains(&p.to) {
                exits.push(p.to);
            }
        }
        exits
    }
}

/// OSMS's destination for a portal that goes nowhere.
pub const DEAD_END: u64 = 999_999_999;

/// A made-up world: its maps, its mobs, the names of maps outside it (`lookups.json`), and
/// its Map Marks' pictures.
#[derive(Debug, Clone, Default)]
pub struct World {
    pub maps: Vec<Map>,
    pub mobs: Vec<Mob>,
    pub other_maps: Vec<(u64, String)>,
    /// Mobs with no record of their own, each folded into a mob's record, as OSMS folds
    /// some: (the folded mob, the mob whose record counts its spawn points).
    pub folded: Vec<(u64, u64)>,
    /// `map_mark_icons`, each mark's name and its picture's path; `None` leaves it out.
    pub mark_icons: Option<Vec<(String, String)>>,
}

pub fn world(maps: Vec<Map>, mobs: Vec<Mob>) -> World {
    World { maps, mobs, other_maps: Vec::new(), folded: Vec::new(), mark_icons: Some(Vec::new()) }
}

impl World {
    /// The same world with `map` in place of the map with its ID, or added.
    pub fn with(mut self, map: Map) -> Self {
        match self.maps.iter_mut().find(|m| m.id == map.id) {
            Some(old) => *old = map,
            None => self.maps.push(map),
        }
        self
    }

    /// The same world with map `id` changed by `change`.
    pub fn changing(mut self, id: u64, change: impl FnOnce(Map) -> Map) -> Self {
        let at = self.maps.iter().position(|m| m.id == id).unwrap_or_else(|| panic!("no map {id}"));
        self.maps[at] = change(self.maps[at].clone());
        self
    }

    /// The same world without map `id`.
    pub fn without(mut self, id: u64) -> Self {
        self.maps.retain(|m| m.id != id);
        self
    }

    /// The same world with `mob` in place of the mob with its ID, or added.
    pub fn with_mob(mut self, mob: Mob) -> Self {
        match self.mobs.iter_mut().find(|m| m.id == mob.id) {
            Some(old) => *old = mob,
            None => self.mobs.push(mob),
        }
        self
    }

    /// The same world with the title `street` / `name` given to the maps `ids` and to no
    /// other, those listed last, in the order given. A minimap sample showing that title
    /// then stands in for those maps': there are no samples of titles maps really share.
    pub fn titled(mut self, street: &str, name: &str, ids: &[u64]) -> Self {
        let same = |m: &Map| m.street_name.trim() == street && m.name.trim() == name;
        self.maps.retain(|m| !same(m) || ids.contains(&m.id));
        for &id in ids {
            let at = self.maps.iter().position(|m| m.id == id).unwrap_or_else(|| panic!("no map {id}"));
            let mut map = self.maps.remove(at);
            (map.street_name, map.name) = (street.into(), name.into());
            self.maps.push(map);
        }
        self
    }

    /// The same world, with a picture of Map Mark `mark` where OSMS keeps them
    /// (`images/map_marks/<mark>.png`).
    pub fn with_mark_icon(self, mark: &str) -> Self {
        self.with_mark_icon_at(mark, &format!("images/map_marks/{mark}.png"))
    }

    /// The same world, with a picture of Map Mark `mark` at `path`.
    pub fn with_mark_icon_at(mut self, mark: &str, path: &str) -> Self {
        self.mark_icons.get_or_insert_with(Vec::new).push((mark.into(), path.into()));
        self
    }

    /// The same world, with no `map_mark_icons` at all.
    pub fn without_mark_icons(mut self) -> Self {
        self.mark_icons = None;
        self
    }

    /// The same world, with mob `id` folded into mob `into`'s record, as OSMS folds some:
    /// `monsters.json` has no record of `id`, `lookups.json` names it as `into`, and
    /// `into`'s count on each map has `id`'s spawn points with its own.
    pub fn folding(mut self, id: u64, into: u64) -> Self {
        self.folded.push((id, into));
        self
    }

    /// The same world, with `lookups.json` naming map `id`, which isn't in it.
    pub fn naming(mut self, id: u64, name: &str) -> Self {
        self.other_maps.push((id, name.into()));
        self
    }

    /// The world as OSMS's five files.
    pub fn files(&self) -> OsmsFiles {
        OsmsFiles {
            maps: self.maps_json().to_string(),
            portals: self.portals_json().to_string(),
            monsters: self.monsters_json().to_string(),
            map_manifest: self.manifest_json().to_string(),
            lookups: self.lookups_json().to_string(),
        }
    }

    fn maps_json(&self) -> Value {
        let mut regions: Vec<(String, Vec<Value>)> = Vec::new();
        for map in &self.maps {
            let mut m = Object::new();
            m.insert("id".into(), json!(map.id));
            m.insert("name".into(), json!(map.name));
            m.insert("street_name".into(), json!(map.street_name));
            m.insert("region".into(), json!(map.region));
            m.insert("minimap".into(), json!(format!("images/maps/{:09}.png", map.id)));
            m.insert("bgm".into(), json!("Bgm00/MadeUp"));
            m.insert("map_mark".into(), json!(map.mark.as_deref().unwrap_or("None")));
            if map.town {
                m.insert("is_town".into(), json!(true));
            }
            if let Some(town) = map.return_map {
                m.insert("return_map_id".into(), json!(town));
            }
            if let Some(rate) = map.mob_rate {
                m.insert("mob_rate".into(), json!(rate));
            }
            let exits = map.exit_ids();
            if !exits.is_empty() {
                m.insert("exits".into(), json!(exits));
            }
            let mut positions = Vec::new();
            for spawn in &map.spawns {
                for point in 0..spawn.points {
                    positions.push(match spawn.at {
                        SpawnAt::Spread => json!({ "id": spawn.mob, "x": 100 + 40 * positions.len(), "y": 500 + point }),
                        SpawnAt::Here(x, y) => json!({ "id": spawn.mob, "x": x, "y": y }),
                        SpawnAt::Unplaced => json!({ "id": spawn.mob }),
                    });
                }
            }
            if !positions.is_empty() {
                m.insert("mob_capacity".into(), json!(positions.len()));
                m.insert("mob_positions".into(), Value::Array(positions));
            }
            match regions.iter_mut().find(|(region, _)| *region == map.region) {
                Some((_, maps)) => maps.push(Value::Object(m)),
                None => regions.push((map.region.clone(), vec![Value::Object(m)])),
            }
        }
        let regions: Vec<Value> = regions
            .into_iter()
            .map(|(region, maps)| json!({ "region": region, "count": maps.len(), "maps": maps }))
            .collect();
        let mut file = json!({ "regions": regions, "total": self.maps.len(), "npc_lookup": {} });
        if let Some(icons) = &self.mark_icons {
            file["map_mark_icons"] = Value::Object(icons.iter().map(|(mark, path)| (mark.clone(), json!(path))).collect());
        }
        file
    }

    fn portals_json(&self) -> Value {
        let mut portals = Object::new();
        for map in self.maps.iter().filter(|m| !m.portals.is_empty()) {
            let list: Vec<Value> = map
                .portals
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let mut portal = json!({
                        "id": (i + 1).to_string(),
                        "type": p.kind,
                        "dest_map": p.to.to_string(),
                        "name": p.name,
                        "dest_portal": p.landing,
                        "x": p.x,
                        "y": p.y,
                    });
                    if p.within_map {
                        portal["intra_map"] = json!(true);
                    }
                    portal
                })
                .collect();
            portals.insert(format!("{:09}", map.id), Value::Array(list));
        }
        Value::Object(portals)
    }

    fn monsters_json(&self) -> Value {
        let folded = |id: u64| self.folded.iter().any(|&(folded, _)| folded == id);
        let monsters: Vec<Value> = self
            .mobs
            .iter()
            .filter(|mob| !folded(mob.id))
            .map(|mob| {
                let counted = |id: u64| id == mob.id || self.folded.contains(&(id, mob.id));
                let mut m = json!({
                    "id": mob.id,
                    "name": mob.name,
                    "level": mob.level,
                    "hp": mob.hp,
                    "exp": mob.exp,
                    "pushed": 1,
                    "stagger": 600,
                    "gifs": { "hit1": format!("hit{}", mob.id), "die1": format!("die{}", mob.id) },
                    "thumbnail": format!("thumb{}", mob.id),
                });
                let figures = [
                    ("PADamage", mob.p_atk),
                    ("PDDamage", mob.p_def),
                    ("MDDamage", mob.m_def),
                    ("mp", mob.mp),
                    ("acc", mob.acc),
                    ("eva", mob.avoid),
                ];
                for (field, figure) in figures {
                    if let Some(figure) = figure {
                        m[field] = json!(figure);
                    }
                }
                if let Some(speed) = mob.speed {
                    m["speed"] = json!(speed);
                }
                if !mob.elements.is_empty() {
                    let elements: Object<String, Value> =
                        mob.elements.iter().map(|(element, affinity)| (element.clone(), json!(affinity))).collect();
                    m["elements"] = Value::Object(elements);
                }
                if let Some(hash) = &mob.animation {
                    m["gif"] = json!(hash);
                    m["gifs"]["move"] = json!(hash);
                }
                let maps: Vec<Value> = self
                    .maps
                    .iter()
                    .filter_map(|map| {
                        let spawns: Vec<&Spawn> = map.spawns.iter().filter(|s| counted(s.mob)).collect();
                        let count: u32 = spawns.iter().map(|s| s.points).sum();
                        (count > 0).then(|| {
                            let mut on = json!({ "id": map.id, "name": map.name, "count": count });
                            if let Some(timer) = spawns.iter().find_map(|s| s.timer.as_ref()) {
                                on["mob_time"] = json!(timer);
                            }
                            on
                        })
                    })
                    .collect();
                if !maps.is_empty() {
                    m["maps"] = Value::Array(maps);
                }
                m
            })
            .collect();
        json!({ "monsters": monsters, "total": self.mobs.iter().filter(|mob| !folded(mob.id)).count() })
    }

    fn manifest_json(&self) -> Value {
        let manifest: Object<String, Value> =
            self.maps.iter().map(|m| (format!("{:09}", m.id), json!(format!("picture{:09}", m.id)))).collect();
        Value::Object(manifest)
    }

    fn lookups_json(&self) -> Value {
        let names = self
            .maps
            .iter()
            .map(|m| (m.id, m.name.clone()))
            .chain(self.other_maps.iter().cloned())
            .map(|(id, name)| (format!("{id:09}"), json!(name)));
        let name = |id: u64| self.mobs.iter().find(|mob| mob.id == id).map(|mob| json!(mob.name));
        let mob_names = self
            .mobs
            .iter()
            .map(|mob| (mob.id.to_string(), json!(mob.name)))
            .chain(self.folded.iter().filter_map(|&(id, into)| Some((id.to_string(), name(into)?))));
        json!({ "item_names": {}, "npc_names": {}, "mob_names": Object::from_iter(mob_names), "map_names": Object::from_iter(names) })
    }
}

/// A small world of `n` made-up maps ("100000000", "100000001", …) on one street, with
/// nothing on them.
pub fn made_up_world(n: u64) -> World {
    let maps = (0..n).map(|i| map(100_000_000 + i, "Made-up Street", &format!("Made-up Map {i}"))).collect();
    world(maps, Vec::new())
}

#[derive(Deserialize)]
struct NamedMap {
    id: u64,
    street_name: String,
    name: String,
    region: String,
}

/// Every map in OSMS's map list, from the map names fixture: IDs, streets, names and
/// regions as OSMS gives them (untrimmed, the nameless map included), with nothing on
/// them. Generated from OSMS's `maps.json` with:
///
/// ```sh
/// node -e 'const fs=require("fs"); const maps=JSON.parse(fs.readFileSync(process.argv[1]));
///   const rows=maps.regions.flatMap(r=>r.maps.map(m=>JSON.stringify({id:m.id,
///     street_name:m.street_name,name:m.name,region:m.region})));
///   fs.writeFileSync(process.argv[2],"[\n"+rows.join(",\n")+"\n]\n")' maps.json osms-map-names.json
/// ```
pub fn map_names() -> World {
    let json = std::fs::read_to_string(fixtures_dir().join("osms-map-names.json")).expect("fixture");
    let named: Vec<NamedMap> = serde_json::from_str(&json).expect("map names json");
    let maps = named
        .into_iter()
        .map(|n| Map { region: n.region, ..map(n.id, &n.street_name, &n.name) })
        .collect();
    world(maps, Vec::new())
}

pub const KERNING_CITY: u64 = 10_003_000;
pub const SUNSET_SKY: u64 = 10_003_070;
pub const CONSTRUCTION_SITE: u64 = 10_003_080;
pub const NORTHERN_TOP: u64 = 10_003_081;
pub const HENESYS_HUNTING_GROUND_I: u64 = 10_001_010;

/// The mobs of [`game`]'s made-up maps, by OSMS's (and MeowDB's) mob ID.
pub fn game_mobs() -> Vec<Mob> {
    vec![
        mob(3, "Blue Snail", 2, 50, 4),
        mob(4, "Shroom", 2, 80, 6),
        mob(5, "Red Snail", 4, 68, 8),
        mob(6, "Stump", 5, 133, 10),
        mob(7, "Slime", 6, 120, 12),
        mob(9, "Orange Mushroom", 8, 142, 15),
        mob(11, "Dark Stump", 11, 236, 21),
        mob(12, "Octopus", 12, 191, 23),
        mob(13, "Green Mushroom", 14, 233, 26),
        mob(15, "Axe Stump", 17, 371, 32),
        mob(16, "Blue Mushroom", 19, 342, 35),
    ]
}

/// The game's map list (the map names fixture), with made-up figures on the maps the
/// minimap samples show and on Henesys Hunting Ground I:
///
/// - Kerning City, a Town: Doors (invisible portals) into two buildings, and visible
///   exits to Sunset Sky and the Construction Site.
/// - Sunset Sky: 5 mobs, a Mob Rate of ×0.6, two teleports within the map, and visible
///   exits to Kerning City and the Construction Site.
/// - The Construction Site North of Kerning City: 6 mobs, a visible exit to Sunset Sky
///   and an invisible one, a Hidden Portal, up to the Northern Top (a Hidden Street).
/// - The Northern Top of Construction Site: 2 mobs, and a visible way back down.
/// - Henesys Hunting Ground I: the spawn points the spec works its Map Summary from.
pub fn game() -> World {
    let mut world = map_names();
    world.mobs = game_mobs();
    world
        .changing(KERNING_CITY, |m| {
            m.town()
                .portal("in01", 1, 10_003_001)
                .portal("in02", 1, 10_003_002)
                .portal("east00", 2, SUNSET_SKY)
                .portal("north00", 2, CONSTRUCTION_SITE)
        })
        .changing(SUNSET_SKY, |m| {
            m.returning_to(KERNING_CITY)
                .mob_rate(0.6)
                .spawns(3, 6)
                .spawns(6, 12)
                .spawns(9, 8)
                .spawns(11, 6)
                .spawns(13, 8)
                .portal("west00", 2, KERNING_CITY)
                .portal("east00", 2, CONSTRUCTION_SITE)
                .teleport("hide01", "hide01_1")
                .teleport("hide01_1", "hide01")
        })
        .changing(CONSTRUCTION_SITE, |m| {
            m.returning_to(KERNING_CITY)
                .spawns(5, 20)
                .spawns(9, 10)
                .spawns(11, 10)
                .spawns(12, 6)
                .spawns(15, 3)
                .spawns(16, 1)
                .portal("west00", 2, SUNSET_SKY)
                .portal_at("hiden00", 1, NORTHERN_TOP, 1400, 220)
        })
        .changing(NORTHERN_TOP, |m| {
            m.returning_to(KERNING_CITY).spawns(12, 4).spawns(15, 4).portal("out00", 2, CONSTRUCTION_SITE)
        })
        .changing(HENESYS_HUNTING_GROUND_I, |m| {
            m.returning_to(10_001_000)
                .mob_rate(1.5)
                .spawns(3, 4)
                .spawns(4, 10)
                .spawns(5, 6)
                .spawns(6, 4)
                .spawns(7, 5)
                .spawns(9, 5)
                .spawns(13, 5)
                .portal("st01", 1, DEAD_END)
                .portal("west00", 2, 10_001_020)
                .portal("east00", 2, 10_001_000)
                .portal("in00", 3, 10_001_011)
        })
}
