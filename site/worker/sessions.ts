// Signed-in sessions on the site (ADR 0023): a cookie holding a secret, kept in D1 only as
// its hash, and a marker cookie beside it that the pages read to know one is there, so a
// page asks who's signed in only then.
import { hash } from "./keys";

/** The session's cookie: HttpOnly, so no page's script reads the secret. */
const SESSION = "__Host-session";

/** The marker: the pages' script reads it, and asks the API who's signed in only with it. */
const MARKER = "signed_in";

const DAY_S = 24 * 3_600;

/** How long a sign-in lasts: a stolen maintainer's cookie can delete entries, so theirs is
 *  short; players rarely come back to the site. */
const PLAYER_DAYS = 30;
const MAINTAINER_DAYS = 1;

/** A new secret: 32 random bytes, as base64url. */
export function secret(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  return btoa(String.fromCharCode(...bytes))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");
}

/** The request's cookie named `name`, if it has one. */
export function cookie(request: Request, name: string): string | undefined {
  for (const part of (request.headers.get("cookie") ?? "").split(";")) {
    const at = part.indexOf("=");
    if (at > 0 && part.slice(0, at).trim() === name) return part.slice(at + 1).trim();
  }
  return undefined;
}

/** A `Set-Cookie` value for the site, secure and same-site, kept `maxAge` seconds (0 clears it). */
export function setCookie(name: string, value: string, maxAge: number, httpOnly = true): string {
  return `${name}=${value}; Path=/; Secure;${httpOnly ? " HttpOnly;" : ""} SameSite=Lax; Max-Age=${maxAge}`;
}

/** Discord IDs of the maintainers, from the Worker's MAINTAINER_DISCORD_IDS variable. */
function maintainerIds(env: Env): string[] {
  return (env.MAINTAINER_DISCORD_IDS ?? "")
    .split(",")
    .map((id) => id.trim())
    .filter((id) => id !== "");
}

/** Whether `account` signs in with a maintainer's Discord account. */
export async function isMaintainerAccount(env: Env, account: number): Promise<boolean> {
  const ids = maintainerIds(env);
  if (ids.length === 0) return false;
  const row = await env.DB.prepare(
    `SELECT 1 AS yes FROM logins WHERE account_id = ? AND provider = 'discord'
     AND provider_user_id IN (${ids.map(() => "?").join(", ")})`,
  )
    .bind(account, ...ids)
    .first<number>("yes");
  return row !== null;
}

/** Sign `account` in: a new session, and the cookies that carry it. Expired sessions go. */
export async function startSession(env: Env, account: number, now: number): Promise<string[]> {
  const days = (await isMaintainerAccount(env, account)) ? MAINTAINER_DAYS : PLAYER_DAYS;
  const id = secret();
  await env.DB.batch([
    env.DB.prepare("DELETE FROM sessions WHERE expires_at <= ?").bind(now),
    env.DB.prepare("INSERT INTO sessions (id_hash, account_id, expires_at) VALUES (?, ?, ?)").bind(
      await hash(id),
      account,
      now + days * DAY_S * 1000,
    ),
  ]);
  return [setCookie(SESSION, id, days * DAY_S), setCookie(MARKER, "1", days * DAY_S, false)];
}

/** The Account the request is signed in to, or `null`. */
export async function sessionAccount(env: Env, request: Request, now: number): Promise<number | null> {
  const id = cookie(request, SESSION);
  if (id === undefined) return null;
  return env.DB.prepare("SELECT account_id FROM sessions WHERE id_hash = ? AND expires_at > ?")
    .bind(await hash(id), now)
    .first<number>("account_id");
}

/** Sign the request's session out, and the cookies that clear it. */
export async function endSession(env: Env, request: Request): Promise<string[]> {
  const id = cookie(request, SESSION);
  if (id !== undefined) await env.DB.prepare("DELETE FROM sessions WHERE id_hash = ?").bind(await hash(id)).run();
  return clearedCookies();
}

/** The cookies that sign a browser out. */
export function clearedCookies(): string[] {
  return [setCookie(SESSION, "", 0), setCookie(MARKER, "", 0, false)];
}

/** Whether a request comes from the site's own pages: a write signed in by cookie must, so
 *  another site can't make one with the player's cookie (beside `SameSite=Lax`). */
export function sameOrigin(request: Request): boolean {
  return request.headers.get("origin") === new URL(request.url).origin;
}
