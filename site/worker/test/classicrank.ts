// A made-up ClassicRank (ADR 0028), set up before every test file (vitest.config.ts): the
// Worker's asks of ClassicRank's API are taken off the global fetch and answered in its
// `names=` shape from the Characters a test makes up here. It knows nobody until a test says
// so, so an upload in any other test finds no portrait. Nothing real from ClassicRank is in
// it, and nothing leaves the test.

/** ClassicRank's API, as the Worker asks it. */
const API = "https://classicrank.com/api/v1/characters";

/** A made-up Character on ClassicRank: their name there, and their photo and profile. */
export interface MadeUp {
  name: string;
  photo: string | null;
  profile_url: string;
  name_cut_off?: boolean;
}

/** What the made-up ClassicRank knows and does: the Characters it knows, by name in lower
 *  case; whether it is too busy to answer (a 429); and each ask the Worker made, with the key
 *  it sent and how many names it asked about. */
interface State {
  known: Map<string, MadeUp>;
  busy: boolean;
  asks: { names: string[]; key: string | null; fields: string | null; limit: string | null }[];
}

/** One made-up ClassicRank per isolate, however many times this file is set up. */
const global = globalThis as typeof globalThis & { madeUpClassicRank?: State };

export const classicRank: State = (global.madeUpClassicRank ??= { known: new Map(), busy: false, asks: [] });

/** `name` on the made-up ClassicRank, with a made-up photo, or `photo: null` for none. */
export function knownOnClassicRank(name: string, photo: string | null = photoOf(name), more: Partial<MadeUp> = {}): MadeUp {
  const known = { name, photo, profile_url: profileOf(name), ...more };
  classicRank.known.set(name.toLowerCase(), known);
  return known;
}

/** A made-up photo's address for `name`, as ClassicRank gives one, `?v=` and all. */
export function photoOf(name: string, version = "2026-10-26 07:41:13"): string {
  return `https://classicrank.com/api/avatar/${name}?v=${encodeURIComponent(version)}`;
}

export function profileOf(name: string): string {
  return `https://classicrank.com/user/${name}`;
}

/** The names asked about so far, ask by ask, each in lower case. */
export function asked(): string[][] {
  return classicRank.asks.map((a) => a.names.map((n) => n.toLowerCase()));
}

/** ClassicRank's answer to a `names=` ask: the Characters it knows of them, at most `limit`
 *  (60 unless asked for more), the rest of the names left out. */
function answer(url: URL, headers: Headers): Response {
  const names = (url.searchParams.get("names") ?? "").split(",").filter((n) => n !== "");
  const key = headers.get("authorization")?.replace(/^Bearer /, "") ?? null;
  classicRank.asks.push({ names, key, fields: url.searchParams.get("fields"), limit: url.searchParams.get("limit") });
  if (classicRank.busy) {
    return Response.json({ error: { status: 429, message: "Too many requests" } }, { status: 429, headers: { "retry-after": "30" } });
  }
  const limit = Number(url.searchParams.get("limit") ?? 60);
  const found = names.flatMap((n) => classicRank.known.get(n.toLowerCase()) ?? []);
  const data = found.slice(0, limit);
  return Response.json({
    data,
    meta: { total: found.length, page: 1, pages: Math.ceil(found.length / limit), limit, next: found.length > limit ? `${API}?page=2` : null },
  });
}

const realFetch = globalThis.fetch;
const wrapped = globalThis as typeof globalThis & { madeUpClassicRankFetch?: boolean };
if (!wrapped.madeUpClassicRankFetch) {
  wrapped.madeUpClassicRankFetch = true;
  globalThis.fetch = ((input: RequestInfo | URL, init?: RequestInit) => {
    const url = new URL(input instanceof Request ? input.url : String(input));
    if (`${url.origin}${url.pathname}` !== API) return realFetch(input as RequestInfo, init);
    return Promise.resolve(answer(url, new Headers(init?.headers ?? (input instanceof Request ? input.headers : undefined))));
  }) as typeof globalThis.fetch;
}
