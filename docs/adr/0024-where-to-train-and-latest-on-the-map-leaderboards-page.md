---
status: accepted, amended by ADR-0027, ADR-0029
---

# Where to train and Latest on the Map Leaderboards page

The Map Leaderboards page (`/leaderboards/`, ADR 0015) found a map by its name and listed the maps with the most entries. It answered "what do players get on this map?" but not the question a player brings to it: "where should I train at my level?". With few players on the boards so far, every map in the most-uploaded list had one entry. This records what the page shows instead (#157): **Where to train**, a table of the maps for a Level Band, and **Latest**, the newest bests on every map.

## The decision

Where to train and Latest offer Both, Solo and Party filters and mark Party Bests (ADR 0015).

- **Where to train lists a Level Band's maps.** The player picks a band from a row of chips, as on a map's page (`?band=30`). The table lists every map the ceilings table matches to the band, and every map with an entry in it. Each row has the map's Map Mark, its name and street, its mobs' levels, the band's #1 there (name, job, level, EXP/h, date) and how many entries the band has there. Each row links to the map's Map Leaderboard at that band. A map with no entries there says so, and is still listed, so the table says where a player at the band can hunt from the first day.
- **The maps with entries come first**, highest #1 first, then the rest by their mob levels, lowest first. A row of **areas**, each the game's Map Mark and the area's name as the app shows them beside a map's name, narrows the table to one (`&area=KerningCity`, by OSMS's name for the mark). It offers only the areas the band's maps are in, so it never leads to an empty table, and an area the band lacks falls back to every area. A map without a Map Mark shows only among every area's. A filtered band with more than 20 maps shows its first 15 until Show all; otherwise every row shows.
- **A map is matched to a band by the levels of the middle half of its spawns**: from the level a quarter of its spawns are at or under to the level three quarters are, each mob counted as often as the map holds it at once (`MapData::middle_mob_levels`). A map is for a band when that range overlaps the band's levels. The full range of a map's mob levels, lowest to highest, was stretched by one stray mob: Sleepy Dungeon V read as levels 6 to 58, and would have been offered to players at levels 1–9. The middle half reads 33 to 37.
- **The ceilings table carries the levels and the marks.** `probe ceilings` writes `{ "maps": { "010001010": { "ceiling": 153216, "mobs": [3, 8], "mark": "Henesys" } }, "marks": { "Rith": { "name": "Lith Harbor", "picture": "https://osmsdataexplorer.com/data/current/images/map_marks/Rith.png" } } }`: each map's row (`MapSet::solo_ceilings`, which works its figures out with the core's own arithmetic), and each Map Mark as the app names it (`MapSet::site_marks`). The Worker matches maps to bands and areas from it, and the page loads the marks' pictures from OSMS, as a map's page loads the map's. The cap reads the ceilings as before. Where to train shows the levels and the marks, not the ceiling.
- **Latest lists the 10 newest entries on every map**, by when their Visit set them (`setAt`; no upload time is kept), each linking to its Map Leaderboard with the entry marked. An entry that took a top spot from another Character says so: "took #1, past Egbert's 150,000".
- **A top spot taken is kept** in a `taken_tops` table, written where ADR 0022 finds one, whether or not a Discord webhook is set: the Map Leaderboards are the record. A row marks its entry only while that entry stands: it goes when the maintainer deletes the entry, when its Character opts out, or when a new best of the same Character's replaces it. Another Character taking the spot in turn leaves it, since the entry that took it still stands. A Character who opts out loses their name from the spots taken off them; the row keeps what they had, and Latest says "past a best of 150,000".
- **Most uploaded goes.** Latest shows where players are active, with dates. Search still says how many entries each map it finds has, from `GET /api/maps`.

Latest shows each map's Map Mark, as Where to train does.

The page stands two **Pedestals**, each three Characters standing on steps, 2nd, 1st and 3rd from the left, with their Portraits (and ClassicRank's credit under them) and names linking to their pages. They rank Characters, never players or Accounts, as everything on the site does (ADR 0027).

- **Where:** beside the page's title on a wide page, in the room the title art leaves, and after Where to train on a narrow one, so the band choices come first. The page shows one Pedestal at a time, picked by its tab (Most podiums, then Highest EXP/h), with a "?" beside the tabs saying what the one shown ranks.
- **Most podiums** stands the three Characters with the most podiums, places in a Level Band's top 3 on a map. Podiums are counted as a Character's page counts them: each kind of entry is ranked on its own, so a Solo Best's place and a Party Best's in the same band each count, and the two figures always agree. It counts every band, both kinds and every area. A tie goes to whoever holds more #1s, then more #2s, then whoever set their first podium entry first.
- **Highest EXP/h** stands the three highest EXP/h on any map at any level, each Character by their best entry (the earlier on a tie), each with their job, level and map (a link to its page at the entry's band, the entry marked). A tie between them goes to whoever set it first. It ranks levels as much as players, since a higher level earns more EXP/h; Where to train ranks a band's maps for that.
- A step no one stands says "Yours could be here", linking to how to put your bests up.

## The API

Beside ADR 0015's, the Map Leaderboards page reads four more routes. They are public and cached for a minute, as the other reads are.

- `GET /api/bands`: every Level Band with maps, lowest first. `{ "bands": [{ "band": 20, "levels": [20, 29], "maps": 68, "entries": 7 }] }`. `maps` counts the maps the band's table lists, and `entries` the entries those maps have in the band. A band with neither isn't listed.
- `GET /api/bands/<band>`: the band's maps, by ID. `{ "band": 20, "levels": [20, 29], "maps": [{ "map": "010001010", "mobs": [3, 8], "mark": { "id": "Henesys", "name": "Henesys", "picture": "https://osmsdataexplorer.com/data/current/images/map_marks/Henesys.png" }, "entries": 2, "best": { "name": "Egbert", "job": "Magician", "level": 23, "expPerHour": 150000, "setAt": 1760000000000 } }] }`. A map the ceilings table doesn't hold has `mobs` and `mark` `null`, a map without a Map Mark has `mark` `null`, and a map with no entries in the band has `best` `null`. A band with no maps answers `maps: []`. `band` is the band's first level, as in ADR 0015; anything else is a 400.
- `GET /api/latest`: `{ "entries": [{ "name": "Hakuo", "job": "Bandit", "level": 26, "expPerHour": 153000, "setAt": 1760000000000, "map": "010001010", "band": 20, "took": { "from": "Egbert", "fromExpPerHour": 150000 } }] }`, newest first. `took` is `null` for an entry that took no top spot, and `from` is `null` once that Character has opted out.
- `GET /api/pedestals`: both Pedestals, read together, so the page switches between them without asking again. `{ "podiums": [{ "name": "Egbert", "podiums": 7, "portrait": { "photo": "...", "profile": "..." } }], "expPerHour": [{ "name": "Hakuo", "job": "Bandit", "level": 26, "expPerHour": 153000, "setAt": 1760000000000, "party": false, "map": "010001010", "mark": { "id", "name", "picture" }, "band": 20, "portrait": { ... } }] }`, each at most three, highest first, fewer while fewer qualify. `portrait` is as a band's entries give it (ADR 0029).

The page orders, filters and names the maps itself (`site/leaderboards/leaderboards.ts`), from the map names fixture, as it names every map. An Event or Dev map is left out unless it has entries, as search leaves it out.

## Considered options

- **The full range of mob levels.** The simplest rule to explain, but stray mobs stretch it (above).
- **The level of a map's median spawn, one band per map.** Tighter lists, but a player a few levels under a map's band never sees it in theirs. The middle half's overlap lists a map in each band whose players hunt it.
- **Fetching OSMS's set in the browser** to match maps to bands. It would commit nothing more, but means reading OSMS's shape again in TypeScript and loading its whole set for one table. The core already works the levels out; one more pair per map in a table already committed is far less.
- **Only maps with entries.** It would keep the data rule as it was, but with one Character on the boards most bands would be empty, and nothing would say what a map can give.
- **Each map's solo EXP/h ceiling beside its #1**, with the #1 as a share of it, a sort by ceiling, and a mark on a #1 above it (a party, an EXP event or a buff). Where to train shows what players got on each map, and the ceiling stays the cap's alone.
- **An upload time for Latest.** A first upload of an old Visit History would then show as new. But that needs a column on every entry, and Latest's date is when the best was set, which is what a player reads it as.

## Consequences

- The ceilings table now holds each map's mob levels and Map Mark, and the marks' names and pictures' addresses, so it is a third piece of OSMS's data committed, after the map names fixture (ADR 0008) and the ceilings (ADR 0015). AGENTS.md's rule on committed data says so. The marks' pictures themselves stay on OSMS. It is written again with `probe ceilings` when OSMS's set changes, as before.
- An upload that takes a top spot writes a row more, beside ADR 0022's two reads. One that doesn't, the common case, writes one statement more: the delete of its Character's marks whose entry has gone.
- These routes read no key or cookie and may be cached. ADR 0023 records the public-read allowlist.
- Names a player uploads now also show in Latest, for as long as an entry is among the newest. The FAQ says so (`leaderboard-public`), and answers how Where to train picks a band's maps (`where-to-train`).
