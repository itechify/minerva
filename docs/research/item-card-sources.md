# Sources for the Item Card: shop prices, market prices and drops

Checked on 11 October 2026, against OSMS Data Explorer's `data/current` (its "Public Release" set, `items.json` last modified 10 October), BasilMarket.net's live price list (built 11 October 15:25 UTC), and the sites that carry MapleStory Classic World drop tables. Every request identified itself as Minerva. The decisions it led to are in ADR 0010.

## Answer

OSMS's item files supply shop prices, kinds, quests and recipes. The Item Card gets Going rate and Cheapest now from maplestory.market through the site's Worker and links to the item's page there (ADR 0030; `site/worker/market.ts`). The drop-source review below found no qualifying source with enough data; MapleClassic Wiki was the one to watch (#78).

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

## Rejected market sources (11 October 2026)

[BasilMarket](https://basilmarket.net) began refusing automated access that day. [MesoWatch](https://meso.watch) permits links but forbids automated collection and reuse without permission; it has no item-specific links. MeowDB's Free Market and classicfreemarket.com forbid scraping; kerning.gg, EvilEyeFM and mapleclassicfm.com had no JSON source. Nexon's Open API did not cover GMS market prices. These sources were rejected under ADR 0010; ADR 0030 records the later choice of maplestory.market.

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
