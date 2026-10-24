-- Where to train and Latest on the Map Leaderboards page (ADR 0024). Additive only: the
-- Worker before this keeps working until the deploy after.

-- Each top spot an upload took from another Character (ADR 0022), kept while the entry that
-- took it stands: Latest marks that entry "took #1". It goes with the entry, when the
-- maintainer deletes it, the Character opts out, or a new best of theirs replaces it.
CREATE TABLE taken_tops (
  -- The Character that took it, and the entry it took it with.
  name_key TEXT NOT NULL,
  map_id TEXT NOT NULL,
  band INTEGER NOT NULL,
  exp_per_hour INTEGER NOT NULL,
  -- The Character it was taken from, and what it held: the name goes when they opt out.
  from_name_key TEXT,
  from_exp_per_hour INTEGER NOT NULL,
  taken_at INTEGER NOT NULL,
  PRIMARY KEY (name_key, map_id, band)
);

CREATE INDEX taken_tops_by_from ON taken_tops (from_name_key);

-- Where to train reads a Level Band's entries across every map; Latest, the newest.
CREATE INDEX entries_by_band ON entries (band, map_id, exp_per_hour DESC, set_at, name_key);
CREATE INDEX entries_by_date ON entries (set_at DESC);
