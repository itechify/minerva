// Signing in with a made-up Discord, and the account page's and the app's calls, as the
// site's pages and the app make them. Discord's two answers (the token, and who the user
// is) are taken off the global fetch: the code a test signs in with is the Discord user's ID.
import { afterEach, beforeEach } from "vitest";
import { ME, TOKEN } from "../discord";
import { call } from "./api";

export const SITE = "https://owlofminerva.app";

/** The made-up maintainer's Discord ID (vitest.config.ts). */
export const MAINTAINER_ID = "900000000000000001";

/** Each made-up Discord user's username, by ID, as Discord would answer now. */
const usernames = new Map<string, string>();

/** Take Discord's answers off the global fetch for each test of the file that calls this. */
export function withMadeUpDiscord() {
  let realFetch: typeof globalThis.fetch;
  beforeEach(() => {
    realFetch = globalThis.fetch;
    globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = input instanceof Request ? input.url : String(input);
      if (url === TOKEN) {
        const code = new URLSearchParams(String(init?.body)).get("code") ?? "";
        if (!usernames.has(code)) return new Response("{}", { status: 400 });
        return Response.json({ access_token: `token-for-${code}`, token_type: "Bearer" });
      }
      if (url === ME) {
        const id = new Headers(init?.headers).get("authorization")?.replace("Bearer token-for-", "") ?? "";
        return Response.json({ id, username: usernames.get(id) });
      }
      return realFetch(input as RequestInfo, init);
    }) as typeof globalThis.fetch;
  });
  afterEach(() => {
    globalThis.fetch = realFetch;
  });
}

/** The `name=value` of each cookie a response sets, by name. */
export function cookiesSet(response: Response): Map<string, string> {
  const set = new Map<string, string>();
  for (const line of response.headers.getSetCookie()) {
    const [pair] = line.split(";");
    const at = pair.indexOf("=");
    set.set(pair.slice(0, at), line);
  }
  return set;
}

/** A `Cookie` header carrying the cookies a response set, those it cleared left out. */
function cookieHeader(response: Response): string {
  return [...cookiesSet(response).values()]
    .filter((line) => !line.includes("Max-Age=0"))
    .map((line) => line.split(";")[0])
    .join("; ");
}

/** The sign-in's start: the address Discord is sent to, and the state cookie set. */
export async function startSignIn(back = "/account/"): Promise<{ state: string; cookie: string; location: URL }> {
  const response = await call(`/api/auth/discord?return=${encodeURIComponent(back)}`, { redirect: "manual" });
  const location = new URL(response.headers.get("location") ?? "about:blank");
  return { state: location.searchParams.get("state") ?? "", cookie: cookieHeader(response), location };
}

/** Discord sending the player back with `code` for `state`, carrying `cookie`. */
export function finishSignIn(code: string, state: string, cookie: string): Promise<Response> {
  return call(`/api/auth/discord/callback?code=${code}&state=${state}`, { headers: { cookie }, redirect: "manual" });
}

/** A made-up Discord user `id`, named `username` now. */
export function discordUser(id: string, username: string) {
  usernames.set(id, username);
}

/** Sign in as the Discord user `id`, named `username` now: the `Cookie` header of the session. */
export async function signIn(id: string, username: string): Promise<string> {
  discordUser(id, username);
  const { state, cookie } = await startSignIn();
  const back = await finishSignIn(id, state, cookie);
  if (back.status !== 302) throw new Error(`sign-in: ${back.status} ${await back.text()}`);
  return cookieHeader(back);
}

/** A call from the site's own pages, signed in with `cookie`. */
export function asSignedIn(cookie: string, path: string, init: RequestInit = {}, origin = SITE): Promise<Response> {
  return call(path, { ...init, headers: { cookie, origin, ...(init.headers ?? {}) } });
}

/** The account page's read. */
export async function account(cookie: string): Promise<{
  username: string;
  maintainer: boolean;
  pcs: { id: number; linkedAt: number; lastUsedAt: number; characters: string[] }[];
}> {
  const response = await asSignedIn(cookie, "/api/account");
  if (response.status !== 200) throw new Error(`account read: ${response.status} ${await response.text()}`);
  return response.json();
}

/** The app asking for a link token with `key`. */
export async function linkToken(key: string): Promise<string> {
  const response = await call("/api/link", { method: "POST", headers: { authorization: `Bearer ${key}` } });
  if (response.status !== 201) throw new Error(`link token: ${response.status} ${await response.text()}`);
  return ((await response.json()) as { token: string }).token;
}

/** The player confirming `token` on the site, signed in with `cookie`. */
export function confirm(cookie: string, token: string, origin = SITE): Promise<Response> {
  return asSignedIn(cookie, `/api/link/tokens/${token}`, { method: "POST" }, origin);
}

/** Link `key` to the Account signed in with `cookie`, as the app and the page do it. */
export async function link(key: string, cookie: string): Promise<void> {
  const response = await confirm(cookie, await linkToken(key));
  if (response.status !== 200) throw new Error(`link: ${response.status} ${await response.text()}`);
}

/** The app reading its link state with `key`. */
export async function linkState(key: string): Promise<{ linked: boolean; username?: string }> {
  return (await call("/api/link", { headers: { authorization: `Bearer ${key}` } })).json();
}
