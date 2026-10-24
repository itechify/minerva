// The ceilings table (osms-ceilings.json), which `probe ceilings` writes from OSMS's live set
// with the core's own arithmetic: each map's solo EXP/h ceiling, which the cap holds entries
// to (ADR 0015), the levels of the middle half of its spawns, which Where to train matches
// maps to a Level Band by, and its Map Mark, which Where to train picks an area by
// (ADR 0024); and each Map Mark's area and picture.
import table from "./osms-ceilings.json";

/** A map's row in the ceilings table. */
export interface Ceiling {
  /** Its solo EXP/h ceiling. */
  ceiling: number;
  /** The lowest and highest levels of the middle half of its spawns. */
  mobs: [number, number];
  /** Its Map Mark, by OSMS's name for it; none for a map without one. */
  mark?: string;
}

/** A Map Mark: the area it stands for, as players know it, and its picture on OSMS. */
export interface MarkRow {
  name: string;
  picture: string;
}

/** The whole table: each map's row by its 9-digit ID, and each Map Mark by OSMS's name. */
export interface CeilingsTable {
  maps: Record<string, Ceiling>;
  marks: Record<string, MarkRow>;
}

/** The tests' made-up table, or the committed one. */
function tableOf(env: Env): CeilingsTable {
  // JSON's arrays read as number[], not as the pairs `probe ceilings` writes.
  return env.TEST_CEILINGS ?? (table as unknown as CeilingsTable);
}

/** Each map's row, by its 9-digit ID. */
export function ceilingsOf(env: Env): Record<string, Ceiling> {
  return tableOf(env).maps;
}

/** Each Map Mark, by OSMS's name for it. */
export function marksOf(env: Env): Record<string, MarkRow> {
  return tableOf(env).marks;
}
