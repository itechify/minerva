# AGENTS.md

## Build and test

- `cargo test`: runs the `minerva-core` tests. The core is the default workspace member, so this never builds Tauri.
- `cargo test -p minerva --lib`: runs the app's own unit tests, which `cargo test` leaves out.
- `cargo clippy --workspace --all-targets -- -D warnings`: lint everything.
- `pnpm install` then `pnpm tauri build --debug --no-bundle`: build the app. `pnpm tauri dev` runs it.
- `pnpm site:dev` runs the site (`site/`, the player site at owlofminerva.app); `pnpm site:build` builds it, and fails while a screenshot it shows is missing or the home page names a FAQ question there isn't.
- `pnpm site:test`: type-check the site's Worker (`site/worker`, the Map Leaderboards API, ADR 0015) and run its tests in workerd against a local D1 with `site/migrations` applied. Its one-time setup on Cloudflare is `docs/map-leaderboards-setup.md`.
- `cargo run -p minerva --example probe [reads] [out.png]`: read-only smoke test against a running game, with no UI.
- `cargo run -p minerva --example probe record <out-dir> <name>=<x>,<y>,<w>,<h>@<hz> ... [--seconds <n>]`: record timestamped crops of client-area regions from a running game, for samples.
- `cargo run -p minerva --example probe item samples/item-tooltip "<item name>" [--desc <tag>] [--delay <s>]`: after a countdown, save the game's client area named with the item and the pointer's position, for samples.
- `cargo run -p minerva --example probe osms`: fetch OSMS Data Explorer's live file set and the Item Card's item files and read them in full with the core's readers, reporting their counts or what didn't read. No game is needed.
- `cargo run -p minerva --example probe ceilings site/worker/osms-ceilings.json [--osms <osms-set.json>]`: fetch OSMS's live file set (or read the one given) and write each map's solo EXP/h ceiling, by its 9-digit ID, with the core's Map Summary arithmetic: the table the Map Leaderboards' cap loads. Run it and commit the table when OSMS's set changes. No game is needed.
- `cargo run -p minerva --example probe update-manifest <out> [--bundle-dir <dir>]`: after a signed `pnpm tauri build`, write the updater's `latest.json` for the version in `tauri.conf.json`, with the setup `.exe` and the MSI at their GitHub Release addresses and their `.sig`s (from `target/release/bundle` by default). It refuses a bare `windows-x86_64` key, and a `.sig` that is missing or signed as another version. Releasing (`docs/releasing.md`) runs it.
- `cargo run -p minerva --example probe replay <report.zip> [--all] [--osms <osms-set.json>] [--items <osms-items.json>]`: print a Bug Report's timeline, then replay it through the core and say where it diverges. Start here with any Bug Report. No game is needed.

## Releasing

- `docs/releasing.md`: how versions are numbered (the four files that hold the version), and the steps to bump, merge, build, tag and publish a GitHub Release, with the release notes template. Follow it for every release.

## Code rules

- All logic goes in `crates/minerva-core`, which takes plain data in and returns a `PanelState`. It has no OS, UI, network or file dependencies.
- Tests drive the core's public inputs and assert on `PanelState` only, never on internals. Every file in `samples/minimap` must be recognised against the map names fixture, `crates/minerva-core/tests/fixtures/osms-map-names.json`; there, `--window` samples go through `default_title_region` (the probe's capture). The app captures the Title Region the player marks at setup, which `tests/setup.rs` covers with marks on the `--window` samples.
- Every file in `samples/exp-bar` must read as its name (or its recording's `readings.json`) says, or as unreadable, never as a wrong number, in two layers (ADR 0017): the files there are the EXP figures alone, fed as a frame of them (`tests/exp_bar_samples.rs`), and the `--window` samples in `samples/minimap` are found in their Status Bar and read too. A number ever read wrong becomes a sample of the figures there: a Bug Report's `crops/exp-bar-<ms>.png` is one already. A Status Bar whose figures are ever not found, or found in the wrong place, becomes a sample of the bottom of the client area, its name's line painted over as for `samples/status-bar` unless that rule allows it.
- Every file in `samples/item-tooltip` must read as the item its name says (or as no Item Tooltip, for `no-tooltip`), or as unreadable, never as another item (`tests/item_tooltip_samples.rs`). A frame that is ever read wrong becomes a sample there: a Bug Report's `crops/item-tooltip-<ms>@<x>,<y>.png` is one already, renamed `<item-name>@<x>,<y>.png`, cropped to the tooltip so no other player's name or chat is committed (the pointer's spot moved with it).
- Every file in `samples/player-shop` must read as its readings say (`<name>.json`: each listing not sold out, with its item's name, quantity and mesos, or `{ "covered": true }` for one the pointer or the game's tooltip covers), or as unreadable ("?"), never as another item or price (`tests/player_shop_samples.rs`). A frame that is ever read wrong becomes a sample there: a Bug Report's `crops/player-shop-<ms>.png` is one already, cropped to the shop's list so no seller's name or chat is committed.
- Every file in `samples/status-bar` must read as its name says (`<level>__<job>__<Name>--<what>.png`; `<level>__<job>--<what>.png` for a crop with the name taken out; `no-status-bar--<what>.png`), or as unreadable, never as another level, job or name (`tests/status_bar_samples.rs`). A frame that is ever read wrong becomes a sample there: a Bug Report's `crops/status-bar-<ms>.png` is one already, cut to the level, job and name. Names may only come from the maintainer's own Characters, or from players who agree to it in the Bug Report flow; any other crop is cut down to the level and job before it is committed, its name's line painted over in the bar's plain slate (58,67,72).
- Tests run on made-up worlds in OSMS's shape (`crates/minerva-core/tests/common/osms.rs`); the map names fixture (IDs, streets, map names and regions) and the ceilings table (`site/worker/osms-ceilings.json`, each map's solo EXP/h ceiling as the core works it out, written by `probe ceilings`) are the only data committed from OSMS. The Worker's tests use made-up ceilings (`site/worker/vitest.config.ts`), and check only the table's shape. `probe osms` must read OSMS's live set and item files in full. A shape OSMS ever serves that reads wrong becomes a made-up fixture with that shape. Item Lookup's tests write their own made-up answers in MeowDB's search shape (`tests/item_lookup.rs`); nothing from MeowDB is committed. The Item Card's tests use made-up item files in OSMS's shape (`tests/common/items.rs`).
- Bug Reports (ADR 0009): every input `Minerva` takes writes itself down, so a new input needs an `Event` and a line in `bug_report::replay`. A Bug Report is never committed: it becomes the smallest test that reproduces its bug, on a made-up world, and a frame in it read wrong becomes a sample.
- The site (`site/`) is for players, and the README keeps only the developer's side. Its home page shows what Minerva does, a feature at a time in a sentence or two; the FAQ (`site/faq/index.html`) holds every detail a player needs, as the questions they ask, each written once there and shown again on the home page by its id. A change a player notices updates the site in the same PR (the feature's words, or the FAQ's answer), the setup warning's wording included, and lists each screenshot it alters under Retake in `site/screenshots/README.md`.
- `src-tauri` only adapts window lookup, capture and rendering. It must follow ADR 0001: no `PrintWindow`, no messages, input, hooks or process access to the game.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues on `itechify/minerva`, using the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the five default triage labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human` and `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
