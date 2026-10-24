<h1><img src="assets/logo.png" width="300" alt="Minerva"></h1>

Your guide for MapleStory Classic World: a helper overlay that reads the minimap title by ordinary screen capture and shows which map you are on. It never touches the game's memory, files or input ([ADR 0001](docs/adr/0001-observe-game-from-outside-only.md)).

**[The Minerva site](https://owlofminerva.app/)** explains what it does and how to use it: setup, the panel, hunting, the Grind Tracker, the Item Card, Item Lookup, the Deal Finder and the hotkeys, and holds the Map Leaderboards. This README is the developer's side. Players and developers talk about Minerva on [its Discord](https://discord.gg/NnbnVWkRrF).

Its map and mob data come from the files [OSMS Data Explorer](https://osmsdataexplorer.com) publishes, and its links open [NiaMeowDB](https://meowdb.com)'s pages ([ADR 0008](docs/adr/0008-map-and-mob-data-from-osms-data-explorer.md)).

## Install

Download the latest Windows installer from [Releases](https://github.com/itechify/minerva/releases/latest). Each release says which file to pick. Minerva isn't made, endorsed or supported by NEXON, and using any third-party tool with MapleStory is at your own risk. Help & Setup's About & Credits credits the data, the recognition model and the typeface, and [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) lists everything bundled.

## Layout

- `crates/minerva-core`: the headless core (no OS or UI dependencies). Captures of the game go in, a panel state comes out. It is the only test seam.
- `src-tauri`: the Tauri v2 shell. It finds the "MapleStory" window and moves the overlay with it (20 times a second), and on a thread of its own captures the game's client area about once a second until the core finds the Minimap Title in it, open, collapsed or on a map without a minimap, then only where it was found about 3 times a second, feeding the readings to the core ([ADR 0021](docs/adr/0021-the-minimap-title-is-found-in-the-client-area.md)). It also fetches OSMS Data Explorer's five data files (under 1 MB, one at a time) at startup when its saved set is a day old or another version saved it, again every 24 hours while it runs, and when the player presses Refresh under Game Data in Settings. From them the core knows every map's mobs, portals and Map Summary and every mob's Mob Card, snaps readings to the map list, and searches the whole world from the first launch. For the Item Card it fetches OSMS's two item files (about 1.2 MB) on the same schedule, saved apart from the map set; the market prices the player keeps on Item Cards are saved in `market-prices.json` beside the settings, and the Market Imports a tool of the player's own leaves in `market-imports` beside it are taken in every 2 seconds ([ADR 0013](docs/adr/0013-market-imports-taken-in-while-minerva-runs.md)). The set is saved in `%LOCALAPPDATA%\com.itechify.minerva\cache` and used offline after that; once OSMS hasn't been reached for over a week, the panel says it shows a saved copy, with its date. The pictures of Search's matches (first, while it is open), every Map Mark, the shown map's picture and its mobs' animations are fetched from OSMS once, one at a time, and kept in the same folder until they go 30 days unused. MeowDB's caches from earlier versions are deleted at startup. While the player has the Deal Finder on, the same thread captures the game's whole client area every 400 ms and the core finds and reads the open Player Shop's listings in it ([ADR 0014](docs/adr/0014-deal-finder-finds-the-player-shop-in-the-client-area.md)). Every second it captures a strip across the bottom of the client area, and the core finds the game's Status Bar in it and reads the Character playing: their level, job and name ([ADR 0016](docs/adr/0016-characters-are-found-in-the-status-bar.md)); and, while the Grind Tracker is on, the EXP figures there, with no mark ([ADR 0017](docs/adr/0017-exp-figures-are-found-in-the-status-bar.md)).
- `src`, `index.html`, `setup.html`: the web UI (vanilla TypeScript) for the overlay and the setup window. It only renders state from the core.
- `site`: the Minerva site for players at owlofminerva.app: hand-written pages, the home page (`site/index.html`), the FAQ (`site/faq/index.html`), whose questions the other pages show again by id (`site/questions.ts`), and the Map Leaderboards (`site/leaderboards`), whose pages read the Worker's API (`site/worker`, [ADR 0015](docs/adr/0015-map-leaderboards-on-the-sites-worker.md)) and work out what they show in `site/leaderboards/leaderboards.ts`, built with Vite from `site/vite.config.ts` and sharing the app's tokens (`src/theme.css`) and hotkey names (`src/hotkeys.ts`). The Worker and the pages share `site/level-bands.ts`, `site/map-names.ts` and `site/entries.ts`, so a Level Band, a map's name and what counts as a name or a map ID are the same wherever they show; when an upload takes a map's top spot off another Character, the Worker posts a line about it to a Discord webhook (`site/worker/announce.ts`, [ADR 0022](docs/adr/0022-a-new-1-on-a-map-is-posted-to-discord.md)). It is published to Cloudflare (`site/wrangler.jsonc`) from a release's tag when the release is published (`.github/workflows/site.yml`), and a pull request that touches it is built by `.github/workflows/site-check.yml`. Its screenshots are taken by hand: `site/screenshots/README.md`.
- `samples/minimap`: test screenshots named `street-name__map-name[--variant].png`.
- `samples/player-shop`: Player Shop item lists, each with `<name>.json` saying what its listings read as.
- `samples/status-bar`: the Status Bar's level, job and name, named `<level>__<job>__<Name>--<what>.png` with what they read as.

## Build, test, run (Windows)

Needs Rust (stable, MSVC), the Visual Studio C++ Build Tools, Node and pnpm.

```sh
cargo test                                      # core tests (the default workspace member)
cargo test -p minerva                           # the app's and the probe's own unit tests
pnpm install
pnpm tauri dev                                  # run the overlay beside the game
pnpm tauri build --debug --no-bundle            # build target/debug/minerva.exe
cargo run -p minerva --example probe       # no UI: capture the title and print what the core reads
cargo run -p minerva --example probe osms  # no game: fetch OSMS's live data and read it in full
cargo run -p minerva --example probe replay <report.zip>  # no game: a Bug Report's timeline, replayed through the core
pnpm site:dev                                   # the site, at http://localhost:5173/ (its /api/ from wrangler dev, below)
pnpm site:build                                 # the site, into site/dist (fails while a screenshot is missing)
pnpm site:test                                  # the site's Worker (the Map Leaderboards API, against a local D1) and what its pages show
pnpm exec wrangler d1 migrations apply minerva-leaderboards --local --config site/wrangler.jsonc  # once: a local D1
pnpm exec wrangler dev --config site/wrangler.jsonc  # the built site and its API, at http://localhost:8787/
cargo run -p minerva --example probe ceilings site/worker/osms-ceilings.json  # no game: the Map Leaderboards' cap and Where to train, from OSMS
```

The tests run on made-up worlds in OSMS's shape and a list of the game's map names; `probe osms` checks the live data, by hand when OSMS changes and before a release.

Run the game windowed or borderless, with the minimap open.

When play shows a portal the Map Preview's rule gets wrong, add it to the override list in `crates/minerva-core/src/portal_overrides.rs`, by map ID and the portal's name in OSMS's `portals.json`.

## Credits

Map, mob and item data, map pictures, Map Marks, mob animations and item pictures come from [OSMS Data Explorer](https://osmsdataexplorer.com), fetched as it runs and never bundled. Map, mob and item links open [NiaMeowDB](https://meowdb.com)'s pages, and Item Lookup uses its search ([ADR 0007](docs/adr/0007-item-lookup-on-a-hotkey-through-meowdb-search.md)); the Item Card's market line opens [MesoWatch](https://meso.watch) ([ADR 0010](docs/adr/0010-item-card-from-osms-item-files.md)). The Map Summary is worked out as NiaMeowDB works it out.

The windows are set in NEXON's Maplestory typeface; see `assets/fonts/NOTICE.md`.

Title recognition uses [ocrs](https://github.com/robertknight/ocrs). Its recognition model is CC BY-SA 4.0; see `crates/minerva-core/models/NOTICE.md`.

Created in [T3 Code](https://t3.codes).

## Licence

Minerva's code and docs are under the [MIT licence](LICENSE), copyright Jeffrey Davis. The licence doesn't cover:

- The name Minerva and the owl's art (`assets/icon*.png`, `assets/logo.png`, `assets/deal-finder-owl*.png` and `src-tauri/icons/`). A fork can reuse the code, under a name and art of its own.
- `samples/`: frames of the game, whose pictures are NEXON's.
- `crates/minerva-core/tests/fixtures/osms-map-names.json`: the game's map names, from OSMS Data Explorer ([ADR 0008](docs/adr/0008-map-and-mob-data-from-osms-data-explorer.md)).
- `assets/fonts/` and `site/fonts/`: NEXON's Maplestory typeface; see `assets/fonts/NOTICE.md`.
- `crates/minerva-core/models/`: ocrs's text-recognition model, under CC BY-SA 4.0; see `crates/minerva-core/models/NOTICE.md`.

[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) lists everything else the app bundles, each under its own licence.
