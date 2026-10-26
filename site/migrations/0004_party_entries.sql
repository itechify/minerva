-- Party entries on the Map Leaderboards (ADR 0015, amended): a Character's Solo Best and Party
-- Best on a map in a Level Band are entries of their own. Every entry before this is solo.
-- SQLite can't change a primary key, so `entries` and `removed_entries` are made again with
-- `party` in theirs. The Worker before this keeps working until the deploy after: its
-- inserts leave `party` at 0.

CREATE TABLE entries_with_party (
  name_key TEXT NOT NULL REFERENCES characters (name_key),
  map_id TEXT NOT NULL,
  band INTEGER NOT NULL,
  level INTEGER NOT NULL,
  job TEXT NOT NULL,
  exp_per_hour INTEGER NOT NULL,
  set_at INTEGER NOT NULL,
  -- 1 for the Character's Party Best on the map in the band, 0 for its Solo Best.
  party INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (name_key, map_id, band, party)
);

INSERT INTO entries_with_party (name_key, map_id, band, level, job, exp_per_hour, set_at, party)
SELECT name_key, map_id, band, level, job, exp_per_hour, set_at, 0 FROM entries;

DROP TABLE entries;
ALTER TABLE entries_with_party RENAME TO entries;

CREATE INDEX entries_by_rank ON entries (map_id, band, exp_per_hour DESC, set_at, name_key);
CREATE INDEX entries_by_band ON entries (band, map_id, exp_per_hour DESC, set_at, name_key);
CREATE INDEX entries_by_date ON entries (set_at DESC);

CREATE TABLE removed_entries_with_party (
  name_key TEXT NOT NULL,
  map_id TEXT NOT NULL,
  band INTEGER NOT NULL,
  exp_per_hour INTEGER NOT NULL,
  party INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (name_key, map_id, band, exp_per_hour, party)
);

INSERT INTO removed_entries_with_party (name_key, map_id, band, exp_per_hour, party)
SELECT name_key, map_id, band, exp_per_hour, 0 FROM removed_entries;

DROP TABLE removed_entries;
ALTER TABLE removed_entries_with_party RENAME TO removed_entries;

-- The kind of the entry a report is against, and of the entry that took a top spot.
ALTER TABLE reports ADD COLUMN party INTEGER NOT NULL DEFAULT 0;
ALTER TABLE taken_tops ADD COLUMN party INTEGER NOT NULL DEFAULT 0;
