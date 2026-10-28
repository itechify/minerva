// Calls to the Map Leaderboards API, as the app and the site's pages make them, on
// made-up maps: 100000000 (solo ceiling 100,000) and 100000001 (2,000,000), from the test
// config's TEST_CEILINGS, and 100000002, with none.
import { exports } from "cloudflare:workers";
import type { BandAnswer } from "../../entries";

export const FIELD = "100000000";
export const RICH_FIELD = "100000001";
export const NO_CEILING = "100000002";

const ADMIN = "made-up-maintainer-token";

/** A made-up Leaderboard Key, a different one for each `seed`. */
export function key(seed: string): string {
  return `test-key-${seed}`.padEnd(40, "x");
}

export interface Entry {
  map: string;
  level: number;
  job: string;
  expPerHour: number;
  setAt: number;
  /** A Party Best's if true, a Solo Best's if false; left out, a Party Best's: an entry with no
   *  known kind is one. */
  party?: boolean;
}

/** A Solo Best's entry on `map` at `level`, with `expPerHour`, set at `setAt`. */
export function entry(map: string, level: number, expPerHour: number, setAt = 1_760_000_000_000): Entry {
  return { ...unkindedEntry(map, level, expPerHour, setAt), party: false };
}

/** An entry as a Minerva from before party entries uploaded it: no `party` in it. */
export function unkindedEntry(map: string, level: number, expPerHour: number, setAt = 1_760_000_000_000): Entry {
  return { map, level, job: "Magician", expPerHour, setAt };
}

/** A Party Best's entry on `map` at `level`, with `expPerHour`, set at `setAt`. */
export function partyEntry(map: string, level: number, expPerHour: number, setAt = 1_760_000_000_000): Entry {
  return { ...unkindedEntry(map, level, expPerHour, setAt), party: true };
}

export function call(path: string, init: RequestInit = {}): Promise<Response> {
  return exports.default.fetch(new Request(`https://owlofminerva.app${path}`, init));
}

function bearer(token: string): Record<string, string> {
  return { authorization: `Bearer ${token}` };
}

/** The app uploading `name`'s whole set of entries with its key. */
export function upload(name: string, withKey: string, entries: Entry[]): Promise<Response> {
  return call(`/api/characters/${encodeURIComponent(name)}/entries`, {
    method: "PUT",
    headers: { ...bearer(withKey), "content-type": "application/json" },
    body: JSON.stringify({ entries }),
  });
}

/** The app opting `name` out: all its entries go. */
export function optOut(name: string, withKey: string): Promise<Response> {
  return call(`/api/characters/${encodeURIComponent(name)}/entries`, { method: "DELETE", headers: bearer(withKey) });
}

/** A map's Level Band, as the site's page reads it, with `name`'s entry if given, and only
 *  the solo or the party entries if `kind` is. */
export async function band(map: string, first: number, name?: string, kind?: "solo" | "party"): Promise<BandAnswer> {
  const query = new URLSearchParams();
  if (name !== undefined) query.set("name", name);
  if (kind !== undefined) query.set("kind", kind);
  const response = await call(`/api/maps/${map}/bands/${first}${query.size > 0 ? `?${query}` : ""}`);
  if (response.status !== 200) throw new Error(`band read: ${response.status} ${await response.text()}`);
  return response.json();
}

/** The names in a band, best first. */
export async function names(map: string, first: number, kind?: "solo" | "party"): Promise<string[]> {
  return (await band(map, first, undefined, kind)).entries.map((e) => e.name);
}

export function report(body: unknown, from = "203.0.113.7"): Promise<Response> {
  return call("/api/reports", {
    method: "POST",
    headers: { "content-type": "application/json", "cf-connecting-ip": from },
    body: JSON.stringify(body),
  });
}

export function admin(path: string, init: RequestInit = {}, token = ADMIN): Promise<Response> {
  return call(`/api/admin${path}`, { ...init, headers: { ...bearer(token), ...(init.headers ?? {}) } });
}
