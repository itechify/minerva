-- maplestory.market's tables, a row per item, dropped (ADR 0030). Since 0009 the Worker keeps
-- each table whole in its `market_pulls` row and never reads or writes these. 0009 kept them
-- only so the Worker before it kept working until the deploy after.

DROP TABLE market_prices;
DROP TABLE market_listed;
