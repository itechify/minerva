// The maintainer's Overview (ADR 0031): counts from what D1 keeps already, and when the
// portraits and the market prices were last had.
import type { Answer } from "./http";

const DAY_MS = 24 * 3_600_000;

export async function readOverview(env: Env, now = Date.now()): Promise<Answer> {
  const day = now - DAY_MS;
  const week = now - 7 * DAY_MS;
  const counts = await env.DB.prepare(
    `SELECT (SELECT COUNT(DISTINCT name_key) FROM entries) AS characters,
            (SELECT COUNT(*) FROM entries) AS entries,
            (SELECT COUNT(*) FROM accounts) AS accounts,
            (SELECT COUNT(*) FROM account_keys) AS pcs,
            (SELECT COUNT(*) FROM reports) AS reports,
            (SELECT COUNT(*) FROM removed_entries) AS removed,
            (SELECT COUNT(*) FROM blocks) AS blocks,
            (SELECT COUNT(*) FROM entries WHERE set_at >= ?1) AS bests_day,
            (SELECT COUNT(*) FROM entries WHERE set_at >= ?2) AS bests_week,
            (SELECT COUNT(*) FROM taken_tops WHERE taken_at >= ?1) AS tops_day,
            (SELECT COUNT(*) FROM taken_tops WHERE taken_at >= ?2) AS tops_week,
            (SELECT MAX(checked_at) FROM portraits) AS portraits_at`,
  )
    .bind(day, week)
    .first<Record<string, number | null>>();
  const pulls = await env.DB.prepare("SELECT source, pulled_at, tried_at FROM market_pulls").all<{
    source: string;
    pulled_at: number | null;
    tried_at: number;
  }>();
  const pull = (source: string) => {
    const p = pulls.results.find((r) => r.source === source);
    return p === undefined ? null : { pulledAt: p.pulled_at, triedAt: p.tried_at };
  };
  const c = counts!;
  return {
    status: 200,
    body: {
      characters: c.characters,
      entries: c.entries,
      accounts: c.accounts,
      pcs: c.pcs,
      reports: c.reports,
      removed: c.removed,
      blocks: c.blocks,
      newBests: { day: c.bests_day, week: c.bests_week },
      topsTaken: { day: c.tops_day, week: c.tops_week },
      portraits: { on: Boolean(env.CLASSICRANK_KEY), checkedAt: c.portraits_at },
      market: { on: Boolean(env.MM_API_KEY), prices: pull("prices"), listed: pull("listed") },
    },
  };
}
