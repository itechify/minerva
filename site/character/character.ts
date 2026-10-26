// A Character's page (character/index.html, ADR 0027), worked out as plain data: the name its
// address names, and what the page shows from GET /api/characters/<name>: who they are as
// their newest entry says, their #1s, podiums, maps and best EXP/h, and every entry of theirs,
// a Level Band at a time. The page's script (page.ts) only fetches and draws what this gives it.

import { dayText } from "../../src/dates.ts";
import { levels, rate, whole } from "../../src/figures.ts";
import { type CharacterAnswer, isName, type MapMark } from "../entries.ts";
import { levelsOf } from "../level-bands.ts";
import { linkTo, type MapNames, placeOf, type Said } from "../leaderboards/leaderboards.ts";

export type { CharacterAnswer } from "../entries.ts";

/** The Character a page's address (its `location.search`) names, `?name=Egbert`; `null` for
 *  none, or one that can't be a Character's name. */
export function readName(search: string): string | null {
  const name = new URLSearchParams(search).get("name");
  return name !== null && isName(name) ? name : null;
}

/** A figure in the row under the Character's name: "5" over "#1s held". `detail` says more,
 *  under it: what a best was set on, or how many Level Bands. */
export interface Stat {
  value: string;
  label: string;
  detail: string | null;
}

/** One of the Character's entries as a band's table writes it. */
export interface CharacterRow {
  /** "#2", and how many the band has there: "of 12". */
  rank: string;
  of: string;
  /** Whether it's the band's #1 there: a crown marks it. */
  top: boolean;
  map: string;
  street: string;
  /** The map's Map Mark, shown before its name; `null` for a map without one. */
  mark: { name: string; picture: string } | null;
  /** "Lv 22–27", the levels of the middle half of its spawns; `null` for a map the ceilings
   *  table doesn't hold. */
  mobs: string | null;
  job: string;
  level: string;
  expPerHour: string;
  date: string;
  /** The map's Map Leaderboard at the band, with the entry marked. */
  href: string;
}

/** A Level Band of the Character's: its levels as its heading, what its table holds, and its
 *  rows. */
export interface CharacterBand {
  band: number;
  levels: string;
  caption: string;
  rows: CharacterRow[];
}

/** A Character's page: their name as last uploaded, who their newest entry says they are,
 *  the figures under the name, and their Level Bands, highest first. */
export interface CharacterPage {
  name: string;
  /** "Cleric Lv 31 · newest best 22 Oct". */
  who: string;
  stats: Stat[];
  bands: CharacterBand[];
}

/** `count` of `thing`, plural when it isn't 1: "1 map", "12 maps". */
function counted(count: number, thing: string): string {
  return `${whole(count)} ${thing}${count === 1 ? "" : "s"}`;
}

/** Order as the core's strings order: by code unit, not the locale's. */
function byText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** A Level Band's levels as the panel writes mob levels: "Lv 20–29". */
function bandLevels(band: number): string {
  const [lowest, highest] = levelsOf(band);
  return levels({ lowest, highest });
}

/** A Character's page from GET /api/characters/<name>, with maps named from `names` and
 *  dates written as of `now`. Their newest entry (by when its Visit set it) says who they are
 *  now, as near as the Map Leaderboards know. Each #1 is a map and Level Band's, so a
 *  Character on top of a map in two bands holds two; a podium is a place in a band's top 3.
 *  Each band's entries go highest EXP/h first. */
export function characterPage(answer: CharacterAnswer, names: MapNames, now: Date): CharacterPage {
  const dated = (setAt: number) => {
    const set = new Date(setAt);
    return dayText(set, set.getFullYear() !== now.getFullYear());
  };
  const entries = answer.entries.map((e) => ({ ...e, place: placeOf(names, e.map) }));
  type Placed = (typeof entries)[number];
  const newest = entries.reduce((a, b) => (b.setAt > a.setAt || (b.setAt === a.setAt && b.level > a.level) ? b : a));
  const best = entries.reduce((a, b) => (b.expPerHour > a.expPerHour ? b : a));
  const tops = entries.filter((e) => e.rank === 1).length;
  const podiums = entries.filter((e) => e.rank <= 3).length;
  const maps = new Set(entries.map((e) => e.map)).size;
  const bandsHeld = [...new Set(entries.map((e) => e.band))].sort((a, b) => b - a);

  const row = (e: Placed): CharacterRow => ({
    rank: `#${whole(e.rank)}`,
    of: `of ${whole(e.total)}`,
    top: e.rank === 1,
    map: e.place.name,
    street: e.place.street,
    mark: e.mark === null ? null : markOf(e.mark),
    mobs: e.mobs === null ? null : levels({ lowest: e.mobs[0], highest: e.mobs[1] }),
    job: e.job,
    level: String(e.level),
    expPerHour: rate(e.expPerHour),
    date: dated(e.setAt),
    href: linkTo({ map: e.map, band: e.band, name: answer.name }),
  });
  const bands = bandsHeld.map((band): CharacterBand => {
    const rows = entries
      .filter((e) => e.band === band)
      .sort((a, b) => b.expPerHour - a.expPerHour || byText(a.place.name, b.place.name) || byText(a.map, b.map));
    return { band, levels: bandLevels(band), caption: `${counted(rows.length, "map")} at ${bandLevels(band)}`, rows: rows.map(row) };
  });

  return {
    name: answer.name,
    who: `${newest.job} Lv ${newest.level} · newest best ${dated(newest.setAt)}`,
    stats: [
      { value: whole(tops), label: tops === 1 ? "#1 held" : "#1s held", detail: null },
      { value: whole(podiums), label: podiums === 1 ? "Podium" : "Podiums", detail: "Top 3 in a band" },
      { value: whole(maps), label: maps === 1 ? "Map" : "Maps", detail: `in ${counted(bandsHeld.length, "Level Band")}` },
      { value: rate(best.expPerHour), label: "Best EXP/h", detail: best.place.name },
    ],
    bands,
  };
}

function markOf(mark: MapMark): { name: string; picture: string } {
  return { name: mark.name, picture: mark.picture };
}

/** What the page says when it has no Character to show: a link naming none, or a name with no
 *  entries, whether never uploaded, opted out or taken down, which the page never tells
 *  apart; with a name, where a player's own entry would be, and why it might not be there. */
export function noCharacter(name: string | null): Said {
  if (name === null) return { text: "This link doesn't name a Character.", link: { text: "Find a map on the Map Leaderboards.", href: "/leaderboards/" } };
  return {
    text: `${name} has no bests on the Map Leaderboards.`,
    link: { text: "Where's my character's entry?", href: "/faq/#leaderboard-link" },
  };
}
