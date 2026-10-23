-- Accounts (ADR 0023): optional, signed in with Discord, each with the Leaderboard Keys its
-- player linked. Additive only: the Worker before this keeps working until the deploy after.

-- An Account: ours, so a provider other than Discord can sign in to it later.
CREATE TABLE accounts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  created_at INTEGER NOT NULL
);

-- How an Account signs in: Discord's user ID, and its username as at the last sign-in.
CREATE TABLE logins (
  provider TEXT NOT NULL,
  provider_user_id TEXT NOT NULL,
  account_id INTEGER NOT NULL REFERENCES accounts (id),
  username TEXT NOT NULL,
  PRIMARY KEY (provider, provider_user_id)
);

CREATE INDEX logins_by_account ON logins (account_id);

-- The Leaderboard Keys linked to an Account, one Account each. `id` names a PC on the account
-- page without its key's hash.
CREATE TABLE account_keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  key_hash TEXT NOT NULL UNIQUE,
  account_id INTEGER NOT NULL REFERENCES accounts (id),
  linked_at INTEGER NOT NULL,
  -- The key's last write, or its linking: a name its PC is unlinked from moves to the
  -- Account's most recently used other key.
  last_used_at INTEGER NOT NULL
);

CREATE INDEX account_keys_by_account ON account_keys (account_id);

-- Signed-in sessions on the site, by the hash of the cookie's secret.
CREATE TABLE sessions (
  id_hash TEXT PRIMARY KEY,
  account_id INTEGER NOT NULL REFERENCES accounts (id),
  expires_at INTEGER NOT NULL
);

-- One-time link tokens, by their hash: the app asks for one with its key, and the player
-- confirms it on the site while signed in.
CREATE TABLE link_tokens (
  token_hash TEXT PRIMARY KEY,
  key_hash TEXT NOT NULL,
  expires_at INTEGER NOT NULL
);

-- The Account a name's Characters belong to, through a linked key: any key linked to it may
-- write for the name, beside the key that claimed it.
ALTER TABLE characters ADD COLUMN account_id INTEGER REFERENCES accounts (id);
