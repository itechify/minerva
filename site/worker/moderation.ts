// Reports against entries, and the maintainer's routes: reading and dismissing the reports,
// taking an entry down (or all of a Character's) and restoring it, looking a name up, and
// reassigning it to the key its player has now. Each write says what the log keeps of it
// (ADR 0031).
import type { Kind } from "../entries";
import { linesOf } from "./admin-log";
import { blockReaching } from "./blocks";
import { type Answer, badRequest, body } from "./http";
import { hash } from "./keys";
import { isParty, kindCondition, sqlKind, sqlParty } from "./kinds";
import { REPORTS, limit } from "./rate-limits";

/** A report's reason is cut to this many characters. */
const REASON_LENGTH = 500;

/** Record a report against the entry `{ name, map, band, party? }` (the Party Best's if
 *  `party`, otherwise the Solo Best's), with an optional `reason`. */
export async function report(env: Env, request: Request): Promise<Answer> {
  const { name, map, band, reason, party = false } = ((await body(request)) ?? {}) as Record<string, unknown>;
  if (typeof name !== "string" || typeof map !== "string" || !Number.isInteger(band)) {
    return badRequest("a report is { name, map, band, party?, reason? }");
  }
  if (typeof party !== "boolean") return badRequest("a report's party is true or false");
  if (reason !== undefined && typeof reason !== "string") return badRequest("a report's reason is text");
  const reporter = await hash(request.headers.get("cf-connecting-ip") ?? "unknown");
  const limited = await limit(env, `report:${reporter}`, REPORTS);
  if (limited !== undefined) return limited;
  const reported = await env.DB.prepare(
    `INSERT INTO reports (name_key, map_id, band, exp_per_hour, reason, reported_at, party)
     SELECT name_key, map_id, band, exp_per_hour, ?4, ?5, party FROM entries
     WHERE name_key = ?1 AND map_id = ?2 AND band = ?3 AND party = ?6`,
  )
    .bind(name.toLowerCase(), map, band, (reason ?? "").slice(0, REASON_LENGTH), Date.now(), sqlParty(party))
    .run();
  if (reported.meta.changes === 0) return { status: 404, body: { error: "no-entry" } };
  return { status: 201, body: {} };
}

/** Every report, newest first, with the reported entry's name as it is now, and whether the
 *  Worker made it (ADR 0032) or a player. */
export async function readReports(env: Env): Promise<Answer> {
  const rows = await env.DB.prepare(
    `SELECT r.id, COALESCE(c.name, r.name_key) AS name, r.map_id AS map, r.band, r.exp_per_hour AS expPerHour,
            r.reason, r.reported_at AS reportedAt, r.party = 1 AS party, r.automatic
     FROM reports r LEFT JOIN characters c USING (name_key) ORDER BY r.reported_at DESC, r.id DESC`,
  ).all<{ party: number; automatic: number }>();
  const reports = rows.results.map((r) => ({ ...r, party: isParty(r.party), automatic: r.automatic === 1 }));
  return { status: 200, body: { reports } };
}

/** Delete `name`'s entry on `map` in the Level Band starting at `band`, of the `kind` asked for
 *  or, with none, both its Solo Best's and its Party Best's, with their reports and the top
 *  spot taken with it, if one was. The same Visit (the same EXP/h there, of that kind) uploaded
 *  again is rejected as "removed". */
export async function deleteEntry(env: Env, name: string, map: string, band: string, kind: Kind | null): Promise<Answer> {
  const at = [name.toLowerCase(), map, Number(band), sqlKind(kind)];
  const [removed] = await env.DB.batch([
    env.DB.prepare(
      `INSERT OR IGNORE INTO removed_entries (name_key, map_id, band, exp_per_hour, party)
       SELECT name_key, map_id, band, exp_per_hour, party FROM entries
       WHERE name_key = ?1 AND map_id = ?2 AND band = ?3 AND ${kindCondition("?4")}`,
    ).bind(...at),
    env.DB.prepare(`DELETE FROM entries WHERE name_key = ?1 AND map_id = ?2 AND band = ?3 AND ${kindCondition("?4")}`).bind(...at),
    env.DB.prepare(`DELETE FROM reports WHERE name_key = ?1 AND map_id = ?2 AND band = ?3 AND ${kindCondition("?4")}`).bind(...at),
    env.DB.prepare(`DELETE FROM taken_tops WHERE name_key = ?1 AND map_id = ?2 AND band = ?3 AND ${kindCondition("?4")}`).bind(...at),
  ]);
  if (removed.meta.changes === 0) return { status: 404, body: { error: "no-entry" } };
  return { status: 204, logged: { nameKey: name.toLowerCase(), detail: `${map} ${band} ${kind ?? "solo and party"}` } };
}

/** Take every entry of `name` down, as `deleteEntry` takes each: each Visit stays off until
 *  restored. */
export async function deleteAllEntries(env: Env, name: string): Promise<Answer> {
  const nameKey = name.toLowerCase();
  const [removed] = await env.DB.batch([
    env.DB.prepare(
      `INSERT OR IGNORE INTO removed_entries (name_key, map_id, band, exp_per_hour, party)
       SELECT name_key, map_id, band, exp_per_hour, party FROM entries WHERE name_key = ?`,
    ).bind(nameKey),
    env.DB.prepare("DELETE FROM entries WHERE name_key = ?").bind(nameKey),
    env.DB.prepare("DELETE FROM reports WHERE name_key = ?").bind(nameKey),
    env.DB.prepare("DELETE FROM taken_tops WHERE name_key = ?").bind(nameKey),
  ]);
  const entries = removed.meta.changes;
  if (entries === 0) return { status: 404, body: { error: "no-entry" } };
  return { status: 204, logged: { nameKey, detail: `${entries} entr${entries === 1 ? "y" : "ies"}` } };
}

/** Every entry the maintainer took down, by its Character's name as it is now: the Visits
 *  kept off the Map Leaderboards until restored. */
export async function readRemoved(env: Env): Promise<Answer> {
  const rows = await env.DB.prepare(
    `SELECT COALESCE(c.name, r.name_key) AS name, r.map_id AS map, r.band, r.exp_per_hour AS expPerHour, r.party
     FROM removed_entries r LEFT JOIN characters c USING (name_key) ORDER BY name, map, band, r.party`,
  ).all<{ name: string; map: string; band: number; expPerHour: number; party: number }>();
  return { status: 200, body: { removed: rows.results.map((r) => ({ ...r, party: isParty(r.party) })) } };
}

/** Restore the entry taken down for `name` on `map` in the Level Band starting at `band`, the
 *  Visit of `kind` (solo, with none) with `expPerHour`: the next upload of that Visit goes up
 *  again. */
export async function restoreEntry(
  env: Env,
  name: string,
  map: string,
  band: string,
  expPerHour: string,
  kind: Kind | null,
): Promise<Answer> {
  if (!/^\d+$/.test(band) || !/^\d+$/.test(expPerHour)) return badRequest("a band and an EXP/h are numbers");
  const restored = await env.DB.prepare(
    "DELETE FROM removed_entries WHERE name_key = ? AND map_id = ? AND band = ? AND exp_per_hour = ? AND party = ?",
  )
    .bind(name.toLowerCase(), map, Number(band), Number(expPerHour), sqlParty(kind === "party"))
    .run();
  if (restored.meta.changes === 0) return { status: 404, body: { error: "no-removed" } };
  return { status: 204, logged: { nameKey: name.toLowerCase(), detail: `${map} ${band} ${kind ?? "solo"}` } };
}

/** Dismiss report `id`: the report goes, and the entry stays. */
export async function dismissReport(env: Env, id: string): Promise<Answer> {
  if (!/^\d+$/.test(id)) return badRequest("a report is its number");
  const dismissed = await env.DB.prepare("DELETE FROM reports WHERE id = ? RETURNING name_key, map_id, band, party")
    .bind(Number(id))
    .first<{ name_key: string; map_id: string; band: number; party: number }>();
  if (dismissed === null) return { status: 404, body: { error: "no-report" } };
  const kind = isParty(dismissed.party) ? "party" : "solo";
  return { status: 204, logged: { nameKey: dismissed.name_key, detail: `${dismissed.map_id} ${dismissed.band} ${kind}` } };
}

/** What the maintainer sees of `name`: when it was claimed, when another key was last
 *  refused for it, its Account's Discord username, its entries (hidden or not), the Block
 *  that reaches it, and its lines in the log. */
export async function readCharacter(env: Env, name: string): Promise<Answer> {
  const nameKey = name.toLowerCase();
  const character = await env.DB.prepare(
    `SELECT c.name, c.claimed_at, c.refused_at, c.account_id, l.username
     FROM characters c LEFT JOIN logins l ON l.account_id = c.account_id AND l.provider = 'discord'
     WHERE c.name_key = ?`,
  )
    .bind(nameKey)
    .first<{ name: string; claimed_at: number; refused_at: number | null; account_id: number | null; username: string | null }>();
  if (character === null) return { status: 404, body: { error: "no-character" } };
  const entries = await env.DB.prepare(
    `SELECT map_id AS map, band, party, level, job, exp_per_hour AS expPerHour, set_at AS setAt FROM entries
     WHERE name_key = ? ORDER BY band DESC, exp_per_hour DESC, map_id, party`,
  )
    .bind(nameKey)
    .all<{ map: string; band: number; party: number; level: number; job: string; expPerHour: number; setAt: number }>();
  return {
    status: 200,
    body: {
      name: character.name,
      claimedAt: character.claimed_at,
      refusedAt: character.refused_at,
      account: character.account_id === null ? null : { id: character.account_id, username: character.username },
      entries: entries.results.map((e) => ({ ...e, party: isParty(e.party) })),
      blocked: await blockReaching(env, nameKey),
      log: await linesOf(env, nameKey),
    },
  };
}

/** The most Accounts one search of usernames lists. */
const MOST_ACCOUNTS = 20;

/** Accounts whose Discord username holds `query`, whatever its case, with how many PCs each
 *  has linked: to find the one a player's name should go to. */
export async function findAccounts(env: Env, query: string | null): Promise<Answer> {
  if (query === null || query.trim() === "") return badRequest("search with ?username=");
  const rows = await env.DB.prepare(
    `SELECT l.account_id AS id, l.username, COUNT(k.id) AS pcs, MAX(k.last_used_at) AS lastUsedAt
     FROM logins l LEFT JOIN account_keys k ON k.account_id = l.account_id
     WHERE l.provider = 'discord' AND instr(lower(l.username), lower(?1)) > 0
     GROUP BY l.account_id, l.username ORDER BY l.username LIMIT ?2`,
  )
    .bind(query.trim(), MOST_ACCOUNTS)
    .all<{ id: number; username: string; pcs: number; lastUsedAt: number | null }>();
  return { status: 200, body: { accounts: rows.results } };
}

/** Move `name`'s claim to the key its player has now: by default, the last key refused for
 *  it (after they lost the one that claimed it: a reinstall, a new PC), its Account going
 *  with it; with `{ "account": <id> }`, that Account's most recently used key. The
 *  maintainer never sees a key; the answer says when the refused key was refused, to check
 *  with the player. */
export async function reassign(env: Env, request: Request, name: string): Promise<Answer> {
  const nameKey = name.toLowerCase();
  const asked = ((await body(request)) ?? {}) as { account?: unknown };
  if (asked.account !== undefined && !Number.isSafeInteger(asked.account)) return badRequest("an account is its number");
  const character = await env.DB.prepare("SELECT name, refused_key_hash, refused_at FROM characters WHERE name_key = ?")
    .bind(nameKey)
    .first<{ name: string; refused_key_hash: string | null; refused_at: number | null }>();
  if (character === null) return { status: 404, body: { error: "no-character" } };
  const now = Date.now();
  if (asked.account !== undefined) {
    const to = await env.DB.prepare("SELECT key_hash FROM account_keys WHERE account_id = ? ORDER BY last_used_at DESC, id DESC LIMIT 1")
      .bind(asked.account)
      .first<string>("key_hash");
    if (to === null) return { status: 409, body: { error: "no-linked-key" } };
    const account = asked.account as number;
    await env.DB.prepare(
      `UPDATE characters SET key_hash = ?2, account_id = ?3, claimed_at = ?4, refused_key_hash = NULL, refused_at = NULL
       WHERE name_key = ?1`,
    )
      .bind(nameKey, to, account, now)
      .run();
    return { status: 200, body: { name: character.name }, logged: { nameKey, accountId: account, detail: "to the Account" } };
  }
  if (character.refused_key_hash === null) return { status: 409, body: { error: "no-refused-key" } };
  // The name's Account becomes the refused key's (none, if it isn't linked), so no key the
  // old claimant linked can write for it any more.
  await env.DB.prepare(
    `UPDATE characters SET key_hash = refused_key_hash, claimed_at = ?3, refused_key_hash = NULL, refused_at = NULL,
            account_id = (SELECT account_id FROM account_keys WHERE key_hash = ?2)
     WHERE name_key = ?1 AND refused_key_hash = ?2`,
  )
    .bind(nameKey, character.refused_key_hash, now)
    .run();
  return { status: 200, body: { name: character.name, refusedAt: character.refused_at }, logged: { nameKey, detail: "to the refused key" } };
}
