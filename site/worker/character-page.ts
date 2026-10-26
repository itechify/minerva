// A Character's page (ADR 0027): every entry of theirs on the Map Leaderboards, each with its
// rank in its band, and the map's mob levels and Map Mark as Where to train gives them. The
// page works out the rest (its header and the #1s and podiums) from these.
import { type CharacterAnswer, type CharacterEntry, isName } from "../entries";
import { ceilingsOf, markOf } from "./ceilings";
import { type Answer, badRequest } from "./http";

/** What a name with no entries reads as, whatever the reason: never uploaded, opted out, or
 *  every entry taken down. The page tells none of them apart, so neither does this. */
const NO_ENTRIES: Answer = { status: 404, body: { error: "not-found" } };

export async function readCharacterPage(env: Env, name: string): Promise<Answer> {
  if (!isName(name)) return badRequest("a Character's name is 1 to 16 letters and digits");
  const rows = await env.DB.prepare(
    `WITH places AS (SELECT map_id, band FROM entries WHERE name_key = ?1),
       ranked AS (
         SELECT e.name_key, e.map_id, e.band, e.job, e.level, e.exp_per_hour, e.set_at,
                ROW_NUMBER() OVER (PARTITION BY e.map_id, e.band
                                   ORDER BY e.exp_per_hour DESC, e.set_at, e.name_key) AS rank,
                COUNT(*) OVER (PARTITION BY e.map_id, e.band) AS total
         FROM entries e JOIN places p ON p.map_id = e.map_id AND p.band = e.band)
     SELECT r.*, c.name FROM ranked r JOIN characters c USING (name_key)
     WHERE r.name_key = ?1 ORDER BY r.band DESC, r.exp_per_hour DESC, r.map_id`,
  )
    .bind(name.toLowerCase())
    .all<{
      name: string;
      map_id: string;
      band: number;
      job: string;
      level: number;
      exp_per_hour: number;
      set_at: number;
      rank: number;
      total: number;
    }>();
  if (rows.results.length === 0) return NO_ENTRIES;
  const table = ceilingsOf(env);
  const entries: CharacterEntry[] = rows.results.map((r) => ({
    map: r.map_id,
    band: r.band,
    rank: r.rank,
    total: r.total,
    job: r.job,
    level: r.level,
    expPerHour: r.exp_per_hour,
    setAt: r.set_at,
    mobs: table[r.map_id]?.mobs ?? null,
    mark: markOf(env, r.map_id),
  }));
  const answer: CharacterAnswer = { name: rows.results[0].name, entries };
  return { status: 200, body: answer };
}
