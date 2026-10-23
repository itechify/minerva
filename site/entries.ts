// A Map Leaderboards entry as the Worker (`site/worker`) checks and answers it and the
// pages (`site/leaderboards`) read it: what a Character's name and a map's ID may be, and a
// Level Band's answer. Plain data, so both can take it.

/** Whether `name` can be a Character's: 1 to 16 letters and digits. */
export function isName(name: string): boolean {
  return /^[A-Za-z0-9]{1,16}$/.test(name);
}

/** Whether `map` is a 9-digit map ID. */
export function isMap(map: string): boolean {
  return /^\d{9}$/.test(map);
}

/** An entry in a Level Band, with its rank there. */
export interface Ranked {
  rank: number;
  name: string;
  job: string;
  level: number;
  expPerHour: number;
  /** When the Visit set it, in ms since the Unix epoch. */
  setAt: number;
}

/** GET /api/maps/<map>/bands/<band>?name=<name>: the band's top 50, and the named
 *  Character's entry with its rank, wherever it is (`null` with none there). */
export interface BandAnswer {
  map: string;
  band: number;
  levels: [number, number];
  total: number;
  entries: Ranked[];
  character: Ranked | null;
}
