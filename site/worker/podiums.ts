// The Podiums Pedestal on the Map Leaderboards page (ADR 0024, amended): the three Characters
// with the most podiums, places in a Level Band's top 3 on a map, counted as a Character's page
// counts them (ADR 0027): each kind of entry ranked on its own, so a Solo Best's place and a
// Party Best's each count.
import type { PodiumsAnswer } from "../entries";
import type { Answer } from "./http";
import { PORTRAIT_COLUMNS, PORTRAIT_JOIN, type PortraitColumns, portraitIn } from "./portraits";

/** GET /api/podiums: the three with the most podiums, most first; a tie goes to whoever
 *  holds more #1s, then more #2s, then whoever set their first podium entry first. */
export async function readPodiums(env: Env): Promise<Answer> {
  const rows = await env.DB.prepare(
    `WITH ranked AS (
       SELECT name_key, set_at,
              ROW_NUMBER() OVER (PARTITION BY map_id, band, party ORDER BY exp_per_hour DESC, set_at, name_key) AS rank
       FROM entries),
     podiums AS (
       SELECT name_key, COUNT(*) AS podiums, SUM(rank = 1) AS firsts, SUM(rank = 2) AS seconds, MIN(set_at) AS first_set
       FROM ranked WHERE rank <= 3 GROUP BY name_key)
     SELECT c.name, d.podiums, ${PORTRAIT_COLUMNS} FROM podiums d JOIN characters c USING (name_key) ${PORTRAIT_JOIN}
     ORDER BY d.podiums DESC, d.firsts DESC, d.seconds DESC, d.first_set, d.name_key LIMIT 3`,
  ).all<PortraitColumns & { name: string; podiums: number }>();
  const answer: PodiumsAnswer = {
    characters: rows.results.map((r) => ({ name: r.name, podiums: r.podiums, ...portraitIn(env, r) })),
  };
  return { status: 200, body: answer };
}
