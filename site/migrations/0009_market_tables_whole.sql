-- maplestory.market's tables kept whole (ADR 0030): each one a single JSON object in its
-- `market_pulls` row, in place of a row per item. A pull wrote every item's row again, and
-- deleted the last pull's: thousands of D1 rows written an hour, more than the day's allowance
-- on the Workers Free plan. Now it writes one. A row holds at most 2 MB in D1, and the whole
-- database, both tables in it, was under 800 kB. Additive only: the Worker before this keeps working
-- until the deploy after, and the first read after that pulls each table again.
-- `market_prices` and `market_listed` are dropped by a later migration.

-- The table as last kept, by item ID: /api/prices' figures for each item less its figures by
-- attack roll and its deal price, or /api/items' `min_unit_price` and `listings`. NULL until
-- the Worker after this pulls it.
ALTER TABLE market_pulls ADD COLUMN items TEXT;
