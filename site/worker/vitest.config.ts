// The Worker's tests (`pnpm site:test`), run in workerd against a local D1 with the
// migrations applied, a made-up ceilings table, a made-up maintainer token, a made-up Discord
// application and webhook, a made-up ClassicRank and a made-up maplestory.market.
import path from "node:path";
import { cloudflareTest, readD1Migrations } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

export default defineConfig(async () => {
  const migrations = await readD1Migrations(path.join(import.meta.dirname, "../migrations"));
  return {
    plugins: [
      cloudflareTest({
        wrangler: { configPath: path.join(import.meta.dirname, "../wrangler.jsonc") },
        miniflare: {
          // A second database, for the test that runs migration 0005 on rows from before it.
          d1Databases: ["OLD_DB"],
          bindings: {
            TEST_MIGRATIONS: migrations,
            ADMIN_TOKEN: "made-up-maintainer-token",
            // A made-up Discord webhook. The tests take the posts to it off the global fetch and
            // answer them from a made-up webhook (test/discord-webhook.ts); nothing leaves the
            // test.
            DISCORD_WEBHOOK_URL: "https://discord.invalid/api/webhooks/made-up/webhook",
            // A made-up Discord application, and a made-up maintainer's Discord ID. The tests
            // take Discord's answers off the global fetch (test/accounts.ts).
            DISCORD_CLIENT_ID: "made-up-client",
            DISCORD_CLIENT_SECRET: "made-up-client-secret",
            MAINTAINER_DISCORD_IDS: "900000000000000001",
            // A made-up ClassicRank key. The tests take the Worker's asks of ClassicRank off the
            // global fetch and answer them from a made-up ClassicRank (test/classicrank.ts).
            CLASSICRANK_KEY: "crk_made-up-key",
            // A made-up maplestory.market key. test/market.test.ts takes the Worker's asks of
            // maplestory.market off the global fetch and answers them with made-up tables.
            MM_API_KEY: "mm_made-up-key",
            // A made-up ceilings table, in place of osms-ceilings.json: test/api.ts's maps, and
            // test/where-to-train.test.ts's, with a made-up Map Mark that test/latest.test.ts and
            // test/announce.test.ts show on 100000060.
            TEST_CEILINGS: {
              maps: {
                "100000000": { ceiling: 100_000, mobs: [70, 75] },
                "100000001": { ceiling: 2_000_000, mobs: [100, 105] },
                "100000060": { ceiling: 300_000, mobs: [22, 27], mark: "MadeUpTown" },
                "100000061": { ceiling: 500_000, mobs: [25, 34] },
                "100000062": { ceiling: 50_000, mobs: [5, 8], mark: "MadeUpTown" },
              },
              marks: { MadeUpTown: { name: "Made-up Town", picture: "https://osms.invalid/images/map_marks/MadeUpTown.png" } },
            },
          },
        },
      }),
    ],
    // Run from the repo root with --config, Vite's root would be the whole repo.
    test: { root: import.meta.dirname, setupFiles: [path.join(import.meta.dirname, "test/apply-migrations.ts"), path.join(import.meta.dirname, "test/classicrank.ts"), path.join(import.meta.dirname, "test/discord-webhook.ts")] },
  };
});
