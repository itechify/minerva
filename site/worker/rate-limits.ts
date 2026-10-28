// Rate limits, counted in D1 over a sliding window: per Leaderboard Key for writes, per
// reporter for reports, and per IP address for the items market reads ask about.
import type { Answer } from "./http";

/** The window the limits count over. */
const WINDOW_MS = 10 * 60_000;

/** Writes one Leaderboard Key may make in the window, whatever names they are for: an
 *  upload or an opt-out each. The app writes a Character's set once at opting in and then
 *  as each Visit ends, so a player never comes near it. */
export const WRITES = 30;

/** Reports one reporter (by IP address) may make in the window. */
export const REPORTS = 10;

/** Items one IP address may ask maplestory.market's prices of in the window (ADR 0030): 200
 *  a minute, so a player reading shop after shop never comes near it, and nobody copies the
 *  whole table through it. */
export const MARKET_ITEMS = 2_000;

/** Count a request in `bucket` as `weight`, and refuse it if it would take the bucket past
 *  `most` in the window. A refused request isn't counted. */
export async function limit(env: Env, bucket: string, most: number, weight = 1): Promise<Answer | undefined> {
  const now = Date.now();
  const since = now - WINDOW_MS;
  const [, made] = await env.DB.batch<{ n: number; oldest: number | null }>([
    env.DB.prepare("DELETE FROM rate_events WHERE at <= ?").bind(since),
    env.DB.prepare("SELECT COALESCE(SUM(weight), 0) AS n, MIN(at) AS oldest FROM rate_events WHERE bucket = ? AND at > ?").bind(bucket, since),
  ]);
  const { n, oldest } = made.results[0];
  if (n + weight > most) {
    const retryAfter = Math.max(1, Math.ceil(((oldest ?? now) + WINDOW_MS - now) / 1000));
    return { status: 429, body: { error: "rate-limited" }, headers: { "retry-after": String(retryAfter) } };
  }
  await env.DB.prepare("INSERT INTO rate_events (bucket, at, weight) VALUES (?, ?, ?)").bind(bucket, now, weight).run();
  return undefined;
}
