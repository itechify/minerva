// Signing in with Discord (ADR 0023): OAuth2's authorization code flow, asking only for the
// `identify` scope. The Account is ours; Discord's user ID is how it signs in, and its
// username, refreshed at each sign-in, is kept for the maintainer to match a player by.
import { type Answer, redirect } from "./http";
import { cookie, secret, setCookie, startSession } from "./sessions";

const AUTHORIZE = "https://discord.com/oauth2/authorize";
export const TOKEN = "https://discord.com/api/oauth2/token";
export const ME = "https://discord.com/api/users/@me";

/** The cookie holding OAuth's `state` and the page to return to, for the 10 minutes a
 *  sign-in may take. */
const STATE = "__Host-oauth";
const STATE_S = 600;

/** Where a sign-in returns when the request names nowhere it may. */
const HOME = "/account/";

/** The page a sign-in returns to: a path on the site, never an address elsewhere. */
function returnPath(asked: string | null): string {
  if (asked === null || !asked.startsWith("/") || asked.startsWith("//") || asked.startsWith("/\\")) return HOME;
  return asked;
}

/** The address Discord sends the player back to, which the Discord application lists. */
function callbackOf(url: URL): string {
  return `${url.origin}/api/auth/discord/callback`;
}

/** Send the player to Discord to sign in, and on to `?return=` after. */
export function start(env: Env, url: URL): Answer {
  if (!env.DISCORD_CLIENT_ID || !env.DISCORD_CLIENT_SECRET) return { status: 503, body: { error: "sign-in-off" } };
  const state = secret();
  const to = new URL(AUTHORIZE);
  to.search = new URLSearchParams({
    response_type: "code",
    client_id: env.DISCORD_CLIENT_ID,
    scope: "identify",
    state,
    redirect_uri: callbackOf(url),
  }).toString();
  const back = encodeURIComponent(returnPath(url.searchParams.get("return")));
  return redirect(to.toString(), [setCookie(STATE, `${state}.${back}`, STATE_S)]);
}

interface DiscordUser {
  id: string;
  username: string;
}

/** The Discord user a code from the sign-in signs in, or `null` if Discord won't say. */
async function userOf(env: Env, code: string, url: URL): Promise<DiscordUser | null> {
  const token = await fetch(TOKEN, {
    method: "POST",
    headers: {
      "content-type": "application/x-www-form-urlencoded",
      authorization: `Basic ${btoa(`${env.DISCORD_CLIENT_ID}:${env.DISCORD_CLIENT_SECRET}`)}`,
    },
    body: new URLSearchParams({ grant_type: "authorization_code", code, redirect_uri: callbackOf(url) }),
  });
  if (!token.ok) return null;
  const { access_token } = (await token.json()) as { access_token?: string };
  if (typeof access_token !== "string") return null;
  const me = await fetch(ME, { headers: { authorization: `Bearer ${access_token}` } });
  if (!me.ok) return null;
  const user = (await me.json()) as Partial<DiscordUser>;
  return typeof user.id === "string" && typeof user.username === "string" ? { id: user.id, username: user.username } : null;
}

/** The Account `user` signs in to, made on their first sign-in, with their username as now. */
async function accountFor(env: Env, user: DiscordUser, now: number): Promise<number> {
  const known = await env.DB.prepare("SELECT account_id FROM logins WHERE provider = 'discord' AND provider_user_id = ?")
    .bind(user.id)
    .first<number>("account_id");
  if (known !== null) {
    await env.DB.prepare("UPDATE logins SET username = ? WHERE provider = 'discord' AND provider_user_id = ?")
      .bind(user.username, user.id)
      .run();
    return known;
  }
  const account = (await env.DB.prepare("INSERT INTO accounts (created_at) VALUES (?) RETURNING id").bind(now).first<number>("id"))!;
  await env.DB.prepare("INSERT INTO logins (provider, provider_user_id, account_id, username) VALUES ('discord', ?, ?, ?)")
    .bind(user.id, account, user.username)
    .run();
  return account;
}

/** Where Discord sends the player back: sign them in, and return them to the page they
 *  came from. A sign-in that didn't go through returns them there with `?signin=failed`. */
export async function callback(env: Env, request: Request, url: URL): Promise<Answer> {
  // The state, then the page to return to, encoded: a page's address may hold a "." of its own.
  const held = cookie(request, STATE) ?? "";
  const dot = held.indexOf(".");
  const state = dot < 0 ? held : held.slice(0, dot);
  const to = returnPath(dot < 0 ? null : decodeURIComponent(held.slice(dot + 1)));
  const clearState = setCookie(STATE, "", 0);
  const failed = () => {
    const page = new URL(to, url.origin);
    page.searchParams.set("signin", "failed");
    return redirect(`${page.pathname}${page.search}`, [clearState]);
  };
  const code = url.searchParams.get("code");
  if (!state || url.searchParams.get("state") !== state || code === null) return failed();
  if (!env.DISCORD_CLIENT_ID || !env.DISCORD_CLIENT_SECRET) return failed();
  const user = await userOf(env, code, url);
  if (user === null) return failed();
  const now = Date.now();
  const account = await accountFor(env, user, now);
  return redirect(to, [clearState, ...(await startSession(env, account, now))]);
}
