// A Character's page's tests (`pnpm site:test`): what the page shows, worked out from its
// address, the map names fixture's shape and the API's answer (character.ts), in plain Node.
import { defineConfig } from "vitest/config";

export default defineConfig({
  // Run from the repo root with --config, Vite's root would be the whole repo.
  test: { root: import.meta.dirname, include: ["test/**/*.test.ts"] },
});
