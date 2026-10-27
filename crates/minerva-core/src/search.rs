//! Search: the search view (the query; the mobs, maps and items whose name contains it;
//! the picked mob's map list; and their highlights) and the Hunting Target or Destination
//! it sets, as the overlay renders them, over every map in OSMS's file set and every item
//! in its item files. A picked item's card is the Item Card's to show.

use crate::OSMS_ONLY_CREDIT;
use crate::item_card::{ShopPrice, icon_path, item_id};
use crate::map_data::Mob;
use crate::map_index::MapEntry;
use crate::map_preview::PicturePoint;
use crate::osms::{MapSet, Npc};
use crate::osms_items::ItemSet;
use crate::world::{Distance, Distances, Step, Travel, World};
use serde::Serialize;
use std::collections::HashMap;

/// How many of each kind the search view shows of what the query matches: the rest are
/// counted, for the player to type more.
const SHOWN: usize = 5;

/// Which step of the search view is open. It takes over the whole panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchView {
    /// The query box and what it matches: mobs, maps and items, each kind in a section of
    /// its own.
    Matches,
    /// The picked mob's ranked map list, to pick the Hunting Target's map from.
    Maps,
    /// The picked NPC's maps, nearest first, to pick the Destination from.
    NpcMaps,
}

/// The mob and map the player picked to go and hunt, with the map's figures and the way
/// there from the map the panel shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntingTarget {
    pub mob: MobChoice,
    /// The map, as the mob's map list ranks it now: Density Tier, Density, Spawn Share,
    /// and Distance and route from the map the panel shows.
    pub map: SearchResult,
    /// The player has come to the map from another: they are there, and the target
    /// stays to say so until they leave it.
    pub arrived: bool,
    /// The way there as it stood when the player picked it; `None` when they picked the
    /// map they were on, or no route led there.
    pub trip: Option<Trip>,
    /// Where the overlay loads OSMS's default animation of the mob from, as its Mob Card
    /// does; `None` until it's in, if it can't be had, or if OSMS names none.
    pub image: Option<String>,
}

/// A map the player picked from Search to go to, with the way there from the map the
/// panel shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub map_id: String,
    pub street_name: String,
    pub map_name: String,
    /// The map's Map Mark, by OSMS's name for it (see [`crate::PanelState::map_marks`]);
    /// `None` for a map without one.
    pub mark: Option<String>,
    /// How many map changes it is from the map the panel shows.
    pub distance: Distance,
    /// A shortest route there (Quest Portals aside), as [`SearchResult::route`].
    pub route: Vec<RouteStop>,
    /// The kinds of travel besides portals that route takes, each once, in route order.
    pub travel: Vec<Step>,
    /// The player is there: they came to it, or picked it while on it. It stays to say so
    /// until they leave it.
    pub arrived: bool,
    /// The way there as it stood when the player picked it; `None` when they picked the
    /// map they were on, or no route led there.
    pub trip: Option<Trip>,
    /// The NPC the player picked the map for, who stands on it; `None` for a map picked as
    /// itself.
    pub npc: Option<NpcChoice>,
    /// The Map Preview marks where the NPC stands: the player is on its map, OSMS says where
    /// it stands there, and the preview shows. Arrived without it, the NPC is somewhere on
    /// the map, not shown.
    pub npc_marked: bool,
}

/// A map the picked NPC stands on, in its map list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcMap {
    pub map_id: String,
    pub street_name: String,
    pub map_name: String,
    /// The map's Map Mark, by OSMS's name for it (see [`crate::PanelState::map_marks`]);
    /// `None` for a map without one.
    pub mark: Option<String>,
    /// How many map changes it is from the map the panel shows.
    pub distance: Distance,
    /// The kinds of travel besides portals the route there takes, each once, in route
    /// order, as [`SearchResult::travel`].
    pub travel: Vec<Step>,
}

/// An NPC the player picked, to go to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcChoice {
    /// The NPC's ID, OSMS's.
    pub id: String,
    pub name: String,
    /// Where the overlay loads OSMS's picture of the NPC from; `None` until it's in, if it
    /// can't be had, or if OSMS has none.
    pub image: Option<String>,
}

/// The way to a Hunting Target or a Destination as it stood when the player picked it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trip {
    /// The name of the map the player picked it on.
    pub from: String,
    /// How many map changes the route from there took.
    pub map_changes: u32,
    /// The kinds of travel besides portals it took, each once, in route order.
    pub travel: Vec<Step>,
}

/// A mob the player can pick: one per mob ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobChoice {
    /// The mob's ID, the game's (OSMS's and MeowDB's alike).
    pub id: String,
    pub name: String,
    pub level: u32,
}

/// What the query matches of one kind: the best few, and how many more there are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Matches<T> {
    /// At most five, best first: names that start with the query, then those with a word
    /// that starts with it, then the rest; each by name.
    pub shown: Vec<T>,
    /// How many more match.
    pub more: usize,
}

impl<T> Default for Matches<T> {
    fn default() -> Self {
        Self { shown: Vec::new(), more: 0 }
    }
}

/// A mob the query matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobMatch {
    #[serde(flatten)]
    pub mob: MobChoice,
    /// Where the overlay loads OSMS's default animation of the mob from, as its Mob Card
    /// does; `None` until it's in, if it can't be had, or if OSMS names none.
    pub image: Option<String>,
}

/// A map the query matches, by its Map Name: picked, it becomes the Destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapMatch {
    pub map_id: String,
    pub street_name: String,
    pub map_name: String,
    /// The map's Map Mark, by OSMS's name for it (see [`crate::PanelState::map_marks`]);
    /// `None` for a map without one.
    pub mark: Option<String>,
}

/// An NPC the query matches, by its name: picked, its map becomes the Destination, or with
/// more than one its maps are listed to pick from (ADR 0028).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcMatch {
    /// The NPC's ID, OSMS's.
    pub id: String,
    pub name: String,
    /// The maps it stands on (Event and Dev maps aside), by Map Name: at least one.
    pub maps: Vec<MapMatch>,
    /// Where the overlay loads OSMS's picture of the NPC from; `None` until it's in, if it
    /// can't be had, or if OSMS has none.
    pub image: Option<String>,
}

/// Items the query matches that look alike (they share their name, kind and shop price,
/// as on the Item Card): picked, their card shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemMatch {
    /// The first one's game item ID, 8 digits.
    pub id: String,
    pub name: String,
    /// What kind of item it is, as the Item Card names it ("Overall", "Use", "Cash").
    pub kind: String,
    /// Where the overlay loads its picture from (OSMS's, as the game draws it), once it is
    /// in; `None` until then.
    pub icon: Option<String>,
}

/// Why Search has no items to match: OSMS's item files aren't in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemsMissing {
    /// They haven't arrived yet (they are asked for at startup).
    Loading,
    /// OSMS couldn't be reached and none are saved.
    Unavailable,
    /// They don't read, and there are none earlier to fall back on.
    FormatChanged,
}

/// How a map's Density for a mob compares with the mob's densest map (Event and Dev maps
/// aside). Ordered densest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DensityTier {
    /// From two-thirds of the densest map's Density.
    High,
    /// From one-third of it.
    Medium,
    Low,
}

/// A map the picked mob spawns on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub map_id: String,
    pub street_name: String,
    pub map_name: String,
    /// The map's Map Mark, by OSMS's name for it (see [`crate::PanelState::map_marks`]);
    /// `None` for a map without one.
    pub mark: Option<String>,
    /// The mob's Density on the map in hundredths: its Spawn Count times the map's Mob
    /// Rate (10 spawns at a 1.5x Mob Rate is 1500, a Density of 15).
    pub density_hundredths: u32,
    /// The mob's Spawn Share on the map, in whole percent.
    pub spawn_share_percent: u32,
    pub tier: DensityTier,
    /// How many map changes it is from the map the panel shows.
    pub distance: Distance,
    /// A shortest route there (Quest Portals aside): the map the panel shows, then each
    /// map in turn with how it is reached. Empty when the Distance isn't a number.
    pub route: Vec<RouteStop>,
    /// The kinds of travel besides portals that route takes, each once, in route order
    /// ("5 · ship"). Empty for a route by portals alone.
    pub travel: Vec<Step>,
}

/// One map on a route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteStop {
    pub map_id: String,
    /// Empty for a map that isn't in the map index.
    pub street_name: String,
    /// The map's ID for a map that isn't in the map index.
    pub map_name: String,
    /// The map's Map Mark, by OSMS's name for it (see [`crate::PanelState::map_marks`]);
    /// `None` for a map without one, or not in the map index.
    pub mark: Option<String>,
    /// How the player gets here from the stop before; `None` for the first stop, the
    /// map the panel shows.
    pub by: Option<Step>,
}

/// Search, as the overlay renders it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchState {
    /// The search view's open step; `None` for the normal view. While it is open the
    /// overlay has the keyboard (ADR 0001).
    pub view: Option<SearchView>,
    /// What the player has typed in the query box.
    pub query: String,
    /// The known mobs whose name contains the query (case-insensitive).
    pub mobs: Matches<MobMatch>,
    /// The maps whose Map Name contains it, Event and Dev maps aside.
    pub maps: Matches<MapMatch>,
    /// The NPCs whose name contains it, those standing only on Event and Dev maps aside.
    pub npcs: Matches<NpcMatch>,
    /// The items whose name contains it, those that look alike as one.
    pub items: Matches<ItemMatch>,
    /// Why no items are matched, while the query box holds something: OSMS's item files
    /// aren't in. `None` once they are.
    pub items_missing: Option<ItemsMissing>,
    /// Which match Enter would pick, counting through the mobs shown, then the maps, then
    /// the NPCs, then the items; `None` when there are none.
    pub highlighted: Option<usize>,
    /// The mob whose map list the search view shows; `None` outside that step.
    pub picked: Option<MobChoice>,
    /// Every map the picked mob spawns on (Event and Dev maps left out), by Density Tier
    /// and then Density: the search view's map list.
    pub results: Vec<SearchResult>,
    /// The picked mob spawns only on maps search leaves out (Event maps; the Dev map has
    /// no mobs): why it has no results.
    pub only_on_event_maps: bool,
    /// The NPC whose map list the search view shows; `None` outside that step.
    pub picked_npc: Option<NpcChoice>,
    /// Every map the picked NPC stands on (Event and Dev maps left out), nearest first and
    /// then by name: the search view's map list for it.
    pub npc_maps: Vec<NpcMap>,
    /// The map Enter would pick from the map list, the mob's ([`SearchState::results`])
    /// or the NPC's ([`SearchState::npc_maps`]); `None` when it lists none.
    pub highlighted_result: Option<usize>,
    /// The headline: which of the results is the nearest with a route, whatever its
    /// tier (of those equally near, the densest). `None` with no map shown or no route
    /// to any.
    pub nearest: Option<usize>,
    /// [`crate::OSMS_ONLY_CREDIT`] whenever there are results (a mob's maps or an NPC's), a
    /// Hunting Target or a Destination.
    pub credit: Option<&'static str>,
    /// What Distances count besides portals and ships (the checkboxes).
    pub travel: Travel,
    /// The Hunting Target the panel's Navigation section shows, until the player leaves its
    /// map having arrived, or clears it. Kept for this session only.
    pub hunting: Option<HuntingTarget>,
    /// The Destination the Navigation section shows in its place, as long. There is one or
    /// the other: the newest pick.
    pub destination: Option<Destination>,
}

impl SearchState {
    /// The next map on the way to the Hunting Target or the Destination, when the way goes
    /// there from the map the panel shows through a portal; `None` once the player has
    /// arrived, or when there is no route.
    pub(crate) fn next_map_by_portal(&self) -> Option<&str> {
        let (route, arrived) = match (&self.hunting, &self.destination) {
            (Some(target), _) => (&target.map.route, target.arrived),
            (_, Some(destination)) => (&destination.route, destination.arrived),
            _ => return None,
        };
        match route.get(1) {
            Some(RouteStop { map_id, by: Some(Step::Portal), .. }) if !arrived => Some(map_id),
            _ => None,
        }
    }

    /// Where the Destination's NPC stands on map `map_id`: nowhere unless the Destination
    /// is that map, picked for the NPC.
    pub(crate) fn npc_spots(&self, set: &MapSet, map_id: &str) -> Vec<PicturePoint> {
        match &self.destination {
            Some(Destination { npc: Some(npc), map_id: to, .. }) if to == map_id => {
                set.npc(&npc.id).and_then(|npc| npc.spots_on(map_id)).map(<[_]>::to_vec).unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }

    /// The pictures the search view's matches want, below [`crate::osms::PICTURES_URL`]:
    /// the items', then the mobs' animations, then the NPCs' pictures (the matched NPCs',
    /// the one whose map list shows, and the Destination's).
    pub(crate) fn pictures(&self, set: Option<&MapSet>) -> Vec<String> {
        let items = self.items.shown.iter().map(|item| icon_path(&item.id));
        let mobs = self.mobs.shown.iter().filter_map(|found| set?.animation(&found.mob.id)).map(String::from);
        let npcs = self.npcs.shown.iter().map(|npc| &npc.id).chain(self.npc_ids());
        let npcs = npcs.filter_map(|id| set?.npc(id)?.picture.clone());
        items.chain(mobs).chain(npcs).collect()
    }

    /// The IDs of the NPC whose map list shows and the Destination's NPC, if any.
    fn npc_ids(&self) -> impl Iterator<Item = &String> {
        let destination = self.destination.as_ref().and_then(|to| to.npc.as_ref());
        self.picked_npc.iter().chain(destination).map(|npc| &npc.id)
    }

    /// Fill in where the overlay loads the pictures from that `picture` has (by their path
    /// below [`crate::osms::PICTURES_URL`]): the matched mobs', NPCs' and items', the NPC
    /// whose map list shows, the Hunting Target mob's and the Destination's NPC.
    pub(crate) fn fill_pictures(&mut self, set: &MapSet, picture: impl Fn(&str) -> Option<String>) {
        let animation = |id: &str| set.animation(id).and_then(&picture);
        let npc_picture = |id: &str| set.npc(id)?.picture.as_deref().and_then(&picture);
        for found in &mut self.mobs.shown {
            found.image = animation(&found.mob.id);
        }
        for npc in &mut self.npcs.shown {
            npc.image = npc_picture(&npc.id);
        }
        for item in &mut self.items.shown {
            item.icon = picture(&icon_path(&item.id));
        }
        if let Some(hunting) = &mut self.hunting {
            hunting.image = animation(&hunting.mob.id);
        }
        let destination = self.destination.as_mut().and_then(|to| to.npc.as_mut());
        for npc in self.picked_npc.iter_mut().chain(destination) {
            npc.image = npc_picture(&npc.id);
        }
    }
}

/// What Search works over: every map in OSMS's file set (`None` before it arrives), every
/// item in its item files (or why they aren't in), the ID of the map the panel shows
/// (`None`: no map, so no Distances), and the kinds of travel the player has turned on.
#[derive(Clone, Copy)]
pub(crate) struct Searched<'a> {
    pub set: Option<&'a MapSet>,
    pub items: Result<&'a ItemSet, ItemsMissing>,
    pub here: Option<&'a str>,
    pub travel: Travel,
}

/// What a pick in the search view comes to for the Item Card: the IDs of the items, which
/// look alike, whose card shows.
pub(crate) type PickedItems = Vec<u32>;

/// One of the matches the search view shows, as a highlight is kept on it: by its kind
/// and ID, so it stays on it as the matches change.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    Mob(String),
    Map(String),
    Npc(String),
    Item(String),
}

/// Everything the query matches, best first, each kind in full. Items that look alike are
/// one match, with the IDs of all of them.
#[derive(Default)]
struct Found {
    mobs: Vec<MobChoice>,
    maps: Vec<MapMatch>,
    npcs: Vec<NpcMatch>,
    items: Vec<(ItemMatch, PickedItems)>,
}

impl Found {
    /// The matches the search view shows, in order: the first few mobs, then maps, then
    /// NPCs, then items.
    fn rows(&self) -> Vec<Row> {
        let mobs = self.mobs.iter().take(SHOWN).map(|mob| Row::Mob(mob.id.clone()));
        let maps = self.maps.iter().take(SHOWN).map(|map| Row::Map(map.map_id.clone()));
        let npcs = self.npcs.iter().take(SHOWN).map(|npc| Row::Npc(npc.id.clone()));
        let items = self.items.iter().take(SHOWN).map(|(item, _)| Row::Item(item.id.clone()));
        mobs.chain(maps).chain(npcs).chain(items).collect()
    }
}

/// The player's side of Search: what they typed, highlighted and picked, and the Hunting
/// Target or Destination.
#[derive(Default)]
pub(crate) struct Search {
    view: Option<SearchView>,
    query: String,
    /// The match the player moved the highlight to; `None` for the first.
    highlighted: Option<Row>,
    /// The mob whose map list the search view shows.
    picked: Option<MobChoice>,
    /// The NPC whose map list the search view shows.
    picked_npc: Option<NpcChoice>,
    /// The ID of the map the player moved the map list's highlight to; `None` for the
    /// first map. Kept by ID, so the highlight stays on it as the list reorders.
    highlighted_result: Option<String>,
    target: Option<Target>,
}

/// The Hunting Target, or with no mob the Destination (with the NPC it was picked for, if
/// any): the ID of its map, the trip there as it stood when the player picked it, and
/// whether they have arrived.
#[derive(Clone)]
struct Target {
    mob: Option<MobChoice>,
    npc: Option<NpcChoice>,
    map_id: String,
    trip: Option<Trip>,
    arrived: bool,
}

impl Search {
    /// Open the search view on its first step, with nothing typed yet. Already open, it
    /// stays as it is.
    pub fn open(&mut self) {
        if self.view.is_none() {
            self.view = Some(SearchView::Matches);
            self.type_query("");
        }
    }

    /// Open the search view on the Hunting Target mob's map list, highlighting its map.
    /// Without a Hunting Target (none, or a Destination), nothing happens.
    pub fn reopen(&mut self) {
        if let Some(Target { mob: Some(mob), map_id, .. }) = self.target.clone() {
            self.type_query("");
            self.view = Some(SearchView::Maps);
            self.picked = Some(mob);
            self.highlighted_result = Some(map_id);
        }
    }

    pub fn is_open(&self) -> bool {
        self.view.is_some()
    }

    /// The text box now holds `query`.
    pub fn type_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.highlighted = None;
    }

    /// Move the open step's highlight `by` items (up when negative), stopping at either
    /// end: through the matches, from one kind on to the next, or through the map list.
    pub fn move_highlight(&mut self, by: isize, searched: &Searched) {
        match self.view {
            Some(SearchView::Matches) => {
                let rows = self.found(searched).rows();
                move_through(&mut self.highlighted, &rows, by, Row::clone);
            }
            Some(SearchView::Maps) => {
                let results = self.results(searched);
                move_through(&mut self.highlighted_result, &results, by, |r| r.map_id.clone());
            }
            Some(SearchView::NpcMaps) => {
                let maps = self.npc_map_list(searched);
                move_through(&mut self.highlighted_result, &maps, by, |m| m.map_id.clone());
            }
            None => {}
        }
    }

    /// Enter: pick the highlighted match (a mob, to list its maps; a map, to make it the
    /// Destination; an NPC, to make its map the Destination or list its maps; or items, for
    /// their card), or pick the highlighted map from the map list and make it the Hunting
    /// Target, or from the NPC's the Destination. Nothing to pick (nothing typed, or a
    /// query matching nothing) leaves the view as it is.
    pub fn enter(&mut self, searched: &Searched) -> Option<PickedItems> {
        match self.view {
            Some(SearchView::Matches) => {
                let rows = self.found(searched).rows();
                match rows.get(highlight_in(&rows, &self.highlighted, Row::clone)?)? {
                    Row::Mob(id) => self.pick(id, searched),
                    Row::Map(id) => self.pick_destination(id, searched),
                    Row::Npc(id) => self.pick_npc(id, searched),
                    Row::Item(id) => return self.pick_item(id, searched),
                }
            }
            Some(SearchView::Maps) => {
                let results = self.results(searched);
                let at = highlight_in(&results, &self.highlighted_result, |r| r.map_id.clone())?;
                self.pick(&results[at].map_id, searched);
            }
            Some(SearchView::NpcMaps) => {
                let maps = self.npc_map_list(searched);
                let at = highlight_in(&maps, &self.highlighted_result, |m| m.map_id.clone())?;
                self.pick_destination(&maps[at].map_id, searched);
            }
            None => {}
        }
        None
    }

    /// The player picked the mob with this ID from the matches, to list its maps; or the
    /// map with this ID from the map list, to make it the Hunting Target.
    pub fn pick(&mut self, id: &str, searched: &Searched) {
        match self.view {
            Some(SearchView::Matches) => {
                if let Some(mob) = self.found(searched).mobs.into_iter().take(SHOWN).find(|mob| mob.id == id) {
                    self.picked = Some(mob);
                    self.highlighted_result = None;
                    self.view = Some(SearchView::Maps);
                }
            }
            Some(SearchView::Maps) => {
                let listed = self.results(searched).into_iter().find(|result| result.map_id == id);
                if let (Some(mob), Some(result)) = (&self.picked, listed) {
                    let way = Way { distance: result.distance, route: result.route, travel: result.travel };
                    let mob = Some(mob.clone());
                    let target = Target { mob, npc: None, map_id: id.to_string(), trip: trip(&way), arrived: false };
                    self.target = Some(target);
                    self.close();
                }
            }
            Some(SearchView::NpcMaps) | None => {}
        }
    }

    /// The player picked the map with this ID from the matches, or from the NPC's map
    /// list (for the NPC): it becomes the Destination, in place of any Hunting Target, and
    /// the search view closes. Picked while on it, the player is there already.
    pub fn pick_destination(&mut self, id: &str, searched: &Searched) {
        match self.view {
            Some(SearchView::Matches) => {
                if self.found(searched).maps.iter().take(SHOWN).any(|map| map.map_id == id) {
                    self.go_to(id, None, searched);
                }
            }
            Some(SearchView::NpcMaps) => {
                if self.npc_map_list(searched).iter().any(|map| map.map_id == id) {
                    self.go_to(id, self.picked_npc.clone(), searched);
                }
            }
            Some(SearchView::Maps) | None => {}
        }
    }

    /// The player picked the NPC with this ID from the matches: on one map, that map
    /// becomes the Destination, with the NPC, as [`Search::pick_destination`] has it; on
    /// more, they are listed to pick from.
    pub fn pick_npc(&mut self, id: &str, searched: &Searched) {
        if self.view != Some(SearchView::Matches) {
            return;
        }
        let Some(npc) = self.found(searched).npcs.into_iter().take(SHOWN).find(|npc| npc.id == id) else { return };
        let choice = NpcChoice { id: npc.id, name: npc.name, image: None };
        match &npc.maps[..] {
            [map] => self.go_to(&map.map_id, Some(choice), searched),
            _ => {
                self.picked_npc = Some(choice);
                self.highlighted_result = None;
                self.view = Some(SearchView::NpcMaps);
            }
        }
    }

    /// Map `map_id` becomes the Destination, for `npc` if one, in place of any Hunting
    /// Target, and the search view closes. Picked while on it, the player is there already.
    fn go_to(&mut self, map_id: &str, npc: Option<NpcChoice>, searched: &Searched) {
        let Some(set) = searched.set else { return };
        let trip = trip(&way_to(map_id, set, searched.here, searched.travel));
        let arrived = searched.here == Some(map_id);
        self.target = Some(Target { mob: None, npc, map_id: map_id.to_string(), trip, arrived });
        self.close();
    }

    /// The player picked the items with this ID (the first of those that look alike) from
    /// the matches: the search view closes, and their card is to show.
    pub fn pick_item(&mut self, id: &str, searched: &Searched) -> Option<PickedItems> {
        if self.view != Some(SearchView::Matches) {
            return None;
        }
        let (_, ids) = self.found(searched).items.into_iter().take(SHOWN).find(|(item, _)| item.id == id)?;
        self.close();
        Some(ids)
    }

    /// Esc or the back arrow: back a step, with nothing changed. From a map list (the mob's
    /// or the NPC's) to the matches as they were (with nothing typed when the Navigation section reopened the
    /// list); from the matches to the normal view.
    pub fn escape(&mut self) {
        match self.view {
            Some(SearchView::Maps | SearchView::NpcMaps) => {
                self.view = Some(SearchView::Matches);
                self.picked = None;
                self.picked_npc = None;
                self.highlighted_result = None;
            }
            _ => self.close(),
        }
    }

    /// Back to the normal view, with nothing changed.
    pub fn close(&mut self) {
        self.view = None;
        self.type_query("");
        self.picked = None;
        self.picked_npc = None;
        self.highlighted_result = None;
    }

    /// The Navigation section's ×: no Hunting Target or Destination any more.
    pub fn clear(&mut self) {
        self.target = None;
    }

    /// The panel now shows map `map_id`: if that is the Hunting Target's or the
    /// Destination's map, the player has arrived there; if they had arrived, they have left
    /// it, and the target is done with.
    pub fn arrived(&mut self, map_id: &str) {
        let Some(target) = &mut self.target else { return };
        if target.map_id == map_id {
            target.arrived = true;
        } else if target.arrived {
            self.clear();
        }
    }

    /// The search state for the panel, without its pictures
    /// ([`SearchState::fill_pictures`]).
    pub fn state(&self, searched: &Searched) -> SearchState {
        let Searched { set, travel, .. } = *searched;
        let hunting = self.hunting(searched);
        let destination = self.destination(searched);
        let found = self.found(searched);
        let rows = found.rows();
        let results = self.results(searched);
        let npc_maps = self.npc_map_list(searched);
        let highlighted_result = match self.view {
            Some(SearchView::NpcMaps) => highlight_in(&npc_maps, &self.highlighted_result, |m| m.map_id.clone()),
            _ => highlight_in(&results, &self.highlighted_result, |r| r.map_id.clone()),
        };
        let only_on_event_maps = match (&self.picked, set) {
            (Some(mob), Some(set)) if results.is_empty() => {
                set.maps().any(|(entry, data)| entry.is_left_out() && data.mobs.iter().any(|m| m.id == mob.id))
            }
            _ => false,
        };
        let typed = self.view == Some(SearchView::Matches) && !self.query.trim().is_empty();
        SearchState {
            view: self.view,
            query: self.query.clone(),
            highlighted: highlight_in(&rows, &self.highlighted, Row::clone),
            mobs: shown(found.mobs.into_iter().map(|mob| MobMatch { mob, image: None }).collect()),
            maps: shown(found.maps),
            npcs: shown(found.npcs),
            items: shown(found.items.into_iter().map(|(item, _)| item).collect()),
            items_missing: searched.items.err().filter(|_| typed),
            picked: self.picked.clone(),
            credit: (!results.is_empty() || !npc_maps.is_empty() || hunting.is_some() || destination.is_some())
                .then_some(OSMS_ONLY_CREDIT),
            nearest: nearest(&results),
            picked_npc: self.picked_npc.clone(),
            npc_maps,
            highlighted_result,
            only_on_event_maps,
            results,
            travel,
            hunting,
            destination,
        }
    }

    /// The Hunting Target with its map's figures, ranked among the mob's maps, and its
    /// route from the map the panel shows. Without its mob's animation, which is a picture
    /// ([`HuntingTarget::image`]).
    fn hunting(&self, searched: &Searched) -> Option<HuntingTarget> {
        let (target, set) = (self.target.as_ref()?, searched.set?);
        let mob = target.mob.as_ref()?;
        let map = results(&mob.id, set, searched.here, searched.travel).into_iter().find(|r| r.map_id == target.map_id)?;
        Some(HuntingTarget { mob: mob.clone(), map, arrived: target.arrived, trip: target.trip.clone(), image: None })
    }

    /// The Destination, with its route from the map the panel shows.
    fn destination(&self, searched: &Searched) -> Option<Destination> {
        let (target, set) = (self.target.as_ref()?, searched.set?);
        let entry = set.entry(&target.map_id).filter(|_| target.mob.is_none())?;
        let Way { distance, route, travel } = way_to(&target.map_id, set, searched.here, searched.travel);
        Some(Destination {
            map_id: entry.id.clone(),
            street_name: entry.street_name.clone(),
            map_name: entry.map_name.clone(),
            mark: entry.mark.clone(),
            distance,
            route,
            travel,
            arrived: target.arrived,
            trip: target.trip.clone(),
            npc: target.npc.clone(),
            // Once the Map Preview is made (`Minerva::sync_pictured`).
            npc_marked: false,
        })
    }

    /// The picked mob's map list; none outside that step.
    fn results(&self, searched: &Searched) -> Vec<SearchResult> {
        match (&self.picked, searched.set) {
            (Some(mob), Some(set)) => results(&mob.id, set, searched.here, searched.travel),
            _ => Vec::new(),
        }
    }

    /// The picked NPC's map list, nearest first, then by name; none outside that step.
    fn npc_map_list(&self, searched: &Searched) -> Vec<NpcMap> {
        let (Some(picked), Some(set)) = (&self.picked_npc, searched.set) else { return Vec::new() };
        let Some(npc) = set.npc(&picked.id) else { return Vec::new() };
        let distances = searched.here.map(|here| World::of(set, searched.travel).distances_from(here));
        let mut maps: Vec<NpcMap> = maps_of(npc, set)
            .into_iter()
            .map(|map| {
                let (distance, travel) = match &distances {
                    Some(distances) => (distances.to(&map.map_id), travel_on(&route(distances, &map.map_id, set))),
                    None => (Distance::Unknown, Vec::new()),
                };
                let MapMatch { map_id, street_name, map_name, mark } = map;
                NpcMap { map_id, street_name, map_name, mark, distance, travel }
            })
            .collect();
        // Already by name: a stable sort keeps them so among equals.
        maps.sort_by_key(|map| map.distance);
        maps
    }

    /// Everything the query matches, whatever the case, each kind best first: the mobs
    /// (each once), the maps by their Map Name (Event and Dev maps aside), the NPCs on any
    /// other map, and the items, those that look alike as one. Nothing outside the view's first step, or with
    /// nothing typed.
    fn found(&self, searched: &Searched) -> Found {
        let query = self.query.trim().to_lowercase();
        if self.view != Some(SearchView::Matches) || query.is_empty() {
            return Found::default();
        }
        let mut found = Found::default();
        if let Some(set) = searched.set {
            let mut mobs: Vec<(Fit, MobChoice)> = Vec::new();
            let mut maps: Vec<(Fit, MapMatch)> = Vec::new();
            for (entry, data) in set.maps() {
                for mob in &data.mobs {
                    let Some(fit) = fit(&mob.name, &query) else { continue };
                    if !mobs.iter().any(|(_, m)| m.id == mob.id) {
                        mobs.push((fit, MobChoice { id: mob.id.clone(), name: mob.name.clone(), level: mob.level }));
                    }
                }
                if let Some(fit) = fit(&entry.map_name, &query).filter(|_| !entry.is_left_out()) {
                    maps.push((fit, map_match(entry)));
                }
            }
            mobs.sort_by(|(a_fit, a), (b_fit, b)| (a_fit, &a.name, a.level, &a.id).cmp(&(b_fit, &b.name, b.level, &b.id)));
            maps.sort_by(|(a_fit, a), (b_fit, b)| {
                (a_fit, &a.map_name, &a.street_name, &a.map_id).cmp(&(b_fit, &b.map_name, &b.street_name, &b.map_id))
            });
            found.mobs = mobs.into_iter().map(|(_, mob)| mob).collect();
            found.maps = maps.into_iter().map(|(_, map)| map).collect();
            found.npcs = npcs_found(set, &query);
        }
        if let Ok(items) = searched.items {
            found.items = items_found(items, &query);
        }
        found
    }
}

/// How well a name matches the query: it starts with it, a word in it does, or it holds it
/// elsewhere. Ordered best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Fit {
    Start,
    WordStart,
    Within,
}

/// How well `name` matches `query` (lower case), whatever the case; `None` when it doesn't
/// hold it. A word starts after anything but a letter or a digit.
fn fit(name: &str, query: &str) -> Option<Fit> {
    let name = name.to_lowercase();
    name.match_indices(query)
        .map(|(at, _)| match name[..at].chars().next_back() {
            None => Fit::Start,
            Some(before) if !before.is_alphanumeric() => Fit::WordStart,
            Some(_) => Fit::Within,
        })
        .min()
}

/// The NPCs whose name holds `query` (lower case), best first, each with the maps it
/// stands on (Event and Dev maps aside, and those standing only there left out).
fn npcs_found(set: &MapSet, query: &str) -> Vec<NpcMatch> {
    let mut found: Vec<(Fit, NpcMatch)> = Vec::new();
    for npc in set.npcs() {
        let Some(fit) = fit(&npc.name, query) else { continue };
        let maps = maps_of(npc, set);
        if !maps.is_empty() {
            found.push((fit, NpcMatch { id: npc.id.clone(), name: npc.name.clone(), maps, image: None }));
        }
    }
    // Of NPCs sharing a name, the one on the map first by name first.
    let first_map = |npc: &NpcMatch| npc.maps.first().map(|map| map.map_name.clone());
    found.sort_by(|(a_fit, a), (b_fit, b)| (a_fit, &a.name, first_map(a), &a.id).cmp(&(b_fit, &b.name, first_map(b), &b.id)));
    found.into_iter().map(|(_, npc)| npc).collect()
}

/// The maps `npc` stands on, Event and Dev maps aside, by Map Name.
fn maps_of(npc: &Npc, set: &MapSet) -> Vec<MapMatch> {
    let mut maps: Vec<MapMatch> = npc
        .maps
        .iter()
        .filter_map(|on| set.entry(&on.map_id))
        .filter(|entry| !entry.is_left_out())
        .map(map_match)
        .collect();
    maps.sort_by(|a, b| (&a.map_name, &a.street_name, &a.map_id).cmp(&(&b.map_name, &b.street_name, &b.map_id)));
    maps
}

/// The map `entry` lists, as a match.
fn map_match(entry: &MapEntry) -> MapMatch {
    MapMatch {
        map_id: entry.id.clone(),
        street_name: entry.street_name.clone(),
        map_name: entry.map_name.clone(),
        mark: entry.mark.clone(),
    }
}

/// The items whose name holds `query` (lower case), best first; those that share their
/// name, kind and shop price look alike, so they are one match, under the first of them.
fn items_found(items: &ItemSet, query: &str) -> Vec<(ItemMatch, PickedItems)> {
    let mut found: Vec<(Fit, ItemMatch, PickedItems)> = Vec::new();
    let mut alike: HashMap<(&str, &str, ShopPrice), usize> = HashMap::new();
    for item in items.all() {
        let Some(fit) = fit(&item.name, query).filter(|_| !item.name.is_empty()) else { continue };
        match alike.get(&(item.name.as_str(), item.kind.as_str(), item.shop)) {
            Some(&at) => found[at].2.push(item.id),
            None => {
                alike.insert((&item.name, &item.kind, item.shop), found.len());
                let first = ItemMatch { id: item_id(item.id), name: item.name.clone(), kind: item.kind.clone(), icon: None };
                found.push((fit, first, vec![item.id]));
            }
        }
    }
    found.sort_by(|(a_fit, a, _), (b_fit, b, _)| (a_fit, &a.name, &a.kind, &a.id).cmp(&(b_fit, &b.name, &b.kind, &b.id)));
    found.into_iter().map(|(_, item, ids)| (item, ids)).collect()
}

/// The first few of `all`, and how many more.
fn shown<T>(mut all: Vec<T>) -> Matches<T> {
    let more = all.len().saturating_sub(SHOWN);
    all.truncate(SHOWN);
    Matches { shown: all, more }
}

/// Where a highlight kept by key (`moved_to`) is in `items`: on the item with that key,
/// or on the first when it was never moved or its item is gone; `None` with no items.
fn highlight_in<T, K: PartialEq>(items: &[T], moved_to: &Option<K>, key: impl Fn(&T) -> K) -> Option<usize> {
    if items.is_empty() {
        return None;
    }
    let moved_to = moved_to.as_ref().and_then(|moved_to| items.iter().position(|item| key(item) == *moved_to));
    Some(moved_to.unwrap_or(0))
}

/// Move a highlight kept by key `by` items through `items` (up when negative), stopping
/// at either end. Left as it is when there are no items.
fn move_through<T, K: PartialEq>(highlight: &mut Option<K>, items: &[T], by: isize, key: impl Fn(&T) -> K) {
    if let Some(at) = highlight_in(items, highlight, &key) {
        let to = at.saturating_add_signed(by).min(items.len() - 1);
        *highlight = Some(key(&items[to]));
    }
}

/// Which of the ranked `results` is the nearest with a route, whatever its tier; of
/// those equally near, the densest (then the first ranked).
fn nearest(results: &[SearchResult]) -> Option<usize> {
    results
        .iter()
        .enumerate()
        .filter_map(|(at, r)| match r.distance {
            Distance::MapChanges(changes) => Some((changes, std::cmp::Reverse(r.density_hundredths), at)),
            Distance::NoRoute | Distance::QuestOnly | Distance::Unknown => None,
        })
        // Of equals, the first ranked.
        .min()
        .map(|(_, _, at)| at)
}

/// The way to a map from the map the player is on: how far, by which route, and the
/// kinds of travel besides portals it takes.
struct Way {
    distance: Distance,
    route: Vec<RouteStop>,
    travel: Vec<Step>,
}

/// The way to map `map_id` from `here` (by portals, ships and the `travel` turned on).
fn way_to(map_id: &str, set: &MapSet, here: Option<&str>, travel: Travel) -> Way {
    let Some(distances) = here.map(|here| World::of(set, travel).distances_from(here)) else {
        return Way { distance: Distance::Unknown, route: Vec::new(), travel: Vec::new() };
    };
    let route = route(&distances, map_id, set);
    Way { distance: distances.to(map_id), travel: travel_on(&route), route }
}

/// The trip a pick sets out on along `way`; none when the player is there already, or no
/// route leads there.
fn trip(way: &Way) -> Option<Trip> {
    match (way.distance, way.route.first()) {
        (Distance::MapChanges(n @ 1..), Some(from)) => {
            Some(Trip { from: from.map_name.clone(), map_changes: n, travel: way.travel.clone() })
        }
        _ => None,
    }
}

/// The kinds of travel besides portals `route` takes, each once, in route order.
fn travel_on(route: &[RouteStop]) -> Vec<Step> {
    let mut travel = Vec::new();
    for step in route.iter().filter_map(|stop| stop.by) {
        if step != Step::Portal && !travel.contains(&step) {
            travel.push(step);
        }
    }
    travel
}

/// The shortest route to map `map_id` with its maps' names, or none.
fn route(distances: &Distances, map_id: &str, set: &MapSet) -> Vec<RouteStop> {
    distances
        .route_to(map_id)
        .into_iter()
        .map(|(map_id, by)| {
            let (street_name, map_name, mark) = match set.entry(&map_id) {
                Some(entry) => (entry.street_name.clone(), entry.map_name.clone(), entry.mark.clone()),
                None => (String::new(), map_id.clone(), None),
            };
            RouteStop { map_id, street_name, map_name, mark, by }
        })
        .collect()
}

/// Every map mob `mob_id` spawns on, Event and Dev maps aside, ranked: by Density Tier;
/// within a tier, maps with a route by Distance from the map the player is on (`here`; by
/// portals, ships and the `travel` turned on), then those with no known route, then the
/// quest-only ones; then by Density (densest first), then name.
fn results(mob_id: &str, set: &MapSet, here: Option<&str>, travel: Travel) -> Vec<SearchResult> {
    let distances = here.map(|here| World::of(set, travel).distances_from(here));
    let distance = |map_id: &str| distances.as_ref().map_or(Distance::Unknown, |d| d.to(map_id));
    let found: Vec<(&MapEntry, &Mob, u32)> = set
        .maps()
        .filter(|(entry, _)| !entry.is_left_out())
        .flat_map(|(entry, data)| {
            data.mobs
                .iter()
                .filter(|mob| mob.id == mob_id)
                .map(move |mob| (entry, mob, mob.spawn_count * data.mob_rate_percent))
        })
        .collect();
    let densest = found.iter().map(|&(_, _, density)| density).max().unwrap_or(0);
    let tier = |density: u32| {
        // Whole numbers: exactly two-thirds is High, exactly one-third Medium.
        let (density, densest) = (u64::from(density) * 3, u64::from(densest));
        if density >= densest * 2 {
            DensityTier::High
        } else if density >= densest {
            DensityTier::Medium
        } else {
            DensityTier::Low
        }
    };
    let mut results: Vec<SearchResult> = found
        .into_iter()
        .map(|(entry, mob, density)| {
            let route = distances.as_ref().map(|d| route(d, &entry.id, set)).unwrap_or_default();
            SearchResult {
                map_id: entry.id.clone(),
                street_name: entry.street_name.clone(),
                map_name: entry.map_name.clone(),
                mark: entry.mark.clone(),
                density_hundredths: density,
                spawn_share_percent: mob.spawn_share_percent,
                tier: tier(density),
                distance: distance(&entry.id),
                travel: travel_on(&route),
                route,
            }
        })
        .collect();
    results.sort_by(|a, b| {
        (a.tier, a.distance, std::cmp::Reverse(a.density_hundredths), &a.street_name, &a.map_name, &a.map_id).cmp(&(
            b.tier,
            b.distance,
            std::cmp::Reverse(b.density_hundredths),
            &b.street_name,
            &b.map_name,
            &b.map_id,
        ))
    });
    results
}
