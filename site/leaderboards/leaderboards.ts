// The Map Leaderboards pages (site/leaderboards), worked out as plain data: where a link
// opens, the maps a search finds, and what a page shows from the API's answers (ADR 0015):
// a map's Level Bands, and on the Map Leaderboards page Where to train and Latest
// (ADR 0024). The pages' scripts (search.ts, training.ts, latest.ts, map.ts) only fetch and
// draw what this gives them.

import { dayText } from "../../src/dates.ts";
import { levels, rate, short, whole } from "../../src/figures.ts";
import {
  type BandAnswer,
  type BandsAnswer,
  characterLinkTo,
  isMap,
  isName,
  type Kind,
  type LatestAnswer,
  type MapMark,
  type PodiumsAnswer,
  type PortraitAnswer,
  type Ranked,
  type TrainingAnswer,
} from "../entries.ts";
import { classOf, type JobClass } from "../jobs.ts";
import { bandOf, levelsOf } from "../level-bands.ts";
import { type MapNames, type MapPlace, mapNames, placeOf } from "../map-names.ts";

// A map's name is worked out in one place for the whole site (map-names.ts), and a
// Character's page's address in another (entries.ts), so a page still takes everything it
// shows from this module.
export { characterLinkTo, mapNames, placeOf };
export type { MapNameRow, MapNames, MapPlace } from "../map-names.ts";
export type { JobClass } from "../jobs.ts";

/** A place in a band's top 3, crowned in gold, silver or bronze. */
export type Medal = 1 | 2 | 3;

/** A Character's portrait as the pages show it beside their name (ADR 0029): their photo on
 *  ClassicRank; `"none"`, the question-mark figure, once ClassicRank has been asked and has no
 *  photo of them or doesn't show them; or `"waiting"`, the same figure, until it's been
 *  asked. */
export type PortraitSlot = { photo: string } | "none" | "waiting";

/** A Character's portrait from the API's answer; `null`, none at all, while the API gives
 *  none (the Worker has no ClassicRank key). */
export function portraitSlot(answer: PortraitAnswer | undefined): PortraitSlot | null {
  if (answer === undefined) return null;
  if (answer === null) return "waiting";
  return answer.photo === null ? "none" : { photo: answer.photo };
}

/** The crown an entry at `rank` wears: gold for #1, silver for #2, bronze for #3; none
 *  past them. */
export function medalOf(rank: number): Medal | null {
  return rank >= 1 && rank <= 3 ? (rank as Medal) : null;
}
export type { BandAnswer, BandsAnswer, Kind, LatestAnswer, MapMark, PodiumsAnswer, Ranked, TrainingAnswer } from "../entries.ts";

/** A map's page as its address names it: `/leaderboards/map/?map=<ID>&band=<first level>&name=<name>&kind=<kind>`.
 *  Any part may be missing, or dropped for not being one. */
export interface Link {
  /** The map's 9-digit ID ("010001010"). */
  map: string | null;
  /** The Level Band, by its first level: 1 (1-9), 10 (10-19), 20 (20-29)... */
  band: number | null;
  /** The Character whose entry the page shows: 1 to 16 letters and digits. */
  name: string | null;
  /** The entries to rank: only the Solo Bests' or only the Party Bests'; `null`, each
   *  Character's higher one (ADR 0015, amended). */
  kind: Kind | null;
}

/** The kind a link names: `solo` or `party`; anything else is both. */
function kindIn(value: string | null): Kind | null {
  return value === "solo" || value === "party" ? value : null;
}

/** The kinds a page offers to show, in order: both, then each. */
const KINDS: { kind: Kind | null; label: string }[] = [
  { kind: null, label: "Both" },
  { kind: "solo", label: "Solo" },
  { kind: "party", label: "Party" },
];

/** A choice of kind, the one shown marked (`open`), `href` being where it goes. */
export interface KindChoice {
  kind: Kind | null;
  label: string;
  href: string;
  open: boolean;
}

/** The map page's choice of kinds: each a link to the page with that kind, the band and the
 *  Character kept. */
export function kindChoices(link: Link): KindChoice[] {
  return KINDS.map(({ kind, label }) => ({ kind, label, href: linkTo({ ...link, kind }), open: kind === link.kind }));
}

/** The words for a kind of entry, in "a solo entry", "12 party entries": none for both. */
function kindWord(kind: Kind | null): string {
  return kind === null ? "" : `${kind} `;
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
  return { map: mapIn(query.get("map")), band: bandIn(query.get("band")), name: nameIn(query.get("name")), kind: kindIn(query.get("kind")) };
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
 *  `null` before the API has said (or when it can't be reached), and its Map Mark, `null`
 *  for a map without one. */
export interface FoundMap extends MapPlace {
  entries: string | null;
  mark: { name: string; picture: string } | null;
}

/** Each map's Map Mark, by its 9-digit ID: its area's name and picture. */
export type MapMarks = Map<string, { name: string; picture: string }>;

/** Each map's Map Mark from the ceilings table (site/worker/osms-ceilings.json), which
 *  `probe ceilings` writes and Where to train takes its marks from: a map it doesn't hold,
 *  or holds without a mark, has none. */
export function mapMarks(table: {
  maps: Record<string, { mobs: number[]; mark?: string }>;
  marks: Record<string, { name: string; picture: string }>;
}): MapMarks {
  const marks: MapMarks = new Map();
  for (const [map, row] of Object.entries(table.maps)) {
    const mark = row.mark === undefined ? undefined : table.marks[row.mark];
    if (mark !== undefined) marks.set(map, { name: mark.name, picture: mark.picture });
  }
  return marks;
}

/** The region OSMS puts the maps off the islands' roads in: mostly the job advancement
 *  tests, the party quest and the Free Market, maps a player is sent to rather than goes to
 *  train on, and a few hidden maps beside them. */
const OFF_THE_ISLANDS = "Other";

/** The maps whose Map Name holds `query`, whatever the case, much as the panel's search finds
 *  them (`search.rs`): those that start with it, then those with a word that starts with it,
 *  then the rest; Event and Dev maps aside. Within each, the islands' maps come before those
 *  off them, so "ant" opens Ant Tunnel I, not the Bowman's job advancement test, Ant Tunnel
 *  For Bowman (the panel's search doesn't do this yet); then each by name. A map named the
 *  query itself, whatever the case, comes first of all. Then, with `counts`, the maps with
 *  entries come first, so the first map, the one Enter opens, isn't an empty one when another
 *  isn't. The first `shown`, each with its Map Mark from `marks`, and how many more. */
export function searchMaps(
  names: MapNames,
  query: string,
  { shown = 10, counts = null, marks = new Map() }: { shown?: number; counts?: EntryCounts | null; marks?: MapMarks } = {},
): { maps: FoundMap[]; more: number } {
  const lower = query.trim().toLowerCase();
  if (lower === "") return { maps: [], more: 0 };
  const found: { exact: boolean; fit: number; empty: boolean; off: boolean; map: MapPlace }[] = [];
  for (const map of names.values()) {
    if (LEFT_OUT_REGIONS.includes(map.region)) continue;
    const f = fit(map.name, lower);
    if (f === null) continue;
    found.push({ exact: map.name.toLowerCase() === lower, fit: f, empty: !counts?.get(map.id), off: map.region === OFF_THE_ISLANDS, map });
  }
  found.sort(
    (a, b) =>
      Number(b.exact) - Number(a.exact) ||
      Number(a.empty) - Number(b.empty) ||
      a.fit - b.fit ||
      Number(a.off) - Number(b.off) ||
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
    maps: found.slice(0, shown).map(({ map }) => ({ ...map, entries: entries(map.id), mark: marks.get(map.id) ?? null })),
    more: Math.max(0, found.length - shown),
  };
}

/** What the search says under its field, for `query` and what it found: how many maps, and
 *  the one Enter opens (the list marks it, and this says it in words). */
export function searchSaid(query: string, found: { maps: MapPlace[]; more: number }): string {
  if (query.trim() === "") return "Type part of a map's name as it's spelled in the game, like Ant Tunnel.";
  if (found.maps.length === 0) return `No map has “${query.trim()}” in its name. Check that it's spelled as in the game.`;
  const shown = counted(found.maps.length, "map");
  const all = found.more === 0 ? shown : `${shown}, and ${whole(found.more)} more: type more of the name`;
  return `${all}. Press Enter to open ${found.maps[0].name}.`;
}

/** `count` of `thing`, plural when it isn't 1: "1 map", "12 maps". */
function counted(count: number, thing: string): string {
  return `${whole(count)} ${thing}${count === 1 ? "" : "s"}`;
}

/** How many entries, the one word the pages count a Map Leaderboard in: "1 entry", "12 entries";
 *  of one `kind`, "1 solo entry", "12 party entries". */
function entriesCounted(count: number, kind: Kind | null = null): string {
  return `${whole(count)} ${kindWord(kind)}${count === 1 ? "entry" : "entries"}`;
}

/** GET /api/maps: every map with entries, the most first. */
export interface MapsAnswer {
  maps: { map: string; characters: number; entries: number }[];
}

/** Words, some of them links: a Character's name in a line, linking to their page. */
export type Words = (string | { text: string; href: string })[];

/** What a page says in place of what it can't show, and where to go from there. */
export interface Said {
  text: string;
  link?: { text: string; href: string };
}

/** Where an empty Map Leaderboard sends the player: putting their own bests there, which
 *  only Minerva does. */
const PUT_YOURS = { text: "How to add yours from Minerva", href: "/faq/#leaderboard-appear" };

/** A Level Band's levels as the panel writes mob levels: "Lv 20–29". */
function bandLevels(band: number): string {
  const [lowest, highest] = levelsOf(band);
  return levels({ lowest, highest });
}

/** GET /api/maps/<map>: the map's Level Bands with entries, lowest first: how many
 *  Characters are ranked in each (by their higher entry), and how many entries of each kind. */
export interface MapAnswer {
  map: string;
  bands: { band: number; levels: [number, number]; entries: number; solo: number; party: number }[];
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
 *  show), or why it has none. Not `found`, there's no such map: the page says so under its
 *  title alone, with no street, picture or board. */
export interface MapPage {
  place: MapPlace;
  bands: BandChoice[];
  open: number | null;
  empty: Said | null;
  found: boolean;
}

/** Where a link to no map, or to one there isn't, sends the player: finding one. */
const FIND_A_MAP = { text: "Find a map on the Map Leaderboards.", href: "/leaderboards/" };

/** What a map's page says for a link that names no map. */
export const NO_MAP: Said = { text: "This link doesn't name a map.", link: FIND_A_MAP };

/** A map's page as `link` opens it, from its bands as the API gives them. It opens at the
 *  link's band, even with no entries there (Minerva links a Character's own band, and the
 *  page says they have no entry there), or else at the lowest band with entries. A map the
 *  map names fixture doesn't name and no one has a best on isn't one: the page says there's
 *  no such map. One with bests is a map newer than the fixture (Minerva's map list is
 *  OSMS's live one), named by its ID, as the Personal Bests window names it. */
export function mapPage(link: Link, names: MapNames, answer: MapAnswer): MapPage {
  const place = placeOf(names, answer.map);
  if (!place.known && answer.bands.length === 0) {
    const empty = { text: `There's no map ${answer.map} in the map list.`, link: FIND_A_MAP };
    return { place, bands: [], open: null, empty, found: false };
  }
  // Of the kind the link names, each band's count: only the bands that have some are offered.
  const counted = (b: MapAnswer["bands"][number]) => (link.kind === null ? b.entries : b[link.kind]);
  const withEntries = answer.bands.filter((b) => counted(b) > 0);
  const open = link.band ?? withEntries[0]?.band ?? null;
  const counts = new Map(withEntries.map((b) => [b.band, counted(b)]));
  if (open !== null && !counts.has(open)) counts.set(open, 0);
  const bands = [...counts]
    .sort(([a], [b]) => a - b)
    .map(([band, entries]) => ({
      band,
      levels: bandLevels(band),
      entries: entries === 0 ? "No entries" : entriesCounted(entries),
      href: linkTo({ map: answer.map, band, name: link.name, kind: link.kind }),
      open: band === open,
    }));
  const best = link.kind === null ? "a best" : `a ${link.kind} best`;
  return {
    place,
    bands,
    open,
    empty: open === null ? { text: `No one has ${best} on this Map Leaderboard yet.`, link: PUT_YOURS } : null,
    found: true,
  };
}

/** An entry as a band's table writes it. */
export interface Row {
  rank: string;
  /** Its crown, for a place in the band's top 3. */
  medal: Medal | null;
  name: string;
  /** The Character's page. */
  nameHref: string;
  job: string;
  /** The job's class, whose icon goes beside it; `null` for a job the game doesn't name. */
  jobClass: JobClass | null;
  level: string;
  expPerHour: string;
  date: string;
  /** Whether it's the linked Character's. */
  you: boolean;
  /** Whether it's a Party Best's: the table marks it. */
  party: boolean;
  /** How far along its lane the EXP/h runs, from 0 to 1: the band's #1 runs it full. */
  lane: number;
  /** The Character's portrait, before their name; `null` for none at all. */
  portrait: PortraitSlot | null;
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
  /** Whether its rows show portraits, so the page credits ClassicRank under the table. */
  portraits: boolean;
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
 *  any), with dates written as of `now`. Of one `kind` of entry, it says so in what it says
 *  of the table; with none, each Character is ranked by their higher entry. */
export function bandTable(answer: BandAnswer, name: string | null, now: Date, kind: Kind | null = null): BandTable {
  const yours = answer.character;
  const top = Math.max(1, answer.entries[0]?.expPerHour ?? 1);
  const row = (entry: Ranked): Row => {
    const set = new Date(entry.setAt);
    return {
      rank: String(entry.rank),
      medal: medalOf(entry.rank),
      name: entry.name,
      nameHref: characterLinkTo(entry.name),
      job: entry.job,
      jobClass: classOf(entry.job),
      level: String(entry.level),
      expPerHour: rate(entry.expPerHour),
      date: dayText(set, set.getFullYear() !== now.getFullYear()),
      you: entry.rank === yours?.rank,
      party: entry.party,
      lane: Math.min(1, entry.expPerHour / top),
      portrait: portraitSlot(entry.portrait),
    };
  };
  const rows = answer.entries.map(row);
  const at = bandLevels(answer.band);
  const total = whole(answer.total);
  let note: string | null = null;
  if (yours !== null) note = `${yours.name} is #${yours.rank} of ${total} ${kind === null ? "" : `${kind} entries `}at ${at}.`;
  else if (name !== null) note = `${name} has no ${kindWord(kind)}entry here at ${at}.`;
  const below = yours !== null && !rows.some((r) => r.you);
  const between = below ? yours.rank - rows.length - 1 : 0;
  const caption =
    rows.length === 0
      ? null
      : answer.total > rows.length
        ? `Top ${rows.length} of ${total} ${kind === null ? "" : `${kind} entries `}at ${at}`
        : `${entriesCounted(rows.length, kind)} at ${at}`;
  return {
    rows,
    gap: between > 0 ? `${whole(between)} more ${between === 1 ? "entry" : "entries"}` : null,
    yours: below ? row(yours) : null,
    note,
    caption,
    empty: rows.length === 0 ? { text: `No one at ${at} has a ${kindWord(kind)}best on this map yet.`, link: PUT_YOURS } : null,
    ruler: rows.length === 0 ? [] : rulerTo(top),
    portraits: [...answer.entries, ...(yours === null ? [] : [yours])].some((e) => e.portrait !== undefined),
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
      return { taken: true, said: "Thanks for the report. Minerva's maker will look at it." };
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
  if (link.kind !== null) query.set("kind", link.kind);
  return `/leaderboards/map/?${query}`;
}

/** A map's picture in OSMS's live manifest. Only a content hash can name a picture;
 *  absent or malformed entries have no picture, and never supply an arbitrary URL. */
export function mapPicture(manifest: unknown, map: string): string | null {
  if (manifest === null || typeof manifest !== "object" || Array.isArray(manifest)) return null;
  const hash = (manifest as Record<string, unknown>)[map];
  return typeof hash === "string" && /^[a-f0-9]{32}$/i.test(hash) ? `https://osmsdataexplorer.com/data/maps/${hash}.webp` : null;
}

/** The Map Leaderboards page as its address names it:
 *  `/leaderboards/?band=<first level>&area=<Map Mark>`, the area by OSMS's name for its Map
 *  Mark ("KerningCity"). Either part may be missing, or dropped for not being one: with no
 *  band, Where to train asks for one; in every area unless it names one. */
export interface TrainingLink {
  band: number | null;
  area: string | null;
  /** The entries Where to train ranks (`&kind=solo` or `&kind=party`); `null`, each
   *  Character's higher one. */
  kind: Kind | null;
}

/** Where the Map Leaderboards page's address (its `location.search`) opens. */
export function readTrainingLink(search: string): TrainingLink {
  const query = new URLSearchParams(search);
  const area = query.get("area");
  return {
    band: bandIn(query.get("band")),
    area: area !== null && /^[A-Za-z0-9]{1,32}$/.test(area) ? area : null,
    kind: kindIn(query.get("kind")),
  };
}

/** The address of the Map Leaderboards page, from the site's root. */
export function trainingLinkTo(link: TrainingLink): string {
  const query = new URLSearchParams();
  if (link.band !== null) query.set("band", String(link.band));
  if (link.area !== null) query.set("area", link.area);
  if (link.kind !== null) query.set("kind", link.kind);
  const search = String(query);
  return search === "" ? "/leaderboards/" : `/leaderboards/?${search}`;
}

/** Where to train's choice of kinds: each a link to the page with that kind, the band and the
 *  area kept. */
export function trainingKinds(link: TrainingLink): KindChoice[] {
  return KINDS.map(({ kind, label }) => ({ kind, label, href: trainingLinkTo({ ...link, kind }), open: kind === link.kind }));
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

/** Where to train's Level Bands, from GET /api/bands, each keeping the link's area. */
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
  /** Its Map Mark, shown before its name; `null` for a map without one. */
  mark: { name: string; picture: string } | null;
  /** Its Map Leaderboard, at the band. */
  href: string;
  /** The levels of the middle half of its spawns: "Lv 22–27"; `null` for a map the
   *  ceilings table doesn't hold. */
  mobs: string | null;
  /** The band's #1 there: its EXP/h, who set it (their name, linking to their page, and
   *  "Magician Lv 25") and when; `null` with no entries, its cell left empty, as its entries
   *  say there are none. */
  best: { expPerHour: string; name: string; nameHref: string; who: string; jobClass: JobClass | null; date: string; party: boolean } | null;
  /** "3 entries", or "No entries yet"; `null` when none of the maps shown has any, which
   *  the table says once, over them, its cell left empty. */
  entries: string | null;
}

/** An area Where to train offers to show the maps of: its Map Mark, and a link to it. */
export interface TrainingArea extends MapMark {
  href: string;
  /** Whether it's the area the table shows. */
  open: boolean;
}

/** Where to train's table for a band: its rows, the areas their Map Marks name (for the
 *  choice), the area it shows (`null`, all of them), what it says of itself, what it says
 *  over its rows when none of them has an entry, and why there are no rows when there are
 *  none. */
export interface TrainingTable {
  rows: TrainingRow[];
  areas: TrainingArea[];
  area: string | null;
  caption: string | null;
  /** The button under a long table's first rows, that shows the rest: "Show all 69 maps";
   *  `null` when every row is shown. */
  more: string | null;
  /** Said once over the rows when none of the maps shown has an entry: that no one has put a
   *  best on them yet, and how to put yours there. */
  none: Said | null;
  empty: Said | null;
}

/** How many of a band's maps Where to train shows before "Show all": the first 15, of a band
 *  with more than 20, so the button never hides only a few. */
const TRAINING_SHOWN = 15;
const TRAINING_ALL_UP_TO = 20;

/** Where to train's table for the band GET /api/bands/<band> answered, in the area `link`
 *  names, with dates written as of `now`. A map the player can't normally enter (an Event
 *  or Dev map) is left out unless it has entries, as search leaves it out. An area the
 *  band has no maps in shows every area; the areas are offered by name, and a map without
 *  a Map Mark shows only among every area's. The maps with entries come first, highest #1
 *  first, then the rest by their mob levels, lowest first. Ties by name. A band with more
 *  than 20 maps shows its first 15, and a button to show them `all`. When none of the maps
 *  shown has an entry, the table says so once, over them, and their rows leave their bests
 *  and entries empty rather than each say it again. */
export function trainingTable(
  answer: TrainingAnswer,
  names: MapNames,
  link: TrainingLink,
  now: Date,
  { all = false }: { all?: boolean } = {},
): TrainingTable {
  const maps = answer.maps
    .map((m) => ({ ...m, place: placeOf(names, m.map) }))
    .filter((m) => m.best !== null || !LEFT_OUT_REGIONS.includes(m.place.region));
  const marks = new Map<string, MapMark>();
  for (const m of maps) if (m.mark !== null) marks.set(m.mark.id, m.mark);
  const area = link.area !== null && marks.has(link.area) ? link.area : null;
  const areas = [...marks.values()]
    .sort((a, b) => byText(a.name, b.name))
    .map((mark) => ({ ...mark, href: trainingLinkTo({ band: answer.band, area: mark.id, kind: link.kind }), open: mark.id === area }));
  const shown = maps.filter((m) => area === null || m.mark?.id === area);
  type Shown = (typeof shown)[number];
  const best = (m: Shown) => m.best?.expPerHour ?? -1;
  // A map the ceilings table doesn't hold, with no mob levels, goes after the rest.
  const mobs = (m: Shown, end: 0 | 1) => m.mobs?.[end] ?? Number.MAX_SAFE_INTEGER;
  shown.sort(
    (a, b) =>
      best(b) - best(a) ||
      mobs(a, 0) - mobs(b, 0) ||
      mobs(a, 1) - mobs(b, 1) ||
      byText(a.place.name, b.place.name) ||
      byText(a.map, b.map),
  );
  const cut = !all && shown.length > TRAINING_ALL_UP_TO;
  const none = shown.length > 0 && shown.every((m) => m.entries === 0);
  const rows = (cut ? shown.slice(0, TRAINING_SHOWN) : shown).map((m): TrainingRow => {
    const set = m.best === null ? null : new Date(m.best.setAt);
    return {
      name: m.place.name,
      street: m.place.street,
      mark: m.mark === null ? null : { name: m.mark.name, picture: m.mark.picture },
      href: linkTo({ map: m.map, band: answer.band, name: null, kind: link.kind }),
      mobs: m.mobs === null ? null : levels({ lowest: m.mobs[0], highest: m.mobs[1] }),
      best:
        m.best === null || set === null
          ? null
          : {
              expPerHour: rate(m.best.expPerHour),
              name: m.best.name,
              nameHref: characterLinkTo(m.best.name),
              who: `${m.best.job} Lv ${m.best.level}`,
              jobClass: classOf(m.best.job),
              date: dayText(set, set.getFullYear() !== now.getFullYear()),
              party: m.best.party,
            },
      entries: none ? null : m.entries === 0 ? "No entries yet" : entriesCounted(m.entries, link.kind),
    };
  });
  const at = bandLevels(answer.band);
  let caption: string | null = null;
  const areaName = area === null ? null : marks.get(area)!.name;
  const listed = areaName === null ? counted(shown.length, "map") : `${whole(shown.length)} ${areaName} ${shown.length === 1 ? "map" : "maps"}`;
  if (rows.length > 0) caption = cut ? `${rows.length} of ${listed} for ${at}` : `${listed} for ${at}`;
  return {
    rows,
    areas,
    area,
    caption,
    more: cut ? `Show all ${counted(shown.length, "map")}` : null,
    none: !none
      ? null
      : shown.length === 1
        ? { text: `No one at ${at} has a ${kindWord(link.kind)}best on this map yet. Its mobs are at your level.`, link: PUT_YOURS }
        : {
            text: `No one at ${at} has a ${kindWord(link.kind)}best on these maps yet. Their mobs are at your level.`,
            link: PUT_YOURS,
          },
    empty:
      rows.length === 0 ? { text: `No map has mobs at ${at}, and no one at those levels has a ${kindWord(link.kind)}best yet.`, link: PUT_YOURS } : null,
  };
}

/** A Character on a Pedestal's step: their name (a link to their page), their portrait, and
 *  what they stand it for, its figure over what it counts ("7" over "podiums"), with more
 *  under the pointer. On the EXP/h Pedestal, also their job and level, and the map they set
 *  it on. */
export interface Stood {
  name: string;
  nameHref: string;
  /** `null` for none at all, while the API gives none. */
  portrait: PortraitSlot | null;
  value: string;
  label: string;
  detail: string | null;
  /** "Magician Lv 25", and the job's class, whose icon goes beside it. */
  who: { text: string; jobClass: JobClass | null } | null;
  /** The map, with its Map Mark (`null` for a map without one), and its page at the band
   *  with the entry marked. */
  map: { name: string; href: string; mark: { name: string; picture: string } | null } | null;
  /** Whether it's a Party Best's: the step marks it. */
  party: boolean;
}

/** A step on a Pedestal: its place, and who stands it; `null` while no one does. */
export interface PedestalStep {
  place: Medal;
  stood: Stood | null;
}

/** A Pedestal (ADR 0024, amended): its three steps, 1st to 3rd (the page stands them 2nd,
 *  1st, 3rd), the link a step no one stands shows, and whether its steps show portraits, so
 *  the page credits ClassicRank under it. */
export interface Pedestal {
  steps: [PedestalStep, PedestalStep, PedestalStep];
  empty: { text: string; href: string };
  portraits: boolean;
}

/** What a step no one stands says, and where it goes. */
const YOURS_HERE = { text: "Yours could be here", href: PUT_YOURS.href };

function pedestalOf(stood: Stood[]): Pedestal {
  const step = (place: Medal): PedestalStep => ({ place, stood: stood[place - 1] ?? null });
  return { steps: [step(1), step(2), step(3)], empty: YOURS_HERE, portraits: stood.some((s) => s.portrait !== null) };
}

/** The Podiums Pedestal, from GET /api/podiums: the three with the most podiums, each counted
 *  as their page counts them. */
export function podiumsPedestal(answer: PodiumsAnswer): Pedestal {
  return pedestalOf(
    answer.characters.map((c) => ({
      name: c.name,
      nameHref: characterLinkTo(c.name),
      portrait: portraitSlot(c.portrait),
      value: whole(c.podiums),
      label: c.podiums === 1 ? "podium" : "podiums",
      detail: "Top 3 in a band",
      who: null,
      map: null,
      party: false,
    })),
  );
}

/** The EXP/h Pedestal for the band GET /api/bands/<band> answered, of the `kind` Where to train
 *  shows, in `area` (by its Map Mark's ID; `null`, every area), as Where to train shows it
 *  (`trainingTable`'s area, not the link's, which may name one the band has no maps in); with
 *  its title, "Highest EXP/h in Lith Harbor at Lv 20–29"; each map linking to its page at the
 *  band and the kind, with the entry marked. `null` where no one has an entry. */
export function expPedestal(
  answer: TrainingAnswer,
  names: MapNames,
  kind: Kind | null,
  area: string | null,
): (Pedestal & { title: string }) | null {
  const entries = area === null ? answer.pedestal.all : (answer.pedestal.areas[area] ?? []);
  if (entries.length === 0) return null;
  const marks = new Map(answer.maps.map((m) => [m.map, m.mark]));
  const areaName = area === null ? null : ([...marks.values()].find((m) => m?.id === area)?.name ?? null);
  const at = bandLevels(answer.band);
  const title = areaName === null ? `Highest EXP/h at ${at}` : `Highest EXP/h in ${areaName} at ${at}`;
  const pedestal = pedestalOf(
    entries.map((e) => {
      const mark = marks.get(e.map) ?? null;
      return {
        name: e.name,
        nameHref: characterLinkTo(e.name),
        portrait: portraitSlot(e.portrait),
        value: rate(e.expPerHour),
        label: "EXP/h",
        detail: null,
        who: { text: `${e.job} Lv ${e.level}`, jobClass: classOf(e.job) },
        map: {
          name: placeOf(names, e.map).name,
          href: linkTo({ map: e.map, band: answer.band, name: e.name, kind }),
          mark: mark === null ? null : { name: mark.name, picture: mark.picture },
        },
        party: e.party,
      };
    }),
  );
  return { ...pedestal, title };
}

/** What Where to train says before a band is picked. */
export const PICK_A_BAND = "Pick a Level Band to see its maps and the best that players got on each.";

/** An entry as Latest lists it. */
export interface LatestLine {
  name: string;
  /** The Character's page. */
  nameHref: string;
  /** "Magician Lv 23". */
  who: string;
  /** The job's class, whose icon goes beside it; `null` for a job the game doesn't name. */
  jobClass: JobClass | null;
  /** "153,000", its EXP/h. */
  expPerHour: string;
  map: string;
  /** The map's Map Mark, shown before its name; `null` for a map without one. */
  mark: { name: string; picture: string } | null;
  levels: string;
  date: string;
  /** The map's page at the band, with the entry marked, ranking its own kind of entry. */
  href: string;
  /** Whether it's a Party Best's: Latest marks it. */
  party: boolean;
  /** "Beat Egbert's 80,000 to take #1", Egbert linking to their page, or "Beat a best of
   *  80,000 to take #1" once that Character has left the Map Leaderboards; `null` for an entry that
   *  took no top spot. */
  took: Words | null;
}

/** Latest's choice of kinds, the one shown (`shown`) marked: it lists both kinds of entry, or
 *  only the Solo Bests' or the Party Bests'. */
export function latestKinds(shown: Kind | null): { kind: Kind | null; label: string; open: boolean }[] {
  return KINDS.map(({ kind, label }) => ({ kind, label, open: kind === shown }));
}

/** Latest, from GET /api/latest, with dates written as of `now`, or why it lists nothing. */
export function latestLines(answer: LatestAnswer, names: MapNames, now: Date, kind: Kind | null = null): { lines: LatestLine[]; empty: Said | null } {
  const lines = answer.entries.map((e): LatestLine => {
    const set = new Date(e.setAt);
    let took: Words | null = null;
    if (e.took !== null) {
      const past = rate(e.took.fromExpPerHour);
      took =
        e.took.from === null
          ? [`Beat a best of ${past} to take #1`]
          : ["Beat ", { text: e.took.from, href: characterLinkTo(e.took.from) }, `'s ${past} to take #1`];
    }
    return {
      name: e.name,
      nameHref: characterLinkTo(e.name),
      who: `${e.job} Lv ${e.level}`,
      jobClass: classOf(e.job),
      expPerHour: rate(e.expPerHour),
      map: placeOf(names, e.map).name,
      mark: e.mark === null ? null : { name: e.mark.name, picture: e.mark.picture },
      levels: bandLevels(e.band),
      date: dayText(set, set.getFullYear() !== now.getFullYear()),
      href: linkTo({ map: e.map, band: e.band, name: e.name, kind: e.party ? "party" : "solo" }),
      party: e.party,
      took,
    };
  });
  const empty = { text: `No one has a ${kindWord(kind)}best on the Map Leaderboards yet.`, link: PUT_YOURS };
  return { lines, empty: lines.length === 0 ? empty : null };
}
