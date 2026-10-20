// The minerva-site Worker: the site's build, and the Map Leaderboards API under /api/
// (ADR 0015, where the routes and their answers are written down).
import { type Answer, badRequest, json } from "./http";
import { isMaintainer } from "./keys";
import { optOut, readBand, readMap, readMaps, upload } from "./leaderboards";
import { deleteEntry, readReports, reassign, report } from "./moderation";

export default {
  async fetch(request, env, ctx): Promise<Response> {
    const url = new URL(request.url);
    if (!url.pathname.startsWith("/api/")) return env.ASSETS.fetch(request);
    const answer = await route(request, url, env, ctx);
    // Public reads may be cached a minute; nothing else, the maintainer's reads included.
    const isPublicRead = request.method === "GET" && !url.pathname.startsWith("/api/admin/");
    return json(answer, isPublicRead ? "public, max-age=60" : "no-store");
  },
} satisfies ExportedHandler<Env>;

async function route(request: Request, url: URL, env: Env, ctx: ExecutionContext): Promise<Answer> {
  let path: string[];
  try {
    path = url.pathname.split("/").slice(2).map(decodeURIComponent);
  } catch {
    return badRequest("the path isn't URL-encoded right");
  }
  const method = request.method;
  if (path[0] === "admin") {
    if (!(await isMaintainer(request, env))) return { status: 401, body: { error: "not-maintainer" } };
    return routeAdmin(path.slice(1), method, env);
  }
  switch (true) {
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
    case path.length === 1 && path[0] === "reports" && method === "POST":
      return report(env, request);
    default:
      return { status: 404, body: { error: "not-found" } };
  }
}

/** The maintainer's routes, under /api/admin/. */
function routeAdmin(path: string[], method: string, env: Env): Promise<Answer> | Answer {
  switch (true) {
    case path.length === 1 && path[0] === "reports" && method === "GET":
      return readReports(env);
    case path.length === 4 && path[0] === "entries" && method === "DELETE":
      return deleteEntry(env, path[1], path[2], path[3]);
    case path.length === 3 && path[0] === "characters" && path[2] === "reassign" && method === "POST":
      return reassign(env, path[1]);
    default:
      return { status: 404, body: { error: "not-found" } };
  }
}
