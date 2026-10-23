// Leaderboard Keys and the maintainer's token: what a request carries, and the hashes kept.

/** The Bearer token a request carries, if it has one of the shape a Leaderboard Key has:
 *  32 to 256 letters, digits, `-` or `_`. */
export function bearer(request: Request): string | undefined {
  const match = /^Bearer ([A-Za-z0-9_-]{32,256})$/.exec(request.headers.get("authorization") ?? "");
  return match?.[1];
}

/** SHA-256 of `text`, in hex. */
export async function hash(text: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** Whether the request carries the maintainer's token (the ADMIN_TOKEN secret). With no
 *  secret set, nobody does. The maintainer may sign in with Discord instead (sessions.ts). */
export async function isMaintainerToken(request: Request, env: Env): Promise<boolean> {
  const given = /^Bearer (.+)$/.exec(request.headers.get("authorization") ?? "")?.[1];
  if (!env.ADMIN_TOKEN || given === undefined) return false;
  // Compared as hashes, so the comparison takes as long whatever matches.
  const [a, b] = await Promise.all([hash(given), hash(env.ADMIN_TOKEN)]);
  return crypto.subtle.timingSafeEqual(new TextEncoder().encode(a), new TextEncoder().encode(b));
}
