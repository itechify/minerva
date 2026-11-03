// Map Loot (ADR 0033): the mesos and Items Characters shown on the Map Leaderboards picked up on
// each map, as the app sends each Visit, kept as each Character's running sums on the map; and,
// once a day, each map's figures worked out from them, for its page: the middle Character's mesos
// an hour, and each Item's average an hour, of each kind. A figure shows once enough Characters
// gave it Visits, so no one Character's play stands for a map. No name is ever shown.
import { isMap, type LootFigures, type MapLootAnswer } from "../entries";
import { HIDDEN } from "./blocks";
import { type Answer, badRequest, body } from "./http";
import { itemNameOf } from "./items";
import { claim } from "./leaderboards";

const HOUR = 3_600_000;
/** A Visit the app sends has at least 10 minutes of active play, and at most a day's. */
const LEAST_ACTIVE_MS = 10 * 60_000;
const MOST_ACTIVE_MS = 24 * HOUR;
/** The most mesos an hour a Visit may have: far above any map's, so only a made-up one is held to it. */
const MOST_MESOS_PER_HOUR = 50_000_000;
/** The most Items one Visit may hold, and the most of one an hour. */
const MOST_ITEMS = 500;
const MOST_OF_AN_ITEM_PER_HOUR = 100_000;
/** A map's figures of a kind show once this many Characters gave it Visits of that kind. */
const LEAST_CHARACTERS = 3;
/** An Item shows once this many of them picked it up there. */
const LEAST_PICKED_UP_BY = 2;

/** A Visit as the app sends it. */
interface Visit {
  map: string;
  party: boolean;
  activeMs: number;
  mesos: number;
  items: { id: number; count: number }[];
}

function isWhole(value: unknown, least: number, most = Number.MAX_SAFE_INTEGER): value is number {
  return Number.isSafeInteger(value) && (value as number) >= least && (value as number) <= most;
}

/** The Visit a body holds, or why it can't be taken. */
function visitOf(given: unknown): Visit | string {
  const { map, party, activeMs, mesos, items } = (given ?? {}) as Record<string, unknown>;
  if (typeof map !== "string" || !isMap(map)) return "map isn't a 9-digit ID";
  if (typeof party !== "boolean") return "party is true or false";
  if (!isWhole(activeMs, LEAST_ACTIVE_MS, MOST_ACTIVE_MS)) return "activeMs isn't 10 minutes to a day, in ms";
  const hours = activeMs / HOUR;
  if (!isWhole(mesos, 1) || mesos / hours > MOST_MESOS_PER_HOUR) return "mesos isn't a positive whole number a Visit could pick up";
  if (!Array.isArray(items) || items.length > MOST_ITEMS) return `items is a list of at most ${MOST_ITEMS}`;
  const taken: Visit["items"] = [];
  for (const item of items) {
    const { id, count } = (item ?? {}) as Record<string, unknown>;
    if (!isWhole(id, 1)) return "an item's id isn't a positive whole number";
    if (!isWhole(count, 1) || count / hours > MOST_OF_AN_ITEM_PER_HOUR) return "an item's count isn't a positive whole number a Visit could pick up";
    taken.push({ id, count });
  }
  return { map, party, activeMs, mesos, items: taken };
}

/** The app sending a Visit's Map Loot for `name`, with the key that holds the name, as an upload
 *  of entries is: it claims a name no key holds, and a Block refuses it. Items the Item names
 *  table doesn't hold are left out, and the rest of the Visit taken. */
export async function sendMapLoot(env: Env, request: Request, name: string): Promise<Answer> {
  const visit = visitOf(await body(request));
  if (typeof visit === "string") return badRequest(visit);
  // Never null: a write claims a name no key holds.
  const claimed = (await claim(env, request, name, Date.now()))!;
  if (!("nameKey" in claimed)) return claimed;
  const rejectedItems: number[] = [];
  const items: Record<string, number> = {};
  for (const { id, count } of visit.items) {
    if (itemNameOf(env, id) === undefined) rejectedItems.push(id);
    else items[id] = (items[id] ?? 0) + count;
  }
  await env.DB.batch([
    env.DB.prepare(
      `INSERT INTO map_loot (name_key, map_id, party, active_ms, mesos, items) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
       ON CONFLICT (name_key, map_id, party) DO UPDATE SET
         active_ms = map_loot.active_ms + excluded.active_ms,
         mesos = map_loot.mesos + excluded.mesos,
         items = (SELECT json_group_object(key, total) FROM (
                    SELECT key, SUM(value) AS total
                    FROM (SELECT key, value FROM json_each(map_loot.items) UNION ALL SELECT key, value FROM json_each(excluded.items))
                    GROUP BY key))`,
    ).bind(claimed.nameKey, visit.map, visit.party ? 1 : 0, visit.activeMs, visit.mesos, JSON.stringify(items)),
    env.DB.prepare("INSERT OR IGNORE INTO map_loot_changed (map_id) VALUES (?)").bind(visit.map),
  ]);
  return { status: 200, body: { rejectedItems } };
}

/** The statements that take the Map Loot of the Characters `which` names (SQL after `name_key IN`,
 *  bound to `binds`) off the site: their maps are worked out afresh at the next daily run. */
function forget(env: Env, which: string, binds: unknown[]): D1PreparedStatement[] {
  return [
    env.DB.prepare(`INSERT OR IGNORE INTO map_loot_changed (map_id) SELECT DISTINCT map_id FROM map_loot WHERE name_key IN (${which})`).bind(
      ...binds,
    ),
    env.DB.prepare(`DELETE FROM map_loot WHERE name_key IN (${which})`).bind(...binds),
  ];
}

/** Take `nameKey`'s Map Loot off the site: they opted out, or were taken down. */
export function forgetMapLoot(env: Env, nameKey: string): D1PreparedStatement[] {
  return forget(env, "?", [nameKey]);
}

/** Take the Map Loot of every Character a Block reaches off the site. Lifting the Block brings
 *  none of it back. */
export function forgetBlockedMapLoot(env: Env): D1PreparedStatement[] {
  return forget(env, HIDDEN, []);
}

/** One Character's Visits on a map, added up. */
interface Sums {
  activeMs: number;
  mesos: number;
  items: Map<string, number>;
}

interface Row {
  name_key: string;
  party: number;
  active_ms: number;
  mesos: number;
  items: string;
}

/** Each Character's sums of the `rows` `keep` keeps, their solo and party rows together. */
function sumsOf(rows: Row[], keep: (row: Row) => boolean): Sums[] {
  const byName = new Map<string, Sums>();
  for (const row of rows.filter(keep)) {
    const sums = byName.get(row.name_key) ?? { activeMs: 0, mesos: 0, items: new Map() };
    sums.activeMs += row.active_ms;
    sums.mesos += row.mesos;
    for (const [id, count] of Object.entries(JSON.parse(row.items) as Record<string, number>)) {
      sums.items.set(id, (sums.items.get(id) ?? 0) + count);
    }
    byName.set(row.name_key, sums);
  }
  return [...byName.values()];
}

function median(sorted: number[]): number {
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

/** One kind's figures from its Characters' sums. */
function figuresOf(env: Env, characters: Sums[]): LootFigures {
  if (characters.length < LEAST_CHARACTERS) return { characters: characters.length };
  const hours = (c: Sums) => c.activeMs / HOUR;
  const mesosPerHour = Math.round(median(characters.map((c) => c.mesos / hours(c)).sort((a, b) => a - b)));
  const ids = new Set(characters.flatMap((c) => [...c.items.keys()]));
  const items: NonNullable<LootFigures["items"]> = [];
  for (const id of ids) {
    const name = itemNameOf(env, Number(id));
    const pickedUpBy = characters.filter((c) => (c.items.get(id) ?? 0) > 0).length;
    if (name === undefined || pickedUpBy < LEAST_PICKED_UP_BY) continue;
    // Each Character's own rate, averaged over them all: one who picked none up adds a 0.
    const total = characters.reduce((sum, c) => sum + (c.items.get(id) ?? 0) / hours(c), 0);
    items.push({ id: Number(id), name, perHour: Math.round((total / characters.length) * 10_000) / 10_000 });
  }
  items.sort((a, b) => b.perHour - a.perHour || a.name.localeCompare(b.name));
  return { characters: characters.length, mesosPerHour, items };
}

/** The daily run: work out afresh the figures of each map whose sums changed since the last. */
export async function workOutMapLoot(env: Env, now: number): Promise<void> {
  const changed = await env.DB.prepare("SELECT map_id FROM map_loot_changed").all<{ map_id: string }>();
  for (const { map_id: map } of changed.results) {
    // Marked done before it's read, so a Visit sent meanwhile marks it again for the next run.
    await env.DB.prepare("DELETE FROM map_loot_changed WHERE map_id = ?").bind(map).run();
    const rows = (await env.DB.prepare("SELECT name_key, party, active_ms, mesos, items FROM map_loot WHERE map_id = ?").bind(map).all<Row>())
      .results;
    const figures = {
      solo: figuresOf(env, sumsOf(rows, (row) => row.party === 0)),
      party: figuresOf(env, sumsOf(rows, (row) => row.party === 1)),
      both: figuresOf(env, sumsOf(rows, () => true)),
    };
    await env.DB.prepare("INSERT OR REPLACE INTO map_loot_figures (map_id, figures, built_at) VALUES (?, ?, ?)")
      .bind(map, JSON.stringify(figures), now)
      .run();
  }
}

/** Map `map`'s Map Loot, as the daily run last worked it out. */
export async function readMapLoot(env: Env, map: string): Promise<Answer> {
  if (!isMap(map)) return badRequest("a map is a 9-digit ID");
  const row = await env.DB.prepare("SELECT figures, built_at FROM map_loot_figures WHERE map_id = ?")
    .bind(map)
    .first<{ figures: string; built_at: number }>();
  const none: LootFigures = { characters: 0 };
  const answer: MapLootAnswer =
    row === null
      ? { map, builtAt: null, solo: none, party: none, both: none }
      : { map, builtAt: row.built_at, ...(JSON.parse(row.figures) as Pick<MapLootAnswer, "solo" | "party" | "both">) };
  return { status: 200, body: answer };
}
