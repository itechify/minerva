// The Worker's tests (`pnpm site:test`), run in workerd against a local D1 with the
// migrations applied, a made-up ceilings table, a made-up maintainer token and a made-up Discord application.
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
          bindings: {
            TEST_MIGRATIONS: migrations,
            ADMIN_TOKEN: "made-up-maintainer-token",
            // A made-up Discord webhook. The tests take the posts to it off the global
            // fetch (test/announce.test.ts); nothing leaves the test.
            DISCORD_WEBHOOK_URL: "https://discord.invalid/api/webhooks/made-up/webhook",
            // A made-up Discord application, and a made-up maintainer's Discord ID. The tests
            // take Discord's answers off the global fetch (test/accounts.ts).
            DISCORD_CLIENT_ID: "made-up-client",
            DISCORD_CLIENT_SECRET: "made-up-client-secret",
            MAINTAINER_DISCORD_IDS: "900000000000000001",
            // A made-up ceilings table, in place of osms-ceilings.json: test/api.ts's maps, and
            // test/where-to-train.test.ts's, with a made-up Map Mark.
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
    test: { root: import.meta.dirname, setupFiles: [path.join(import.meta.dirname, "test/apply-migrations.ts")] },
  };
});
