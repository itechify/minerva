//! The hand-kept portal override list: exceptions to the Hidden Portal rule found in
//! play. Add an entry when a portal the panel lists turns out to be plainly visible in
//! the game, or when one it leaves out turns out to be hidden. Each entry is keyed by the
//! map's 9-digit ID and the portal's name (`HiddenPortal::name`, or the `name` of the
//! portal's entry under the map in OSMS's `portals.json`).

/// Force one portal in or out of a map's Hidden Portals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalOverride {
    /// The map the portal is on: its 9-digit ID, zero padding kept.
    pub map_id: &'static str,
    /// The portal's name, as OSMS's `portals.json` gives it, e.g. `"in00"`.
    pub portal: &'static str,
    /// `true` lists the portal as a Hidden Portal whatever its type or map; `false`
    /// leaves it out. An exit into the same map or to the dead end is never listed.
    pub hidden: bool,
}

/// The overrides the app ships with. None has been found in play yet.
pub const PORTAL_OVERRIDES: &[PortalOverride] = &[];

/// Where a portal leads that OSMS gives the dead end. OSMS can't follow a script portal, so
/// it gives every one the dead end, and a way into a map that is only a script portal leaves
/// that map out of every route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PortalDestination {
    /// The map the portal is on: its 9-digit ID, zero padding kept.
    pub map_id: &'static str,
    /// The portal's name, as OSMS's `portals.json` gives it.
    pub portal: &'static str,
    /// The map it leads to.
    pub to_map_id: &'static str,
}

/// The portals the app gives a destination, each only while OSMS gives it the dead end: a
/// destination OSMS gives stands. Add one only where OSMS's own files show where the
/// portal leads, as a way back that lands at it.
///
/// - The ways down into Forgotten Hollow (November 2026), at the foot of The Tree Tunnel At
///   the Forest Up North and of Sleepy Dungeon V. Forgotten Entrance III's `east00` lands at
///   the first, and the Forgotten Dungeon Tunnel's `top00` at the second.
///
/// Ellinia's `in03` is left out, though Ellinia Station's `out00` lands at it: the station
/// is the ship to Orbis's, which stays out while Ossyria isn't in the game (see
/// [`crate::travel_links`]), and nothing says the player can walk in.
pub(crate) const PORTAL_DESTINATIONS: &[PortalDestination] = &[
    PortalDestination { map_id: "010002071", portal: "down00", to_map_id: "010006060" },
    PortalDestination { map_id: "010005054", portal: "down00", to_map_id: "010006071" },
];

/// Where portal `portal` on map `map_id` leads, given that OSMS says `to_map_id`: the
/// hand-kept destination in place of the dead end, else OSMS's.
pub(crate) fn destination<'a>(map_id: &str, portal: &str, to_map_id: &'a str) -> &'a str {
    if to_map_id != crate::portals::DEAD_END {
        return to_map_id;
    }
    PORTAL_DESTINATIONS
        .iter()
        .find(|d| d.map_id == map_id && d.portal == portal)
        .map_or(to_map_id, |d| d.to_map_id)
}
