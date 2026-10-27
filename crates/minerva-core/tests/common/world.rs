//! Small hand-built worlds for mob search: made-up maps written out in OSMS's shape, the
//! map list and every map's data.

use super::{FakeSource, SUNSET_SAMPLE, arrive};
use super::osms::{self, Spawn, SpawnAt};
use minerva_core::{Action, Distance, Minerva, Mob, PanelState, PortalKind, RouteStop, SearchResult};

/// The search hotkey (the default).
pub const SEARCH: &str = "ctrl+shift+f";

/// A map in a hand-built world, and the mobs and NPCs it holds.
#[derive(Debug, Clone)]
pub struct WorldMap {
    pub map: osms::Map,
    pub mobs: Vec<osms::Mob>,
    pub npcs: Vec<osms::Npc>,
}

/// A map on Victoria Road, Victoria Island, with this Mob Rate (hundredths) and these
/// mobs (their Spawn Counts as spawn points).
pub fn map(id: &str, name: &str, mob_rate_percent: u32, mobs: Vec<Mob>) -> WorldMap {
    let mut map = osms::map(id.parse().expect("a map ID"), "Victoria Road", name).mob_rate(f64::from(mob_rate_percent) / 100.0);
    map.spawns = mobs
        .iter()
        .map(|mob| Spawn {
            mob: mob.id.parse().expect("a mob ID"),
            points: mob.spawn_count,
            timer: None,
            at: SpawnAt::Spread,
        })
        .collect();
    let mobs = mobs
        .into_iter()
        .map(|mob| osms::mob(mob.id.parse().expect("a mob ID"), &mob.name, mob.level, mob.hp, mob.exp))
        .collect();
    WorldMap { map, mobs, npcs: Vec::new() }
}

/// A mob with ID `id`, `spawn_count` of it on the map.
pub fn mob(id: &str, name: &str, level: u32, spawn_count: u32) -> Mob {
    Mob { id: id.into(), name: name.into(), level, hp: 0, exp: 0, spawn_count, spawn_share_percent: 0 }
}

pub fn orange(spawn_count: u32) -> Mob {
    mob("9", "Orange Mushroom", 8, spawn_count)
}

impl WorldMap {
    /// The same map with a portal of this kind to map `to`. A Door is an invisible portal
    /// on a Town, so the map becomes one.
    pub fn exit(mut self, kind: PortalKind, to: &str) -> Self {
        let name = format!("portal{}", self.map.portals.len());
        let kind = match kind {
            PortalKind::Visible => 2,
            PortalKind::Hidden => 10,
            PortalKind::Quest => 11,
            PortalKind::Door => {
                self.map.town = true;
                1
            }
        };
        self.map = self.map.portal(&name, kind, to.parse().expect("a map ID"));
        self
    }

    /// The same map in another of OSMS's regions ("Event", "Dev", …).
    pub fn in_region(mut self, region: &str) -> Self {
        self.map.region = region.into();
        self
    }

    /// The same map with a portal the player can see to each of the maps `to`.
    pub fn to(self, to: &[&str]) -> Self {
        to.iter().fold(self, |map, to| map.exit(PortalKind::Visible, to))
    }

    /// The same map with NPC `npc` standing on it at (`x`, `y`) in its picture.
    pub fn npc(mut self, npc: osms::Npc, x: i64, y: i64) -> Self {
        self.map = self.map.npc_at(npc.id, x, y);
        self.npcs.push(npc);
        self
    }

    /// The same map with map `town` as its nearest town.
    pub fn returning_to(mut self, town: &str) -> Self {
        self.map.return_map = Some(town.parse().expect("a map ID"));
        self
    }
}

/// The OSMS world of these maps, each mob once.
pub fn osms_world(maps: Vec<WorldMap>) -> osms::World {
    let mut world = osms::world(Vec::new(), Vec::new());
    for WorldMap { map, mobs, npcs } in maps {
        world = mobs.into_iter().fold(world.with(map), osms::World::with_mob);
        world = npcs.into_iter().fold(world, osms::World::with_npc);
    }
    world
}

/// A source serving a hand-built world as OSMS's file set.
pub fn world(maps: Vec<WorldMap>) -> FakeSource {
    FakeSource::with_world(osms_world(maps))
}

/// A core over `maps`, its file set in hand (answered at once, on the first tick).
pub fn loaded(maps: Vec<WorldMap>) -> Minerva {
    let mut minerva = Minerva::new(world(maps));
    minerva.tick(0);
    minerva
}

/// Open the search view afresh (closing it first if it is open: Esc goes back a step at a
/// time), type `query` and press Enter: the mob's map list.
pub fn search_for(minerva: &mut Minerva, query: &str) -> PanelState {
    minerva.act(Action::SearchEscape);
    minerva.act(Action::SearchEscape);
    minerva.on_hotkey(SEARCH);
    minerva.search_query(query);
    minerva.act(Action::SearchEnter).clone()
}

/// The result the headline names as the nearest.
pub fn nearest(state: &PanelState) -> Option<&SearchResult> {
    state.search.nearest.map(|at| &state.search.results[at])
}

/// Tick every 50 ms from `from_ms` up to `to_ms`.
pub fn tick_through(minerva: &mut Minerva, from_ms: u64, to_ms: u64) -> PanelState {
    for at in (from_ms..to_ms).step_by(50) {
        minerva.tick(at);
    }
    minerva.tick(to_ms).clone()
}

/// A mob with this Spawn Share (whole percent).
pub fn sharing(share: u32, mut mob: Mob) -> Mob {
    mob.spawn_share_percent = share;
    mob
}

/// A core over `maps`, its file set in hand, with the player arrived on Sunset Sky.
pub fn on_sunset_sky(maps: Vec<WorldMap>) -> Minerva {
    on_sunset_sky_from(world(maps))
}

/// As [`on_sunset_sky`], with the file set and pictures from `source`.
pub fn on_sunset_sky_from(source: FakeSource) -> Minerva {
    let mut minerva = Minerva::new(source);
    minerva.tick(0);
    minerva.tick(50);
    arrive(&mut minerva, SUNSET_SAMPLE, 100);
    minerva
}

/// A core over `maps`, its file set in hand, with the player arrived on Sunset Sky and the
/// Orange Mushroom picked.
pub fn searched_from_sunset_sky(maps: Vec<WorldMap>) -> PanelState {
    let mut minerva = on_sunset_sky(maps);
    search_for(&mut minerva, "orange")
}

/// Make the Orange Mushroom on map `map_id` the Hunting Target.
pub fn hunt_orange_on(minerva: &mut Minerva, map_id: &str) -> PanelState {
    search_for(minerva, "orange");
    minerva.search_pick(map_id).clone()
}

/// The highlighted map's name in the search view's map list.
pub fn highlighted_map(state: &PanelState) -> Option<&str> {
    state.search.highlighted_result.map(|at| state.search.results[at].map_name.as_str())
}

/// A route's map names with how each is reached.
pub fn stops(route: &[RouteStop]) -> Vec<String> {
    route
        .iter()
        .map(|stop| match stop.by {
            None => stop.map_name.clone(),
            Some(step) => format!("{step:?}: {}", stop.map_name),
        })
        .collect()
}

/// A result's route as "Start, portal: Next, ship: Then…".
pub fn route_of(result: &SearchResult) -> String {
    stops(&result.route).join(", ")
}

/// Each result as "Map: Distance", a distance in map changes as the number.
pub fn distances(state: &PanelState) -> Vec<String> {
    state
        .search
        .results
        .iter()
        .map(|r| match r.distance {
            Distance::MapChanges(n) => format!("{}: {n}", r.map_name),
            other => format!("{}: {other:?}", r.map_name),
        })
        .collect()
}
