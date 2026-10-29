// `pnpm check`: every check CI runs (.github/workflows/check.yml, then site-check.yml), in
// CI's order, plus a guard on test names. Each check's full output goes to
// _scratch/checks/<name>.log, and it prints one line per check, read from that log: PASS or
// FAIL, the exit code, the counts, the first failure, how long it took and the log. Exits 1
// if any check failed.
//
//   pnpm check                     every check
//   pnpm check --only core,site    only these (a name, or the start of one: `site` is both)
//   pnpm check --from core         this check and every one after it

import { spawn } from "node:child_process";
import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync, createWriteStream } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = fileURLToPath(new URL("..", import.meta.url));
const LOGS = join(REPO, "_scratch", "checks");

// The Worker's tests print "uncaught exception … internal error" while they pass (retro §10).
// Until they call the Worker with an execution context and wait on it, the line is only a
// warning; then set this, as site-check.yml fails on it too.
const UNCAUGHT_FAILS = false;

const CHECKS = [
  { name: "names", run: testNames },
  // First: the app embeds dist/ when it compiles (tauri.conf.json's frontendDist).
  { name: "build", cmd: "pnpm build", read: readGeneric },
  { name: "clippy", cmd: "cargo clippy --workspace --all-targets -- -D warnings", read: readClippy },
  // --no-fail-fast: CI's command, but every failing test binary is listed, not the first.
  { name: "core", cmd: "cargo test --no-fail-fast", read: readCargoTest },
  { name: "app", cmd: "cargo test -p minerva --lib --no-fail-fast", read: readCargoTest },
  { name: "site-build", cmd: "pnpm site:build", read: readGeneric },
  { name: "site-test", cmd: "pnpm site:test", read: readVitest },
];

function parseArgs(argv) {
  let only = null;
  let from = null;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--only") only = (argv[++i] ?? "").split(",").filter(Boolean);
    else if (arg.startsWith("--only=")) only = arg.slice(7).split(",").filter(Boolean);
    else if (arg === "--from") from = argv[++i] ?? "";
    else if (arg.startsWith("--from=")) from = arg.slice(7);
    else usage(`unknown argument ${arg}`);
  }
  const matches = (name, check) => check.name === name || check.name.startsWith(`${name}-`);
  let checks = CHECKS;
  if (from !== null) {
    const start = CHECKS.findIndex((c) => matches(from, c));
    if (start < 0) usage(`no check named ${from}`);
    checks = checks.slice(start);
  }
  if (only !== null) {
    for (const name of only) if (!CHECKS.some((c) => matches(name, c))) usage(`no check named ${name}`);
    checks = checks.filter((c) => only.some((name) => matches(name, c)));
  }
  return checks;
}

function usage(problem) {
  console.error(`pnpm check: ${problem}`);
  console.error(`usage: pnpm check [--only <name>,…] [--from <name>]`);
  console.error(`checks: ${CHECKS.map((c) => c.name).join(", ")}`);
  process.exit(2);
}

/** Runs `cmd` from the repo's root with all its output into `log`; resolves to its exit code. */
function runLogged(cmd, log) {
  return new Promise((resolve) => {
    const out = createWriteStream(log);
    out.write(`$ ${cmd}\n\n`);
    const child = spawn(`${cmd} 2>&1`, {
      cwd: REPO,
      shell: true,
      stdio: ["ignore", "pipe", "inherit"],
      env: { ...process.env, CARGO_TERM_COLOR: "never", NO_COLOR: "1", FORCE_COLOR: "0" },
    });
    child.stdout.pipe(out, { end: false });
    child.on("error", (error) => {
      out.end(`\n${error.message}\n`);
      resolve(127);
    });
    child.on("close", (code) => {
      out.end(`\nexit ${code}\n`, () => resolve(code ?? 1));
    });
  });
}

// --- Reading a log: { counts, first, warn } --------------------------------------------

/** The first line at `i` and, for a compiler error, its `-->` location below it. */
function withLocation(lines, i) {
  const location = lines.slice(i + 1, i + 4).find((l) => /^\s*-->/.test(l));
  return location ? `${lines[i].trim()} ${location.trim()}` : lines[i].trim();
}

function firstCompileError(lines) {
  const i = lines.findIndex((l) => /^error(\[E\d+\])?:/.test(l));
  return i < 0 ? null : withLocation(lines, i);
}

function firstTscError(lines) {
  return lines.find((l) => /error TS\d+:/.test(l))?.trim() ?? null;
}

function readCargoTest(lines) {
  let passed = 0;
  let failed = 0;
  let ignored = 0;
  let suites = 0;
  for (const line of lines) {
    const m = /^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;/.exec(line);
    if (!m) continue;
    suites++;
    passed += Number(m[1]);
    failed += Number(m[2]);
    ignored += Number(m[3]);
  }
  const failing = lines.find((l) => /^test \S+ \.\.\. FAILED$/.test(l));
  const panicked = lines.find((l) => /panicked at /.test(l));
  return {
    counts: `${passed} passed, ${failed} failed${ignored ? `, ${ignored} ignored` : ""}, ${suites} suites`,
    first: failing?.replace(/^test /, "").replace(/ \.\.\. FAILED$/, "") ?? firstCompileError(lines) ?? panicked?.trim() ?? null,
  };
}

function readClippy(lines) {
  const errors = lines.filter((l) => /^error(\[E\d+\])?:/.test(l) && !/^error: could not compile/.test(l)).length;
  return { counts: `${errors} errors`, first: firstCompileError(lines) };
}

function readVitest(lines) {
  let passed = 0;
  let failed = 0;
  let skipped = 0;
  for (const line of lines) {
    const m = /^\s*Tests\s{2,}(.*)$/.exec(line);
    if (!m) continue;
    passed += Number(/(\d+) passed/.exec(m[1])?.[1] ?? 0);
    failed += Number(/(\d+) failed/.exec(m[1])?.[1] ?? 0);
    skipped += Number(/(\d+) skipped/.exec(m[1])?.[1] ?? 0);
  }
  const uncaught = lines.filter((l) => /uncaught exception/i.test(l)).length;
  const failing = lines.find((l) => /^\s*FAIL\s/.test(l)) ?? lines.find((l) => /^\s*[×✗]\s/.test(l));
  return {
    counts: `${passed} passed, ${failed} failed${skipped ? `, ${skipped} skipped` : ""}`,
    first: firstTscError(lines) ?? failing?.trim() ?? null,
    warn: uncaught ? `${uncaught} "uncaught exception" lines (retro §10)` : null,
    failOnWarn: UNCAUGHT_FAILS,
  };
}

function readGeneric(lines) {
  return { counts: "", first: firstTscError(lines) ?? lines.find((l) => /\berror\b/i.test(l))?.trim() ?? null };
}

// --- The guard on test names -----------------------------------------------------------

// `cargo test` prints a passing test as "test <name> ... ok", so a test named `result…` reads
// as cargo's "test result:" summary to anything that greps for it, and passes for a failure.
function testNames(log) {
  const found = [];
  const crates = join(REPO, "crates");
  for (const crate of readdirSync(crates)) {
    const tests = join(crates, crate, "tests");
    if (!statSync(tests, { throwIfNoEntry: false })?.isDirectory()) continue;
    for (const file of rustFiles(tests)) {
      const text = readFileSync(file, "utf8");
      for (const m of text.matchAll(/#\[test\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?fn\s+(result\w*)/g)) {
        const line = text.slice(0, m.index + m[0].length).split("\n").length;
        found.push(`${relative(REPO, file).replace(/\\/g, "/")}:${line} ${m[1]}`);
      }
    }
  }
  writeFileSync(log, found.length ? `Tests named result…, which read as cargo's "test result:":\n${found.join("\n")}\n` : "No test is named result….\n");
  return {
    code: found.length ? 1 : 0,
    counts: `${found.length} tests named result…`,
    first: found[0] ? `${found[0]}: rename it` : null,
  };
}

function* rustFiles(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* rustFiles(path);
    else if (entry.name.endsWith(".rs")) yield path;
  }
}

// --- Main ------------------------------------------------------------------------------

function duration(ms) {
  const s = Math.round(ms / 1000);
  return s < 60 ? `${s}s` : `${Math.floor(s / 60)}m${String(s % 60).padStart(2, "0")}s`;
}

const clip = (text, n) => (text.length > n ? `${text.slice(0, n - 1)}…` : text);

const checks = parseArgs(process.argv.slice(2));
mkdirSync(LOGS, { recursive: true });
const width = Math.max(...checks.map((c) => c.name.length));
const started = Date.now();
let failures = 0;
for (const check of checks) {
  const log = join(LOGS, `${check.name}.log`);
  const shownLog = relative(REPO, log).replace(/\\/g, "/");
  if (process.stdout.isTTY) process.stdout.write(`....  ${check.name.padEnd(width)}  ${check.cmd ?? ""}`);
  const t0 = Date.now();
  let code;
  let read;
  if (check.cmd) {
    code = await runLogged(check.cmd, log);
    read = check.read(readFileSync(log, "utf8").split(/\r?\n/));
  } else {
    read = check.run(log);
    code = read.code;
  }
  const failed = code !== 0 || (read.warn && read.failOnWarn);
  if (failed) failures++;
  const parts = [`exit ${code}`];
  if (read.counts) parts.push(read.counts);
  if (failed && read.first) parts.push(`first: ${clip(read.first, 140)}`);
  parts.push(duration(Date.now() - t0), shownLog);
  if (process.stdout.isTTY) process.stdout.write("\r\x1b[K");
  console.log(`${failed ? "FAIL" : "PASS"}  ${check.name.padEnd(width)}  ${parts.join("   ")}`);
  if (read.warn) console.log(`WARN  ${check.name.padEnd(width)}  ${read.warn}   ${shownLog}`);
}
console.log(`${failures ? "FAIL" : "PASS"}  ${checks.length - failures} of ${checks.length} checks passed in ${duration(Date.now() - started)}`);
process.exit(failures ? 1 : 0);
