// The account pages' tests (`pnpm site:test`): what the account, link and admin pages say,
// worked out from the API's answers (account.ts), in plain Node.
import { defineConfig } from "vitest/config";

export default defineConfig({
  // Run from the repo root with --config, Vite's root would be the whole repo.
  test: { root: import.meta.dirname, include: ["test/**/*.test.ts"] },
});
