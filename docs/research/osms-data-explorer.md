# OSMS Data Explorer as the data source in place of MeowDB

Checked on 10 October 2026, against OSMS's `data/current` (its "Public Release" set, dated 6 October 2026), the MeowDB pages then kept in `crates/minerva-core/tests/fixtures` (saved 7 October, since removed), and three live MeowDB pages. The decisions it led to, and what shipped, are in ADR 0008.

This is the pre-migration field comparison. [ADR 0008](../adr/0008-map-and-mob-data-from-osms-data-explorer.md) records the completed switch, source-access decision, Map Preview, dropped movement line and retained MeowDB Item Lookup. Counts and comparisons below describe the October 2026 snapshot, not current coverage.

## What OSMS is

[osmsdataexplorer.com](https://osmsdataexplorer.com/#overview) is a GitHub Pages site behind Cloudflare, built from the public repo [ohmi69/osms_datamine_dashboard](https://github.com/ohmi69/osms_datamine_dashboard) ("Made by @ohmi on Discord"). The repo was last pushed on 10 October 2026. The browser fetches these files from `https://osmsdataexplorer.com/data/current/`:

| File | Size | What Minerva would use |
| --- | --- | --- |
| `maps.json` | 349 KB | every map: ID, name, street, region, `is_town`, `return_map_id`, `exits`, `mob_rate`, `mob_positions` (one entry per spawn point), `map_mark` (the Mini Map's emblem for its area, `"None"` for none); and `map_mark_icons`, each mark's 38×38 PNG under `data/current/images/map_marks/` (11 in October 2026) |
| `portals.json` | 121 KB | every map's portals: `name`, `type`, `dest_map`, `x`, `y` |
| `monsters.json` | 90 KB | every mob's stats, elements, and its maps with spawn counts and `mob_time` |
| `lookups.json` | 223 KB | names by ID: 7,431 items, 318 maps, 101 mobs, 226 NPCs |
| `items.json` | 511 KB | items and scrolls with game IDs and categories |
| `map_manifest.json` | 15 KB | map ID to rendered map image (`data/maps/<hash>.webp`) |

Past datasets are kept and fixed under `data/patches/<version>/` (`cot1`, `cot2`, `v49`, ...), listed in `data/patches/index.json`. `patch_notes.json` lists what changed from COT2 to the Public Release.

## Field by field

### Map index (ADR 0003)

`maps.json` has every map's ID, name, street and region, which is everything `MapEntry` needs. IDs are numbers (`10001010`), not MeowDB's 9-digit strings, so they need zero-padding.

### A map's data (`MapData`)

Compared on the 9 fixture maps OSMS has:

| `MapData` field | MeowDB | OSMS | Result |
| --- | --- | --- | --- |
| mobs and Spawn Counts | `monsters[].count` | `mob_positions` grouped by mob, or `monsters[].maps[].count` | identical on all 9 |
| mob name, level, EXP, HP | flight data + Monsters table | `monsters.json` | identical |
| `connected_maps` | `connectedMaps` | `exits` | identical, except that Kerning City also leads to Backalley Semi-Basement (`088001000`), new in the Public Release |
| `portals` (name, type, destination) | `portals` | `portals.json` | identical on all 9 after the core's existing filter (OSMS also lists portals to the same map, which the core drops) |
| portal types | 1 invisible, 10 hidden, 11 script | same numbers | identical, so the Hidden Portal rule and `PORTAL_OVERRIDES` carry over unchanged |
| `mob_rate_percent` | `mobRate` | `mob_rate`, absent when ×1 | identical once absent is read as ×1. The fixtures' differences (Sunset Sky ×0.5→×0.6, Northern Top ×2.0→×1.8) are Public Release changes, and live MeowDB has them too |
| `nearest_town` | `returnMap` | `return_map_id` | identical where OSMS gives one. OSMS leaves it out on 18 of its 92 Towns (Henesys, Kerning City, the Mushroom Towns), where MeowDB gives the Town itself, and on 4 Hidden Street event maps. The reader would take a missing one as the map itself on a Town, and as none elsewhere |
| `is_town` | `isTown` | `is_town`, absent when false | identical: the same as MeowDB's town flag on all 298 maps both have. It also covers shops and event rooms (92 maps), so Doors work as they do now |
| Hidden Portal destination name and street | `toMapName`, `toStreetName` | `lookups.json` names + `maps.json` streets | available |
| Hidden Portal position | portal x, y with `minimapWidth/Height`, `miniMapCenterX/Y` | portal x, y in the rendered map image's pixels; **no minimap bounds** | **gap**, see below |

### Map Summary

MeowDB computes the Map Summary itself. Its figures come out exactly from OSMS's spawn data with this rule, built on the same 7.56 s respawn that OSMS's own map table uses:

- A spawn point without a timer gives `3600 / 7.56` mobs an hour to a full party, and 0.75 of that alone.
- A timed spawn point gives `3600 / mob_time` an hour, alone or in a party.
- Each mob's figure is rounded, and the map's figure is the sum of its mobs'. The map's Mob Rate isn't applied, as MeowDB doesn't apply it.

All four figures (mobs and EXP an hour, alone and with a full party) matched to the unit on the 6 fixture maps with mobs, and on live MeowDB's The Pig Beach, whose Iron Hog spawns every 3600 s. Ranged timers weren't checked: OSMS gives some as a range with an average (`"180.00-1800.00 [720.00]"`), and MeowDB has a `timedRespawnAvg`, but whether the two averages agree is unknown.

### Mob Card (ADR 0006)

Compared on the 12 fixture mobs OSMS has:

| `MobDetails` field | OSMS | Result |
| --- | --- | --- |
| elements | `elements` (`{"Fire":"Weak"}`) | identical |
| ACC, AVOID, P.DEF, M.DEF | `acc`, `eva`, `PDDamage`, `MDDamage`; absent or 0 where MeowDB shows "—" | identical |
| MP | `mp` | available. MeowDB omits Mushmom's (200), while OSMS gives it |
| HP per EXP | `hp / exp` | identical to 2 decimals. The rank becomes "of 92" (mobs in the game now), not MeowDB's "of 182", which includes COT2-only mobs |
| Touch DMG | `PADamage` (one number) | MeowDB's range fits `floor(1.1 × P.ATK)` to `ceil(1.5 × P.ATK) − 1` on all 12 mobs, Blue Snail (5) to Mushmom (532). This is fitted, not documented: OSMS's formulas pages don't cover mob damage |
| Speed | `speed` | fits on all 12: below 0 is "Slow", absent is "Normal", above 0 is "Fast". Fitted |
| Movement | the animation names in `gifs` | "Flies" when there is a `fly` animation. **"Jumps between floors" vs "Stays on its floor" is a gap**: Slime and Orange Mushroom jump, but OSMS lists no `jump` animation for them |

### Mob images

Each mob in `monsters.json` names its animations (`move`, `stand`, `fly`, `attack1`, ...) as animated WebP files under `data/images/monsters/<hash>.webp`, plus a PNG thumbnail. They are served by Cloudflare with a 4-hour cache. They could replace maplestory.io, and as animations, which ADR 0006 passed on for lack of frame timings.

### Item Lookup (ADR 0007)

OSMS has every item's name with its game ID (7,431 in `lookups.json`), straight from the game's data. A reading could be matched offline against that list, so ADR 0007's problem of MeowDB's list missing 68 items goes away. "Hat MP Scroll: Greater", ADR 0007's example of a missing item, is in it, as are all 54 item names in the MeowDB search answers the tests then kept. But MeowDB's item pages are numbered by MeowDB's own IDs, and OSMS doesn't carry them. Opening the right MeowDB page still needs one request per press to MeowDB's search, as now. Otherwise Item Lookup could open OSMS's own listing, which needs no request to anyone. OSMS has no page per item. Its Items tab (consumables, etc., setup, scrolls) and Equipment tab take a search in the link, such as `#items?q=<name>` or `id:<game ID>`, which its own cross-links use. That wasn't tried for the Equipment tab.

## How this was checked

OSMS's site code (`main.js`, `lib/data.js`, `tabs/maps/MapBrowser.js`, `tabs/maps/MapPortalOverlay.js`, `tabs/formulas.js`) and its `data/current` and `data/patches/cot2` files were read. Node scripts compared the 12 MeowDB map fixtures and 14 mob fixtures with OSMS, field by field: 9 maps and 12 mobs overlap, and the rest are Ossyria content. Three live MeowDB requests were made, each identifying itself as Minerva: Sunset Sky (`010003070`, freshness), the map index (coverage), and The Pig Beach (`010000021`, timed spawns). OSMS's `robots.txt` was read, and its files were fetched once more with Minerva's user agent. Later, two MeowDB search URLs were tried, the second also in a browser.
