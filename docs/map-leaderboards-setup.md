# Setting up the Map Leaderboards API

The Map Leaderboards API, and the Accounts beside it, run on the site's Worker with a D1 database (ADR 0015, ADR 0023). The Site workflow (`.github/workflows/site.yml`) applies the database's migrations and deploys the Worker when a release is published, or when it's run by hand; merging to `main` deploys nothing. The database and Discord IDs are already in `site/wrangler.jsonc`. For a replacement deployment, provision its resources in this order and set its secrets before deploying. Wrangler is a dev dependency, so after `pnpm install` each command runs as `pnpm exec wrangler ...`, from the repo root, signed in to the account in `site/wrangler.jsonc` (`pnpm exec wrangler login`).

## 1. Create the database, and commit its ID

```bash
pnpm exec wrangler d1 create minerva-leaderboards
```

It prints the new database's `database_id`. For a replacement, update `d1_databases` in `site/wrangler.jsonc` with that ID and set `account_id` to its account, then merge it into `main`. These IDs aren't secrets. Local runs and tests use local databases.

## 2. Let the API token edit D1

The `site` environment's `CLOUDFLARE_API_TOKEN` (GitHub: Settings → Environments → site) needs Worker deploy and **D1 → Edit** permissions on the replacement account. Check them under Cloudflare's My Profile → API Tokens.

## 3. Set the maintainer's secret

The maintainer's routes (`/api/admin/...`: reading reports, deleting an entry, reassigning a name) need a token of their own, kept as a Worker secret:

```bash
pnpm exec wrangler secret put ADMIN_TOKEN --config site/wrangler.jsonc
```

Paste a long random string (say `openssl rand -hex 32`) and keep it in a password manager. It stays across deploys. Without it, the maintainer's routes refuse everyone. For a local run (`pnpm site:local`, which runs `wrangler dev`), put `ADMIN_TOKEN=...` in `site/.dev.vars`, which is gitignored.

## 4. Set the Discord webhook (optional)

A new #1 on a Map Leaderboard is posted to a Discord webhook (ADR 0022). It is optional: with no secret set, nothing is posted and uploads are unchanged.

In Discord, on the Minerva server: **#map-leaderboards → Edit Channel → Integrations → Webhooks → New Webhook**, name it (say "Map Leaderboards"), then **Copy Webhook URL**. The URL is a secret — anyone holding it can post to that channel — so it is never committed; keep it in a password manager beside `ADMIN_TOKEN`.

```bash
pnpm exec wrangler secret put DISCORD_WEBHOOK_URL --config site/wrangler.jsonc
```

Paste the URL. For a local run, put `DISCORD_WEBHOOK_URL=...` in `site/.dev.vars`, which is gitignored; leave it out and nothing is posted. While it's there, `pnpm site:local --seed` refuses to run, so its made-up Characters never reach that channel. To stop the posts, delete the webhook in Discord or `pnpm exec wrangler secret delete DISCORD_WEBHOOK_URL --config site/wrangler.jsonc`.

## 5. Set up signing in with Discord

Accounts (ADR 0023) sign in with Discord: players to link their PCs, the maintainer to moderate at `/admin/`. With either of the client ID and secret unset, signing in is off and everything else works as before.

1. For a replacement Discord application, choose **New Application** in the [Discord Developer Portal](https://discord.com/developers/applications) and name it "Minerva". Under **OAuth2**, copy the **Client ID**, then **Reset Secret** and copy the **Client Secret** into the password manager beside `ADMIN_TOKEN`.
2. Still under **OAuth2 → Redirects**, add the three addresses Discord may send a player back to:
   - `https://owlofminerva.app/api/auth/discord/callback`
   - `https://www.owlofminerva.app/api/auth/discord/callback`
   - `http://localhost:5173/api/auth/discord/callback`, for `pnpm site:dev`
3. The current IDs are committed in `site/wrangler.jsonc`. For a replacement, set `vars.DISCORD_CLIENT_ID` to the client ID and `vars.MAINTAINER_DISCORD_IDS` to your Discord user ID (Discord: **Settings → Advanced → Developer Mode**, then right-click your name → **Copy User ID**; more than one, comma-separated). Neither is secret, so commit them.
4. Set the secret:

   ```bash
   pnpm exec wrangler secret put DISCORD_CLIENT_SECRET --config site/wrangler.jsonc
   ```

For a local run, put `DISCORD_CLIENT_SECRET=...` in `site/.dev.vars`. Sign in at `http://localhost:5173/admin/` with `pnpm site:local` running and `pnpm site:dev` started with the `MINERVA_WORKER_PORT` it prints.

## 6. Set the ClassicRank key (optional)

Characters' portraits come from ClassicRank's API (ADR 0029), which the Worker asks with a key: a Partner read key, applied for at [classicrank.com/api-access](https://classicrank.com/api-access) (sign in with Discord) and shown once on [classicrank.com/me](https://classicrank.com/me). It is optional: with no secret set, the Worker never asks ClassicRank, and the site shows no portraits. The key is a secret, so it is never committed; keep it in the password manager beside `ADMIN_TOKEN`.

```bash
pnpm exec wrangler secret put CLASSICRANK_KEY --config site/wrangler.jsonc
```

Paste the key. Check it with `curl -i -H "Authorization: Bearer <key>" https://classicrank.com/api/v1`: the `X-Read-Key` header says `accepted`, and the answer's `your_key` gives its tier. Portraits show from each Character's next upload, and for everyone at the next daily run (the cron trigger in `site/wrangler.jsonc`, 05:17 UTC). Until then the boards show the question-mark figure, so once the deploy with the key is live, run the refresh at once: `curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" https://owlofminerva.app/api/admin/portraits`, which answers 204 when it's done. For a local run, put `CLASSICRANK_KEY=...` in `site/.dev.vars`, or start `pnpm site:local --key`, which gives it a made-up one; an unknown key is served as ClassicRank's keyless tier, which is enough to try it. To stop the portraits, `pnpm exec wrangler secret delete CLASSICRANK_KEY --config site/wrangler.jsonc`: from then on the answers carry none, and the pages show none.

## 7. Set the maplestory.market key (optional)

The Item Card's and the Deal Finder's prices come from maplestory.market's API (ADR 0030), which the Worker asks with a key: asked for on the Account page at [maplestory.market](https://maplestory.market) and approved by its admins, under its [API terms](https://maplestory.market/docs/api/terms). It is optional: with no secret set, the Worker never asks maplestory.market, and the app shows its prices as not available. The key is a secret, so it is never committed; keep it in the password manager beside `ADMIN_TOKEN`. The key's link code (`?ref=`) isn't secret: the app adds it to its links to maplestory.market.

```bash
pnpm exec wrangler secret put MM_API_KEY --config site/wrangler.jsonc
```

Paste the key. Check it with `curl -s "https://owlofminerva.app/api/market?ids=4010000"` once the deploy is live: the first read pulls both tables, and `prices` and `listed` come back with their `pulledAt`. The key's limit (6 requests a minute) is far above what the Worker asks: each table at most once an hour. To stop the prices, `pnpm exec wrangler secret delete MM_API_KEY --config site/wrangler.jsonc`: from then on reads give none, and the app says they're not available.

## 8. Deploy the API before any app that uses it

Deploy the API and migrations a release depends on before releasing the app. Once they are on `main`, publish from `main`:

```bash
gh workflow run site.yml -f ref=main
```

It applies the migrations to the remote database, then deploys. Check it with `curl https://owlofminerva.app/api/maps`, which must answer with JSON.

Never publish from a ref missing API behavior the released app needs: the workflow deploys that ref's Worker too.

## Afterwards

- **A new migration** goes in `site/migrations` as the next `NNNN_<what>.sql` (`pnpm exec wrangler d1 migrations create minerva-leaderboards <what> --config site/wrangler.jsonc`). The Site workflow applies it before the deploy that needs it. Try it locally first: `pnpm site:local` applies it to its local D1 when it starts (after `pnpm site:local stop`, if one is running).
- **The ceilings table** (`site/worker/osms-ceilings.json`) is written again when OSMS's set changes, as `probe osms` is run: `cargo run -p minerva --example probe ceilings site/worker/osms-ceilings.json`.
- **A lost Leaderboard Key.** The player says on Discord that their Character's name is "claimed". If they have an Account, ask for its Discord username and that they link their new PC to it; at `/admin/`, under **Characters**, look the name up and **Reassign** it to their Account. Otherwise, ask them to try again from Minerva (toggling the upload off and on), then **Reassign to refused key** there, or with the maintainer's token:

  ```bash
  curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" https://owlofminerva.app/api/admin/characters/<name>/reassign
  ```

  It moves the name to the last key refused for it and says when that was. Check the time with the player before telling them it's done.
- **Reports**: at `/admin/`, under **Reports**, each with **Dismiss** (the report goes) and **Take entry down** (the entry goes too, and stays off). An entry taken down by mistake comes back with **Restore** under **Taken down**. An entry nobody reported comes down the same way from its Character under **Characters**, where **Take all down** takes every entry of theirs. Without Discord, `curl -H "Authorization: Bearer $ADMIN_TOKEN" https://owlofminerva.app/api/admin/reports`, and to delete an entry, `curl -X DELETE -H "Authorization: Bearer $ADMIN_TOKEN" "https://owlofminerva.app/api/admin/entries/<name>/<map>/<band>?kind=party"` (`kind` is `solo` or `party`; without it, both of a band's entries go).
- **A player who cheats** (ADR 0031): at `/admin/`, under **Characters**, look the name up and **Block** it, with a note to say why, and **Block the name too** to keep the name off whatever key uploads it. It refuses uploads from their Account, every PC linked to it and the key that holds the name, and hides every Character they hold. Their entries stay, so **Unblock** puts them back: on the Character, on its Account under **Accounts**, or in the **Blocked** list on **Overview**, where every Block is. The player's Personal Bests say their uploads are blocked and to ask on Discord, as the FAQ does. Every change made at `/admin/`, by whom and when, is under **Log**.
- **A player who asks for their Account to go**: at `/admin/`, under **Accounts**, find it by Discord username, then **Delete Account**, or **Unlink** a single PC, as they could themselves on their account page.
- **Portraits and market prices**: **Overview** says when each was last had, with **Refresh portraits** and **Pull market prices** to have them now (`curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" https://owlofminerva.app/api/admin/market` without Discord).
