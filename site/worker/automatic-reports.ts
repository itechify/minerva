// Automatic reports (ADR 0032): an entry far above a busy map's #1, reported to the maintainer
// by the Worker itself at upload, as a player would. It stays on the Map Leaderboards, and the
// maintainer deals with it as with any report.
import { whole } from "../../src/figures";
import { SHOWN } from "./blocks";
import { sqlParty } from "./kinds";

/** A map is busy enough to judge an entry by once it has this many entries of other
 *  Characters, of every band and both kinds. */
const BUSY_MAP = 8;

/** An entry is reported once its EXP/h is more than this many times the best of another
 *  Character's of its map, band and kind. */
const TIMES_THE_TOP = 2;

/** How many entries one read asks about at a time. Each takes three bound parameters, beside
 *  the Character's own, and a D1 statement takes at most 100. */
const ENTRIES_PER_READ = 30;

/** An entry an upload raised: new, or higher than its Character's before. */
export interface Raised {
  map: string;
  band: number;
  party: boolean;
  expPerHour: number;
}

/** An entry to report, and why. */
export interface Flagged extends Raised {
  reason: string;
}

/** Of the entries `nameKey`'s upload raised, those to report: each on a map with at least
 *  [`BUSY_MAP`] entries of other Characters, and more than [`TIMES_THE_TOP`] times the best of
 *  another Character's of its band and kind, as the public boards show them. */
export async function flaggedOf(env: Env, nameKey: string, raised: Raised[]): Promise<Flagged[]> {
  const flagged: Flagged[] = [];
  for (let from = 0; from < raised.length; from += ENTRIES_PER_READ) {
    const asked = raised.slice(from, from + ENTRIES_PER_READ);
    const values = asked.map((_, i) => `(?${3 * i + 2}, ?${3 * i + 3}, ?${3 * i + 4})`).join(", ");
    const rows = await env.DB.prepare(
      `WITH places (map_id, band, party) AS (VALUES ${values}),
         others AS (SELECT * FROM ${SHOWN} WHERE name_key != ?1 AND map_id IN (SELECT map_id FROM places)),
         counts AS (SELECT map_id, COUNT(*) AS entries FROM others GROUP BY map_id),
         ranked AS (
           SELECT o.map_id, o.band, o.party, o.exp_per_hour, c.name,
                  ROW_NUMBER() OVER (PARTITION BY o.map_id, o.band, o.party
                                     ORDER BY o.exp_per_hour DESC, o.set_at, o.name_key) AS rank
           FROM others o JOIN characters c USING (name_key))
       SELECT p.map_id, p.band, p.party, r.exp_per_hour, r.name
       FROM places p JOIN counts n ON n.map_id = p.map_id
       JOIN ranked r ON r.map_id = p.map_id AND r.band = p.band AND r.party = p.party AND r.rank = 1
       WHERE n.entries >= ${BUSY_MAP}`,
    )
      .bind(nameKey, ...asked.flatMap((e) => [e.map, e.band, sqlParty(e.party)]))
      .all<{ map_id: string; band: number; party: number; exp_per_hour: number; name: string }>();
    for (const top of rows.results) {
      const entry = asked.find((e) => e.map === top.map_id && e.band === top.band && sqlParty(e.party) === top.party)!;
      if (entry.expPerHour <= TIMES_THE_TOP * top.exp_per_hour) continue;
      const times = timesText(entry.expPerHour / top.exp_per_hour);
      flagged.push({ ...entry, reason: `${times}× the #1 it passed: ${top.name}'s ${whole(top.exp_per_hour)} EXP/h.` });
    }
  }
  return flagged;
}

/** How many times the #1 an entry is, to one decimal, rounded down, so it never reads more
 *  than it is; to two just above [`TIMES_THE_TOP`], where one would read as the line itself. */
function timesText(times: number): string {
  const tenths = Math.floor(times * 10) / 10;
  return tenths > TIMES_THE_TOP ? tenths.toFixed(1) : (Math.floor(times * 100) / 100).toFixed(2);
}
