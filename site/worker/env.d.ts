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
     *  (docs/map-leaderboards-setup.md, ADR 0017). Without it, nothing is posted and
     *  uploads are unchanged. */
    DISCORD_WEBHOOK_URL?: string;
    /** The tests' made-up solo ceilings, in place of osms-ceilings.json. */
    TEST_CEILINGS?: Record<string, number>;
  }
  interface GlobalProps {
    mainModule: typeof import("./index");
  }
}

type Env = Cloudflare.Env;
