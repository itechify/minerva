// `pnpm site:local [--seed] [--key] [--build]`: the built site and its Worker (site/worker)
// under `wrangler dev`, on a free port, with a local D1 under _scratch/wrangler-state that its
// migrations are applied to. Other worktrees run their own at the same time, so it never takes
// a fixed port, and `pnpm site:local stop` stops only the one it started, by the process in
// _scratch/worker.json, never a `workerd` by name.
//
// --seed  uploads a few made-up Characters, as the app would, on maps from the ceilings table.
// --key   gives the Worker a made-up ClassicRank key (served as ClassicRank's keyless tier),
//         so the portraits are asked for without a site/.dev.vars to write.
// --build builds the site again first; without it, only when site/dist has no build.
// (`serve` is its own background half: see `detach`.)
// MINERVA_WORKER_PORT picks the port instead of a free one.
import { spawn, spawnSync } from "node:child_process";
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT = fileURLToPath(import.meta.url);
const ROOT = resolve(dirname(SCRIPT), "..");
const SCRATCH = join(ROOT, "_scratch");
const STATE = join(SCRATCH, "wrangler-state");
const RECORD = join(SCRATCH, "worker.json");
const LOG = join(SCRATCH, "worker.log");
const CONFIG = join(ROOT, "site", "wrangler.jsonc");
const WRANGLER = join(ROOT, "node_modules", "wrangler", "bin", "wrangler.js");
const WINDOWS = process.platform === "win32";

const args = process.argv.slice(2);
const has = (flag) => args.includes(flag);

if (args[0] === "stop") stop();
else if (args[0] === "serve") serve();
else await start();

async function start() {
  mkdirSync(SCRATCH, { recursive: true });
  if (has("--seed") && devVarsSet("DISCORD_WEBHOOK_URL")) {
    fail("site/.dev.vars sets DISCORD_WEBHOOK_URL, so --seed would post its made-up top spots to that Discord. Take it out first.");
  }
  // wrangler serves site/dist as it is on disk, so a new build reaches a running Worker too.
  if (has("--build") || !existsSync(join(ROOT, "site", "dist", "index.html"))) {
    console.log("Building the site (pnpm site:build)...");
    const built = spawnSync("pnpm", ["site:build"], { cwd: ROOT, stdio: "inherit", shell: WINDOWS });
    if (built.status !== 0) fail("pnpm site:build failed.");
  }
  let port = await running();
  if (port !== undefined) {
    console.log(`Already running, from _scratch/worker.json; \`pnpm site:local stop\` stops it.`);
    if (has("--key") || (process.env.MINERVA_WORKER_PORT && Number(process.env.MINERVA_WORKER_PORT) !== port)) {
      console.log("It keeps the port and --key it was started with; stop it first to change them.");
    }
  } else {
    port = await launch();
  }
  if (has("--seed")) await seed(port);
  const cron = /"crons"\s*:\s*\[\s*"([^"]+)"/.exec(readFileSync(CONFIG, "utf8"))?.[1] ?? "* * * * *";
  console.log(`
The site and its Worker: http://localhost:${port}/
The pages from source, with this Worker behind /api/:
  MINERVA_WORKER_PORT=${port} pnpm site:dev${WINDOWS ? `\n  $env:MINERVA_WORKER_PORT=${port}; pnpm site:dev   (PowerShell)` : ""}
The daily run (cron trigger); /__scheduled is the site's 404 page:
  curl "http://localhost:${port}/cdn-cgi/handler/scheduled?cron=${cron.replaceAll(" ", "+")}"
Its log: _scratch/worker.log. Stop it: pnpm site:local stop`);
}

/** The port of the Worker _scratch/worker.json records, if it is still this worktree's and
 *  answers. One of ours that doesn't answer is stopped, so a new one takes its place. */
async function running() {
  const record = readRecord();
  if (record === undefined || !isOurs(record.pid)) return undefined;
  if (await answers(record.port)) return record.port;
  kill(record.pid);
  return undefined;
}

async function launch() {
  console.log("Applying the D1 migrations to _scratch/wrangler-state...");
  const migrated = spawnSync(
    process.execPath,
    [WRANGLER, "d1", "migrations", "apply", "minerva-leaderboards", "--local", "--config", CONFIG, "--persist-to", STATE],
    { cwd: ROOT, stdio: "inherit", env: wranglerEnv() },
  );
  if (migrated.status !== 0) fail("Applying the migrations failed.");

  const port = process.env.MINERVA_WORKER_PORT ? Number(process.env.MINERVA_WORKER_PORT) : await freePort();
  if (!Number.isInteger(port) || port < 1 || port > 65535) fail(`MINERVA_WORKER_PORT isn't a port: ${process.env.MINERVA_WORKER_PORT}`);
  // Taken, wrangler would fail to start, and whatever holds the port (another worktree's
  // Worker) could answer the wait below and take the seed.
  if (!(await isFree(port))) fail(`Port ${port} is taken (another worktree's Worker?). Unset MINERVA_WORKER_PORT, or pick another.`);
  // wrangler's devtools port is 9229 unless given, and another worktree's may hold it.
  const inspector = await freePort();
  const flags = ["dev", "--config", CONFIG, "--port", String(port), "--inspector-port", String(inspector)];
  flags.push("--persist-to", STATE, "--show-interactive-dev-session=false");
  if (has("--key")) flags.push("--var", "CLASSICRANK_KEY:crk_not-a-real-key");
  writeFileSync(LOG, "");
  const pid = await detach(flags);
  writeFileSync(RECORD, `${JSON.stringify({ port, pid }, null, 2)}\n`);
  console.log(`Starting wrangler dev on port ${port} (PID ${pid})...`);

  for (let waited = 0; waited < 90; waited++) {
    if (!alive(pid)) break;
    if (await answers(port)) return port;
    await new Promise((done) => setTimeout(done, 1000));
  }
  const tail = readFileSync(LOG, "utf8").split("\n").slice(-30).join("\n");
  if (alive(pid)) kill(pid);
  rmSync(RECORD, { force: true });
  fail(`The Worker didn't answer /api/maps on port ${port}. The end of _scratch/worker.log:\n${tail}`);
}

/** Starts this script's `serve` in the background, to run wrangler with `flags`, and gives its
 *  PID: the root of the tree `stop` ends (it, wrangler, and the workerd wrangler starts). On
 *  Windows a spawned process inherits every inheritable handle, the caller's stdout included,
 *  and a caller waiting for that to close (a pipe, an agent's shell) would wait for as long as
 *  the Worker runs. So there it starts through Start-Process, which hands down none. */
async function detach(flags) {
  const env = { ...process.env, SITE_LOCAL_FLAGS: JSON.stringify(flags) };
  if (!WINDOWS) {
    const child = spawn(process.execPath, [SCRIPT, "serve"], { cwd: ROOT, detached: true, stdio: "ignore", env });
    child.unref();
    return child.pid;
  }
  const quoted = (text) => `'${text.replaceAll("'", "''")}'`;
  const started = spawnSync(
    "powershell",
    [
      "-NoProfile",
      "-Command",
      // Start-Process joins -ArgumentList with spaces, so a path in it carries its own quotes.
      `(Start-Process -FilePath ${quoted(process.execPath)} -ArgumentList ${quoted(`"${SCRIPT}"`)},'serve' ` +
        `-WorkingDirectory ${quoted(ROOT)} -WindowStyle Hidden -PassThru).Id`,
    ],
    { encoding: "utf8", env },
  );
  const pid = Number(started.stdout.trim());
  if (started.status !== 0 || !Number.isInteger(pid) || pid <= 0) fail(`Start-Process failed: ${started.stderr}`);
  return pid;
}

/** `serve`, in the background: runs wrangler with the flags `detach` passed, its output to
 *  _scratch/worker.log, until wrangler ends. */
function serve() {
  const log = openSync(LOG, "a");
  const wrangler = spawn(process.execPath, [WRANGLER, ...JSON.parse(process.env.SITE_LOCAL_FLAGS)], {
    cwd: ROOT,
    windowsHide: true,
    stdio: ["ignore", log, log],
    env: wranglerEnv(),
  });
  closeSync(log);
  wrangler.on("exit", (code) => process.exit(code ?? 1));
}

function stop() {
  const record = readRecord();
  if (record === undefined) {
    console.log("Nothing to stop: no _scratch/worker.json.");
    return;
  }
  if (isOurs(record.pid)) {
    kill(record.pid);
    console.log(`Stopped the Worker on port ${record.port} (PID ${record.pid} and the processes it started).`);
  } else {
    console.log(`PID ${record.pid} isn't the Worker this script started any more, so nothing was stopped.`);
  }
  rmSync(RECORD, { force: true });
}

/** Made-up Characters, uploaded the way the app uploads (site/worker/test/api.ts), on the first
 *  three maps in the ceilings table with mobs of level 20 or more, each under its map's cap. */
async function seed(port) {
  const table = JSON.parse(readFileSync(join(ROOT, "site", "worker", "osms-ceilings.json"), "utf8"));
  const maps = Object.entries(table.maps)
    .filter(([, row]) => row.ceiling > 0 && row.mobs[0] >= 20)
    .sort(([a], [b]) => a.localeCompare(b))
    .slice(0, 3);
  const now = Date.now();
  const madeUpEntry = (i, [map, row], share, party = false) => ({
    map,
    level: row.mobs[0],
    expPerHour: Math.round(row.ceiling * share),
    setAt: now - i * 3_600_000,
    party,
  });
  const characters = {
    MadeUpMage: { job: "Magician", entries: [madeUpEntry(1, maps[0], 0.6), madeUpEntry(2, maps[1], 0.5)] },
    MadeUpThief: { job: "Assassin", entries: [madeUpEntry(3, maps[0], 0.4), madeUpEntry(4, maps[0], 0.9, true)] },
    MadeUpBowman: { job: "Hunter", entries: [madeUpEntry(5, maps[0], 0.5), madeUpEntry(6, maps[2], 0.7)] },
  };
  console.log("Seeding made-up Characters...");
  for (const [name, { job, entries }] of Object.entries(characters)) {
    const response = await fetch(`http://localhost:${port}/api/characters/${name}/entries`, {
      method: "PUT",
      headers: { authorization: `Bearer ${`local-seed-${name}`.padEnd(40, "x")}`, "content-type": "application/json" },
      body: JSON.stringify({ entries: entries.map((e) => ({ ...e, job })) }),
    });
    const body = await response.text();
    if (response.status !== 200) fail(`Seeding ${name}: ${response.status} ${body}`);
    console.log(`  ${name}: ${body}  (http://localhost:${port}/api/characters/${name})`);
  }
}

function readRecord() {
  if (!existsSync(RECORD)) return undefined;
  try {
    const { port, pid } = JSON.parse(readFileSync(RECORD, "utf8"));
    return Number.isInteger(port) && Number.isInteger(pid) ? { port, pid } : undefined;
  } catch {
    return undefined;
  }
}

/** Whether `pid` is still the `serve` this worktree's script started: after a restart, the
 *  PID may be any other process's. */
function isOurs(pid) {
  if (!alive(pid)) return false;
  const command = WINDOWS
    ? spawnSync(
        "powershell",
        ["-NoProfile", "-Command", `(Get-CimInstance Win32_Process -Filter "ProcessId=${pid}").CommandLine`],
        { encoding: "utf8" },
      ).stdout
    : spawnSync("ps", ["-o", "command=", "-p", String(pid)], { encoding: "utf8" }).stdout;
  return (command ?? "").includes(SCRIPT) && / serve$/.test(command.trim());
}

function alive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error.code === "EPERM";
  }
}

/** Ends `pid` and every process under it. */
function kill(pid) {
  if (WINDOWS) spawnSync("taskkill", ["/T", "/F", "/PID", String(pid)], { stdio: "ignore" });
  else {
    try {
      // Started detached, it leads its own process group.
      process.kill(-pid, "SIGTERM");
    } catch {}
  }
}

async function answers(port) {
  try {
    const response = await fetch(`http://localhost:${port}/api/maps`, { signal: AbortSignal.timeout(2000) });
    return response.status === 200;
  } catch {
    return false;
  }
}

function freePort() {
  return new Promise((done, failed) => {
    const server = createServer();
    server.once("error", failed);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => done(port));
    });
  });
}

function isFree(port) {
  return new Promise((done) => {
    const server = createServer();
    server.once("error", () => done(false));
    server.listen(port, "127.0.0.1", () => server.close(() => done(true)));
  });
}

function devVarsSet(name) {
  const file = join(ROOT, "site", ".dev.vars");
  return existsSync(file) && new RegExp(`^\\s*${name}\\s*=\\s*\\S`, "m").test(readFileSync(file, "utf8"));
}

function wranglerEnv() {
  return { ...process.env, WRANGLER_SEND_METRICS: "false" };
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
