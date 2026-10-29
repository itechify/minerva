// Where to train (ADR 0024): the maps for each Level Band, each with its mob levels, its Map
// Mark and the band's #1 there, and the band's EXP/h Pedestal (ADR 0024, amended). A map is for
// a band when the middle half of its spawns is at the band's levels (the ceilings table), or
// when it has an entry in the band.
import type { Best, Kind, PedestalEntry, TrainingMap, TrainingPedestal } from "../entries";
import { bandFrom, bandOf, levelsOf } from "../level-bands";
import { type Ceiling, ceilingsOf, markOf } from "./ceilings";
import { type Answer, badRequest } from "./http";
import { boardOf, isParty } from "./kinds";
import { PORTRAIT_COLUMNS, PORTRAIT_JOIN, type PortraitColumns, portraitIn } from "./portraits";

/** How many Characters a Pedestal stands. */
const PEDESTAL = 3;

/** Whether the middle half of a map's spawns is at any of the band's levels. */
function isFor(row: Ceiling, band: number): boolean {
  const [first, last] = levelsOf(band);
  return row.mobs[0] <= last && row.mobs[1] >= first;
}

/** Every Level Band the middle half of a map's spawns is at, lowest first. */
function bandsOf(row: Ceiling): number[] {
  const bands: number[] = [];
  for (let band = bandOf(row.mobs[0]); band <= row.mobs[1]; band = band === 1 ? 10 : band + 10) bands.push(band);
  return bands;
}

/** Every Level Band with maps, lowest first: how many maps each lists, and how many entries
 *  those have there. The page leaves out an Event or Dev map with no entries, which this
 *  counts; the two agree while the ceilings table holds none, as it doesn't. */
export async function readBands(env: Env): Promise<Answer> {
  const maps = new Map<number, Set<string>>();
  const add = (band: number, map: string) => {
    if (!maps.has(band)) maps.set(band, new Set());
    maps.get(band)!.add(map);
  };
  for (const [map, row] of Object.entries(ceilingsOf(env))) for (const band of bandsOf(row)) add(band, map);
  const rows = await env.DB.prepare(`SELECT band, map_id, COUNT(*) AS entries FROM ${boardOf(null)} GROUP BY band, map_id`)
    .all<{ band: number; map_id: string; entries: number }>();
  const entries = new Map<number, number>();
  for (const r of rows.results) {
    add(r.band, r.map_id);
    entries.set(r.band, (entries.get(r.band) ?? 0) + r.entries);
  }
  const bands = [...maps]
    .sort(([a], [b]) => a - b)
    .map(([band, at]) => ({ band, levels: levelsOf(band), maps: at.size, entries: entries.get(band) ?? 0 }));
  return { status: 200, body: { bands } };
}

/** The maps for the Level Band starting at level `first`, by ID, with the #1 and the entries
 *  there of one `kind` or, with none, of the board that ranks each Character by their higher
 *  entry; and the band's EXP/h Pedestal, of the same entries. */
export async function readTraining(env: Env, first: string, kind: Kind | null): Promise<Answer> {
  const band = bandFrom(first);
  if (band === null) return badRequest("a Level Band is its first level: 1, 10, 20...");
  const rows = await env.DB.prepare(
    `WITH ranked AS (
       SELECT e.name_key, e.map_id, e.job, e.level, e.exp_per_hour, e.set_at, e.party,
              ROW_NUMBER() OVER (PARTITION BY e.map_id ORDER BY e.exp_per_hour DESC, e.set_at, e.name_key) AS rank,
              COUNT(*) OVER (PARTITION BY e.map_id) AS entries
       FROM ${boardOf(kind)} e
       WHERE e.band = ?)
     SELECT r.*, c.name, ${PORTRAIT_COLUMNS} FROM ranked r JOIN characters c USING (name_key) ${PORTRAIT_JOIN}
     WHERE r.rank <= ${PEDESTAL} ORDER BY r.exp_per_hour DESC, r.set_at, r.name_key`,
  )
    .bind(band)
    .all<PortraitColumns & {
      map_id: string;
      name: string;
      job: string;
      level: number;
      exp_per_hour: number;
      set_at: number;
      party: number;
      rank: number;
      entries: number;
    }>();
  const tops = new Map(rows.results.filter((r) => r.rank === 1).map((r) => [r.map_id, r]));
  const table = ceilingsOf(env);
  const ids = new Set([...Object.keys(table).filter((map) => isFor(table[map], band)), ...tops.keys()]);
  const maps: TrainingMap[] = [...ids].sort().map((map) => {
    const top = tops.get(map);
    const best: Best | null =
      top === undefined
        ? null
        : { name: top.name, job: top.job, level: top.level, expPerHour: top.exp_per_hour, setAt: top.set_at, party: isParty(top.party) };
    return { map, mobs: table[map]?.mobs ?? null, mark: markOf(env, map), entries: top?.entries ?? 0, best };
  });
  // A Character's best among some maps is in the top 3 of its own map, or 3 others' bests
  // there beat it, and it isn't on the Pedestal; so each map's top 3, highest first, is
  // enough to stand it.
  const pedestal: TrainingPedestal = { all: [], areas: {} };
  const stood = new Map<PedestalEntry[], Set<string>>();
  const stand = (on: PedestalEntry[], r: (typeof rows.results)[number]) => {
    const names = stood.get(on) ?? stood.set(on, new Set()).get(on)!;
    if (on.length === PEDESTAL || names.has(r.name)) return;
    names.add(r.name);
    on.push({
      name: r.name,
      job: r.job,
      level: r.level,
      expPerHour: r.exp_per_hour,
      setAt: r.set_at,
      party: isParty(r.party),
      map: r.map_id,
      ...portraitIn(env, r),
    });
  };
  for (const r of rows.results) {
    stand(pedestal.all, r);
    const mark = markOf(env, r.map_id);
    if (mark !== null) stand((pedestal.areas[mark.id] ??= []), r);
  }
  return { status: 200, body: { band, levels: levelsOf(band), maps, pedestal } };
}
