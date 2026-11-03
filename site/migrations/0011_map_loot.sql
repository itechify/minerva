-- Map Loot (ADR 0033): each Character's running sums on each map, of each kind, from the Visits
-- the app sends; and each map's figures, worked out from them once a day. Additive only: the
-- Worker before this keeps working until the deploy after.

-- A Character's Visits on a map of one kind (party 1, solo 0), added up: their active time and
-- mesos, and each Item's count as a JSON object by Item ID ({"4000000": 60}).
CREATE TABLE map_loot (
  name_key TEXT NOT NULL,
  map_id TEXT NOT NULL,
  party INTEGER NOT NULL,
  active_ms INTEGER NOT NULL,
  mesos INTEGER NOT NULL,
  items TEXT NOT NULL,
  PRIMARY KEY (name_key, map_id, party)
);

-- The maps whose running sums changed since the daily run last worked their figures out.
CREATE TABLE map_loot_changed (map_id TEXT PRIMARY KEY);

-- Each map's figures as the daily run last worked them out: a JSON object of each kind's, as
-- GET /api/maps/<map>/loot gives them.
CREATE TABLE map_loot_figures (
  map_id TEXT PRIMARY KEY,
  figures TEXT NOT NULL,
  built_at INTEGER NOT NULL
);
