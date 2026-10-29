# AGENTS.md

## Build and test

- `cargo test`: runs the `minerva-core` tests. The core is the default workspace member, so this never builds Tauri.
- `cargo test -p minerva`: runs the app's own unit tests and the probe's (`update-manifest`'s), which `cargo test` leaves out.
- `cargo clippy --workspace --all-targets -- -D warnings`: lint everything.
- `pnpm install` then `pnpm tauri build --debug --no-bundle`: build the app. `pnpm tauri dev` runs it.
- `pnpm site:dev` runs the site (`site/`, the player site at owlofminerva.app); `pnpm site:build` builds it, and fails while a screenshot it shows is missing or the home page names a FAQ question there isn't.
- `pnpm site:test`: type-check the site's Worker (`site/worker`, the Map Leaderboards API, ADR 0015) and run its tests in workerd against a local D1 with `site/migrations` applied. Its one-time setup on Cloudflare is `docs/map-leaderboards-setup.md`.
- `pnpm shot <url> [--width 1280,390] [--full] [--eval '<js>'] [--wait <selector>]` and `pnpm preview:window <window> [state…]`: screenshots of a site page, or of an app window fed the fixture views in `preview/fixtures/<window>/` at its real width, into `_scratch/shots/` under a new name each time.
- `cargo run -p minerva --example probe [reads] [out.png]`: read-only smoke test against a running game, with no UI.
- `cargo run -p minerva --example probe record <out-dir> <name>=<x>,<y>,<w>,<h>@<hz> ... [--seconds <n>]`: record timestamped crops of client-area regions from a running game, for samples.
- `cargo run -p minerva --example probe item samples/item-tooltip "<item name>" [--desc <tag>] [--delay <s>]`: after a countdown, save the game's client area named with the item and the pointer's position, for samples.
- `cargo run -p minerva --example probe osms`: fetch OSMS Data Explorer's live file set and the Item Card's item files and read them in full with the core's readers, reporting their counts or what didn't read. No game is needed.
- `cargo run -p minerva --example probe loot <report.zip> [--items <osms-items.json>] [--from-ms <ms>] [--to-ms <ms>]`: replay a Bug Report's Pickup Log readings through the core's current Item naming and print the Loot tally, how many Item lines were named another Item than they read (and what plain edits alone would have named), and which stay as read; the window is in the trace's capture-time ms. The figures in ADR 0026 come from it. No game is needed.
- `cargo run -p minerva --example probe ceilings site/worker/osms-ceilings.json [--osms <osms-set.json>]`: fetch OSMS's live file set (or read the one given) and write each map's solo EXP/h ceiling, the levels of the middle half of its spawns and its Map Mark, by its 9-digit ID, and each Map Mark's area and picture, with the core's arithmetic: the table the Map Leaderboards' cap and Where to train load (ADR 0024). Run it and commit the table when OSMS's set changes. No game is needed.
- `cargo run -p minerva --example probe update-manifest <out> [--bundle-dir <dir>] [--base-url <url>]`: after a signed `pnpm tauri build`, write the updater's `latest.json` for the version in `tauri.conf.json`, with the setup `.exe` and the MSI at their GitHub Release addresses (or under `--base-url`, for the update smoke test's local server) and their `.sig`s (from `target/release/bundle` by default). It refuses a bare `windows-x86_64` key, and a `.sig` that is missing or signed as another version. Releasing (`docs/releasing.md`) runs it.
- `cargo run -p minerva --example probe replay <report.zip> [--all] [--osms <osms-set.json>] [--items <osms-items.json>]`: print a Bug Report's timeline, then replay it through the core and say where it diverges. Start here with any Bug Report. No game is needed.
- `.github/workflows/check.yml` runs the cargo checks and `pnpm build` on a pull request that touches code, and on `main`; `site-check.yml` the site's; `adr-check.yml` fails a pull request that gives two ADRs one number.

## Releasing

- `docs/releasing.md`: how versions are numbered (the four files that hold the version), and the steps to bump, merge, build, tag and publish a GitHub Release, with the release notes template. Follow it for every release.

## Code rules

- All logic goes in `crates/minerva-core`, which takes plain data in and returns a `PanelState`. It has no OS, UI, network or file dependencies.
- Tests drive the core's public inputs and assert on `PanelState` only, never on internals.
- Every file under `samples/` is a fixture a test reads, and must read as its name says, never as something else; a frame a Bug Report read wrong becomes one. Before adding, cutting or naming a sample, read `samples/README.md`: each directory's naming, cropping and privacy rules are there.
- Tests run on made-up worlds in OSMS's shape (`crates/minerva-core/tests/common/osms.rs`); the map names fixture (IDs, streets, map names and regions) and the ceilings table (`site/worker/osms-ceilings.json`, each map's solo EXP/h ceiling, the levels of the middle half of its spawns and its Map Mark, and each mark's area and picture, as the core works them out, written by `probe ceilings`) are the only data committed from OSMS. The Worker's tests use made-up ceilings (`site/worker/vitest.config.ts`), and check only the table's shape. `probe osms` must read OSMS's live set and item files in full. A shape OSMS ever serves that reads wrong becomes a made-up fixture with that shape. Item Lookup's tests write their own made-up answers in MeowDB's search shape (`tests/item_lookup.rs`); nothing from MeowDB is committed. The Worker's tests answer its asks of ClassicRank from a made-up ClassicRank in its `names=` shape (`site/worker/test/classicrank.ts`); nothing from ClassicRank is committed. The Item Card's tests use made-up item files in OSMS's shape (`tests/common/items.rs`).
- Bug Reports (ADR 0009): every input `Minerva` takes writes itself down, so a new input needs an `Event` and a line in `bug_report::replay`. A Bug Report is never committed: it becomes the smallest test that reproduces its bug, on a made-up world, and a frame in it read wrong becomes a sample.
- The site (`site/`) is for players, and the README keeps only the developer's side. Its home page shows what Minerva does, a feature at a time in a sentence or two; the FAQ (`site/faq/index.html`) holds every detail a player needs, as the questions they ask, each written once there and shown again on the home page by its id. A change a player notices updates the site in the same PR (the feature's words, or the FAQ's answer), the setup warning's wording included, and lists each screenshot it alters under Retake in `site/screenshots/README.md`.
- UI work (an app window, a site page, `/admin/` included) starts from DESIGN.md's entry for that surface and reuses its patterns. A change to how it looks updates that entry in the same PR, and shows the user renders (`pnpm shot` / `pnpm preview:window`) of each state it touches before the PR. Edit UI files with Edit/Write, so the design hook sees them.
- Player-facing text, anywhere (a window, tooltip, notice, tray item, site page, FAQ answer, alt text, Discord post), is written or edited with the `humanizer` skill before the PR. UI names and Glossary terms (Deal Line, Going rate, Help & Setup) keep their wording and capitals. Say a switch's other state as "Turn this off and…", give a trailing clause its own sentence, and split any sentence that carries a whole paragraph.
- `src-tauri` only adapts window lookup, capture and rendering. It must follow ADR 0001: no `PrintWindow`, no messages, input, hooks or process access to the game.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues on `itechify/minerva`, using the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the five default triage labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human` and `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
