// Blocks (ADR 0031): a player kept off the Map Leaderboards until the maintainer lifts it. A
// Block reaches the Character's Account (if it had one), the keys kept beside it, and, if the
// maintainer asks, the name itself. Uploads it reaches are refused, and every Character it
// reaches is left out of the public reads, its entries kept for when it's lifted.
import { type Answer, badRequest, body } from "./http";

/** Whether Block `b` reaches the Character `c` (`characters` rows, by those aliases): by its
 *  name, its Account, or the key that holds it. */
const REACHES = `((b.blocks_name = 1 AND b.name_key = c.name_key)
  OR (b.account_id IS NOT NULL AND b.account_id = c.account_id)
  OR EXISTS (SELECT 1 FROM block_keys k WHERE k.block_id = b.id AND k.key_hash = c.key_hash))`;

/** Every Character a Block reaches, by its name in lower case: to put after `IN`. */
export const HIDDEN = `SELECT c.name_key FROM characters c JOIN blocks b ON ${REACHES}`;

/** The entries the public reads show, as SQL to put where `entries` would go. */
export const SHOWN = `(SELECT * FROM entries WHERE name_key NOT IN (${HIDDEN}))`;

/** A Block's note is cut to this many characters. */
const NOTE_LENGTH = 500;

/** Whether an upload for `nameKey` with the key `keyHash`, linked to `account` (or not, `null`),
 *  is one a Block reaches. */
export async function isBlocked(env: Env, nameKey: string, keyHash: string, account: number | null): Promise<boolean> {
  const row = await env.DB.prepare(
    `SELECT 1 AS yes FROM blocks b
     WHERE (b.blocks_name = 1 AND b.name_key = ?1) OR (b.account_id IS NOT NULL AND b.account_id = ?3)
        OR EXISTS (SELECT 1 FROM block_keys k WHERE k.block_id = b.id AND k.key_hash = ?2)
     LIMIT 1`,
  )
    .bind(nameKey, keyHash, account)
    .first<number>("yes");
  return row !== null;
}

/** The refusal of an upload a Block reaches. */
export const BLOCKED: Answer = { status: 403, body: { error: "blocked" } };

/** The Block that reaches `nameKey`, the oldest if more than one does, or `null`. */
export async function blockReaching(env: Env, nameKey: string): Promise<{ id: number; blockedAt: number; note: string } | null> {
  return env.DB.prepare(
    `SELECT b.id, b.blocked_at AS blockedAt, b.note FROM characters c JOIN blocks b ON ${REACHES}
     WHERE c.name_key = ? ORDER BY b.id LIMIT 1`,
  )
    .bind(nameKey)
    .first<{ id: number; blockedAt: number; note: string }>();
}

/** Block `name`, with `{ name?: boolean, note?: string }`: its Account, the key that claimed
 *  it and every key linked to the Account now, and with `name`, the name itself. The reports
 *  against the Characters it hides are dismissed. */
export async function block(env: Env, request: Request, name: string): Promise<Answer> {
  const nameKey = name.toLowerCase();
  const asked = ((await body(request)) ?? {}) as { name?: unknown; note?: unknown };
  if (asked.name !== undefined && typeof asked.name !== "boolean") return badRequest("a Block's name is true or false");
  if (asked.note !== undefined && typeof asked.note !== "string") return badRequest("a Block's note is text");
  const character = await env.DB.prepare("SELECT key_hash, account_id FROM characters WHERE name_key = ?")
    .bind(nameKey)
    .first<{ key_hash: string; account_id: number | null }>();
  if (character === null) return { status: 404, body: { error: "no-character" } };
  if ((await blockReaching(env, nameKey)) !== null) return { status: 409, body: { error: "blocked" } };
  const note = (asked.note ?? "").trim().slice(0, NOTE_LENGTH);
  const id = (await env.DB.prepare(
    "INSERT INTO blocks (name_key, blocks_name, account_id, note, blocked_at) VALUES (?, ?, ?, ?, ?) RETURNING id",
  )
    .bind(nameKey, asked.name === true ? 1 : 0, character.account_id, note, Date.now())
    .first<number>("id"))!;
  const [, , dismissed] = await env.DB.batch([
    env.DB.prepare("INSERT OR IGNORE INTO block_keys (block_id, key_hash) VALUES (?, ?)").bind(id, character.key_hash),
    env.DB.prepare("INSERT OR IGNORE INTO block_keys (block_id, key_hash) SELECT ?1, key_hash FROM account_keys WHERE account_id = ?2").bind(
      id,
      character.account_id,
    ),
    env.DB.prepare(`DELETE FROM reports WHERE name_key IN (${HIDDEN})`),
  ]);
  const reports = dismissed.meta.changes;
  const detail = [asked.name === true ? "the name too" : "", reports > 0 ? `${reports} report${reports === 1 ? "" : "s"} dismissed` : "", note]
    .filter((part) => part !== "")
    .join("; ");
  return { status: 201, body: { id }, logged: { nameKey, accountId: character.account_id ?? undefined, detail } };
}

/** Every Block, the newest first: the Character it was made from, by its name as it is now,
 *  its Account and how many keys it holds. */
export async function readBlocks(env: Env): Promise<Answer> {
  const rows = await env.DB.prepare(
    `SELECT b.id, COALESCE(c.name, b.name_key) AS name, b.blocks_name, b.account_id, l.username, b.note, b.blocked_at,
            (SELECT COUNT(*) FROM block_keys k WHERE k.block_id = b.id) AS keys
     FROM blocks b LEFT JOIN characters c USING (name_key)
     LEFT JOIN logins l ON l.account_id = b.account_id AND l.provider = 'discord'
     ORDER BY b.blocked_at DESC, b.id DESC`,
  ).all<{
    id: number;
    name: string;
    blocks_name: number;
    account_id: number | null;
    username: string | null;
    note: string;
    blocked_at: number;
    keys: number;
  }>();
  const blocks = rows.results.map((r) => ({
    id: r.id,
    name: r.name,
    blocksName: r.blocks_name === 1,
    account: r.account_id === null ? null : { id: r.account_id, username: r.username },
    keys: r.keys,
    note: r.note,
    blockedAt: r.blocked_at,
  }));
  return { status: 200, body: { blocks } };
}

/** Lift Block `id`: uploads it refused go through again, and the Characters it hid are back
 *  where their entries put them. */
export async function unblock(env: Env, id: string): Promise<Answer> {
  if (!/^\d+$/.test(id)) return badRequest("a Block is its number");
  const lifted = await env.DB.prepare("SELECT name_key, account_id FROM blocks WHERE id = ?")
    .bind(Number(id))
    .first<{ name_key: string; account_id: number | null }>();
  if (lifted === null) return { status: 404, body: { error: "no-block" } };
  await env.DB.batch([
    env.DB.prepare("DELETE FROM block_keys WHERE block_id = ?").bind(Number(id)),
    env.DB.prepare("DELETE FROM blocks WHERE id = ?").bind(Number(id)),
  ]);
  return { status: 204, logged: { nameKey: lifted.name_key, accountId: lifted.account_id ?? undefined } };
}
