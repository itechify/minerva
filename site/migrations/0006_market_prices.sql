-- maplestory.market's prices, for the Item Card and the Deal Finder (ADR 0030). Additive only:
-- the Worker before this keeps working until the deploy after.

-- When each of maplestory.market's tables was last pulled whole, and last asked for. The
-- Worker asks for one at most once a minute, and only once the copy it has is an hour old.
CREATE TABLE market_pulls (
  -- 'prices' (its /api/prices) or 'listed' (its /api/items).
  source TEXT PRIMARY KEY,
  -- When it last answered in full and was kept (ms since the Unix epoch); NULL until then.
  pulled_at INTEGER,
  -- When it was last asked, whatever came of it (ms since the Unix epoch).
  tried_at INTEGER NOT NULL
);

-- Each item's figures in /api/prices, as it gave them, less its figures by attack roll and
-- its deal price: an object with `reference`, `sold` and `seen`.
CREATE TABLE market_prices (
  item_id INTEGER PRIMARY KEY,
  body TEXT NOT NULL
);

-- Each item listed now, from /api/items: its cheapest price for one, and how many listings.
CREATE TABLE market_listed (
  item_id INTEGER PRIMARY KEY,
  min_unit_price REAL NOT NULL,
  listings INTEGER NOT NULL
);

-- A rate-limited request may count as more than one: a market read counts its items.
ALTER TABLE rate_events ADD COLUMN weight INTEGER NOT NULL DEFAULT 1;
