// The minerva-site Worker: the site's build, and the API under /api/: the Map Leaderboards
// (ADR 0015, where its routes and their answers are written down), a Character's page
// (ADR 0027) and Accounts (ADR 0023); and once a day, Characters' portraits from ClassicRank
// (ADR 0028).
import { readCharacterPage } from "./character-page";
import { confirmLinkToken, deleteAccount, linkState, mintLinkToken, readAccount, readLinkToken, unlinkPc } from "./accounts";
import { callback, start } from "./discord";
import { type Answer, badRequest, json } from "./http";
import { isMaintainerToken } from "./keys";
import { readLatest } from "./latest";
import { optOut, readBand, readMap, readMaps, upload } from "./leaderboards";
import { deleteEntry, dismissReport, findAccounts, readCharacter, readRemoved, readReports, reassign, report, restoreEntry } from "./moderation";
import { refreshPortraits } from "./portraits";
import { endSession, isMaintainerAccount, sameOrigin, sessionAccount } from "./sessions";
import { readBands, readTraining } from "./where-to-train";

/** The Map Leaderboards' reads, which read no key or cookie, a Character's page's among them
 *  (the app's writes under /api/characters aren't GETs). */
const PUBLIC_READS = ["/api/maps", "/api/bands", "/api/latest", "/api/characters"];

export default {
  async fetch(request, env, ctx): Promise<Response> {
    const url = new URL(request.url);
    if (!url.pathname.startsWith("/api/")) return env.ASSETS.fetch(request);
    const answer = await route(request, url, env, ctx);
    // The Map Leaderboards' reads may be cached a minute; nothing else, since everything else
    // reads a key or a cookie.
    const isPublicRead = request.method === "GET" && PUBLIC_READS.some((path) => url.pathname.startsWith(path));
    return json(answer, isPublicRead ? "public, max-age=60" : "no-store");
  },
  // The daily cron trigger (site/wrangler.jsonc): every Character's portrait, asked for again
  // (ADR 0028).
  async scheduled(controller, env, ctx): Promise<void> {
    ctx.waitUntil(refreshPortraits(env, controller.scheduledTime));
  },
} satisfies ExportedHandler<Env>;

const NOT_FOUND: Answer = { status: 404, body: { error: "not-found" } };
const SIGNED_OUT: Answer = { status: 401, body: { error: "signed-out" } };
const WRONG_ORIGIN: Answer = { status: 403, body: { error: "wrong-origin" } };

async function route(request: Request, url: URL, env: Env, ctx: ExecutionContext): Promise<Answer> {
  let path: string[];
  try {
    path = url.pathname.split("/").slice(2).map(decodeURIComponent);
  } catch {
    return badRequest("the path isn't URL-encoded right");
  }
  const method = request.method;
  switch (path[0]) {
    case "admin":
      return routeAdmin(request, url, path.slice(1), env);
    case "auth":
      return routeAuth(request, url, path.slice(1), env);
    case "account":
      return routeAccount(request, path.slice(1), env);
    case "link":
      return routeLink(request, url, path.slice(1), env);
  }
  switch (true) {
    case path.length === 2 && path[0] === "characters" && method === "GET":
      return readCharacterPage(env, path[1]);
    case path.length === 3 && path[0] === "characters" && path[2] === "entries" && method === "PUT":
      return upload(env, request, path[1], ctx);
    case path.length === 3 && path[0] === "characters" && path[2] === "entries" && method === "DELETE":
      return optOut(env, request, path[1]);
    case path.length === 1 && path[0] === "maps" && method === "GET":
      return readMaps(env);
    case path.length === 2 && path[0] === "maps" && method === "GET":
      return readMap(env, path[1]);
    case path.length === 4 && path[0] === "maps" && path[2] === "bands" && method === "GET":
      return readBand(env, path[1], path[3], url.searchParams.get("name"));
    case path.length === 1 && path[0] === "bands" && method === "GET":
      return readBands(env);
    case path.length === 2 && path[0] === "bands" && method === "GET":
      return readTraining(env, path[1]);
    case path.length === 1 && path[0] === "latest" && method === "GET":
      return readLatest(env);
    case path.length === 1 && path[0] === "reports" && method === "POST":
      return report(env, request);
    default:
      return NOT_FOUND;
  }
}

/** The Account a request is signed in to, so long as a write comes from the site's own
 *  pages; otherwise the answer refusing it. */
async function signedIn(request: Request, env: Env): Promise<number | Answer> {
  const account = await sessionAccount(env, request, Date.now());
  if (account === null) return SIGNED_OUT;
  if (request.method !== "GET" && !sameOrigin(request)) return WRONG_ORIGIN;
  return account;
}

/** Signing in with Discord, and out, under /api/auth/. */
async function routeAuth(request: Request, url: URL, path: string[], env: Env): Promise<Answer> {
  const method = request.method;
  switch (true) {
    case path.length === 1 && path[0] === "discord" && method === "GET":
      return start(env, url);
    case path.length === 2 && path[0] === "discord" && path[1] === "callback" && method === "GET":
      return callback(env, request, url);
    case path.length === 1 && path[0] === "sign-out" && method === "POST":
      if (!sameOrigin(request)) return WRONG_ORIGIN;
      return { status: 204, cookies: await endSession(env, request) };
    default:
      return NOT_FOUND;
  }
}

/** The account page's routes, under /api/account, signed in. */
async function routeAccount(request: Request, path: string[], env: Env): Promise<Answer> {
  const account = await signedIn(request, env);
  if (typeof account !== "number") return account;
  const method = request.method;
  switch (true) {
    case path.length === 0 && method === "GET":
      return readAccount(env, account);
    case path.length === 0 && method === "DELETE":
      return deleteAccount(env, account);
    case path.length === 2 && path[0] === "pcs" && method === "DELETE":
      return unlinkPc(env, account, path[1]);
    default:
      return NOT_FOUND;
  }
}

/** Linking a Leaderboard Key, under /api/link: the app's routes, with its key, and the
 *  confirmation page's, signed in. */
async function routeLink(request: Request, url: URL, path: string[], env: Env): Promise<Answer> {
  const method = request.method;
  if (path.length === 0 && method === "POST") return mintLinkToken(env, request, url);
  if (path.length === 0 && method === "GET") return linkState(env, request);
  if (path.length !== 2 || path[0] !== "tokens" || (method !== "GET" && method !== "POST")) return NOT_FOUND;
  const account = await signedIn(request, env);
  if (typeof account !== "number") return account;
  return method === "GET" ? readLinkToken(env, account, path[1]) : confirmLinkToken(env, account, path[1]);
}

/** The maintainer's routes, under /api/admin/: with the ADMIN_TOKEN secret, or signed in
 *  with a maintainer's Discord account. */
async function routeAdmin(request: Request, url: URL, path: string[], env: Env): Promise<Answer> {
  if (!(await isMaintainerToken(request, env))) {
    const account = await signedIn(request, env);
    if (account === WRONG_ORIGIN) return WRONG_ORIGIN;
    if (typeof account !== "number" || !(await isMaintainerAccount(env, account))) {
      return { status: 401, body: { error: "not-maintainer" } };
    }
  }
  const method = request.method;
  switch (true) {
    case path.length === 1 && path[0] === "reports" && method === "GET":
      return readReports(env);
    case path.length === 2 && path[0] === "reports" && method === "DELETE":
      return dismissReport(env, path[1]);
    case path.length === 4 && path[0] === "entries" && method === "DELETE":
      return deleteEntry(env, path[1], path[2], path[3]);
    case path.length === 1 && path[0] === "removed" && method === "GET":
      return readRemoved(env);
    case path.length === 5 && path[0] === "removed" && method === "DELETE":
      return restoreEntry(env, path[1], path[2], path[3], path[4]);
    case path.length === 2 && path[0] === "characters" && method === "GET":
      return readCharacter(env, path[1]);
    case path.length === 3 && path[0] === "characters" && path[2] === "reassign" && method === "POST":
      return reassign(env, request, path[1]);
    case path.length === 1 && path[0] === "accounts" && method === "GET":
      return findAccounts(env, url.searchParams.get("username"));
    default:
      return NOT_FOUND;
  }
}
