-- The maintainer's tools (ADR 0031): Blocks and the log. Additive only: the Worker before this
-- keeps working until the deploy after.

-- A Block, made from a Character's lookup: it reaches the Character's Account, if it had one,
-- the keys kept beside it, and, with `blocks_name`, the name itself.
CREATE TABLE blocks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  -- The Character it was made from, in lower case.
  name_key TEXT NOT NULL,
  -- 1 when the name itself is kept off, whatever key uploads it.
  blocks_name INTEGER NOT NULL DEFAULT 0,
  -- The Character's Account when it was made, or NULL: a key linked to it later is blocked too.
  account_id INTEGER,
  -- Only the maintainer sees it.
  note TEXT NOT NULL DEFAULT '',
  blocked_at INTEGER NOT NULL
);

-- The keys a Block holds: the one that claimed the name, and each linked to its Account when
-- it was made, so unlinking a PC or deleting the Account frees none of them.
CREATE TABLE block_keys (
  block_id INTEGER NOT NULL REFERENCES blocks (id),
  key_hash TEXT NOT NULL,
  PRIMARY KEY (block_id, key_hash)
);
CREATE INDEX block_keys_by_key ON block_keys (key_hash);

-- Every write the maintainer made under /api/admin/ that went through.
CREATE TABLE admin_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  at INTEGER NOT NULL,
  -- The maintainer's Discord username, or 'token' for ADMIN_TOKEN.
  actor TEXT NOT NULL,
  -- What it was: 'take-down', 'block', 'reassign', ...
  action TEXT NOT NULL,
  -- The Character it touched, in lower case, and the Account, where it did.
  name_key TEXT,
  account_id INTEGER,
  -- The entry's map and band, a Block's note, and the like.
  detail TEXT NOT NULL DEFAULT ''
);
CREATE INDEX admin_log_by_name ON admin_log (name_key, id);
