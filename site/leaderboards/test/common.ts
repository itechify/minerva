// What the Level Band tests build their answers from: a made-up band's entries and the API's
// answer holding them, on a made-up map, dated so the day is the same in any time zone.
import { levelsOf } from "../../level-bands.ts";
import type { BandAnswer, Ranked } from "../leaderboards.ts";

/** Noon UTC, so the day is the same in any time zone the tests run in. */
export const OCT_3 = Date.UTC(2026, 9, 3, 12);
export const NOW = new Date(Date.UTC(2026, 9, 14, 12));

/** An entry at `rank` by `name`, a Magician at level 23. */
export function ranked(rank: number, name: string, expPerHour = 100_000 - rank * 1_000, setAt = OCT_3): Ranked {
  return { rank, name, job: "Magician", level: 23, expPerHour, setAt };
}

/** The API's answer for the Level Band starting at `band`, on map 100000000. */
export function answer(entries: Ranked[], character: Ranked | null = null, total = entries.length, band = 20): BandAnswer {
  return { map: "100000000", band, levels: levelsOf(band), total, entries, character };
}
