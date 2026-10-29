#!/usr/bin/env node
// `pnpm preview:window <window> [state…]`: shoots an app window's page fed a fixture view,
// at the window's real width, into `_scratch/shots/`, and prints the paths. No states: all
// of the window's. The window is its page's name (`item-card` for item-card.html); its
// states are `preview/fixtures/<window>/<state>.ts` (a file starting with `_` holds what its
// states share).
//
// Vite serves the app's pages on a free port (never 1420, which `tauri dev` and other
// worktrees use), with `preview/tauri-mock.ts` put before each page's script to answer
// `page_view` with the fixture. Only this server adds the mock: `vite build` never sees it.
import { readdirSync, readFileSync, existsSync } from "node:fs";
import { createServer as freePort } from "node:net";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { launch, report, shotPath } from "./shot.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const FIXTURES = join(ROOT, "preview", "fixtures");
const TAURI_DEV_PORT = 1420;

/** The window's `WindowSpec` in src-tauri: its width, and its heights (CSS px). */
function windowSpec(window) {
  const dir = join(ROOT, "src-tauri", "src");
  const event = `event: "${window}-state"`;
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".rs"))) {
    const text = readFileSync(join(dir, file), "utf8");
    const at = text.indexOf(event);
    if (at < 0) continue;
    const spec = text.slice(text.lastIndexOf("WindowSpec {", at), text.indexOf("};", at));
    const width = /\bwidth:\s*([\d.]+)/.exec(spec);
    const heights = /Heights\s*\{\s*default:\s*([\d.]+),\s*min:\s*([\d.]+),\s*max:\s*([\d.]+)/.exec(spec);
    if (!width || !heights) throw new Error(`can't read the WindowSpec in src-tauri/src/${file}`);
    const [, def, min, max] = heights.map(Number);
    return { width: Number(width[1]), height: { default: def, min, max } };
  }
  throw new Error(`no WindowSpec in src-tauri/src has ${event}`);
}

function port() {
  return new Promise((resolve, reject) => {
    const server = freePort();
    server.unref();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => (port === TAURI_DEV_PORT ? resolve(port + 1) : resolve(port)));
    });
  });
}

/** Puts the mock on each page before its own script, so the page's first `invoke` meets it. */
const tauriMock = {
  name: "minerva-preview-tauri-mock",
  transformIndexHtml: () => [{ tag: "script", attrs: { type: "module", src: "/preview/tauri-mock.ts" }, injectTo: "head-prepend" }],
};

const clamp = (n, min, max) => Math.min(Math.max(n, min), max);

/** Loads the state, sizes the viewport as the shell would size the window, and shoots it. */
async function shootState(page, base, window, state, spec) {
  let width = spec.width;
  let height = spec.height.default || 400;
  await page.viewport(width, height);
  await page.goto(`${base}/${window}.html?fixture=${encodeURIComponent(state)}`);
  const until = Date.now() + 10_000;
  while (!(await page.evaluate("window.__preview?.ready === true"))) {
    if (Date.now() > until) throw new Error(`${window}/${state}: the page never asked for its view`);
    await new Promise((r) => setTimeout(r, 50));
  }
  // The page reports its size once drawn, and again as each new size changes it, as the
  // shell resizes the window until the page agrees.
  for (let i = 0; ; i++) {
    if (i === 40) throw new Error(`${window}/${state}: the page's size never settled (last ${width}×${height})`);
    await page.settle();
    const reported = await page.evaluate("window.__preview");
    const w = reported.width ? Math.min(reported.width, spec.width) : spec.width;
    const h =
      reported.fixtureHeight ??
      (spec.height.max > 0 ? clamp(reported.height ?? spec.height.default, spec.height.min, spec.height.max) : spec.height.default);
    if (!(h > 0)) throw new Error(`${window}/${state}: the window is placed, not sized to its page: give the fixture a height`);
    if (w === width && h === height) break;
    [width, height] = [w, h];
    await page.viewport(width, height);
  }
  const file = await page.screenshot(shotPath(`${window}-${state}`, width));
  return { file, errors: [...page.errors] };
}

async function main() {
  const [window, ...asked] = process.argv.slice(2);
  if (!window) {
    const windows = existsSync(FIXTURES) ? readdirSync(FIXTURES).join(", ") : "none";
    throw new Error(`usage: pnpm preview:window <window> [state…]  (windows: ${windows})`);
  }
  const dir = join(FIXTURES, window);
  if (!existsSync(dir)) throw new Error(`no fixtures for ${window}: preview/fixtures/${window}/ doesn't exist`);
  const states = readdirSync(dir).filter((f) => f.endsWith(".ts") && !f.startsWith("_")).map((f) => f.slice(0, -3)).sort();
  const unknown = asked.filter((s) => !states.includes(s));
  if (unknown.length) throw new Error(`${window} has no state ${unknown.join(", ")} (it has ${states.join(", ")})`);
  const spec = windowSpec(window);

  const server = await createServer({
    configFile: join(ROOT, "vite.config.ts"),
    root: ROOT,
    logLevel: "warn",
    clearScreen: false,
    server: { port: await port(), strictPort: true, host: "127.0.0.1", hmr: false },
    // Served as they are: a dependency found mid-run would reload the page under the shot.
    optimizeDeps: { noDiscovery: true, include: [] },
    plugins: [tauriMock],
  });
  let browser;
  try {
    await server.listen();
    const base = server.resolvedUrls.local[0].replace(/\/$/, "");
    browser = await launch();
    const page = await browser.page();
    // The windows are transparent: what they don't draw shows the game, here a dark backdrop.
    await page.backdrop("#2b2f36");
    const shots = [];
    for (const state of asked.length ? asked : states) shots.push(await shootState(page, base, window, state, spec));
    report(shots);
  } finally {
    await browser?.close();
    await server.close();
  }
}

try {
  await main();
} catch (e) {
  console.error(e.message);
  process.exit(1);
}
