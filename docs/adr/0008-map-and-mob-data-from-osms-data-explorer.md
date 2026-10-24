---
status: accepted, amended by ADR-0010 (the Item Card reads OSMS's item files, a set of their own), ADR-0015 (a table of each map's solo ceiling is committed, for the Map Leaderboards' cap), ADR-0024 (the table holds each map's mob levels too, for Where to train), and 2026-10-21 (a set another version saved is fetched afresh at launch, and the player can Refresh it now)
---

# Map and mob data from OSMS Data Explorer's files, not MeowDB's pages

Minerva now takes every map's and mob's data from the JSON files that OSMS Data Explorer (osmsdataexplorer.com) publishes for its own site, not from MeowDB's pages. The files cover the whole game, so one fetch of under 1 MB replaces the crawl of about 420 map pages and the Mob Card page fetches. MeowDB's terms forbid scrapers (ADR 0003). OSMS has no terms, its `robots.txt` sets no rules, and its files are plain data, not pages to pick apart. Where both sites have a figure they agree (`docs/research/osms-data-explorer.md`). This supersedes ADR 0004 and ADR 0006, and ADR 0003 for maps and mobs (Saved Copies stay). It lifts ADR 0003's ban on a public release.

## What comes from where

- **The files:** from `data/current/`:
  - `maps.json`: every map's names, street, region, Town, nearest town, Mob Rate, exits and spawn points.
  - `portals.json`: every portal's name, type, destination and position, and for a teleport within the map, the portal it lands at.
  - `monsters.json`: every mob's figures, elements and animations, and the maps it spawns on.
  - `map_manifest.json`: each map's picture.
  - `lookups.json`: names of maps outside `maps.json`.

  They are fetched one at a time with Minerva's user agent, and no other pace. At launch the core looks at the set saved before, and uses it as it is while it's under a day old and this version of Minerva saved it (the saved file says which version did): a new version may read a set the old one couldn't, as the Grand Launch's set showed. Otherwise, and every 24 hours while running, the core has the set fetched afresh, so a set used at one daily ask is always fetched afresh at the next. The player can have it fetched afresh at once with Refresh under Game Data in Settings, which says when the set was fetched and what a Refresh came to: up to date, OSMS unreachable, or Minerva needing an update. A fetched set is saved only once the core's reader reads it, and is swapped in whole. A file that fails or isn't shaped as expected keeps the previous set. With no set saved, the core asks again every minute.
- **Saved Copies:** the panel says it shows a Saved Copy, with its date, only once the saved set is over 7 days old by the wall clock, because game data changes only on patch days. It says the format changed only when there's no previous set to fall back on, and that the map list couldn't be loaded only when nothing is saved.
- **Not bundled:** OSMS's repo has no licence, so nothing grants redistribution. Minerva fetches the files and ships none of them. The tests use made-up worlds in OSMS's shape and a committed list of the game's map names (IDs, streets, names and regions only). Nothing on the site restricts fetching, so we didn't ask OSMS's author.
- **The map list:** IDs are zero-padded to the game's 9 digits, which are MeowDB's too, and names are trimmed. The one map with no name or street is left out, so 304 of the 305 are listed. Regions are OSMS's, and Search and routes leave out its "Event" and "Dev" maps.
- **Map Summary:** OSMS doesn't give one, so Minerva computes it with MeowDB's rule.
  - A spawn point without a timer gives `3600 / 7.56` mobs an hour to a full party, and 0.75 of that to a player alone.
  - A timed spawn point gives `3600 / its timer`, alone or in a party. For a ranged timer (`"180.00-1800.00 [720.00]"`), the average OSMS gives is used.
  - Each mob's figure is rounded, and the map's is their sum, without the Mob Rate.

  This reproduced MeowDB's figures exactly on every map checked, though none had a ranged timer.
- **Mob Card:** its figures come from `monsters.json` with the rest of the set, so a listed mob's card is ready the moment its map is shown. The card has no loading, unavailable, saved copy or format states of its own.
  - Touch DMG is `floor(1.1 × P.ATK)` to `ceil(1.5 × P.ATK) − 1`.
  - Speed is "Slow" below 0, "Normal" when absent and "Fast" above 0.
  - HP per EXP is ranked among all the set's mobs, the mobs in the game now. Tied mobs share a rank, and a mob without EXP has no HP per EXP and ranks below the rest.
  - A figure the mob doesn't have, or has as 0, shows as "—". An element affinity the reader doesn't know makes the whole set a changed format.

  The Touch DMG and Speed rules were fitted to MeowDB's pages, not documented. The card drops its movement line, because OSMS can't tell a mob that jumps between floors from one that stays on its floor. The card's image is OSMS's default animation for the mob, which replaces maplestory.io's still.
- **Links out stay on MeowDB.** A mob's name, the map's title and the open-page hotkey still open MeowDB's pages; its IDs are the game's. Item Lookup keeps calling MeowDB's search (ADR 0007), Minerva's only remaining request to MeowDB. With the crawl gone, its searches keep MeowDB's pace of one every 2 seconds themselves. The letter of MeowDB's terms (§4, "automated scripts, bots, or scrapers") arguably covers that search. Its Content Reuse Policy, aimed at bulk scraping and republishing, doesn't. We take it as a grey area to raise with MeowDB, not a bar to release. MeowDB has no search page a browser could be sent to instead: `/msclassic/search` doesn't exist, and `/msclassic/item-db?q=` ignores the query.

## Portals on the Map Preview

OSMS gives a portal's position in the pixels of its own picture of the map, not against the minimap's bounds, which it doesn't give. So the Hidden Portals section becomes **Portals**, and stops naming a rough spot ("far right, lower") on a 5×3 grid.

- **The picture:** a Map Preview of the shown map, on every map OSMS has a picture of, Towns included. It is fitted to the panel's width and at most about 180 px tall.
- **Marks:** every portal but those to the dead end, as on OSMS's site:
  - blue to another map, Doors included;
  - green, with dashed arrows to where it lands, for a teleport within the map;
  - grey to a map Minerva doesn't know.
- **Hidden ones:** Hidden Portals and Quest Portals are dashed. A teleport within the map is never a Door, so an invisible one on a Town is dashed as hidden too, but it is never a Hidden Portal row. The Quest tag turns violet, so green means only "within the map".
- **Rows and hover:** a row per Hidden Portal stays below the picture, or "No hidden portals". Hovering a mark or a row lights the other, and a mark's card names where it leads, its street and its kind. A click does nothing. Collapsing the section hides both, and its heading and the icon chip still count Hidden Portals.
- **Shared titles:** maps sharing a title are still shown without asking when they'd show the same Map Summary, mobs and Hidden Portals, now compared by destination and kind. But they show no Map Preview until it's settled which one the player is on, since each has its own layout.

Pictures and mob animations are named by content hash. The core asks for them one at a time, the shown map's picture ahead of its mobs' animations, and keeps each answer for the run. It asks again five minutes on when OSMS couldn't be reached, but not for a picture OSMS doesn't have. The shell keeps them on disk, touched on each use and deleted after 30 days unused, and the overlay loads them through Minerva's own `osmsimage` URL scheme, never from OSMS. While the picture loads, the preview says "Loading map…"; if it can't be had, "Couldn't load the map picture" stands above the rows.

## Considered options

- **Keep MeowDB and ask for its permission** (ADR 0003's way out). It keeps the movement line and the minimap grid, but the crawl, its pace and the ban on a public release stay until MeowDB answers.
- **Keep the 5×3 grid, worked out from the picture's size.** It put 31 of 38 portals in MeowDB's cell. Every miss read "lower" as "middle", because the picture has margins that vary by map. The Map Preview shows the real spot and needs no bounds.
- **Read a fixed dataset** (`data/patches/<version>/`). It would never change shape under us, but it would miss each patch's changes.
- **Commit OSMS's files as test fixtures.** They're the most faithful fixtures, but the repo is going public and nothing grants redistribution. Tests use made-up files in OSMS's shape, a committed list of the game's map names, and a `probe osms` check that reads the live set in full.

## Consequences

- No world crawl and no "Mapping the world" progress. Search and Distance have the whole world from the first launch, and offline after it. Until the first set arrives, the search view says "Loading the map list…".
- The data is the game's client as it is now: 305 maps. Orbis, El Nath and Forgotten Hollow aren't in it. The hand-kept Ship and Teleport Scroll links into Ossyria are removed, and must be added back by hand when it returns.
- OSMS's files are undocumented and have changed shape before. The reader stays all or nothing: a file not as expected is a changed format, never partial figures. `probe osms` reads the live set in full; it's run by hand when OSMS changes and before a release.
- The Personal Bests page names every map with a best, with its region, mob levels and solo ceiling, from the set. A best on a map the set doesn't have is kept, and reads "Not in the map list".
- On the first launch of this version, the shell deletes MeowDB's old caches (the map index, map pages, mob pages and maplestory.io's images) and the crawl's saved world. Settings, Personal Bests and Item Lookup's saved searches stay.
- Before the repo goes public, MeowDB's pages and search answers are stripped from its history. Closed pull requests on GitHub still reach the old commits, which we accept.
- Each screen credits what it shows. The panel's footer (as the tooltip of its info icon) and the Personal Bests page read "Data: OSMS Data Explorer (osmsdataexplorer.com) · Pages: NiaMeowDB (meowdb.com)". The search view, and the Hunting section when no map is shown, open no pages, so they read "Data: OSMS Data Explorer (osmsdataexplorer.com)". Setup's test read says "Map names from OSMS Data Explorer", and the Personal Bests page calls solo EXP/h "the map's ceiling, worked out as NiaMeowDB does". The map pictures and mob animations are Nexon's art served by a fan site, as on every fan site.

## Since ADR 0010

The Item Card reads two more of OSMS's files (`items.json` and `cash_shop.json`), fetched on this set's schedule but read, saved and swapped as a set of their own, so a change in them never holds back the map set. Item Lookup's search still opens MeowDB's item pages, now only when the player asks for one.
