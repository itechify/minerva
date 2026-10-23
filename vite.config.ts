import { defineConfig } from "vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // The overlay (with mob search), the Grind Tracker's window, the Personal Bests window,
  // the Item Lookup alert, the Item Card, the Deal Badges and the setup window.
  build: {
    rolldownOptions: {
      input: [
        "index.html",
        "grind-tracker.html",
        "personal-bests.html",
        "item-lookup.html",
        "item-card.html",
        "deal-badges.html",
        "setup.html",
      ],
    },
  },
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`, the Cargo workspace's
      //    `target/` (Windows locks .exe/.pdb files there) and agent worktrees
      ignored: ["**/src-tauri/**", "**/target/**", "**/.claude/**", "**/crates/**"],
    },
  },
}));
