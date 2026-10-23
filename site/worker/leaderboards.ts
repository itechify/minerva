// The Map Leaderboards: each Character's entries, mirrored whole from the app, and each
// map's Level Bands ranked from them.
import { isMap, isName } from "../entries";
import { bandOf, levelsOf } from "../level-bands";
import { type Top, takenTops, tell } from "./announce";
import { type Answer, badRequest, body } from "./http";
import { accountOfKey, touchKey } from "./accounts";
import { bearer, hash } from "./keys";
import { WRITES, limit } from "./rate-limits";
import ceilings from "./osms-ceilings.json";

/** How many entries a band's read lists. */
const TOP = 50;

/** An entry may have up to this many times its map's solo EXP/h ceiling. */
const CEILINGS_OVER = 5;

/** The cap on a map with no solo ceiling (no mobs, or a map OSMS's set didn't have when the
 *  table was written). Above 5× the highest ceiling in the table (4,393,656 EXP/h in
 *  October 2026), so no map's entries are held to less than their own cap would allow. */
const ABSOLUTE_CAP = 25_000_000;

/** The most EXP/h an entry on `map` may have. */
function capOn(env: Env, map: string): number {
  const ceiling = (env.TEST_CEILINGS ?? (ceilings as Record<string, number>))[map];
  return ceiling === undefined ? ABSOLUTE_CAP : ceiling * CEILINGS_OVER;
}

interface Upload {
  map: string;
  band: number;
  level: number;
  job: string;
  expPerHour: number;
  setAt: number;
}

/** The most entries one upload may hold: far more than a Character has (a map's Visit
 *  History keeps one per Level Band at most, on some 200 maps with mobs). */
const MOST_ENTRIES = 2_000;

/** How far ahead of the Worker's clock an entry's date may be, for a PC's clock that's a
 *  little ahead. The date breaks ties, so it can't be set far ahead to win them. */
const CLOCK_SLACK_MS = 24 * 3_600_000;

function isWhole(value: unknown, least: number, most = Number.MAX_SAFE_INTEGER): value is number {
  return Number.isSafeInteger(value) && (value as number) >= least && (value as number) <= most;
}

/** The entries of an upload's body, or why they can't be taken. */
function entriesOf(body: unknown, now: number): Upload[] | string {
  const given = (body as { entries?: unknown } | null)?.entries;
  if (!Array.isArray(given)) return "the body is { entries: [...] }";
  if (given.length > MOST_ENTRIES) return `an upload holds at most ${MOST_ENTRIES} entries`;
  const entries: Upload[] = [];
  const places = new Set<string>();
  for (const e of given) {
    const { map, level, job, expPerHour, setAt } = (e ?? {}) as Record<string, unknown>;
    if (typeof map !== "string" || !isMap(map)) return "an entry's map isn't a 9-digit ID";
    if (!isWhole(level, 1, 200)) return "an entry's level isn't 1 to 200";
    if (typeof job !== "string" || job.length < 1 || job.length > 32) return "an entry's job isn't 1 to 32 characters";
    if (!isWhole(expPerHour, 1)) return "an entry's expPerHour isn't a positive whole number";
    if (!isWhole(setAt, 0, now + CLOCK_SLACK_MS)) return "an entry's setAt isn't ms since the Unix epoch, up to a day from now";
    const band = bandOf(level);
    const place = `${map} ${band}`;
    if (places.has(place)) return "two entries are for one map and Level Band";
    places.add(place);
    entries.push({ map, band, level, job, expPerHour, setAt });
  }
  return entries;
}

/** An entry an upload held that isn't on the Map Leaderboards, and why. */
interface Rejected {
  map: string;
  band: number;
  reason: "over-cap" | "removed";
  /** For "over-cap": the most EXP/h an entry on the map may have. */
  cap?: number;
}

/** A write for `name` that may go ahead: the name in lower case, and the hash of the key
 *  that holds it. */
interface Claim {
  nameKey: string;
  keyHash: string;
}

/** Claim `name` for the request's Leaderboard Key, if no key holds it yet, and say whether
 *  the write may go ahead: only with the key that holds the name, or a key linked to the
 *  name's Account (ADR 0023). Another key is refused as "claimed", and kept as the one the
 *  maintainer would reassign the name to. A name first claimed by a linked key gets its
 *  Account. With `unclaimed: "leave"`, a name no key holds is left so, and the answer is
 *  `null`. */
async function claim(
  env: Env,
  request: Request,
  name: string,
  now: number,
  unclaimed: "claim" | "leave" = "claim",
): Promise<Claim | Answer | null> {
  if (!isName(name)) return badRequest("a Character's name is 1 to 16 letters and digits");
  const key = bearer(request);
  if (key === undefined) return { status: 401, body: { error: "no-key" } };
  const nameKey = name.toLowerCase();
  const keyHash = await hash(key);
  const limited = await limit(env, `key:${keyHash}`, WRITES);
  if (limited !== undefined) return limited;
  const account = await accountOfKey(env, keyHash);
  type Holder = { key_hash: string; account_id: number | null };
  const holder =
    unclaimed === "claim"
      ? await env.DB.prepare(
          `INSERT INTO characters (name_key, name, key_hash, claimed_at, account_id) VALUES (?, ?, ?, ?, ?)
           ON CONFLICT (name_key) DO UPDATE SET name_key = name_key RETURNING key_hash, account_id`,
        )
          .bind(nameKey, name, keyHash, now, account)
          .first<Holder>()
      : await env.DB.prepare("SELECT key_hash, account_id FROM characters WHERE name_key = ?").bind(nameKey).first<Holder>();
  if (holder === null) return null;
  const throughAccount = account !== null && holder.account_id === account;
  if (holder.key_hash !== keyHash && !throughAccount) {
    await env.DB.prepare("UPDATE characters SET refused_key_hash = ?, refused_at = ? WHERE name_key = ?")
      .bind(keyHash, now, nameKey)
      .run();
    return { status: 409, body: { error: "claimed" } };
  }
  if (account !== null) await touchKey(env, keyHash, now);
  return { nameKey, keyHash };
}

/** A name's write goes through with the key that claimed it (`?2`, its hash), or any key
 *  linked to its Account. */
const MAY_WRITE = "(key_hash = ?2 OR account_id IN (SELECT account_id FROM account_keys WHERE key_hash = ?2))";

/** Each statement of a write runs only while the key may still write for the name. */
const HELD = `EXISTS (SELECT 1 FROM characters WHERE name_key = ?1 AND ${MAY_WRITE})`;

/** A place on the Map Leaderboards: one map's Level Band. */
interface Place {
  map: string;
  band: number;
}

/** How many places one top-spots read asks about at a time. Each takes two bound
 *  parameters, and a D1 statement takes at most 100. */
const PLACES_PER_READ = 40;

/** The top entry of each of `places` that has one. Places with no entry are left out. */
async function topsAt(env: Env, places: Place[]): Promise<Top[]> {
  const tops: Top[] = [];
  for (let from = 0; from < places.length; from += PLACES_PER_READ) {
    const asked = places.slice(from, from + PLACES_PER_READ);
    const rows = await env.DB.prepare(
      `WITH places (map_id, band) AS (VALUES ${asked.map(() => "(?, ?)").join(", ")}),
         ranked AS (
           SELECT e.map_id, e.band, e.name_key, c.name, e.exp_per_hour,
                  ROW_NUMBER() OVER (PARTITION BY e.map_id, e.band
                                     ORDER BY e.exp_per_hour DESC, e.set_at, e.name_key) AS rank
           FROM entries e JOIN characters c USING (name_key)
           JOIN places p ON p.map_id = e.map_id AND p.band = e.band)
       SELECT map_id, band, name_key, name, exp_per_hour FROM ranked WHERE rank = 1`,
    )
      .bind(...asked.flatMap((p) => [p.map, p.band]))
      .all<{ map_id: string; band: number; name_key: string; name: string; exp_per_hour: number }>();
    for (const r of rows.results) {
      tops.push({ map: r.map_id, band: r.band, nameKey: r.name_key, name: r.name, expPerHour: r.exp_per_hour });
    }
  }
  return tops;
}

export async function upload(env: Env, request: Request, name: string, ctx?: ExecutionContext): Promise<Answer> {
  const now = Date.now();
  // Read first, so an upload that can't be taken claims nothing.
  const entries = entriesOf(await body(request), now);
  if (typeof entries === "string") return badRequest(entries);
  // Never null: an upload claims a name no key holds.
  const claimed = (await claim(env, request, name, now))!;
  if (!("nameKey" in claimed)) return claimed;
  const { nameKey, keyHash } = claimed;
  type EntryRow = { map_id: string; band: number; exp_per_hour: number };
  const [removed, had] = await env.DB.batch<EntryRow>([
    env.DB.prepare("SELECT map_id, band, exp_per_hour FROM removed_entries WHERE name_key = ?").bind(nameKey),
    env.DB.prepare("SELECT map_id, band, exp_per_hour FROM entries WHERE name_key = ?").bind(nameKey),
  ]);
  const wasRemoved = (e: Upload) =>
    removed.results.some((r) => r.map_id === e.map && r.band === e.band && r.exp_per_hour === e.expPerHour);
  const rejected: Rejected[] = [];
  const accepted = entries.filter((e) => {
    const cap = capOn(env, e.map);
    if (e.expPerHour > cap) rejected.push({ map: e.map, band: e.band, reason: "over-cap", cap });
    else if (wasRemoved(e)) rejected.push({ map: e.map, band: e.band, reason: "removed" });
    else return true;
    return false;
  });
  // Only an entry that is new, or higher than the one it replaces, can take a top spot, so
  // the top spots are read for those places alone. `takenTops` would turn the rest down
  // anyway; this is what keeps the reads off almost every upload, since the app uploads a
  // Character's whole set as each Visit ends and nearly all of it is unchanged (ADR 0022).
  const held = new Map(had.results.map((r) => [`${r.map_id} ${r.band}`, r.exp_per_hour]));
  const improved: Place[] = accepted.filter((e) => (held.get(`${e.map} ${e.band}`) ?? 0) < e.expPerHour);
  const before = improved.length === 0 ? [] : await topsAt(env, improved);
  await env.DB.batch([
    env.DB.prepare(`UPDATE characters SET name = ?3 WHERE name_key = ?1 AND ${MAY_WRITE}`).bind(nameKey, keyHash, name),
    env.DB.prepare(`DELETE FROM entries WHERE name_key = ?1 AND ${HELD}`).bind(nameKey, keyHash),
    ...accepted.map((e) =>
      env.DB.prepare(
        `INSERT INTO entries (name_key, map_id, band, level, job, exp_per_hour, set_at)
         SELECT ?1, ?3, ?4, ?5, ?6, ?7, ?8 WHERE ${HELD}`,
      ).bind(nameKey, keyHash, e.map, e.band, e.level, e.job, e.expPerHour, e.setAt),
    ),
  ]);
  if (improved.length > 0) {
    const told = tell(env, takenTops(before, await topsAt(env, improved), nameKey));
    // The upload is answered without waiting for Discord; with no context to hand it to
    // (a direct call), it is waited for, so nothing is left running after the answer.
    if (ctx === undefined) await told;
    else ctx.waitUntil(told);
  }
  return { status: 200, body: { accepted: accepted.length, rejected } };
}

export async function optOut(env: Env, request: Request, name: string): Promise<Answer> {
  const claimed = await claim(env, request, name, Date.now(), "leave");
  // A name no key holds has no entries, and stays unclaimed.
  if (claimed === null) return { status: 204 };
  if (!("nameKey" in claimed)) return claimed;
  await env.DB.prepare(`DELETE FROM entries WHERE name_key = ?1 AND ${HELD}`).bind(claimed.nameKey, claimed.keyHash).run();
  return { status: 204 };
}

interface Row {
  name_key: string;
  name: string;
  job: string;
  level: number;
  exp_per_hour: number;
  set_at: number;
  rank: number;
  total: number;
}

/** Map `map`'s Level Band starting at level `first`: its top 50, and `name`'s entry there
 *  with its rank, if given and it has one. */
export async function readBand(env: Env, map: string, first: string, name: string | null): Promise<Answer> {
  const band = Number(first);
  if (!isMap(map)) return badRequest("a map is its 9-digit ID");
  if (!/^\d+$/.test(first) || band < 1 || bandOf(band) !== band) return badRequest("a Level Band is its first level: 1, 10, 20...");
  const nameKey = name?.toLowerCase() ?? null;
  const rows = await env.DB.prepare(
    `WITH ranked AS (
       SELECT e.name_key, c.name, e.job, e.level, e.exp_per_hour, e.set_at,
              ROW_NUMBER() OVER (ORDER BY e.exp_per_hour DESC, e.set_at, e.name_key) AS rank,
              COUNT(*) OVER () AS total
       FROM entries e JOIN characters c USING (name_key)
       WHERE e.map_id = ?1 AND e.band = ?2)
     SELECT * FROM ranked WHERE rank <= ?3 OR name_key = ?4 ORDER BY rank`,
  )
    .bind(map, band, TOP, nameKey)
    .all<Row>();
  const ranked = (r: Row) => ({ rank: r.rank, name: r.name, job: r.job, level: r.level, expPerHour: r.exp_per_hour, setAt: r.set_at });
  const character = rows.results.find((r) => r.name_key === nameKey);
  return {
    status: 200,
    body: {
      map,
      band,
      levels: levelsOf(band),
      total: rows.results[0]?.total ?? 0,
      entries: rows.results.filter((r) => r.rank <= TOP).map(ranked),
      character: character === undefined ? null : ranked(character),
    },
  };
}

/** The most maps the maps read lists: more than the game has, so it lists every real map with
 *  entries (the page's search says how many each has), while one made up by the thousand
 *  can't make the answer endless. */
const MOST_MAPS = 10_000;

/** Every map with entries on its Map Leaderboard, the most entries first. */
export async function readMaps(env: Env): Promise<Answer> {
  const rows = await env.DB.prepare(
    `SELECT map_id AS map, COUNT(DISTINCT name_key) AS characters, COUNT(*) AS entries FROM entries
     GROUP BY map_id ORDER BY entries DESC, characters DESC, map_id LIMIT ?`,
  )
    .bind(MOST_MAPS)
    .all<{ map: string; characters: number; entries: number }>();
  return { status: 200, body: { maps: rows.results } };
}

/** Map `map`'s Level Bands that have entries, lowest first, with how many each has. */
export async function readMap(env: Env, map: string): Promise<Answer> {
  if (!isMap(map)) return badRequest("a map is its 9-digit ID");
  const rows = await env.DB.prepare("SELECT band, COUNT(*) AS entries FROM entries WHERE map_id = ? GROUP BY band ORDER BY band")
    .bind(map)
    .all<{ band: number; entries: number }>();
  const bands = rows.results.map((r) => ({ band: r.band, levels: levelsOf(r.band), entries: r.entries }));
  return { status: 200, body: { map, bands } };
}
