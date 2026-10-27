//! The Map Preview: OSMS's picture of the whole of the shown map, with every portal on it
//! marked where it is, as OSMS Data Explorer's own map view marks them (ADR 0008), and
//! every Spawn Point dotted, as the minimap dots mobs.
//!
//! The picture is asked of the source while the preview shows, ahead of the Mob Cards'
//! animations, and its answer kept for the run, so a map shown again shows it at once.

use crate::map_data::PortalKind;
use crate::pictures::Pictures;
use serde::Serialize;

/// The shown map's Map Preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapPreview {
    /// The picture's content-hash name, from `map_manifest.json`: the picture is
    /// `maps/<picture>.webp` under OSMS's data.
    pub picture: String,
    /// Where the overlay loads the picture from, or why it can't yet.
    pub image: PreviewImage,
    /// Every portal on the map but those to the dead end, in OSMS's order.
    pub marks: Vec<PortalMark>,
    /// Every Spawn Point on the map with a position, in OSMS's order.
    pub spawns: Vec<SpawnMark>,
    /// Where the Destination's NPC stands, when the map is the Destination's and the player
    /// picked it for the NPC (ADR 0028); each spot it stands at, in OSMS's order.
    pub npc_spots: Vec<PicturePoint>,
}

/// The Map Preview's picture: where the overlay loads it from, or why it can't.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PreviewImage {
    /// The source is getting it.
    Loading,
    /// The overlay loads it from `url`, in Minerva's own scheme.
    Ready { url: String },
    /// It couldn't be had: OSMS couldn't be reached (it is asked for again a while on),
    /// or has no such picture.
    Unavailable,
}

/// The path under OSMS's data of the picture named `picture`.
pub(crate) fn picture_path(picture: &str) -> String {
    format!("maps/{picture}.webp")
}

impl PreviewImage {
    /// The picture named `picture`, as `pictures` has it.
    pub(crate) fn of(picture: &str, pictures: &Pictures) -> Self {
        match pictures.answer(&picture_path(picture)) {
            None => Self::Loading,
            Some(Ok(url)) => Self::Ready { url: url.to_string() },
            Some(Err(_)) => Self::Unavailable,
        }
    }
}

/// A point in a map's picture, in its pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PicturePoint {
    pub x: i32,
    pub y: i32,
}

/// A portal marked on the Map Preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalMark {
    /// Where it is (`x` and `y` beside the rest for the overlay).
    #[serde(flatten)]
    pub at: PicturePoint,
    pub leads: Leads,
    /// What kind of portal it is, by the Hidden Portal rule.
    pub kind: PortalKind,
    /// The destination's 9-digit ID; within the map, the map's own.
    pub to_map_id: String,
    /// The destination's name and street; within the map, the map's own. A map not in
    /// the data is named from OSMS's lookups, with no street, or else by its ID.
    pub to_map_name: String,
    pub to_street_name: String,
    /// The destination's Map Mark, by OSMS's name for it (see [`crate::PanelState::map_marks`]);
    /// within the map, the map's own.
    pub to_mark: Option<String>,
    /// For a teleport within the map, where it lands.
    pub lands_at: Option<Landing>,
    /// Its row among the shown map's Hidden Portals (an index into
    /// [`crate::MapData::hidden_portals`]); `None` when it has none.
    pub hidden_portal: Option<usize>,
    /// It is the Hunting Target's route's next step: the player hasn't reached the target,
    /// and the route goes on from the shown map through a portal to where this one leads.
    /// Never a Quest Portal, which no route goes through.
    pub next_on_route: bool,
}

impl MapPreview {
    /// The same preview, with the portals to `next_map` (the next map on the Hunting
    /// Target's route, when the route goes there through a portal) marked next on the
    /// route, and no others.
    pub(crate) fn on_route_to(mut self, next_map: Option<&str>) -> Self {
        for mark in &mut self.marks {
            mark.next_on_route = next_map == Some(mark.to_map_id.as_str()) && mark.kind != PortalKind::Quest;
        }
        self
    }

    /// The same preview, with the Destination's NPC marked at `spots`.
    pub(crate) fn with_npc_at(mut self, spots: Vec<PicturePoint>) -> Self {
        self.npc_spots = spots;
        self
    }
}

/// A Spawn Point dotted on the Map Preview: where one of the map's mobs spawns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnMark {
    /// Where it is (`x` and `y` beside the rest for the overlay).
    #[serde(flatten)]
    pub at: PicturePoint,
    /// The ID of the mob that spawns there, one of the map's [`crate::MapData::mobs`].
    pub mob: String,
}

/// Where a portal leads, as OSMS colours its mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Leads {
    /// To another map in the data (blue).
    AnotherMap,
    /// To another spot on the same map: a teleport (green, with an arrow to where it lands).
    WithinTheMap,
    /// To a map that isn't in the data (grey).
    MapNotInTheData,
}

/// Where a teleport within the map lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Landing {
    /// Where it is (`x` and `y` beside the rest for the overlay).
    #[serde(flatten)]
    pub at: PicturePoint,
    /// The mark it lands at (an index into [`MapPreview::marks`]), when the portal it
    /// lands at is marked; a spawn point isn't.
    pub mark: Option<usize>,
    /// That mark is a teleport landing back at this one: the two are a pair, one way
    /// there and the other back.
    pub both_ways: bool,
}
