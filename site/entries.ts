// A Map Leaderboards entry as the Worker (`site/worker`) checks and answers it and the
// pages (`site/leaderboards`, `site/character`) read it: what a Character's name and a map's
// ID may be, and the answers the pages read: a Level Band's, Where to train's, Latest's and a
// Character's. Plain data, so both can take it.

/** Whether `name` can be a Character's: 1 to 16 letters and digits. */
export function isName(name: string): boolean {
  return /^[A-Za-z0-9]{1,16}$/.test(name);
}

/** Whether `map` is a 9-digit map ID. */
export function isMap(map: string): boolean {
  return /^\d{9}$/.test(map);
}

/** A Character's portrait from ClassicRank (ADR 0028), as the Worker last found it there: its
 *  photo (a PNG on classicrank.com) and its ClassicRank profile, each `null` when ClassicRank
 *  has none. Both `null`, ClassicRank doesn't know the Character, or hides them. */
export interface Portrait {
  photo: string | null;
  profile: string | null;
}

/** A Character's portrait in an answer: `null` until the Worker has asked ClassicRank for it.
 *  Every answer that has a Character leaves it out while portraits are off (the Worker has no
 *  ClassicRank key), and the pages then show no portrait at all. */
export type PortraitAnswer = Portrait | null;

/** An entry in a Level Band, with its rank there. */
export interface Ranked {
  rank: number;
  name: string;
  job: string;
  level: number;
  expPerHour: number;
  /** When the Visit set it, in ms since the Unix epoch. */
  setAt: number;
  portrait?: PortraitAnswer;
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

/** A Map Mark: the emblem the game's minimap draws beside a map's title, for its area. */
export interface MapMark {
  id: string;
  name: string;
  picture: string;
}

/** A map as Where to train lists it for a Level Band. */
export interface TrainingMap {
  map: string;
  /** The levels of the middle half of its spawns, from the ceilings table; `null` for a
   *  map with an entry there that the table doesn't hold. */
  mobs: [number, number] | null;
  /** Its Map Mark, from the ceilings table: OSMS's name for it ("Rith"), the area it stands
   *  for as players know it ("Lith Harbor"), and its picture on OSMS; `null` for a map
   *  without one, or one the table doesn't hold. */
  mark: MapMark | null;
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

/** An entry as Latest lists it: where (with the map's Map Mark, `null` for a map without
 *  one), and whether it took the top spot from another Character, from whom (`null` once
 *  they've left the Map Leaderboards) and what they had. */
export interface LatestEntry extends Best {
  map: string;
  mark: MapMark | null;
  band: number;
  took: { from: string | null; fromExpPerHour: number } | null;
  portrait?: PortraitAnswer;
}

/** GET /api/latest: the newest entries, by when their Visit set them. */
export interface LatestAnswer {
  entries: LatestEntry[];
}

/** A Character's page (ADR 0027), from the site's root: `/character/?name=Egbert`. Every
 *  name the site shows links to it, and so does a new #1's post on Discord. */
export function characterLinkTo(name: string): string {
  return `/character/?${new URLSearchParams({ name })}`;
}

/** One of a Character's entries as their page shows it: where, its rank of how many in the
 *  band there, and the map's mob levels and Map Mark, as Where to train gives them. */
export interface CharacterEntry {
  map: string;
  band: number;
  rank: number;
  /** How many entries the band has on the map. */
  total: number;
  job: string;
  level: number;
  expPerHour: number;
  /** When the Visit set it, in ms since the Unix epoch. */
  setAt: number;
  /** The levels of the middle half of the map's spawns; `null` for a map the ceilings
   *  table doesn't hold. */
  mobs: [number, number] | null;
  mark: MapMark | null;
}

/** GET /api/characters/<name>: the Character's name as last uploaded, and every entry of
 *  theirs on the Map Leaderboards, each with its rank. A name with no entries, for whatever
 *  reason, is a 404. */
export interface CharacterAnswer {
  name: string;
  entries: CharacterEntry[];
  portrait?: PortraitAnswer;
}
