<h1><img src="assets/logo.png" width="300" alt="Minerva"></h1>

Your guide for MapleStory Classic World: a helper overlay that reads the minimap title by ordinary screen capture and shows which map you are on. It never touches the game's memory, files or input ([ADR 0001](docs/adr/0001-observe-game-from-outside-only.md)).

**[The Minerva site](https://owlofminerva.app/)** explains what it does and how to use it: setup, the panel, hunting, the Grind Tracker, the Item Card, Item Lookup, the Deal Finder and the hotkeys, and holds the Map Leaderboards. This README is the developer's side. Players and developers talk about Minerva on [its Discord](https://discord.gg/NnbnVWkRrF).

Its map and mob data come from the files [OSMS Data Explorer](https://osmsdataexplorer.com) publishes, and its links open [NiaMeowDB](https://meowdb.com)'s pages ([ADR 0008](docs/adr/0008-map-and-mob-data-from-osms-data-explorer.md)).

## Install

Download the latest Windows installer from [Releases](https://github.com/itechify/minerva/releases/latest). Each release says which file to pick. Minerva isn't made, endorsed or supported by NEXON, and using any third-party tool with MapleStory is at your own risk. Help & Setup's About & Credits credits the data, the recognition model and the typeface, and [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) lists everything bundled.

## Layout

Architecture and test rules live in [AGENTS.md](AGENTS.md).

- `crates/minerva-core`: recognition, matching and product logic; plain inputs to panel state.
- `src-tauri`: window lookup, capture, storage and network adapters ([ADR 0021](docs/adr/0021-the-minimap-title-is-found-in-the-client-area.md), [ADR 0013](docs/adr/0013-market-imports-taken-in-while-minerva-runs.md), [ADR 0014](docs/adr/0014-deal-finder-finds-the-player-shop-in-the-client-area.md), [ADR 0016](docs/adr/0016-characters-are-found-in-the-status-bar.md), [ADR 0017](docs/adr/0017-exp-figures-are-found-in-the-status-bar.md)).
- `src`, `index.html`, `setup.html`: vanilla TypeScript UI for the overlay and setup.
- `site`: player pages and their Worker API ([ADR 0015](docs/adr/0015-map-leaderboards-on-the-sites-worker.md), [ADR 0022](docs/adr/0022-a-new-1-on-a-map-is-posted-to-discord.md)). Vite builds the pages; `.github/workflows/site.yml` deploys them. Screenshot instructions: `site/screenshots/README.md`.
- `samples`: game captures used by recognition tests; naming, cropping and privacy rules are in [samples/README.md](samples/README.md).

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
pnpm site:dev                                   # the site, at http://localhost:5173/ (MINERVA_WORKER_PORT=<port> for site:local's /api/, below)
pnpm site:build                                 # the site, into site/dist (fails while a screenshot is missing)
pnpm site:test                                  # the site's Worker (the Map Leaderboards API, against a local D1) and what its pages show
pnpm site:local [--seed] [--key] [--build]      # the built site and its API on a free port, with a local D1 in _scratch/; prints the port
pnpm site:local stop                            # stop the one this worktree started
cargo run -p minerva --example probe ceilings site/worker/osms-ceilings.json  # no game: the Map Leaderboards' cap and Where to train, from OSMS
```

All probe modes and options, including `record`, `item`, `loot` and `update-manifest`, are documented in the opening comment of [src-tauri/examples/probe/main.rs](src-tauri/examples/probe/main.rs). See [AGENTS.md](AGENTS.md) for fixture rules and when to check live data.

Run the game windowed or borderless, with the minimap open.

When play shows a portal the Map Preview's rule gets wrong, add it to the override list in `crates/minerva-core/src/portal_overrides.rs`, by map ID and the portal's name in OSMS's `portals.json`.

## Credits

Map, mob and item data, map pictures, Map Marks, mob animations and item pictures come from [OSMS Data Explorer](https://osmsdataexplorer.com), fetched as it runs and never bundled. Map, mob and item links open [NiaMeowDB](https://meowdb.com)'s pages, and Item Lookup uses its search ([ADR 0007](docs/adr/0007-item-lookup-on-a-hotkey-through-meowdb-search.md)); the Item Card's Going rate and Cheapest now come from [maplestory.market](https://maplestory.market), asked of the site's Worker, which holds its key, and its market section opens the item's page there ([ADR 0030](docs/adr/0030-market-prices-from-maplestory-market.md)) ([ADR 0010](docs/adr/0010-item-card-from-osms-item-files.md)). The Map Summary is worked out as NiaMeowDB works it out.

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
