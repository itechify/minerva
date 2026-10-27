// The Worker's bindings (site/wrangler.jsonc).
declare namespace Cloudflare {
  interface Env {
    /** The site's build. */
    ASSETS: Fetcher;
    /** The Map Leaderboards (site/migrations). */
    DB: D1Database;
    /** The maintainer's token, a Worker secret (docs/map-leaderboards-setup.md). Without
     *  it, the maintainer's routes refuse everyone. */
    ADMIN_TOKEN?: string;
    /** The Discord webhook a new #1 on a Map Leaderboard is posted to, a Worker secret
     *  (docs/map-leaderboards-setup.md, ADR 0022). Without it, nothing is posted and
     *  uploads are unchanged. */
    DISCORD_WEBHOOK_URL?: string;
    /** The Discord application's client ID, a variable (site/wrangler.jsonc), and its
     *  secret, a Worker secret (docs/map-leaderboards-setup.md, ADR 0023). Without both,
     *  signing in is off. */
    DISCORD_CLIENT_ID?: string;
    DISCORD_CLIENT_SECRET?: string;
    /** The maintainers' Discord user IDs, comma-separated, a variable (site/wrangler.jsonc):
     *  signed in with one, the maintainer's routes and /admin/ are theirs. */
    MAINTAINER_DISCORD_IDS?: string;
    /** ClassicRank's API key, a Worker secret (docs/map-leaderboards-setup.md, ADR 0028).
     *  Without it, the Worker never asks ClassicRank, and its answers carry no portraits. */
    CLASSICRANK_KEY?: string;
    /** The tests' made-up ceilings table, in place of osms-ceilings.json. */
    TEST_CEILINGS?: import("./ceilings").CeilingsTable;
  }
  interface GlobalProps {
    mainModule: typeof import("./index");
  }
}

type Env = Cloudflare.Env;
