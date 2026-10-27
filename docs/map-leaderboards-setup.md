# Setting up the Map Leaderboards API

The Map Leaderboards API, and the Accounts beside it, run on the site's Worker with a D1 database (ADR 0015, ADR 0023). The Site workflow (`.github/workflows/site.yml`) applies the database's migrations and deploys the Worker, but a few steps are done by hand on the Cloudflare account, once, before its first deploy. Do them in this order. Wrangler is a dev dependency, so after `pnpm install` each command runs as `pnpm exec wrangler ...`, from the repo root, signed in to the account in `site/wrangler.jsonc` (`pnpm exec wrangler login`).

## 1. Create the database, and commit its ID

```bash
pnpm exec wrangler d1 create minerva-leaderboards
```

It prints the new database's `database_id`. In `site/wrangler.jsonc`, replace the all-zero placeholder under `d1_databases` with it, and commit that on a branch merged into `main`. The ID isn't a secret (the `account_id` beside it is committed too). Until it's there, the placeholder works for `pnpm site:test` and `wrangler dev`, but the Site workflow fails at its migrations step.

## 2. Let the API token edit D1

The `site` environment's `CLOUDFLARE_API_TOKEN` (GitHub: Settings → Environments → site) can deploy the Worker but not yet touch D1. In the Cloudflare dashboard (My Profile → API Tokens), edit that token and add the account permission **D1 → Edit**. The token's value doesn't change, so the secret stays as it is.

## 3. Set the maintainer's secret

The maintainer's routes (`/api/admin/...`: reading reports, deleting an entry, reassigning a name) need a token of their own, kept as a Worker secret:

```bash
pnpm exec wrangler secret put ADMIN_TOKEN --config site/wrangler.jsonc
```

Paste a long random string (say `openssl rand -hex 32`) and keep it in a password manager. It stays across deploys. Without it, the maintainer's routes refuse everyone. For `wrangler dev`, put `ADMIN_TOKEN=...` in `site/.dev.vars`, which is gitignored.

## 4. Set the Discord webhook (optional)

A new #1 on a Map Leaderboard is posted to a Discord webhook (ADR 0022). It is optional: with no secret set, nothing is posted and uploads are unchanged.

In Discord, on the Minerva server: **#map-leaderboards → Edit Channel → Integrations → Webhooks → New Webhook**, name it (say "Map Leaderboards"), then **Copy Webhook URL**. The URL is a secret — anyone holding it can post to that channel — so it is never committed; keep it in a password manager beside `ADMIN_TOKEN`.

```bash
pnpm exec wrangler secret put DISCORD_WEBHOOK_URL --config site/wrangler.jsonc
```

Paste the URL. For `wrangler dev`, put `DISCORD_WEBHOOK_URL=...` in `site/.dev.vars`, which is gitignored; leave it out and nothing is posted. To stop the posts, delete the webhook in Discord or `pnpm exec wrangler secret delete DISCORD_WEBHOOK_URL --config site/wrangler.jsonc`.

## 5. Set up signing in with Discord

Accounts (ADR 0023) sign in with Discord: players to link their PCs, the maintainer to moderate at `/admin/`. With either of the client ID and secret unset, signing in is off and everything else works as before.

1. In the [Discord Developer Portal](https://discord.com/developers/applications), **New Application**, named "Minerva". Under **OAuth2**, copy the **Client ID**, then **Reset Secret** and copy the **Client Secret** into the password manager beside `ADMIN_TOKEN`.
2. Still under **OAuth2 → Redirects**, add the three addresses Discord may send a player back to:
   - `https://owlofminerva.app/api/auth/discord/callback`
   - `https://www.owlofminerva.app/api/auth/discord/callback`
   - `http://localhost:5173/api/auth/discord/callback`, for `pnpm site:dev`
3. In `site/wrangler.jsonc`, set `vars.DISCORD_CLIENT_ID` to the client ID and `vars.MAINTAINER_DISCORD_IDS` to your Discord user ID (Discord: **Settings → Advanced → Developer Mode**, then right-click your name → **Copy User ID**; more than one, comma-separated). Neither is secret, so commit them.
4. Set the secret:

   ```bash
   pnpm exec wrangler secret put DISCORD_CLIENT_SECRET --config site/wrangler.jsonc
   ```

For `wrangler dev`, put `DISCORD_CLIENT_SECRET=...` in `site/.dev.vars`. Sign in at `http://localhost:5173/admin/` with `pnpm site:dev` and `wrangler dev` both running.

## 6. Set the ClassicRank key (optional)

Characters' portraits come from ClassicRank's API (ADR 0028), which the Worker asks with a key: a Partner read key, applied for at [classicrank.com/api-access](https://classicrank.com/api-access) (sign in with Discord) and shown once on [classicrank.com/me](https://classicrank.com/me). It is optional: with no secret set, the Worker never asks ClassicRank, and the site shows no portraits. The key is a secret, so it is never committed; keep it in the password manager beside `ADMIN_TOKEN`.

```bash
pnpm exec wrangler secret put CLASSICRANK_KEY --config site/wrangler.jsonc
```

Paste the key. Check it with `curl -i -H "Authorization: Bearer <key>" https://classicrank.com/api/v1`: the `X-Read-Key` header says `accepted`, and the answer's `your_key` gives its tier. Portraits show from each Character's next upload, and for everyone at the next daily run (the cron trigger in `site/wrangler.jsonc`, 05:17 UTC). For `wrangler dev`, put `CLASSICRANK_KEY=...` in `site/.dev.vars`; an unknown key there is served as ClassicRank's keyless tier, which is enough to try it. To stop the portraits, `pnpm exec wrangler secret delete CLASSICRANK_KEY --config site/wrangler.jsonc`: from then on the answers carry none, and the pages show none.

## 7. Deploy the API before any app that uses it

The API must be live before any Minerva version that uses it is released: one that uploads (#110), or one that links a PC to an Account (#152). Once this is on `main`, publish the site from `main`:

```bash
gh workflow run site.yml -f ref=main
```

It applies the migrations to the remote database, then deploys. Check it with `curl https://owlofminerva.app/api/maps`, which answers `{"maps":[]}` at first.

From then on, **never publish the site from a ref older than the API**: an older release's tag would deploy a Worker without it, and every upload would fail. A release published after this carries the API, so publishing a release stays safe.

## Afterwards

- **A new migration** goes in `site/migrations` as the next `NNNN_<what>.sql` (`pnpm exec wrangler d1 migrations create minerva-leaderboards <what> --config site/wrangler.jsonc`). The Site workflow applies it before the deploy that needs it. Try it locally first: `pnpm exec wrangler d1 migrations apply minerva-leaderboards --local --config site/wrangler.jsonc`.
- **The ceilings table** (`site/worker/osms-ceilings.json`) is written again when OSMS's set changes, as `probe osms` is run: `cargo run -p minerva --example probe ceilings site/worker/osms-ceilings.json`.
- **A lost Leaderboard Key.** The player says on Discord that their Character's name is "claimed". If they have an Account, ask for its Discord username and that they link their new PC to it; at `/admin/`, under **Characters**, look the name up and **Reassign** it to their Account. Otherwise, ask them to try again from Minerva (toggling the upload off and on), then **Reassign to refused key** there, or with the maintainer's token:

  ```bash
  curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" https://owlofminerva.app/api/admin/characters/<name>/reassign
  ```

  It moves the name to the last key refused for it and says when that was. Check the time with the player before telling them it's done.
- **Reports**: at `/admin/`, under **Reports**, each with **Dismiss** (the report goes) and **Take entry down** (the entry goes too, and stays off). An entry taken down by mistake comes back with **Restore** under **Taken down**. Without Discord, `curl -H "Authorization: Bearer $ADMIN_TOKEN" https://owlofminerva.app/api/admin/reports`, and to delete an entry, `curl -X DELETE -H "Authorization: Bearer $ADMIN_TOKEN" https://owlofminerva.app/api/admin/entries/<name>/<map>/<band>`.
