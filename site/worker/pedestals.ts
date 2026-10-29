// The Map Leaderboards page's two Pedestals (ADR 0024, amended): the three Characters with the
// most podiums, places in a Level Band's top 3 on a map, counted as a Character's page counts
// them (ADR 0027): each kind of entry ranked on its own, so a Solo Best's place and a Party
// Best's each count; and the three with the highest EXP/h on any map at any level, each by
// their best entry. A Character a Block reaches stands on neither, nor counts in anyone's
// podiums (ADR 0031).
import type { ExpStep, PedestalsAnswer, PodiumsStep } from "../entries";
import { SHOWN } from "./blocks";
import { markOf } from "./ceilings";
import type { Answer } from "./http";
import { isParty } from "./kinds";
import { PORTRAIT_COLUMNS, PORTRAIT_JOIN, type PortraitColumns, portraitIn } from "./portraits";

/** GET /api/pedestals: both Pedestals, read together, so the page switches between them
 *  without asking again. */
export async function readPedestals(env: Env): Promise<Answer> {
  const [podiums, fastest] = await env.DB.batch<PortraitColumns & Record<string, string | number>>([
    // The most podiums; a tie goes to whoever holds more #1s, then more #2s, then whoever set
    // their first podium entry first.
    env.DB.prepare(
      `WITH ranked AS (
         SELECT name_key, set_at,
                ROW_NUMBER() OVER (PARTITION BY map_id, band, party ORDER BY exp_per_hour DESC, set_at, name_key) AS rank
         FROM ${SHOWN}),
       podiums AS (
         SELECT name_key, COUNT(*) AS podiums, SUM(rank = 1) AS firsts, SUM(rank = 2) AS seconds, MIN(set_at) AS first_set
         FROM ranked WHERE rank <= 3 GROUP BY name_key)
       SELECT c.name, d.podiums, ${PORTRAIT_COLUMNS} FROM podiums d JOIN characters c USING (name_key) ${PORTRAIT_JOIN}
       ORDER BY d.podiums DESC, d.firsts DESC, d.seconds DESC, d.first_set, d.name_key LIMIT 3`,
    ),
    // Each Character's best entry, the earlier on a tie, then the three highest of those; a tie
    // between them goes to whoever set it first.
    env.DB.prepare(
      `WITH best AS (
         SELECT e.*, ROW_NUMBER() OVER (PARTITION BY e.name_key ORDER BY e.exp_per_hour DESC, e.set_at, e.map_id, e.band, e.party) AS n
         FROM ${SHOWN} e)
       SELECT c.name, b.job, b.level, b.exp_per_hour, b.set_at, b.party, b.map_id, b.band, ${PORTRAIT_COLUMNS}
       FROM best b JOIN characters c USING (name_key) ${PORTRAIT_JOIN}
       WHERE b.n = 1 ORDER BY b.exp_per_hour DESC, b.set_at, b.name_key LIMIT 3`,
    ),
  ]);
  const answer: PedestalsAnswer = {
    podiums: podiums.results.map((r): PodiumsStep => ({ name: String(r.name), podiums: Number(r.podiums), ...portraitIn(env, r) })),
    expPerHour: fastest.results.map(
      (r): ExpStep => ({
        name: String(r.name),
        job: String(r.job),
        level: Number(r.level),
        expPerHour: Number(r.exp_per_hour),
        setAt: Number(r.set_at),
        party: isParty(Number(r.party)),
        map: String(r.map_id),
        mark: markOf(env, String(r.map_id)),
        band: Number(r.band),
        ...portraitIn(env, r),
      }),
    ),
  };
  return { status: 200, body: answer };
}
