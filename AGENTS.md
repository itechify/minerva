# AGENTS.md

## Build and test

- `pnpm check [--only <names>] [--from <name>]`: the build, lint and test checks from `check.yml` and `site-check.yml`, in their order (not the docs wrap or ADR/migration number checks), each logged in full to `_scratch/checks/<name>.log`, with one PASS/FAIL line each (exit code, counts, first failure). Run it once before a PR and quote its summary lines; never pipe cargo's output through `grep` or `head` instead. The full `cargo test` takes ~9 min, so iterate with `cargo test --test <file>`.
- `cargo test`: runs the `minerva-core` tests. The core is the default workspace member, so this never builds Tauri.
- `cargo test -p minerva --lib`: the app's unit tests, as CI and `pnpm check` run them. `cargo test -p minerva` also runs the probe's (`update-manifest`'s) tests; `cargo test` alone runs neither.
- `cargo clippy --workspace --all-targets -- -D warnings`: lint everything.
- `pnpm install` then `pnpm tauri build --debug --no-bundle`: build the app. `pnpm tauri dev` runs it.
- `pnpm site:dev` runs the site (`site/`, the player site at owlofminerva.app); `pnpm site:build` builds it, and fails while a screenshot it shows is missing or the home page names a FAQ question there isn't.
- `pnpm site:test`: type-check the Worker and test it in workerd against a local D1 with migrations applied; also test leaderboards, account and character page logic. For Cloudflare provisioning, see `docs/map-leaderboards-setup.md`.
- `pnpm site:local [--seed]`: the built site and its Worker on a free port, with a local D1 under `_scratch/`; it prints the `MINERVA_WORKER_PORT=<port> pnpm site:dev` and cron-trigger lines for that port, and `--seed` uploads made-up Characters. Other worktrees run their own Workers at the same time: stop yours with `pnpm site:local stop`, which ends only the processes it started.
- `pnpm shot <url> [--width 1280,390] [--full] [--eval '<js>'] [--wait <selector>]` and `pnpm preview:window <window> [state…]`: screenshots of a site page, or of an app window fed the fixture views in `preview/fixtures/<window>/` at its real width, into `_scratch/shots/` under a new name each time.
- Probe syntax and behavior: `src-tauri/examples/probe/main.rs`'s opening comment; common commands are in `README.md`.
  - Default mode smoke-tests capture and recognition without UI; `record` captures timed crops and `item` captures tooltip samples. Read `samples/README.md` before making samples.
  - Start with `probe replay` on any Bug Report; use `--stats` and time windows for counts. Describe the report from its output. Add a missing count to the probe, never a one-off script. Use `probe loot` to inspect Pickup Log Item naming (ADR 0026).
  - Run `probe osms` when OSMS changes and before a release; it must read the live map set and item files in full. Run `probe ceilings` and commit `site/worker/osms-ceilings.json` when OSMS's set changes.
  - Use `probe update-manifest` after a signed build, following `docs/releasing.md`.
- `.github/workflows/check.yml` runs the cargo checks and `pnpm build` on a pull request that touches code, and on `main`; `site-check.yml` the site's; `adr-check.yml` fails a pull request that gives two ADRs, or two D1 migrations, one number, or takes a number another open pull request adds.
- `.github/workflows/site.yml` publishes the site and deploys its Worker, with migrations, only when a GitHub Release is published or when run by hand (`gh workflow run site.yml -f ref=<ref>`). Merging to `main` deploys nothing.

## Releasing

- `docs/releasing.md`: how versions are numbered (the four files that hold the version), and the steps to bump, merge, build, tag and publish a GitHub Release, with the release notes template. Follow it for every release.

## Code rules

- All logic goes in `crates/minerva-core`, which takes plain data in and returns a `PanelState`. It has no OS, UI, network or file dependencies.
- Tests drive the core's public inputs and assert on `PanelState` only, never on internals.
- Every file under `samples/` is a fixture a test reads, and must read as its name says, never as something else; a frame a Bug Report read wrong becomes one. Before adding, cutting or naming a sample, read `samples/README.md`: each directory's naming, cropping and privacy rules are there.
- Committed outside-service fixtures use obviously made-up data in that service's shape, never real figures nudged. OSMS helpers: `crates/minerva-core/tests/common/osms.rs` and `crates/minerva-core/tests/common/items.rs`; MeowDB search: `crates/minerva-core/tests/item_lookup.rs`; ClassicRank: `site/worker/test/classicrank.ts`. Only the map names fixture (`crates/minerva-core/tests/fixtures/osms-map-names.json`) and ceilings table (`site/worker/osms-ceilings.json`, written by `probe ceilings`) are committed from OSMS. Worker tests use made-up ceilings (`site/worker/vitest.config.ts`) and check only the real table's shape. An outside-service shape that reads wrong becomes a made-up fixture with that shape.
- Bug Reports (ADR 0009): every input `Minerva` takes writes itself down, so a new input needs an `Event` and a line in `bug_report::replay`. A Bug Report is never committed: it becomes the smallest test that reproduces its bug, on a made-up world, and a frame in it read wrong becomes a sample.
- The site (`site/`) is for players, and the README keeps only the developer's side. Its home page shows what Minerva does, a feature at a time in a sentence or two; the FAQ (`site/faq/index.html`) holds every detail a player needs, as the questions they ask, each written once there and shown again on the home page by its id. A change a player notices updates the site in the same PR (the feature's words, or the FAQ's answer), the setup warning's wording included, and lists each screenshot it alters under Retake in `site/screenshots/README.md`.
- UI work (an app window, a site page, `/admin/` included) starts from DESIGN.md's entry for that surface and reuses its patterns. A change to how it looks updates that entry in the same PR, and shows the user renders (`pnpm shot` / `pnpm preview:window`) of each state it touches before the PR. Edit UI files with Edit/Write, so the design hook sees them.
- Player-facing text, anywhere (a window, tooltip, notice, tray item, site page, FAQ answer, alt text, Discord post, release notes), is written or edited with the `humanizer` skill before the PR, and again after any rewrite. UI names and Glossary terms (Deal Line, Going rate, Help & Setup) keep their wording and capitals. Say a switch's other state as "Turn this off and…", give a trailing clause its own sentence, and split any sentence that carries a whole paragraph.
- A new network request (anything the app downloads or sends) updates the FAQ's `#data` answer ("What does Minerva download, and what does it send?") in the same PR.
- `src-tauri` only adapts window lookup, capture and rendering. It must follow ADR 0001: no `PrintWindow`, no messages, input, hooks or process access to the game.

## Agent skills

- GitHub Issues on `itechify/minerva`, via `gh`: `docs/agents/issue-tracker.md` for issue and dependency operations.
- Triage uses `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`: meanings in `docs/agents/triage-labels.md`.
- Single-context: one root `GLOSSARY.md` and `docs/adr/`. Before exploring, follow `docs/agents/domain.md`.
