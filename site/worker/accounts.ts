// Accounts (ADR 0023): the Leaderboard Keys linked to each, the account page's reads and
// writes, and linking a key, which the app starts with a one-time token and the player
// confirms on the site while signed in.
import { type Answer, badRequest } from "./http";
import { bearer, hash } from "./keys";
import { WRITES, limit } from "./rate-limits";
import { clearedCookies, isMaintainerAccount, secret } from "./sessions";

/** How long a link token is good for: the player opens it as the app makes it. */
const TOKEN_MS = 5 * 60_000;

/** The Account a Leaderboard Key (by its hash) is linked to, or `null`. */
export function accountOfKey(env: Env, keyHash: string): Promise<number | null> {
  return env.DB.prepare("SELECT account_id FROM account_keys WHERE key_hash = ?").bind(keyHash).first<number>("account_id");
}

/** Note a linked key's write, so the Account's most recently used key is known. */
export async function touchKey(env: Env, keyHash: string, now: number): Promise<void> {
  await env.DB.prepare("UPDATE account_keys SET last_used_at = ? WHERE key_hash = ?").bind(now, keyHash).run();
}

/** The Discord username `account` signs in with. */
function usernameOf(env: Env, account: number): Promise<string | null> {
  return env.DB.prepare("SELECT username FROM logins WHERE account_id = ? AND provider = 'discord'")
    .bind(account)
    .first<string>("username");
}

/** The names `keyHash` claimed, in `account` if given, as last uploaded, A to Z. */
async function namesClaimedBy(env: Env, keyHash: string, account?: number): Promise<string[]> {
  const rows = await env.DB.prepare(
    `SELECT name FROM characters WHERE key_hash = ?1 AND (?2 IS NULL OR account_id = ?2) ORDER BY name_key`,
  )
    .bind(keyHash, account ?? null)
    .all<{ name: string }>();
  return rows.results.map((r) => r.name);
}

/** The account page: who's signed in, whether they're the maintainer, and their linked PCs
 *  (the most recently used first), each with the Characters it claimed. */
export async function readAccount(env: Env, account: number): Promise<Answer> {
  const keys = await env.DB.prepare(
    "SELECT id, key_hash, linked_at, last_used_at FROM account_keys WHERE account_id = ? ORDER BY last_used_at DESC, id DESC",
  )
    .bind(account)
    .all<{ id: number; key_hash: string; linked_at: number; last_used_at: number }>();
  const pcs = [];
  for (const k of keys.results) {
    pcs.push({ id: k.id, linkedAt: k.linked_at, lastUsedAt: k.last_used_at, characters: await namesClaimedBy(env, k.key_hash, account) });
  }
  return {
    status: 200,
    body: { username: await usernameOf(env, account), maintainer: await isMaintainerAccount(env, account), pcs },
  };
}

/** Unlink the PC `id` from `account`: its key loses what it had through the Account. A name
 *  it claimed moves to the Account's most recently used other key, or, with none, stays
 *  with it and leaves the Account. */
export async function unlinkPc(env: Env, account: number, id: string): Promise<Answer> {
  if (!/^\d+$/.test(id)) return badRequest("a PC is its number");
  const pc = await env.DB.prepare("SELECT key_hash FROM account_keys WHERE id = ? AND account_id = ?")
    .bind(Number(id), account)
    .first<string>("key_hash");
  if (pc === null) return { status: 404, body: { error: "no-pc" } };
  const other = await env.DB.prepare(
    "SELECT key_hash FROM account_keys WHERE account_id = ? AND key_hash != ? ORDER BY last_used_at DESC, id DESC LIMIT 1",
  )
    .bind(account, pc)
    .first<string>("key_hash");
  await env.DB.batch([
    other === null
      ? env.DB.prepare("UPDATE characters SET account_id = NULL WHERE key_hash = ? AND account_id = ?").bind(pc, account)
      : env.DB.prepare("UPDATE characters SET key_hash = ? WHERE key_hash = ? AND account_id = ?").bind(other, pc, account),
    env.DB.prepare("DELETE FROM account_keys WHERE key_hash = ?").bind(pc),
  ]);
  return { status: 204 };
}

/** Delete `account`: its logins, linked keys and sessions go, and each of its names stays
 *  with the key that claimed it. The entries stay on the Map Leaderboards. */
export async function deleteAccount(env: Env, account: number): Promise<Answer> {
  await env.DB.batch([
    env.DB.prepare("UPDATE characters SET account_id = NULL WHERE account_id = ?").bind(account),
    env.DB.prepare("DELETE FROM account_keys WHERE account_id = ?").bind(account),
    env.DB.prepare("DELETE FROM sessions WHERE account_id = ?").bind(account),
    env.DB.prepare("DELETE FROM logins WHERE account_id = ?").bind(account),
    env.DB.prepare("DELETE FROM accounts WHERE id = ?").bind(account),
  ]);
  return { status: 204, cookies: clearedCookies() };
}

/** The app asking, with its key, for a one-time link token, and the page that confirms it.
 *  Counted as a write against the key's rate limit. */
export async function mintLinkToken(env: Env, request: Request, url: URL): Promise<Answer> {
  const key = bearer(request);
  if (key === undefined) return { status: 401, body: { error: "no-key" } };
  const keyHash = await hash(key);
  const limited = await limit(env, `key:${keyHash}`, WRITES);
  if (limited !== undefined) return limited;
  const token = secret();
  const now = Date.now();
  const expiresAt = now + TOKEN_MS;
  await env.DB.batch([
    env.DB.prepare("DELETE FROM link_tokens WHERE expires_at <= ?").bind(now),
    env.DB.prepare("INSERT INTO link_tokens (token_hash, key_hash, expires_at) VALUES (?, ?, ?)").bind(await hash(token), keyHash, expiresAt),
  ]);
  return { status: 201, body: { token, url: `${url.origin}/account/link/?t=${token}`, expiresAt } };
}

/** The app asking, with its key, what Account it's linked to. */
export async function linkState(env: Env, request: Request): Promise<Answer> {
  const key = bearer(request);
  if (key === undefined) return { status: 401, body: { error: "no-key" } };
  const account = await accountOfKey(env, await hash(key));
  if (account === null) return { status: 200, body: { linked: false } };
  return { status: 200, body: { linked: true, username: await usernameOf(env, account) } };
}

/** The key a link token, still good, was made for. */
async function keyOfToken(env: Env, token: string, now: number): Promise<string | null> {
  return env.DB.prepare("SELECT key_hash FROM link_tokens WHERE token_hash = ? AND expires_at > ?")
    .bind(await hash(token), now)
    .first<string>("key_hash");
}

/** What the confirmation page shows of a link token: the Characters its key holds, and
 *  whether it's linked already, to this Account or another. */
export async function readLinkToken(env: Env, account: number, token: string): Promise<Answer> {
  const keyHash = await keyOfToken(env, token, Date.now());
  if (keyHash === null) return { status: 404, body: { error: "no-token" } };
  const linkedTo = await accountOfKey(env, keyHash);
  const linked = linkedTo === null ? null : linkedTo === account ? "this" : "other";
  return { status: 200, body: { characters: await namesClaimedBy(env, keyHash), linked } };
}

/** Link a token's key to `account`, as the player confirmed on the site: the token is used
 *  up, the key joins the Account (leaving any other it was linked to), and the names it
 *  claimed join the Account with it. */
export async function confirmLinkToken(env: Env, account: number, token: string): Promise<Answer> {
  const now = Date.now();
  const keyHash = await env.DB.prepare("DELETE FROM link_tokens WHERE token_hash = ? AND expires_at > ? RETURNING key_hash")
    .bind(await hash(token), now)
    .first<string>("key_hash");
  if (keyHash === null) return { status: 404, body: { error: "no-token" } };
  await env.DB.batch([
    env.DB.prepare(
      `INSERT INTO account_keys (key_hash, account_id, linked_at, last_used_at) VALUES (?1, ?2, ?3, ?3)
       ON CONFLICT (key_hash) DO UPDATE SET account_id = ?2, linked_at = ?3, last_used_at = ?3`,
    ).bind(keyHash, account, now),
    env.DB.prepare("UPDATE characters SET account_id = ? WHERE key_hash = ?").bind(account, keyHash),
  ]);
  return { status: 200, body: { characters: await namesClaimedBy(env, keyHash) } };
}
