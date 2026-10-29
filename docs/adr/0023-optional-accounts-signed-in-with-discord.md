---
status: accepted, amended by ADR-0024, ADR-0027, ADR-0031
---

# Optional Accounts, signed in with Discord

Optional Accounts link a player's Leaderboard Keys so a linked PC can write for their names after a reinstall or move. Discord supplies sign-in; the Account belongs to Minerva. A key alone still uploads (ADR 0015).

## The decision

- **Always optional.** Uploading keeps working with a Leaderboard Key alone, as before, and nothing a player does in Minerva needs an Account. No page shows "Sign in" to everyone. A player reaches sign-in from Minerva's Link Discord, the maintainer from `/admin/`. Once signed in, the window's head shows "Signed in as *X* · Sign out".
- **An Account is ours.** `accounts` has our own ID, and `logins (provider, provider_user_id)` says how one signs in. Discord is the only provider, asked for the `identify` scope alone. Another provider can be added later without moving every foreign key. Of Discord's answer we keep the user ID and the username, refreshed at each sign-in, for the maintainer to match a player by. Neither is shown on the site.
- **Sessions** are a cookie (`__Host-session`, HttpOnly, Secure, `SameSite=Lax`) holding a secret that D1 keeps only as its hash. A player's lasts 30 days, the maintainer's 1 day, since a stolen maintainer's cookie can delete entries. A marker cookie beside it (`signed_in`, readable by the pages) says one is there, so a page asks the API who's signed in only then, and a page view by anyone else never reaches D1.
- **Linking starts in the app.** The app asks for a one-time **link token** with its key (`POST /api/link`), good for 5 minutes, and opens `/account/link/?t=<token>`. The page signs the player in if they aren't, then asks: "Link this Minerva to your Discord account *X*? It holds Egbert and Bob. Only do this if you just clicked Link Discord in Minerva." Only the confirm (a `POST` from that page) links. Someone could mint a token with their own key and send the address to a player who is signed in; with a link made silently, their key would join the player's Account and write for the player's names. A key linked again moves to the new Account, taking the names it claimed with it.
- **Claims gain an Account.** `characters.key_hash` stays the key that claimed the name, and `characters.account_id` is added. A write for a name goes through with the claiming key **or any key linked to the name's Account**. Every other key is still refused as `claimed`, and kept as the one the maintainer would reassign the name to. Linking a key gives the names it claimed its Account, and a name first claimed by a linked key gets the Account at once. Each write by a linked key records when it was last used. A player with no Account sees no change at all.
- **The account page** (`/account/`) lists the linked PCs (when linked, when last used, the Characters each claimed). **Unlinking a PC** takes away what its key had through the Account: a name it claimed moves to the Account's most recently used other key, or, with none, stays with it and leaves the Account, so a sold PC can't keep writing. **Deleting the Account** removes its logins, links and sessions, and each name stays with its claiming key. Entries stay on the site either way. There is no opt-out button: the app uploads a Character's whole set as each Visit ends (ADR 0015), so an opt-out made on the site would be undone at the next Visit. Opting out stays in the app.
- **The maintainer** is whoever signs in with a Discord user ID that the Worker's `MAINTAINER_DISCORD_IDS` variable lists. It's a variable, not a column, so no bug in the Worker's SQL can grant it. `ADMIN_TOKEN` stays beside it, for `curl` and for the day Discord's sign-in is down. `/admin/` includes Reports and Characters; ADR 0031 adds Overview, Accounts and Log. **Reports** lists each report with Open, **Dismiss** (the report goes, the entry stays) and **Take entry down** (ADR 0015's deletion, asking first by the Character's name), and under it every entry **Taken down**, each with **Restore**. **Characters** looks a name up (when it was claimed, when another key was last refused for it, its Account's username) and offers **Reassign to refused key**, or **Reassign to an Account** found by its Discord username, which moves the name to that Account's most recently used key. A name reassigned to a refused key takes that key's Account (none, if it isn't linked), so the old claimant's other keys can't write for it any more.
- **Safety.** Only public GETs under `/api/maps`, `/api/bands`, `/api/latest`, `/api/pedestals` and `/api/characters` may be cached (`PUBLIC_READS` in `site/worker/index.ts`); everything else reads a key or a cookie and is `no-store`. A write signed in by cookie must come from the site's own origin (`Origin`), beside `SameSite=Lax`. OAuth's `state` lives in a short-lived cookie, and the page a sign-in returns to must be a path on the site.

## The API

All answers are JSON, as in ADR 0015, and every refusal is `{ "error": "<code>" }`. Signed-in routes answer `401 signed-out` without a session, and `403 wrong-origin` for a write from another site.

**Signing in:**

- `GET /api/auth/discord?return=<path>`: to Discord, then back to `<path>` (`/account/` if it isn't a path on the site). `503 sign-in-off` without `DISCORD_CLIENT_ID` and `DISCORD_CLIENT_SECRET`.
- `GET /api/auth/discord/callback`: Discord's return. Signs in and goes back to the page asked, or to it with `?signin=failed`.
- `POST /api/auth/sign-out`: `204`, the cookies cleared.

**The app**, with its key as `Authorization: Bearer <key>` (`401 no-key` without):

- `POST /api/link`: `201 { "token": "...", "url": "https://owlofminerva.app/account/link/?t=...", "expiresAt": 1760000300000 }`. Counted as a write against the key's rate limit.
- `GET /api/link`: `{ "linked": false }`, or `{ "linked": true, "username": "egberticus" }`.

**The pages**, signed in:

- `GET /api/account`: `{ "username": "egberticus", "maintainer": false, "pcs": [{ "id": 3, "linkedAt": ..., "lastUsedAt": ..., "characters": ["Egbert"] }] }`, the most recently used PC first.
- `DELETE /api/account/pcs/<id>`: unlinks the PC. `204`, or `404 no-pc`.
- `DELETE /api/account`: deletes the Account. `204`, the cookies cleared.
- `GET /api/link/tokens/<token>`: `{ "characters": ["Egbert", "Bob"], "linked": null }`, `linked` being `"this"` or `"other"` for a key linked already. `404 no-token` for one used or past its 5 minutes.
- `POST /api/link/tokens/<token>`: links it. `200 { "characters": [...] }`, or `404 no-token`.

**The maintainer**, with `ADMIN_TOKEN` or signed in as a maintainer (`401 not-maintainer` otherwise), beside ADR 0015's routes:

- `DELETE /api/admin/reports/<id>`: dismisses the report. `204`, or `404 no-report`.
- `GET /api/admin/removed`: `{ "removed": [{ "name": "Egbert", "map": "010004042", "band": 30, "expPerHour": 61536 }] }`, every entry taken down (ADR 0015's `DELETE /api/admin/entries/...`), whose Visit stays off until restored.
- `DELETE /api/admin/removed/<name>/<map>/<band>/<expPerHour>`: restores it, so the next upload of that Visit goes up again. `204`, or `404 no-removed`.
- `GET /api/admin/characters/<name>`: the claim and refusal dates, Account, entries, Block and administrative log; ADR 0031 records the expanded answer. `404 no-character` for an unknown name.
- `GET /api/admin/accounts?username=<part>`: `{ "accounts": [{ "id": 1, "username": "egberticus", "pcs": 2, "lastUsedAt": ... }] }`, at most 20, whose username holds `<part>` whatever its case.
- `POST /api/admin/characters/<name>/reassign` with `{ "account": 1 }`: the name moves to that Account's most recently used key. `200 { "name": "Egbert" }`, or `409 no-linked-key`. Without a body it moves to the last key refused for it, as before.

## Considered options

- **A recovery code instead of an Account.** Minerva shows the key as a code to copy, and takes one on a new PC. It would need no server change at all. But it shows the player a secret the app has always kept from them (the Leaderboard Key, GLOSSARY), and it fails exactly the player who never wrote it down. It also gives the maintainer no sign-in on the site.
- **Discord's user ID as the Account.** One table fewer, but every foreign key would be Discord's, and a player who asked to sign in some other way would mean moving them all.
- **Email, with a magic link.** Not tied to Discord, but it means sending email, a provider and its secret, and deliverability. The players are on Minerva's Discord already.
- **Linking started on the site, with a code typed into the app.** It needs a box in the app to type into, and a code to type. A token the app makes and opens is one click.
- **Required Accounts to appear on the Map Leaderboards.** Moderation would be simpler with every name behind an identity, but the FAQ tells players they never need an account, and it would turn away everyone who uploads today.
- **A maintainer role column on `accounts`.** One place for roles, but a role in the database can be granted by a bug in the Worker's SQL. A variable in the Worker's config can't.

## Consequences

- An Account helps only once a key is linked before it's lost. A player whose old key was never linked still asks on Discord, and the maintainer reassigns the name from `/admin/` instead of `curl`. The app offers Link Discord to players on the Map Leaderboards, and where linking would have saved them.
- The site keeps Discord user IDs and usernames, never shown and removed with the Account. The FAQ describes what is kept.
- Setting up the Discord application (its client ID in `site/wrangler.jsonc`, its secret as `DISCORD_CLIENT_SECRET`, its redirect URLs) and the maintainers' Discord IDs are steps by hand, once: `docs/map-leaderboards-setup.md`. Without them, signing in is off and everything else works as before.
- Two PCs uploading the same Character still replace each other's set: linking lets both write, it doesn't merge them.
