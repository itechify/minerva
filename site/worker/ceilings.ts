// The ceilings table (osms-ceilings.json), which `probe ceilings` writes from OSMS's live set
// with the core's own arithmetic: each map's solo EXP/h ceiling, which the cap holds entries
// to (ADR 0015), and the levels of the middle half of its spawns, which Where to train
// matches maps to a Level Band by (ADR 0024).
import table from "./osms-ceilings.json";

/** A map's row in the ceilings table. */
export interface Ceiling {
  /** Its solo EXP/h ceiling. */
  ceiling: number;
  /** The lowest and highest levels of the middle half of its spawns. */
  mobs: [number, number];
}

/** Each map's row, by its 9-digit ID: the tests' made-up table, or the committed one. */
export function ceilingsOf(env: Env): Record<string, Ceiling> {
  // JSON's arrays read as number[], not as the pairs `probe ceilings` writes.
  return env.TEST_CEILINGS ?? (table as unknown as Record<string, Ceiling>);
}
