---
status: accepted, amended by ADR-0023, ADR-0024, ADR-0026, ADR-0027, ADR-0029, ADR-0031
---

# Map Leaderboards on the site's Worker

Players want to see what EXP/h others at their level get on a map, so Minerva puts their Personal Bests side by side on owlofminerva.app: a **Map Leaderboard** per map, each Level Band ranked by EXP/h (#106). The app uploads; the site shows. This records how the site keeps them.

## The decision

- **The `minerva-site` Worker answers an API under `/api/`**, from a **Cloudflare D1** database (`minerva-leaderboards`, its schema in `site/migrations`). The Worker serves `site/dist` as before: it runs first for `/api/*` only (`run_worker_first`), and hands any other path that isn't a file back to the assets, for the plain 404. One deploy, one domain. The code is `site/worker`, and its tests run in workerd against a local D1 (`pnpm site:test`).
- **An entry** is one Character's best Visit of a kind on a map in a Level Band: its name, job, exact level, EXP/h, the date the Visit set it, and its **kind**, `solo` (the Character's Solo Best) or `party` (their Party Best, ADR 0026). A Character has up to two entries in a band, one of each kind. A band ranks its entries by EXP/h, ties going to whoever set it first (then by name, so a rank never moves between reads). With no kind asked for, a band ranks each Character once, by their higher entry (the Solo Best on an exact tie); `?kind=solo` or `?kind=party` ranks only the entries of that kind. An entry of no known kind is a Party Best: every entry from before the kinds were told apart is one (migration `0005_party_entries.sql`), so a player's earlier bests rank as Party Bests until they set a Solo Best.
- **The site mirrors the app.** Each upload is a Character's whole set of entries and replaces the set before it, so a Visit removed, merged or moved in the app goes from the site too. Opting out deletes every entry of the Character.
- **Name claims by Leaderboard Key, with optional Accounts (ADR 0023).** Each install makes a secret Leaderboard Key. A Character's first upload ties its name (whatever its case) to the key it carries; later writes need that key or one linked to the name's Account. Other keys are refused as `claimed`. Only the key's SHA-256 is kept. A player who loses an unlinked key asks on Discord: the Worker keeps the last key it refused for the name, and the maintainer reassigns the name to it. Neither the player nor the maintainer ever sees a key.
- **The cap.** An entry of either kind above **5× its map's solo EXP/h ceiling** is rejected, the rest of its upload taken. The ceilings are a table, `site/worker/osms-ceilings.json`, that `probe ceilings` writes from OSMS's live set with the core's own Map Summary arithmetic (`MapSet::solo_ceilings`), so the site never works it out again and the app's word isn't taken for it. A map with no ceiling (no mobs, or one newer than the table) is capped at **25,000,000 EXP/h**, a literal above 5× the highest ceiling in the table, so regenerating the table doesn't move it. 5× leaves room for parties, EXP events and buffs, while a misread or a made-up figure several times past anything a map gives is kept off.
- **Rate limits**, counted in D1 over the last 10 minutes: 30 writes per Leaderboard Key, whatever names they're for, and 10 reports per reporter (by IP address, hashed). A Character uploads at opting in and then as each Visit ends, nowhere near the limit.
- **Moderation.** Anyone can report an entry. The maintainer, with the Worker's `ADMIN_TOKEN` secret, reads the reports, deletes an entry (the same Visit uploaded again stays off; a better one comes back), and reassigns a name.

## The API

All answers are JSON. Public reads are cached for a minute (`Cache-Control: public, max-age=60`); everything else is `no-store`. A Character's name is 1 to 16 letters and digits, matched whatever its case. A map is its 9-digit ID (`"010001010"`, as the app keys Personal Bests). A **Level Band** is named by its first level: `1` (1–9), `10` (10–19), `20` (20–29) and on. The Worker works an entry's band out from its level; the app never sends one. A Leaderboard Key is 32 to 256 letters, digits, `-` or `_`, sent as `Authorization: Bearer <key>`.

Every refusal is `{ "error": "<code>" }`: `bad-request` (with a `detail`), `no-key` (401), `claimed` (409), `rate-limited` (429, with `Retry-After` in seconds), `not-found` (404).

**The app** (#110):

- `PUT /api/characters/<name>/entries`, with the key. Body: `{ "entries": [{ "map": "010001010", "level": 23, "job": "Magician", "expPerHour": 153216, "setAt": 1760000000000, "party": true }] }`, the Character's whole set, at most one entry per map, band and kind (`party` is `true` for a Party Best's and `false` for a Solo Best's; an entry with no `party`, which is all an app before #173 sends, is a Party Best's), at most 2,000. `setAt` is when the Visit set it, in ms since the Unix epoch, and at most a day ahead of the Worker's clock: the app has no date for a best set before it knew the wall clock, and such a best can't be uploaded. Answer `200 { "accepted": 1, "rejected": [{ "map": "010001010", "band": 30, "party": true, "reason": "over-cap", "cap": 766080 }] }`; `party` is there only for a Party Best's entry; `reason` is `over-cap` (with the map's `cap`) or `removed` (the maintainer deleted that Visit). A malformed body is a 400 and changes nothing, nor claims the name.
- `DELETE /api/characters/<name>/entries`, with the key: opting out. `204`. The name stays claimed by the key; a name no key holds stays unclaimed.
- `GET /api/characters/<name>/entries`, with the key (#206): the set the site holds, every entry as uploaded, `200 { "entries": [...] }`, only to the key that holds the name or one linked to its Account (`409 claimed` otherwise). A name no key holds has none, and stays unclaimed. It reads a key, so it is never cached, though public reads under `/api/characters` are.

**The site's pages** (#111):

- `GET /api/maps`: every map with entries, the most first (then by Characters with entries there); `entries` counts each Character once in a band, by their higher entry. `{ "maps": [{ "map": "010001010", "characters": 12, "entries": 15 }] }`, at most 10,000, more than the game has maps: the page lists the first 50 as the most uploaded, and its search says how many entries each map it finds has, so a player isn't sent to an empty one.
- `GET /api/maps/<map>`: the map's bands with entries. `{ "map": "010001010", "bands": [{ "band": 20, "levels": [20, 29], "entries": 7, "solo": 5, "party": 4 }] }`: `entries` is how many Characters are ranked in the band, `solo` and `party` how many entries of each kind it has.
- `GET /api/maps/<map>/bands/<band>[?name=<name>][&kind=solo|party]`: the band's top 50, and that Character's entry with its rank, even outside them (`null` with none there), of the kind asked for or, with none, each Character's higher entry. Any other `kind` is a 400. `{ "map": "010001010", "band": 20, "levels": [20, 29], "total": 63, "entries": [{ "rank": 1, "name": "Egbert", "job": "Magician", "level": 23, "expPerHour": 153216, "setAt": 1760000000000, "party": false }], "character": { "rank": 57, ... } }`.
- `POST /api/reports`: `{ "name": "Egbert", "map": "010001010", "band": 20, "party": true, "reason": "..." }` (`party` optional, `false` if left out: the report is against the Solo Best's entry; `reason` optional, cut to 500 characters). `201`, or `404 no-entry`.

**The pages** (#111) are static, and read the API from the browser (`site/leaderboards`): `/leaderboards/` finds a map by its name and shows Where to train and Latest (ADR 0024), and a map's page is `/leaderboards/map/?map=<map>`. A map's page, and Where to train and Latest on `/leaderboards/`, offer **Both**, **Solo** and **Party** (`&kind=solo` or `&kind=party`; `GET /api/bands/<band>` and `GET /api/latest` take `?kind=` too), and a gold **Party** mark is on a party entry.

**Deep links**, which the app opens from the Personal Bests window (#110), name the map, the Level Band and the Character:

```
https://owlofminerva.app/leaderboards/map/?map=010001010&band=20&name=Egbert
```

`map` is the 9-digit ID, `band` the Level Band by its first level (as the API names it), and `name` the Character's name, URL-encoded. The page opens at that band, marks the Character's entry and says its rank, even outside the top 50; with no entry there (opted out, the Visit removed, or deleted by the maintainer), it says the entry has gone. The slash before `?` saves a redirect. Minerva sends all three, but the page takes what it can of a link: without `name` it marks nothing, and without `band` it opens at the lowest band with entries. A `band` that isn't a band's first level is read as the band holding that level (`band=23` is 20), a `map` shorter than 9 digits is padded with zeros, and a part that can't be one (a `map` of more than 9 digits, a `name` that isn't 1 to 16 letters and digits) is dropped.

**The maintainer**, with `Authorization: Bearer <ADMIN_TOKEN>` (`401 not-maintainer` otherwise):

- `GET /api/admin/reports`: `{ "reports": [{ "id": 1, "name": "Egbert", "map": "010001010", "band": 20, "expPerHour": 153216, "reason": "...", "reportedAt": 1760000000000 }] }`, newest first.
- `DELETE /api/admin/entries/<name>/<map>/<band>[?kind=solo|party]`: `204`, or `404 no-entry`. The entry of that kind goes (both, with no kind), and its reports with it. A Visit taken down is kept off by its kind too, and `DELETE /api/admin/removed/<name>/<map>/<band>/<expPerHour>[?kind=party]` restores it.
- `POST /api/admin/characters/<name>/reassign`: the name moves to the last key refused for it. `200 { "name": "Egbert", "refusedAt": 1760000000000 }` (check the time with the player), `409 no-refused-key`, or `404 no-character`.

The app calls the API from the shell, not from a web page, so there is no CORS.

## Considered options

- **Uploading a Visit's readings as evidence.** The EXP bar's readings over the Visit would let the site check the EXP/h, or a moderator replay it. But they hold the player's whole run, and the site would have to reimplement or host the core's Session to make sense of them. A reading that is made up is no harder to make than a figure. The cap keeps off what can't be true, and reports catch the rest.
- **Player-set party, event or buff tags.** An honest player would be ranked below one who omitted them. Party play is told automatically by the Pickup Log (ADR 0026), so Solo Bests and Party Bests are entries of their own. EXP events and buffs cannot be told from the screen; raw, capped EXP/h is ranked as it is.
- **A separate service** (its own Worker or a hosted database elsewhere). Isolation from the site's deploys, but another domain, deploy, token and CORS. The site's Worker already serves owlofminerva.app, and D1 is on the same account.

## Consequences

- Deployment applies D1 migrations before the Worker. Setup lives in `docs/map-leaderboards-setup.md`; releasing lives in `docs/releasing.md`.
- The ceilings table is committed beside the map names fixture (ADR 0008); ADR 0024 adds mob levels and Map Marks. It is written again with `probe ceilings` when OSMS's set changes, as `probe osms` is run.
- Names, jobs, levels, EXP/h and dates a player uploads are public. A Character the app meets for the first time with a best that can go up starts shared, its switch on, and one the player turns off stays off; the Characters an install knew before it did this stay off until the player turns them on. The Leaderboard Key, and the reporters' addresses, are kept only as hashes.
- The app (#110) keeps its key in a file of its own beside the settings, `leaderboard-key`, made on the first upload, separate from settings: settings that don't read are set aside, and the key with them would leave every name it holds claimed by a key no install has. The shell keeps it and makes the requests; the core never has it, since everything the core takes in goes into a Bug Report. The core says what each shown Character's set should be, and tries again after a refusal as `claimed` every half hour and at each change, so a key the maintainer may hand the name to has been refused at least once. Since a set replaces the one before, the app pulls a Character's set from the site and merges it into their bests before it uploads anything of theirs, whenever what is on the PC may be far less (#206): a launch whose bests file didn't read holds every shown Character's uploads, then and at every launch after, until their set is pulled; and a Character never uploaded from this install (a new PC, linked through the Account) is pulled first too. The site keeps one entry per map, band and kind, so a Character's other Visits don't come back. While the pull fails, nothing of theirs goes up; turning them off and on sends what is on the PC instead.
