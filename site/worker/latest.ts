// Latest (ADR 0024): the newest entries on every map, by when their Visit set them, each
// with its map's Map Mark, and marked when it took a top spot from another Character
// (ADR 0022).
import type { LatestEntry } from "../entries";
import { markOf } from "./ceilings";
import type { Answer } from "./http";
import { PORTRAIT_COLUMNS, PORTRAIT_JOIN, type PortraitColumns, portraitIn } from "./portraits";

/** How many entries Latest lists. */
const LATEST = 10;

export async function readLatest(env: Env): Promise<Answer> {
  const rows = await env.DB.prepare(
    `SELECT c.name, e.job, e.level, e.exp_per_hour, e.set_at, e.map_id, e.band,
            t.name_key IS NOT NULL AS took, f.name AS from_name, t.from_exp_per_hour, ${PORTRAIT_COLUMNS}
     FROM entries e JOIN characters c USING (name_key) ${PORTRAIT_JOIN}
     LEFT JOIN taken_tops t ON t.name_key = e.name_key AND t.map_id = e.map_id AND t.band = e.band
                           AND t.exp_per_hour = e.exp_per_hour
     LEFT JOIN characters f ON f.name_key = t.from_name_key
     ORDER BY e.set_at DESC, e.name_key, e.map_id, e.band LIMIT ?`,
  )
    .bind(LATEST)
    .all<PortraitColumns & {
      name: string;
      job: string;
      level: number;
      exp_per_hour: number;
      set_at: number;
      map_id: string;
      band: number;
      took: number;
      from_name: string | null;
      from_exp_per_hour: number | null;
    }>();
  const entries: LatestEntry[] = rows.results.map((r) => ({
    name: r.name,
    job: r.job,
    level: r.level,
    expPerHour: r.exp_per_hour,
    setAt: r.set_at,
    map: r.map_id,
    mark: markOf(env, r.map_id),
    band: r.band,
    took: r.took ? { from: r.from_name, fromExpPerHour: r.from_exp_per_hour! } : null,
    ...portraitIn(env, r),
  }));
  return { status: 200, body: { entries } };
}
