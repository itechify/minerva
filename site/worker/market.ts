// maplestory.market's prices (ADR 0030), for the app's Item Card and Deal Finder. Only the
// Worker asks maplestory.market, with its key (the MM_API_KEY secret): it pulls two of its
// tables whole, its established prices (/api/prices) and what is listed now (/api/items), at
// most once an hour each, as a read finds the copy it has an hour old, and keeps them. The app
// asks for a few items at a time (a card's, or a shop's), never the whole table, as the API's
// terms ask. Without a key it never asks, and every read says it has nothing.
import { type Answer, badRequest } from "./http";
import { hash } from "./keys";
import { MARKET_ITEMS, limit } from "./rate-limits";

/** maplestory.market's API. */
const API = "https://maplestory.market/api";

/** Each table the Worker keeps: the address it pulls it from. */
const SOURCES = { prices: `${API}/prices`, listed: `${API}/items` } as const;
type Source = keyof typeof SOURCES;

/** How old a table may get before a read pulls it again. */
export const PULL_EVERY_MS = 60 * 60_000;

/** How long after asking maplestory.market for a table the Worker waits before asking again,
 *  whether it answered or not. */
export const ASK_AGAIN_MS = 60_000;

/** The most items one read may ask about: a shop's listings. */
export const MOST_IDS = 20;

/** The game's item IDs as numbers, `?ids=4010000,2070000`. */
const IDS = /^\d{1,9}(,\d{1,9}){0,19}$/;

/** What the Worker keeps of a table, given back for the items asked about: when it was pulled
 *  (ms since the Unix epoch), and each item's figures by its ID, an item it doesn't hold left
 *  out. `null` when the table was never pulled. */
interface Table {
  pulledAt: number;
  items: Record<string, unknown>;
}

/** `GET /api/market?ids=…`: maplestory.market's figures for those items, from each table:
 *  `prices`, each item's `reference`, `sold` and `seen` as /api/prices gives them, and
 *  `listed`, each item's `min_unit_price` and `listings` as /api/items gives them. */
export async function readMarket(env: Env, request: Request, url: URL, now = Date.now()): Promise<Answer> {
  const asked = url.searchParams.get("ids") ?? "";
  if (!IDS.test(asked)) return badRequest(`ids: up to ${MOST_IDS} item IDs, comma-separated`);
  const ids = [...new Set(asked.split(",").map(Number))];
  const ip = await hash(request.headers.get("cf-connecting-ip") ?? "unknown");
  const refused = await limit(env, `market:${ip}`, MARKET_ITEMS, ids.length);
  if (refused) return refused;
  const key = env.MM_API_KEY;
  if (!key) return { status: 200, body: { prices: null, listed: null } };
  await Promise.all((Object.keys(SOURCES) as Source[]).map((source) => pullIfDue(env, key, source, now)));
  // Each table is one row (migration 0009): D1 picks the items asked about out of it.
  const kept = await env.DB.prepare(
    `SELECT source, pulled_at,
       (SELECT json_group_object(item.key, json(item.value)) FROM json_each(market_pulls.items) AS item
        WHERE item.key IN (SELECT CAST(value AS TEXT) FROM json_each(?1))) AS items
     FROM market_pulls WHERE items IS NOT NULL`,
  )
    .bind(JSON.stringify(ids))
    .all<{ source: Source; pulled_at: number; items: string }>();
  const table = (source: Source): Table | null => {
    const row = kept.results.find((r) => r.source === source);
    return row === undefined ? null : { pulledAt: row.pulled_at, items: JSON.parse(row.items) };
  };
  return { status: 200, body: { prices: table("prices"), listed: table("listed") } };
}

/** The maintainer's Pull market prices (ADR 0031): both tables pulled at once, whatever their
 *  age. */
export async function pullNow(env: Env, now = Date.now()): Promise<Answer> {
  const key = env.MM_API_KEY;
  if (!key) return { status: 409, body: { error: "market-off" } };
  await Promise.all((Object.keys(SOURCES) as Source[]).map((source) => pullIfDue(env, key, source, now, true)));
  return { status: 204 };
}

/** Pull `source` again if the copy kept is an hour old (or there is none) and nobody asked for
 *  it in the last minute, or at once with `atOnce`. Never throws: a read gives what is kept
 *  whatever comes of it. */
async function pullIfDue(env: Env, key: string, source: Source, now: number, atOnce = false): Promise<void> {
  try {
    // Claim the pull, so reads at the same moment don't all ask. A copy kept a row per item,
    // before migration 0009, is none.
    const claim = await env.DB.prepare(
      `INSERT INTO market_pulls (source, tried_at) VALUES (?1, ?2)
       ON CONFLICT (source) DO UPDATE SET tried_at = ?2
       WHERE (market_pulls.items IS NULL OR market_pulls.pulled_at <= ?2 - ?3) AND market_pulls.tried_at <= ?2 - ?4`,
    )
      .bind(source, now, atOnce ? 0 : PULL_EVERY_MS, atOnce ? 0 : ASK_AGAIN_MS)
      .run();
    if (claim.meta.changes !== 1) return;
    const response = await fetch(SOURCES[source], {
      headers: { authorization: `Bearer ${key}`, accept: "application/json", "user-agent": "Minerva (https://owlofminerva.app)" },
    });
    if (!response.ok) return;
    // The table is read by D1 rather than here: /api/prices is most of a megabyte. What isn't
    // JSON fails the statement, and what isn't a list of rows, or is an empty one, changes
    // nothing: either way the copy kept stays.
    const text = await response.text();
    await env.DB.prepare(
      `UPDATE market_pulls SET pulled_at = ?2,
         items = (SELECT json_group_object(CAST(json_extract(value, '$.item_id') AS TEXT), ${KEPT[source]})
                  FROM json_each(?1) WHERE json_type(value, '$.item_id') = 'integer' AND ${ROW_KEPT[source]})
       WHERE source = ?3 AND json_type(?1) = 'array' AND json_array_length(?1) > 0`,
    )
      .bind(text, now, source)
      .run();
  } catch {
    // Unreachable, or not JSON: the copy kept stays, and the next read a minute on asks again.
  }
}

/** What is kept of each table's row (`value`): /api/prices' figures without those by attack
 *  roll or its deal price, which the app doesn't show, and /api/items' cheapest price for one
 *  and its listings. */
const KEPT: Record<Source, string> = {
  prices: "json_remove(value, '$.item_id', '$.by_attack', '$.typical_attack', '$.deal')",
  listed: "json_object('min_unit_price', json_extract(value, '$.min_unit_price'), 'listings', json_extract(value, '$.listings'))",
};

/** Which of each table's rows are kept: an /api/items row only with both its figures. */
const ROW_KEPT: Record<Source, string> = {
  prices: "1",
  listed: "json_type(value, '$.min_unit_price') IN ('integer', 'real') AND json_type(value, '$.listings') = 'integer'",
};
