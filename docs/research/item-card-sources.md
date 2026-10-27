# Sources for the Item Card: shop prices, market prices and drops

Checked on 11 October 2026, against OSMS Data Explorer's `data/current` (its "Public Release" set, `items.json` last modified 10 October), BasilMarket.net's live price list (built 11 October 15:25 UTC), and the sites that carry MapleStory Classic World drop tables. Every request identified itself as Minerva. The decisions it led to are in ADR 0010.

## Answer

OSMS's item files give every item's shop price, kind, quests and recipes, read as plainly as its map files. No market price source meets ADR 0010's test any more: BasilMarket.net gave players' prices through the JSON its own search downloads, but it began refusing automated access the same day (below). MesoWatch's terms welcome links, so the card links to it. No source that meets the test has enough drop data yet; MapleClassic Wiki is the one to watch (#78).

## OSMS's item files

The site loads them all at startup from `https://osmsdataexplorer.com/data/current/`:

| File | Size | What it holds |
| --- | --- | --- |
| `items.json` | 499 KB | `items` (2,012: Equipment 1,203, Consumable 198, Etc 385, Setup 226) and `scrolls` (208), by the game's item ID |
| `quests.json` | 470 KB | 316 quests: `rewards` (items with `guaranteed`), `reward_weighted` (each item's `chance_pct`), `reward_choices` (pick one), and `requirements_list` (items, mobs, quests, by `type`) |
| `crafting.json` | 92 KB | 348 recipes by discipline and level: `output_id`, `result_item_name`, `meso_cost`, and ingredients by `item_name` only |
| `cash_shop.json` | 679 KB | 3,232 Cash Shop items by category, with their NX price |

- **Shop price:** top-level `price` on equipment, `stats.price` on the rest; 1,834 items have one. Pink Jester's is a string (`"1800"`).
- **Won't buy:** `stats.notSale: 1` on 96 items, which carry a placeholder price of 1.
- **No price:** the 208 scrolls, 23 starter equips (Sword, White Undershirt...) and arrows.
- **Throwing stars and bullets:** `stats.unitPrice` (0.3 to 1.0) beside `price`, which OSMS labels "mesos per unit": what recharging costs. What selling them pays wasn't confirmed.
- **Kinds:** equipment's `sub_category` (Hat, Top, Overall, Earring...), weapons' `weapon_type` ("1H Sword", "Claw"); the rest by `category`.
- **IDs:** a quest's requirement on another quest gives its `id` as a string (`"10002"`); every item ID is a number.
- **Shared names:** 35 names in `items.json` belong to several IDs. Most look alike and differ only in which quests name them (Red Potion 2000000 and 2000007); 4 differ in price or kind (Blue Moon is a top at 11,000 and an earring at 10,500; Hero's Gladius a weapon and a quest item).
- **Not there:** drop tables (no file has them; drops are server-side), NPC shop stock, and any market or player-trade price.
- **Pictures:** `data/current/images/items/<8-digit id>.png`.

## BasilMarket.net

**Since 11 October 2026 (about 18:00 UTC), BasilMarket refuses automated access.** `/api/price-index` answers 403 to Minerva's user agent and to curl, with the body "Automated access to BasilMarket isn't allowed, including AI crawlers and scrapers. If you're a person and you're seeing this, let us know in the BasilMarket Discord.", and the headers `x-robots-tag: noai, noimageai` and `tdm-reservation: 1`. Its `robots.txt` now disallows `/` for a list of AI crawlers and `/api/` (among others) for everyone. Browsers still get the list. What follows was true earlier that day.

A fan-made revival of the original BasilMarket, run by a registered LLC (unnamed), launched 10 September 2026 for MapleStory Classic World. Contact is its Discord (`discord.gg/RezJphGE3t`).

- **Data:** players record sales: trades completed on the site, and sales seen in Free Market shop screenshots they upload. "Only sales set the recommended price … With no sales at all, the NPC shop price is shown" (`/prices`).
- **`/api/price-index`:** the list the site's "What's it worth?" box downloads: 539 KB uncompressed, `{builtAt, items: [{n, s, i, t, v, b, k}]}`. `n` name (unique), `s` page slug, `i` the game's item ID as `"01302006.png"`, `t` type, `v` recommended price or `null`, `b` its basis (`sales` 65, `fm-sales` 206, `npc` 1,034, `none` 3,787 of 5,092), `k` sales behind it. Cached publicly for 5 minutes. No login, token or challenge. 210 of the 271 items with sales are in OSMS's `items.json`; most of the rest are Cash items.
- **`/api/market/sightings?item=<name>`:** one item's asking and sale figures (percentiles, counts, last seen), with the rows behind them, which carry players' usernames and shop names.
- **Terms (`/terms`):** nothing on automated access, scraping, APIs or reuse. "You keep ownership of what you post, and you give BasilMarket permission to show it on the site, in the app and in Discord." "Provided as is, with no guarantee … that any price or stat is correct." No `robots.txt`, privacy page or API docs.
- **MesoWatch** (`meso.watch`): prices read from public stream VODs, a median of observed sales. Its terms forbid any "script, crawler or AI system" collecting the price table or data files, and reuse without written permission, but say "Share a link to the site" and welcome search indexing. Search engines list only its home page and terms: one page with a search box, whose address doesn't change for an item (checked in a browser by the project's owner), so it can't be opened at an item.
- **Others:** MeowDB's Free Market and classicfreemarket.com forbid scraping; kerning.gg, EvilEyeFM and mapleclassicfm.com have no JSON source. Nexon's Open API doesn't cover GMS or market prices.

## maplestory.market

Checked on 27 October 2026, from the site's own scripts (it is a single-page app, so every path, `/robots.txt` and `/terms` among them, answers with its page), with Minerva's user agent.

- **Data:** "Listings are what's in player stores now, from scans of every Free Market room about once an hour." "Sales are inferred when a listing's stock drops between two scans at the same price." An item's established price is its 3-day average sale price, or with too few sales its median asking price; a weapon's is also figured by attack roll. Its item pages link to `maplestory.wiki/MCW/1/item/<id>`: Classic World's items, by the game's IDs.
- **Item pages:** `/items/<id>`, the game's item ID as a number (`/items/4010000`). Also `/stores/<playerId>`, `/owners/<name>`, `/deals`, `/market`.
- **API** (`/docs/api`): keys are free, each asked for on an account and approved by an admin; sent in the `Authorization` header. A new key makes 6 requests an hour; the admins set more on request; past it, 429 with `Retry-After`. Without one, every endpoint answers 401 `{"error":"session_required"}`. Endpoints: `/api/items` (every item for sale now), `/api/items/{id}` (its listings, price figures and recent sales), `/api/items/{id}/history` (hourly prices), `/api/listings` (every listing for sale now), `/api/prices` (each item's established price over the last 3 days), `/api/deals`, `/api/stores/{playerId}`, `/api/owners/{name}`, `/api/market/overview`, `/api/status`.
- **API terms** (`/docs/api/terms`, version 1, 22 October 2026): wherever its data shows to anyone but the key's holder, "Prices from maplestory.market" or similar, linking to the site, or to the item's page for one item, with the key's `?ref=<code>`; text alone where links aren't possible. "Showing prices and listings for what your users ask about is what keys are for"; no reselling, and no republishing in bulk. Cache, and stay within the limit. The key stays "on your server or in your bot, never in a web page's code", unshared, one per project. Keys used on web pages are checked daily for the credit. The data "can be incomplete, late or wrong, and the API can change or stop".
- **Removal:** a player can have their character's stores, sales and name kept off the site, proving the character theirs by a code in a store's title. The same proof is offered to other sites as "Verify with maplestory.market".

## Drops

- **MapleClassic Wiki** (`mapleclassic.wiki`): Classic World only, drop rates players counted from kills (rate, low–high range, kills and drops sampled, stack size), through MediaWiki Cargo (`api.php?action=cargoquery&tables=monster_drops`), which its `robots.txt` allows. 158 rows for 26 of the 119 mobs, almost all from one player, dated 11 October. Its licence reads CC BY-SA 4.0 on its About page and CC BY-NC-SA 4.0 in its footer.
- **Sleepywood** disallows `/api` and `*.json` in its `robots.txt`; **MSCW Guidebook**'s `AppData/monster_drops.json` answered 403; **Henesys.GG** forbids scraping; **MeowDB** forbids scrapers.
- **maplestory.io** (region `MCW`) and OSMS have no drops; private-server tables (HeavenMS, Cosmic) are another game's data.
