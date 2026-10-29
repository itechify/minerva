#!/usr/bin/env node
// Screenshots of a page in headless Chrome, driven over the DevTools Protocol with Node's own
// WebSocket. `pnpm shot <url> [--width 1280,390] [--height 900] [--full] [--eval '<js>']
// [--wait <selector>] [--name <slug>] [--scale <n>]` writes one PNG a width under
// `_scratch/shots/`, a new name every time, and prints each path with the page's console
// errors. `scripts/preview-window.mjs` drives it through `launch()`.
//
// Chrome is found at its usual install path, or at `$CHROME`. Each run starts its own, on a
// port it picks itself, in a fresh profile, and closes only that one.
import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const SHOTS = join(ROOT, "_scratch", "shots");

/** Widths at or under this lay out as a phone's: a mobile viewport, touch, no scrollbars. */
const PHONE_WIDTH = 480;

function chromePath() {
  if (process.env.CHROME) {
    if (!existsSync(process.env.CHROME)) throw new Error(`$CHROME is ${process.env.CHROME}, which doesn't exist`);
    return process.env.CHROME;
  }
  const local = process.env.LOCALAPPDATA ?? "";
  const found = [
    "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
    join(local, "Google/Chrome/Application/chrome.exe"),
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium",
  ].find((p) => existsSync(p));
  if (!found) throw new Error("Chrome isn't at its usual install path: set CHROME to chrome.exe");
  return found;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** One WebSocket to the browser; each page is a session on it (flat mode). */
class Cdp {
  #id = 0;
  #pending = new Map();
  #listeners = new Set();
  constructor(ws) {
    this.ws = ws;
    ws.addEventListener("message", (e) => {
      const msg = JSON.parse(e.data);
      if (msg.id !== undefined) {
        const waiting = this.#pending.get(msg.id);
        this.#pending.delete(msg.id);
        if (msg.error) waiting?.reject(new Error(`${waiting.method}: ${msg.error.message}`));
        else waiting?.resolve(msg.result);
      } else for (const l of this.#listeners) l(msg);
    });
    ws.addEventListener("close", () => {
      for (const w of this.#pending.values()) w.reject(new Error(`${w.method}: Chrome closed the connection`));
      this.#pending.clear();
    });
  }
  send(method, params = {}, sessionId) {
    const id = ++this.#id;
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject, method });
      this.ws.send(JSON.stringify({ id, method, params, sessionId }));
    });
  }
  on(listener) {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }
}

/** A tab: its viewport, what it loads, and the errors it logs while it does. */
class Page {
  /** The console errors, uncaught exceptions and failed loads since the last `goto`. */
  errors = [];
  #inflight = new Set();
  #netAt = Date.now();
  constructor(cdp, sessionId) {
    this.cdp = cdp;
    this.sessionId = sessionId;
    cdp.on((msg) => msg.sessionId === sessionId && this.#hear(msg));
  }
  send(method, params) {
    return this.cdp.send(method, params, this.sessionId);
  }
  #hear({ method, params }) {
    if (method === "Runtime.consoleAPICalled" && params.type === "error") {
      this.errors.push(params.args.map((a) => a.value ?? a.description ?? a.type).join(" "));
    } else if (method === "Runtime.exceptionThrown") {
      const d = params.exceptionDetails;
      this.errors.push(d.exception?.description ?? d.text);
    } else if (method === "Log.entryAdded" && params.entry.level === "error" && !params.entry.url?.endsWith("/favicon.ico")) {
      // (Chrome asks every page for a favicon; a missing one is no error of the page's.)
      this.errors.push(params.entry.url ? `${params.entry.text} ${params.entry.url}` : params.entry.text);
    } else if (method === "Network.requestWillBeSent") {
      this.#inflight.add(params.requestId);
      this.#netAt = Date.now();
    } else if (method === "Network.loadingFinished" || method === "Network.loadingFailed") {
      this.#inflight.delete(params.requestId);
      this.#netAt = Date.now();
    }
  }
  #once(event, timeoutMs) {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        off();
        reject(new Error(`no ${event} in ${timeoutMs / 1000}s`));
      }, timeoutMs);
      const off = this.cdp.on((msg) => {
        if (msg.sessionId === this.sessionId && msg.method === event) {
          clearTimeout(timer);
          off();
          resolve(msg.params);
        }
      });
    });
  }
  async enable() {
    await Promise.all(["Page.enable", "Runtime.enable", "Log.enable", "Network.enable"].map((m) => this.send(m)));
  }
  /** The real device metrics at `width`: media queries, `vw` and the viewport meta all see it. */
  async viewport(width, height, { scale = 1, mobile = width <= PHONE_WIDTH } = {}) {
    this.lastViewport = { scale, mobile };
    await this.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: scale, mobile });
    await this.send("Emulation.setTouchEmulationEnabled", { enabled: mobile });
  }
  /** Draws what is transparent over `color` ("#rrggbb"), or leaves it transparent (null). */
  async backdrop(color) {
    const n = color ? parseInt(color.replace("#", ""), 16) : 0;
    const rgba = color ? { r: n >> 16, g: (n >> 8) & 255, b: n & 255, a: 1 } : { r: 0, g: 0, b: 0, a: 0 };
    await this.send("Emulation.setDefaultBackgroundColorOverride", { color: rgba });
  }
  async goto(url) {
    this.errors = [];
    const loaded = this.#once("Page.loadEventFired", 30_000);
    const { errorText } = await this.send("Page.navigate", { url });
    if (errorText) {
      loaded.catch(() => {});
      throw new Error(`${url}: ${errorText}`);
    }
    await loaded;
    await this.settle();
  }
  /** Until the network has been quiet for a moment (10s at most), the fonts are in and two
   *  frames have been drawn. */
  async settle() {
    const until = Date.now() + 10_000;
    while (Date.now() < until && (this.#inflight.size > 0 || Date.now() - this.#netAt < 300)) await sleep(50);
    await this.evaluate("document.fonts.ready.then(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))))");
  }
  async evaluate(expression) {
    const { result, exceptionDetails } = await this.send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (exceptionDetails) throw new Error(exceptionDetails.exception?.description ?? exceptionDetails.text);
    return result.value;
  }
  async waitFor(selector, timeoutMs = 10_000) {
    const until = Date.now() + timeoutMs;
    while (!(await this.evaluate(`!!document.querySelector(${JSON.stringify(selector)})`))) {
      if (Date.now() > until) throw new Error(`nothing matches ${selector} after ${timeoutMs / 1000}s`);
      await sleep(100);
    }
  }
  /** The viewport as a PNG at `file`; with `full`, the viewport first grown to the page. */
  async screenshot(file, { full = false } = {}) {
    if (full) {
      const metrics = await this.send("Page.getLayoutMetrics");
      const { width } = metrics.cssLayoutViewport;
      const height = Math.ceil(metrics.cssContentSize.height);
      await this.viewport(width, height, this.lastViewport);
      await this.settle();
    }
    const { data } = await this.send("Page.captureScreenshot", { format: "png" });
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, Buffer.from(data, "base64"));
    return file;
  }
}

/** Starts a Chrome of our own; `close()` ends it and only it. */
export async function launch() {
  const profile = mkdtempSync(join(tmpdir(), "minerva-shot-"));
  const chrome = spawn(
    chromePath(),
    [
      "--headless=new",
      "--disable-gpu",
      "--no-first-run",
      "--no-default-browser-check",
      "--disable-extensions",
      "--mute-audio",
      "--remote-debugging-port=0",
      `--user-data-dir=${profile}`,
      "about:blank",
    ],
    { stdio: ["ignore", "ignore", "pipe"] },
  );
  let stderr = "";
  chrome.stderr.on("data", (d) => (stderr += d));
  let exited = false;
  chrome.on("exit", () => (exited = true));

  const kill = () => {
    if (exited) return;
    if (process.platform === "win32") spawnSync("taskkill", ["/PID", String(chrome.pid), "/T", "/F"], { stdio: "ignore" });
    else chrome.kill("SIGKILL");
  };
  const removeProfile = () => rmSync(profile, { recursive: true, force: true, maxRetries: 20, retryDelay: 100 });
  const onSignal = () => {
    kill();
    process.exit(130);
  };
  process.once("SIGINT", onSignal);

  try {
    // Chrome picks the port and writes it, with the browser's path, to DevToolsActivePort.
    const portFile = join(profile, "DevToolsActivePort");
    const until = Date.now() + 20_000;
    while (!existsSync(portFile) || readFileSync(portFile, "utf8").split("\n").length < 2) {
      if (exited) throw new Error(`Chrome exited before it was ready:\n${stderr}`);
      if (Date.now() > until) throw new Error(`Chrome wasn't ready in 20s:\n${stderr}`);
      await sleep(100);
    }
    const [port, path] = readFileSync(portFile, "utf8").split("\n").map((l) => l.trim());
    const ws = new WebSocket(`ws://127.0.0.1:${port}${path}`);
    await new Promise((resolve, reject) => {
      ws.addEventListener("open", resolve, { once: true });
      ws.addEventListener("error", () => reject(new Error(`couldn't reach Chrome on port ${port}`)), { once: true });
    });
    const cdp = new Cdp(ws);
    return {
      async page() {
        const { targetId } = await cdp.send("Target.createTarget", { url: "about:blank" });
        const { sessionId } = await cdp.send("Target.attachToTarget", { targetId, flatten: true });
        const page = new Page(cdp, sessionId);
        await page.enable();
        return page;
      },
      async close() {
        process.off("SIGINT", onSignal);
        await Promise.race([cdp.send("Browser.close").catch(() => {}), sleep(3000)]);
        const until = Date.now() + 5000;
        while (!exited && Date.now() < until) await sleep(100);
        kill();
        ws.close();
        removeProfile();
      },
    };
  } catch (e) {
    process.off("SIGINT", onSignal);
    kill();
    removeProfile();
    throw e;
  }
}

/** `_scratch/shots/<name>-<width>-<HHMMSS>.png`, never one that is there already. */
export function shotPath(name, width) {
  const now = new Date();
  const hhmmss = [now.getHours(), now.getMinutes(), now.getSeconds()].map((n) => String(n).padStart(2, "0")).join("");
  const base = join(SHOTS, `${name}-${width}-${hhmmss}`);
  let file = `${base}.png`;
  for (let n = 2; existsSync(file); n++) file = `${base}-${n}.png`;
  return file;
}

/** A name for a page's shots from its address: its path ("faq", "leaderboards-map"), or
 *  its host for the home page. */
export function slug(url) {
  const u = new URL(url);
  const path = u.pathname.replace(/\.html$/, "").replace(/\/index$/, "");
  const name = u.protocol === "file:" ? path.split("/").pop() : path.split("/").filter(Boolean).join("-") || u.hostname;
  return name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "page";
}

/** Shoots `url` once at each width: loaded afresh, `wait` for, `evalJs` run, then shot. */
export async function shoot(url, { widths = [1280], height = 900, full = false, evalJs, wait, name, scale = 1 } = {}) {
  const browser = await launch();
  const shots = [];
  try {
    const page = await browser.page();
    for (const width of widths) {
      await page.viewport(width, height, { scale });
      await page.goto(url);
      if (wait) await page.waitFor(wait);
      if (evalJs) {
        await page.evaluate(evalJs);
        await page.settle();
      }
      const file = await page.screenshot(shotPath(name ?? slug(url), width), { full });
      shots.push({ file, errors: [...page.errors] });
    }
  } finally {
    await browser.close();
  }
  return shots;
}

/** Each shot's path, then its page's errors under it. */
export function report(shots) {
  for (const { file, errors } of shots) {
    console.log(file);
    for (const e of errors) console.log(`  console error: ${e}`);
  }
}

function parse(argv) {
  const opts = {};
  let url;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const value = () => {
      if (i + 1 >= argv.length) throw new Error(`${arg} needs a value`);
      return argv[++i];
    };
    if (arg === "--width") opts.widths = value().split(",").map(Number);
    else if (arg === "--height") opts.height = Number(value());
    else if (arg === "--full") opts.full = true;
    else if (arg === "--eval") opts.evalJs = value();
    else if (arg === "--wait") opts.wait = value();
    else if (arg === "--name") opts.name = value();
    else if (arg === "--scale") opts.scale = Number(value());
    else if (arg.startsWith("--")) throw new Error(`unknown option ${arg}`);
    else if (url) throw new Error(`one address at a time: ${url}, then ${arg}`);
    else url = arg;
  }
  if (!url) throw new Error("usage: pnpm shot <url> [--width 1280,390] [--height 900] [--full] [--eval '<js>'] [--wait <selector>] [--name <slug>] [--scale <n>]");
  if (opts.widths?.some((w) => !(w > 0)) || (opts.height !== undefined && !(opts.height > 0))) throw new Error("--width and --height take whole px");
  if (!/^[a-z][a-z0-9+.-]*:/i.test(url)) url = existsSync(url) ? pathToFileURL(resolve(url)).href : `http://${url}`;
  return { url, opts };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const { url, opts } = parse(process.argv.slice(2));
    report(await shoot(url, opts));
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
