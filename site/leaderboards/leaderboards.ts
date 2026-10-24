// The Map Leaderboards pages (site/leaderboards), worked out as plain data: where a link
// opens, the maps a search finds, and what a page shows from the API's answers (ADR 0015):
// a map's Level Bands, and on the Map Leaderboards page Where to train and Latest
// (ADR 0024). The pages' scripts (search.ts, training.ts, latest.ts, map.ts) only fetch and
// draw what this gives them.

import { dayText } from "../../src/dates.ts";
import { levels, rate, sharePercent, short, whole } from "../../src/figures.ts";
import { type BandAnswer, type BandsAnswer, isMap, isName, type LatestAnswer, type Ranked, type TrainingAnswer } from "../entries.ts";
import { bandOf, levelsOf } from "../level-bands.ts";
import { type MapNames, type MapPlace, mapNames, placeOf } from "../map-names.ts";

// A map's name is worked out in one place for the whole site (map-names.ts), so a page still
// takes everything it shows from this module.
export { mapNames, placeOf };
export type { MapNameRow, MapNames, MapPlace } from "../map-names.ts";
export type { BandAnswer, BandsAnswer, LatestAnswer, Ranked, TrainingAnswer } from "../entries.ts";

/** A map's page as its address names it: `/leaderboards/map/?map=<ID>&band=<first level>&name=<name>`.
 *  Any part may be missing, or dropped for not being one. */
export interface Link {
  /** The map's 9-digit ID ("010001010"). */
  map: string | null;
  /** The Level Band, by its first level: 1 (1-9), 10 (10-19), 20 (20-29)... */
  band: number | null;
  /** The Character whose entry the page shows: 1 to 16 letters and digits. */
  name: string | null;
}

/** The highest level a Character can have. */
const TOP_LEVEL = 200;

/** The map a link names, as its 9-digit ID: the map names fixture writes IDs as numbers,
 *  without their leading zeros, so a shorter one is padded. */
function mapIn(value: string | null): string | null {
  if (value === null || value === "") return null;
  const id = value.padStart(9, "0");
  return isMap(id) ? id : null;
}

/** The Level Band a link names, or the one holding the level it names. */
function bandIn(value: string | null): number | null {
  if (value === null || !/^\d{1,3}$/.test(value)) return null;
  const level = Number(value);
  return level >= 1 && level <= TOP_LEVEL ? bandOf(level) : null;
}

function nameIn(value: string | null): string | null {
  return value !== null && isName(value) ? value : null;
}

/** Where a map page's address (its `location.search`) opens. */
export function readLink(search: string): Link {
  const query = new URLSearchParams(search);
  return { map: mapIn(query.get("map")), band: bandIn(query.get("band")), name: nameIn(query.get("name")) };
}

/** The regions of maps the player can't normally enter, which search leaves out, as the
 *  panel's does (`MapEntry::is_left_out`). */
const LEFT_OUT_REGIONS = ["Event", "Dev"];

/** How well a name holds a query: it starts with it, a word in it does, or it holds it
 *  elsewhere. Lower is better. */
function fit(name: string, query: string): number | null {
  const lower = name.toLowerCase();
  let best: number | null = null;
  for (let at = lower.indexOf(query); at !== -1; at = lower.indexOf(query, at + 1)) {
    const fit = at === 0 ? 0 : /[\p{L}\p{N}]/u.test(lower[at - 1]) ? 2 : 1;
    best = best === null ? fit : Math.min(best, fit);
  }
  return best;
}

/** Order as the core's strings order: by code unit, not the locale's. */
function byText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** How many entries each map has on its Map Leaderboard, by 9-digit ID; a map it doesn't
 *  hold has none. */
export type EntryCounts = Map<string, number>;

/** The entries each map has, from GET /api/maps. */
export function entryCounts(answer: MapsAnswer): EntryCounts {
  return new Map(answer.maps.map((m) => [m.map, m.entries]));
}

/** A map a search found, with how many entries it has ("12 entries", "No entries yet"), or
 *  `null` before the API has said (or when it can't be reached). */
export interface FoundMap extends MapPlace {
  entries: string | null;
}

/** The maps whose Map Name holds `query`, whatever the case, as the panel's search finds
 *  them (`search.rs`): those that start with it, then those with a word that starts with it,
 *  then the rest, each by name; Event and Dev maps aside. With `counts`, the maps with
 *  entries come first, so the first map, the one Enter opens, isn't an empty one when
 *  another isn't. The first `shown`, and how many more. */
export function searchMaps(
  names: MapNames,
  query: string,
  { shown = 10, counts = null }: { shown?: number; counts?: EntryCounts | null } = {},
): { maps: FoundMap[]; more: number } {
  const lower = query.trim().toLowerCase();
  if (lower === "") return { maps: [], more: 0 };
  const found: { fit: number; empty: boolean; map: MapPlace }[] = [];
  for (const map of names.values()) {
    if (LEFT_OUT_REGIONS.includes(map.region)) continue;
    const f = fit(map.name, lower);
    if (f !== null) found.push({ fit: f, empty: !counts?.get(map.id), map });
  }
  found.sort(
    (a, b) =>
      Number(a.empty) - Number(b.empty) ||
      a.fit - b.fit ||
      byText(a.map.name, b.map.name) ||
      byText(a.map.street, b.map.street) ||
      byText(a.map.id, b.map.id),
  );
  const entries = (id: string) => {
    if (counts === null) return null;
    const count = counts.get(id) ?? 0;
    return count === 0 ? "No entries yet" : entriesCounted(count);
  };
  return {
    maps: found.slice(0, shown).map(({ map }) => ({ ...map, entries: entries(map.id) })),
    more: Math.max(0, found.length - shown),
  };
}

/** What the search says under its field, for `query` and what it found: how many maps, and
 *  the one Enter opens (the list marks it, and this says it in words). */
export function searchSaid(query: string, found: { maps: MapPlace[]; more: number }): string {
  if (query.trim() === "") return "Type part of a map's name as the game writes it, like Ant Tunnel.";
  if (found.maps.length === 0) return `No map's name holds “${query.trim()}”.`;
  const shown = counted(found.maps.length, "map");
  const all = found.more === 0 ? shown : `${shown}, and ${whole(found.more)} more: type more of the name`;
  return `${all}. Enter opens ${found.maps[0].name}.`;
}

/** `count` of `thing`, plural when it isn't 1: "1 map", "12 maps". */
function counted(count: number, thing: string): string {
  return `${whole(count)} ${thing}${count === 1 ? "" : "s"}`;
}

/** How many entries, the one word the pages count a Map Leaderboard in: "1 entry", "12 entries". */
function entriesCounted(count: number): string {
  return `${whole(count)} ${count === 1 ? "entry" : "entries"}`;
}

/** GET /api/maps: every map with entries, the most first. */
export interface MapsAnswer {
  maps: { map: string; characters: number; entries: number }[];
}

/** What a page says in place of what it can't show, and where to go from there. */
export interface Said {
  text: string;
  link?: { text: string; href: string };
}

/** Where an empty Map Leaderboard sends the player: putting their own bests there, which
 *  only Minerva does. */
const PUT_YOURS = { text: "How to put yours here from Minerva", href: "/faq/#leaderboard-appear" };

/** A Level Band's levels as the panel writes mob levels: "Lv 20–29". */
function bandLevels(band: number): string {
  const [lowest, highest] = levelsOf(band);
  return levels({ lowest, highest });
}

/** GET /api/maps/<map>: the map's Level Bands with entries, lowest first. */
export interface MapAnswer {
  map: string;
  bands: { band: number; levels: [number, number]; entries: number }[];
}

/** A Level Band as a map's page offers it. */
export interface BandChoice {
  band: number;
  levels: string;
  entries: string;
  href: string;
  /** Whether it's the band the page shows. */
  open: boolean;
}

/** A map's page: the map, its Level Bands, and the one it opens at (`null` with none to
 *  show), or why it has none. */
export interface MapPage {
  place: MapPlace;
  bands: BandChoice[];
  open: number | null;
  empty: Said | null;
}

/** A map's page as `link` opens it, from its bands as the API gives them. It opens at the
 *  link's band, even with no entries there (Minerva links a Character's own band, and the
 *  page says their entry has gone), or else at the lowest band with entries. */
export function mapPage(link: Link, names: MapNames, answer: MapAnswer): MapPage {
  const open = link.band ?? answer.bands[0]?.band ?? null;
  const counts = new Map(answer.bands.map((b) => [b.band, b.entries]));
  if (open !== null && !counts.has(open)) counts.set(open, 0);
  const bands = [...counts]
    .sort(([a], [b]) => a - b)
    .map(([band, entries]) => ({
      band,
      levels: bandLevels(band),
      entries: entries === 0 ? "No entries" : entriesCounted(entries),
      href: linkTo({ map: answer.map, band, name: link.name }),
      open: band === open,
    }));
  return {
    place: placeOf(names, answer.map),
    bands,
    open,
    empty: open === null ? { text: "No one has a best on this Map Leaderboard yet.", link: PUT_YOURS } : null,
  };
}

/** An entry as the API ranks it: one Character's best Visit on the map in the band. */
/** An entry as a band's table writes it. */
export interface Row {
  rank: string;
  name: string;
  job: string;
  level: string;
  expPerHour: string;
  date: string;
  /** Whether it's the linked Character's. */
  you: boolean;
  /** How far along its lane the EXP/h runs, from 0 to 1: the band's #1 runs it full. */
  lane: number;
}

/** A band's table: its rows, the linked Character's under them when it's outside them (after
 *  how many entries are between), a line about the linked Character over the table and
 *  another at their row, what the table shows of the band, and why there are no rows when
 *  there are none. */
export interface BandTable {
  rows: Row[];
  /** Between the top 50 and the linked Character's row under them: "6 more entries". */
  gap: string | null;
  yours: Row | null;
  note: string | null;
  /** "Top 50 of 63 at Lv 20–29", so the table doesn't stop at 50 silently. */
  caption: string | null;
  empty: Said | null;
  /** The lanes' ruler: 0, then each round step up to #1's EXP/h; none with no rows. */
  ruler: RulerStep[];
}

/** A step on a band's ruler: how far along the lanes it falls, and its figure ("50k"). */
export interface RulerStep {
  at: number;
  text: string;
}

/** The ruler for lanes running from 0 to `top`, at round steps as the Personal Bests
 *  window rules its lanes: 0, then every step up to `top`, the step the smallest of 1, 2,
 *  2.5 and 5 to a power of ten that leaves at most five under `top`. Each round step is at
 *  most twice the one before, so that leaves three to five (fewer only under a `top` of 3). */
function rulerTo(top: number): RulerStep[] {
  let step = 1;
  for (let power = Math.max(1, 10 ** Math.floor(Math.log10(top / 5))); ; power *= 10) {
    const fits = [1, 2, 2.5, 5].map((n) => n * power).find((n) => Math.floor(top / n) <= 5);
    if (fits !== undefined) {
      step = fits;
      break;
    }
  }
  const steps = [0];
  for (let n = step; n <= top; n += step) steps.push(n);
  return steps.map((n) => ({ at: n / top, text: short(n) }));
}

/** A band's table from the API's answer, for the Character `name` the link names (if
 *  any), with dates written as of `now`. */
export function bandTable(answer: BandAnswer, name: string | null, now: Date): BandTable {
  const yours = answer.character;
  const top = Math.max(1, answer.entries[0]?.expPerHour ?? 1);
  const row = (entry: Ranked): Row => {
    const set = new Date(entry.setAt);
    return {
      rank: String(entry.rank),
      name: entry.name,
      job: entry.job,
      level: String(entry.level),
      expPerHour: rate(entry.expPerHour),
      date: dayText(set, set.getFullYear() !== now.getFullYear()),
      you: entry.rank === yours?.rank,
      lane: Math.min(1, entry.expPerHour / top),
    };
  };
  const rows = answer.entries.map(row);
  const at = bandLevels(answer.band);
  const total = whole(answer.total);
  let note: string | null = null;
  if (yours !== null) note = `${yours.name} is #${yours.rank} of ${total} at ${at}.`;
  else if (name !== null) note = `${name} has no entry here at ${at} any more.`;
  const below = yours !== null && !rows.some((r) => r.you);
  const between = below ? yours.rank - rows.length - 1 : 0;
  const caption =
    rows.length === 0 ? null : answer.total > rows.length ? `Top ${rows.length} of ${total} at ${at}` : `${entriesCounted(rows.length)} at ${at}`;
  return {
    rows,
    gap: between > 0 ? `${whole(between)} more ${between === 1 ? "entry" : "entries"}` : null,
    yours: below ? row(yours) : null,
    note,
    caption,
    empty: rows.length === 0 ? { text: `No one at ${at} has put a best on this map yet.`, link: PUT_YOURS } : null,
    ruler: rows.length === 0 ? [] : rulerTo(top),
  };
}

/** The rows that slide to their new rank when the page switches from one Level Band's
 *  `before` to another's `after`: the Characters with an entry in both (names are one
 *  Character's whatever their case), each by its name as either band writes it, with the
 *  `view-transition-name` both its rows take. Each is its name in lower case, anything
 *  but a letter or digit written as its code (`_2d_`), after "lb-row-", so no two meet. */
export function morphNames(before: Row[], after: Row[]): Map<string, string> {
  const key = (name: string) => name.toLowerCase();
  const was = new Set(before.map((r) => key(r.name)));
  const both = new Set(after.map((r) => key(r.name)).filter((k) => was.has(k)));
  const names = new Map<string, string>();
  for (const row of [...before, ...after]) {
    const k = key(row.name);
    if (!both.has(k)) continue;
    const safe = [...k].map((c) => (/[a-z0-9]/.test(c) ? c : `_${c.codePointAt(0)!.toString(16)}_`)).join("");
    names.set(row.name, `lb-row-${safe}`);
  }
  return names;
}

/** What the page says once POST /api/reports answers with `status` (`null` when it
 *  couldn't be reached), and whether the report was taken. */
export function reportOutcome(status: number | null): { taken: boolean; said: string } {
  switch (status) {
    case 201:
      return { taken: true, said: "Reported. Thank you: Minerva's maker will look at it." };
    case 404:
      return { taken: false, said: "This entry isn't on the Map Leaderboard any more." };
    case 429:
      return { taken: false, said: "That's a lot of reports from you. Try again in a few minutes." };
    default:
      return { taken: false, said: "The report didn't go through. Try again later." };
  }
}

/** The address of a map's page, from the site's root. */
export function linkTo(link: Link): string {
  const query = new URLSearchParams();
  if (link.map !== null) query.set("map", link.map);
  if (link.band !== null) query.set("band", String(link.band));
  if (link.name !== null) query.set("name", link.name);
  return `/leaderboards/map/?${query}`;
}

/** A map's picture in OSMS's live manifest. Only a content hash can name a picture;
 *  absent or malformed entries have no picture, and never supply an arbitrary URL. */
export function mapPicture(manifest: unknown, map: string): string | null {
  if (manifest === null || typeof manifest !== "object" || Array.isArray(manifest)) return null;
  const hash = (manifest as Record<string, unknown>)[map];
  return typeof hash === "string" && /^[a-f0-9]{32}$/i.test(hash) ? `https://osmsdataexplorer.com/data/maps/${hash}.webp` : null;
}

/** How Where to train orders a band's maps: by players' bests, or by the maps' solo ceilings. */
export type TrainingSort = "best" | "ceiling";

/** The Map Leaderboards page as its address names it:
 *  `/leaderboards/?band=<first level>&sort=ceiling&street=<street>`. Any part may be
 *  missing, or dropped for not being one: with no band, Where to train asks for one; sorted
 *  by players' bests unless it says `ceiling`; on every street unless it names one. */
export interface TrainingLink {
  band: number | null;
  sort: TrainingSort;
  street: string | null;
}

/** Where the Map Leaderboards page's address (its `location.search`) opens. */
export function readTrainingLink(search: string): TrainingLink {
  const query = new URLSearchParams(search);
  const street = query.get("street")?.trim() || null;
  return { band: bandIn(query.get("band")), sort: query.get("sort") === "ceiling" ? "ceiling" : "best", street };
}

/** The address of the Map Leaderboards page, from the site's root. */
export function trainingLinkTo(link: TrainingLink): string {
  const query = new URLSearchParams();
  if (link.band !== null) query.set("band", String(link.band));
  if (link.sort !== "best") query.set("sort", link.sort);
  if (link.street !== null) query.set("street", link.street);
  const search = String(query);
  return search === "" ? "/leaderboards/" : `/leaderboards/?${search}`;
}

/** A Level Band as Where to train offers it. */
export interface TrainingBand {
  band: number;
  levels: string;
  /** How many maps it lists: "24 maps". */
  maps: string;
  href: string;
  /** Whether it's the band the page shows. */
  open: boolean;
}

/** Where to train's Level Bands, from GET /api/bands, each keeping the link's sort and
 *  street. */
export function trainingBands(answer: BandsAnswer, link: TrainingLink): TrainingBand[] {
  return answer.bands.map(({ band, maps }) => ({
    band,
    levels: bandLevels(band),
    maps: counted(maps, "map"),
    href: trainingLinkTo({ ...link, band }),
    open: band === link.band,
  }));
}

/** A map as Where to train's table writes it. */
export interface TrainingRow {
  name: string;
  street: string;
  /** Its Map Leaderboard, at the band. */
  href: string;
  /** The levels of the middle half of its spawns: "Lv 22–27"; `null` for a map the
   *  ceilings table doesn't hold. */
  mobs: string | null;
  /** Its solo EXP/h ceiling: "300,000"; `null` for a map the table doesn't hold. */
  ceiling: string | null;
  /** The band's #1 there: its EXP/h, who set it ("Bea · Magician Lv 25") and when; `null`
   *  with no entries. */
  best: { expPerHour: string; who: string; date: string } | null;
  /** The #1 as a share of the ceiling: "83%"; `null` without both. */
  share: string | null;
  /** Whether the #1 is above the ceiling: more than the map gives solo, so a party, an EXP
   *  event or a buff. */
  over: boolean;
  entries: string;
}

/** Where to train's table for a band: its rows, the streets they're on (for the filter), the
 *  street it shows (`null`, all of them), what it says of itself, and why there are no rows
 *  when there are none. */
export interface TrainingTable {
  rows: TrainingRow[];
  streets: string[];
  street: string | null;
  caption: string | null;
  empty: Said | null;
}

/** Where an over-ceiling #1's mark leads: what more than a map gives solo means. */
export const OVER_CEILING = {
  text: "Above what this map gives solo: a party, an EXP event or a buff",
  href: "/faq/#leaderboard-over-ceiling",
};

/** `expPerHour` as a share of `ceiling` (> 0), in hundredths of a percent, rounded, halves
 *  up, as the Personal Bests window works it out (`share_hundredths`, bests_window.rs). */
function shareHundredths(expPerHour: number, ceiling: number): number {
  return Math.floor((expPerHour * 10_000 + Math.floor(ceiling / 2)) / ceiling);
}

/** Where to train's table for the band GET /api/bands/<band> answered, as `link` filters
 *  and sorts it, with dates written as of `now`. A map the player can't normally enter
 *  (an Event or Dev map) is left out unless it has entries, as search leaves it out. A
 *  street the band has no maps on shows every street. By players' bests: the maps with
 *  entries, highest #1 first, then the rest by ceiling; by ceiling: highest first, then by
 *  the #1. Ties by name. */
export function trainingTable(answer: TrainingAnswer, names: MapNames, link: TrainingLink, now: Date): TrainingTable {
  const maps = answer.maps
    .map((m) => ({ ...m, place: placeOf(names, m.map) }))
    .filter((m) => m.best !== null || !LEFT_OUT_REGIONS.includes(m.place.region));
  const streets = [...new Set(maps.map((m) => m.place.street))].sort(byText);
  const street = link.street !== null && streets.includes(link.street) ? link.street : null;
  const shown = maps.filter((m) => street === null || m.place.street === street);
  type Shown = (typeof shown)[number];
  const bestOf = (m: Shown) => m.best?.expPerHour ?? -1;
  const ceilingOf = (m: Shown) => m.ceiling ?? -1;
  const [first, then] = link.sort === "best" ? [bestOf, ceilingOf] : [ceilingOf, bestOf];
  shown.sort((a, b) => first(b) - first(a) || then(b) - then(a) || byText(a.place.name, b.place.name) || byText(a.map, b.map));
  const rows = shown.map((m): TrainingRow => {
    const best = m.best;
    const set = best === null ? null : new Date(best.setAt);
    return {
      name: m.place.name,
      street: m.place.street,
      href: linkTo({ map: m.map, band: answer.band, name: null }),
      mobs: m.mobs === null ? null : levels({ lowest: m.mobs[0], highest: m.mobs[1] }),
      ceiling: m.ceiling === null ? null : whole(m.ceiling),
      best:
        best === null || set === null
          ? null
          : {
              expPerHour: rate(best.expPerHour),
              who: `${best.name} · ${best.job} Lv ${best.level}`,
              date: dayText(set, set.getFullYear() !== now.getFullYear()),
            },
      share: best === null || m.ceiling === null ? null : sharePercent(shareHundredths(best.expPerHour, m.ceiling)),
      over: best !== null && m.ceiling !== null && best.expPerHour > m.ceiling,
      entries: m.entries === 0 ? "No entries yet" : entriesCounted(m.entries),
    };
  });
  const at = bandLevels(answer.band);
  let caption: string | null = null;
  if (rows.length > 0) caption = `${counted(rows.length, "map")} for ${at}${street === null ? "" : ` on ${street}`}`;
  return {
    rows,
    streets,
    street,
    caption,
    empty: rows.length === 0 ? { text: `No map's mobs are at ${at}, and no one has put a best there yet.`, link: PUT_YOURS } : null,
  };
}

/** What Where to train says before a band is picked. */
export const PICK_A_BAND = "Pick a Level Band to see the maps for it: what each gives solo, and the best players got there.";

/** An entry as Latest lists it. */
export interface LatestLine {
  name: string;
  /** "Magician Lv 23". */
  who: string;
  /** "153,000 EXP/h". */
  expPerHour: string;
  map: string;
  levels: string;
  date: string;
  /** The map's page at the band, with the entry marked. */
  href: string;
  /** "took #1, past Egbert's 80,000", or "took #1, past a best of 80,000" once that
   *  Character has left the Map Leaderboards; `null` for an entry that took no top spot. */
  took: string | null;
}

/** Latest, from GET /api/latest, with dates written as of `now`, or why it lists nothing. */
export function latestLines(answer: LatestAnswer, names: MapNames, now: Date): { lines: LatestLine[]; empty: Said | null } {
  const lines = answer.entries.map((e): LatestLine => {
    const set = new Date(e.setAt);
    let took: string | null = null;
    if (e.took !== null) {
      const past = rate(e.took.fromExpPerHour);
      took = e.took.from === null ? `took #1, past a best of ${past}` : `took #1, past ${e.took.from}'s ${past}`;
    }
    return {
      name: e.name,
      who: `${e.job} Lv ${e.level}`,
      expPerHour: `${rate(e.expPerHour)} EXP/h`,
      map: placeOf(names, e.map).name,
      levels: bandLevels(e.band),
      date: dayText(set, set.getFullYear() !== now.getFullYear()),
      href: linkTo({ map: e.map, band: e.band, name: e.name }),
      took,
    };
  });
  const empty = { text: "No one has put a best on the Map Leaderboards yet.", link: PUT_YOURS };
  return { lines, empty: lines.length === 0 ? empty : null };
}
