// A Map Leaderboards entry as the Worker (`site/worker`) checks and answers it and the
// pages (`site/leaderboards`) read it: what a Character's name and a map's ID may be, and
// the answers the pages read: a Level Band's, Where to train's and Latest's. Plain data, so
// both can take it.

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

/** An entry as Where to train and Latest show it, without a rank. */
export interface Best {
  name: string;
  job: string;
  level: number;
  expPerHour: number;
  /** When the Visit set it, in ms since the Unix epoch. */
  setAt: number;
}

/** GET /api/bands: the Level Bands Where to train has maps for, lowest first, each with how
 *  many maps it lists and how many entries they have there. */
export interface BandsAnswer {
  bands: { band: number; levels: [number, number]; maps: number; entries: number }[];
}

/** A map as Where to train lists it for a Level Band. */
export interface TrainingMap {
  map: string;
  /** Its solo EXP/h ceiling and the levels of the middle half of its spawns, from the
   *  ceilings table; `null` for a map with an entry there that the table doesn't hold. */
  ceiling: number | null;
  mobs: [number, number] | null;
  /** How many entries it has in the band, and the band's #1 there (`null` with none). */
  entries: number;
  best: Best | null;
}

/** GET /api/bands/<band>: the maps for the Level Band, by ID: each map whose middle half of
 *  spawns is at its levels, and each map with an entry in it. */
export interface TrainingAnswer {
  band: number;
  levels: [number, number];
  maps: TrainingMap[];
}

/** An entry as Latest lists it: where, and whether it took the top spot from another
 *  Character, from whom (`null` once they've left the Map Leaderboards) and what they had. */
export interface LatestEntry extends Best {
  map: string;
  band: number;
  took: { from: string | null; fromExpPerHour: number } | null;
}

/** GET /api/latest: the newest entries, by when their Visit set them. */
export interface LatestAnswer {
  entries: LatestEntry[];
}
