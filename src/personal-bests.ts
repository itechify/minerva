import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { dayText } from "./dates";
import { byId, span, svgEl } from "./dom";
import { type LevelRange, levels, rate, sharePercent, short, whole } from "./figures";
import { mapMark, type MapMarks } from "./map-marks";
import { dragByHandles, followView, heightReporter, pressControls } from "./shell";
import { soloExpHelp } from "./solo-exp-help";

/** Mirror of `minerva_core::PersonalBestsVisit`: one of a map's Visits. */
type BestsVisit = {
  expPerHour: number;
  setAtUnixMs: number | null;
  /** The level and job that set it, as the Status Bar showed them; `null`: not read. */
  level: number | null;
  job: string | null;
  ceilingShareHundredths: number | null;
  /** The Visit under way's best: not in the Visit History until the Visit ends, so it can
   *  be removed but not moved yet. */
  thisVisit: boolean;
};

/** Mirror of `minerva_core::PersonalBestsMap`. */
type BestsMap = {
  id: string;
  streetName: string;
  mapName: string;
  region: string;
  /** Its Map Mark, by OSMS's name for it. */
  mark: string | null;
  url: string;
  /** Its Map Leaderboard on the site, at the band of the Character playing; `null` for the
   *  Unassigned bests and a map not in the map list. */
  leaderboardUrl: string | null;
  /** The Character's best of all maps, which the star marks. */
  bestOfAll: boolean;
  mobLevels: LevelRange | null;
  expPerHour: number;
  soloExpCeiling: number | null;
  ceilingShareHundredths: number | null;
  setAtUnixMs: number | null;
  here: boolean;
  /** Its Visit History, highest first, the Visit under way's best among them. */
  visits: BestsVisit[];
};

/** Mirror of `minerva_core::RejectedListing`: an entry the Map Leaderboards kept off. */
type Rejected = {
  map: string;
  /** Its Level Band's first level and last: 1 and 9, 10 and 19… */
  lowestLevel: number;
  highestLevel: number;
  reason: { overCap: { cap: number } } | { removed: Record<string, never> };
};

/** Mirror of `minerva_core::LeaderboardToggle`: a Character's Share, on the Map Leaderboards. */
type Toggle = {
  on: boolean;
  /** How many maps they have an entry for: what turning it on shares. */
  maps: number;
  /** The switch is offered: they're shown, or have something to upload. */
  offered: boolean;
  status: "off" | "sending" | "upToDate" | "retrying" | "claimed" | "held";
  /** Where they stand is a problem, said as one, which can be sent again. */
  problem: boolean;
  uploadedAtUnixMs: number | null;
  /** The entries the site kept off, while they're shown there. */
  rejected: Rejected[];
};

/** Mirror of `minerva_core::PersonalBestsCharacter`: one of the picker's choices. */
type BestsCharacter = {
  /** The Character's name; `null` for the Unassigned bests. */
  name: string | null;
  /** The Character playing (or Unassigned, while no name is read): the picker's default. */
  current: boolean;
  /** Their rows, highest EXP/h first. */
  maps: BestsMap[];
  /** Their Share, on the Map Leaderboards; `null` for the Unassigned bests, never uploaded. */
  leaderboard: Toggle | null;
};

/** Mirror of `minerva_core::DiscordLinkView`: linking this install to the player's Account
 *  on the site, so their Characters' names follow them to a new PC. */
type DiscordLink = {
  /** Linked to the Account that signs in with this Discord username. */
  linkedAs: string | null;
  /** Link Discord is offered: the site said this install isn't linked. */
  offered: boolean;
  /** The link page is opening or open, and Minerva checks whether it linked. */
  linking: boolean;
  /** Why the link page couldn't be opened, as Link Discord last tried. */
  failed: string | null;
  /** The prompt shows, once, to a player sharing a Character who hasn't linked. */
  prompt: boolean;
};

/** What the shell sends: whether the window shows at all, whose bests it can show, each
 *  with the core's rows, every Map Mark, which each row's map names its own by, and linking
 *  to Discord. */
type BestsView = { shown: boolean; characters: BestsCharacter[]; mapMarks: MapMarks; discord: DiscordLink };

type Column = "map" | "levels" | "exp" | "share" | "set";
type Way = "ascending" | "descending";
type Key = string | number | [number, number] | null;

const panelEl = byId("panel");
const whose = byId("whose");
const count = byId("count");
const countActions = byId("count-actions");
const shareEl = byId("share");
const manageEl = byId("manage");
const problemsEl = byId("problems");
const rows = byId("rows");
const noteMissing = byId("note-missing");
const noteDot = byId("note-dot");
const scaleEl = byId("scale");
byId("solo-exp-help").append(soloExpHelp());
const announce = byId("announce");
const headers = [...document.querySelectorAll<HTMLTableCellElement>("th[data-column]")];

/** What each column sorts by, which way it reads best first, and how a sort by it is
 *  said each way. A map without the figure goes last either way. */
const COLUMNS: Record<Column, { name: string; key: (m: BestsMap) => Key; first: Way; ways: [string, string] }> = {
  map: { name: "map", key: (m) => m.mapName.toLowerCase() || null, first: "ascending", ways: ["A to Z", "Z to A"] },
  levels: {
    name: "mob levels",
    key: (m) => (m.mobLevels ? [m.mobLevels.lowest, m.mobLevels.highest] : null),
    first: "ascending",
    ways: ["lowest first", "highest first"],
  },
  exp: { name: "best EXP/h", key: (m) => m.expPerHour, first: "descending", ways: ["lowest first", "highest first"] },
  share: {
    name: "share of solo EXP/h",
    key: (m) => m.ceilingShareHundredths,
    first: "descending",
    ways: ["lowest first", "highest first"],
  },
  set: { name: "date set", key: (m) => m.setAtUnixMs, first: "descending", ways: ["oldest first", "newest first"] },
};

function compare(a: Key, b: Key): number {
  if (Array.isArray(a) && Array.isArray(b)) return compare(a[0], b[0]) || compare(a[1], b[1]);
  return a! < b! ? -1 : a! > b! ? 1 : 0;
}

/** The maps sorted by `column`, `way`. Ties keep the core's order: highest EXP/h. */
function sorted(maps: BestsMap[], { column, way }: { column: Column; way: Way }) {
  const { key } = COLUMNS[column];
  const sign = way === "ascending" ? 1 : -1;
  return [...maps].sort((a, b) => {
    const [x, y] = [key(a), key(b)];
    if (x === null || y === null) return Number(x === null) - Number(y === null);
    return sign * compare(x, y);
  });
}

/** A day by the calendar in the player's local time: "3 Oct" this year, "3 Oct 2025"
 *  another. */
function day(unixMs: number) {
  const date = new Date(unixMs);
  return dayText(date, date.getFullYear() !== new Date().getFullYear());
}

/** A cell, labelled for the narrowest layout, where each figure stands under its label. */
function cell(label: string | null, className?: string) {
  const td = document.createElement("td");
  if (label) td.dataset.label = label;
  if (className) td.className = className;
  return td;
}

/** A figure that isn't known: a dash, and why. */
function missing(td: HTMLTableCellElement, why: string) {
  td.classList.add("none");
  td.title = why;
  const dash = span("", "—");
  dash.setAttribute("aria-hidden", "true");
  td.append(dash, span("sr-only", why));
  return td;
}

/** Why a map's figures aren't known: it isn't in the map list, or, for its levels and
 *  share, no mobs spawn there. */
const NOT_LISTED = "Not in the map list";
const NO_MOBS = "No mobs spawn on this map";
const NO_DATE = "Set before Minerva knew the date";

/** A map's name as the window writes it. */
const mapName = (m: BestsMap) => m.mapName || `Map ${m.id}`;

/** Why a map's share isn't known. */
function noShare(m: BestsMap) {
  if (m.mapName === "") return NOT_LISTED;
  return m.mobLevels ? "No solo EXP/h for this map" : NO_MOBS;
}

/** The star on the best of all maps: the Grind Tracker's new-best star. */
function star() {
  const svg = svgEl("svg", { class: "best-star", viewBox: "0 0 16 16", role: "img", "aria-label": "Your best of all maps" });
  const title = svgEl("title");
  title.textContent = "Your best of all maps";
  const path = svgEl("path", { d: "M8 1.5L9.76 6.07L14.66 6.34L10.85 9.43L12.11 14.16L8 11.5L3.89 14.16L5.15 9.43L1.34 6.34L6.24 6.07Z" });
  const lit = svgEl("ellipse", { class: "lit", cx: 7.1, cy: 6.9, rx: 1.5, ry: 1.1 });
  svg.append(title, path, lit);
  return svg;
}

/** The scale every map's bar is drawn on: from 0 to the best of all maps, which runs the
 *  lane's whole length, with a line at each round step, three to five of them. */
type Scale = { top: number; lines: number[] };

function scaleOf(maps: BestsMap[]): Scale {
  const top = Math.max(1, ...maps.flatMap((m) => [m.expPerHour, ...m.visits.map((v) => v.expPerHour)]));
  // The smallest of 1, 2, 2.5 and 5 to a power of ten that is at least a fifth of the top.
  const power = 10 ** Math.floor(Math.log10(top / 5));
  const step = Math.max(1, power * ([1, 2, 2.5, 5, 10].find((n) => n * power >= top / 5) ?? 10));
  const lines: number[] = [];
  for (let n = step; n <= top; n += step) lines.push(n);
  return { top, lines };
}

/** How far along the lane an EXP/h is. */
const along = (n: number) => String(Math.min(1, n / scale.top));

/** A span at a place along the lane. */
function at(className: string, n: number) {
  const el = span(className, "");
  el.style.setProperty("--at", along(n));
  return el;
}

/** The header's ruler: each step's figure over a notch, as the Grind Tracker's forecast
 *  hangs its reaches. */
function renderScale() {
  const step = (n: number) => {
    const el = at(n === 0 ? "step zero" : "step", n);
    el.textContent = short(n);
    return el;
  };
  scaleEl.replaceChildren(step(0), ...scale.lines.map(step));
  spaceScale();
}

/** The ruler's figures far enough apart to read: where the lane is too short for every
 *  step's, every other one (or third) keeps its notch alone. */
function spaceScale() {
  const steps = [...scaleEl.children] as HTMLElement[];
  const apart = (scaleEl.clientWidth * (scale.lines[0] ?? scale.top)) / scale.top;
  const widest = Math.max(0, ...steps.map((el) => el.offsetWidth));
  const every = apart > 0 ? Math.ceil((widest + 8) / apart) : 1;
  for (const [i, el] of steps.entries()) el.classList.toggle("quiet", i % every !== 0);
}

/** The scale's lines down a lane's cell, behind its bar. */
const gridLines = () => scale.lines.map((n) => at("lane-grid", n));

/** A map's lane, in its own column beside the figure. */
function laneCell(m: BestsMap) {
  const td = cell(null, "lane-cell");
  td.append(...gridLines(), lane(m));
  return td;
}

/** A map's lane: its best as the EXP gauge's fill, on the scale every map shares; a notch
 *  in it for each of its other Visits; and You Yellow's dot for the Visit under way's. */
function lane(m: BestsMap) {
  const lane = document.createElement("div");
  lane.className = "lane";
  lane.append(at("lane-bar", m.expPerHour));
  const others = m.visits.slice(1).filter((v) => !v.thisVisit);
  lane.append(...others.map((v) => at("notch", v.expPerHour)));
  const now = m.visits.find((v) => v.thisVisit);
  if (now) lane.append(at("visit-dot", now.expPerHour));
  // Its Visits in words, as the bar draws them.
  const lowest = m.visits[m.visits.length - 1]?.expPerHour ?? m.expPerHour;
  const visits = m.visits.length === 1 ? "1 visit" : `${m.visits.length} visits`;
  const spread = m.visits.length === 1 ? `${rate(lowest)} EXP/h` : `${rate(lowest)} to ${rate(m.expPerHour)} EXP/h`;
  const label = `${visits}, ${spread}${now ? `; this visit ${rate(now.expPerHour)}` : ""}`;
  lane.setAttribute("role", "img");
  lane.setAttribute("aria-label", label);
  lane.title = label;
  return lane;
}

/** A Visit's own bar under its map's, on the same scale, the Visit under way's ending in
 *  its dot. */
function visitLane(v: BestsVisit) {
  const td = cell(null, "lane-cell");
  const lane = document.createElement("div");
  lane.className = "lane visit-lane";
  lane.setAttribute("aria-hidden", "true");
  lane.append(at("lane-bar", v.expPerHour));
  if (v.thisVisit) lane.append(at("visit-dot", v.expPerHour));
  td.append(...gridLines(), lane);
  return td;
}

function row(m: BestsMap, marks: MapMarks) {
  const tr = document.createElement("tr");
  const known = m.mapName !== "";

  const mapCell = cell(null, "text map-cell");
  if (m.here) {
    const dot = span("here-dot", "");
    dot.setAttribute("role", "img");
    dot.setAttribute("aria-label", "You're here");
    dot.title = "You're here";
    mapCell.append(dot);
  }
  // The name opens the map's MeowDB page in the browser, as a mob's name in the panel does.
  const name = document.createElement("button");
  name.type = "button";
  name.className = "row-map";
  name.dataset.id = m.id;
  name.title = "Open on MeowDB";
  name.append(mapName(m), span("sr-only", " (opens MeowDB in your browser)"));
  // The map's Map Mark before its name; the street under the name, not under the mark.
  const named = document.createElement("div");
  named.className = "row-name";
  named.append(mapMark(marks, m.mark), name);
  mapCell.append(named);
  const street = document.createElement("div");
  street.className = known ? "row-street" : "row-street unknown";
  // The street and its region: "Victoria Road · Victoria Island".
  street.textContent = known ? [m.streetName, m.region].filter(Boolean).join(" · ") : NOT_LISTED;
  mapCell.append(street);
  tr.append(mapCell);

  const lv = cell("Mob levels");
  if (m.mobLevels) lv.textContent = levels(m.mobLevels);
  else missing(lv, known ? NO_MOBS : NOT_LISTED);
  tr.append(lv);

  const exp = cell("Best EXP/h", "exp");
  if (m.bestOfAll) exp.append(star());
  // Its lane again under the figure, for a window too narrow for the lanes' own column.
  const under = lane(m);
  under.classList.add("under");
  exp.append(rate(m.expPerHour), under);
  tr.append(exp, laneCell(m));

  const share = cell("% of solo");
  if (m.ceilingShareHundredths !== null && m.soloExpCeiling !== null) {
    share.append(sharePercent(m.ceilingShareHundredths), span("ceiling", `of ${whole(m.soloExpCeiling)}`));
  } else {
    missing(share, noShare(m));
  }
  tr.append(share);

  const date = cell("Date set", "date");
  if (m.setAtUnixMs !== null) date.textContent = day(m.setAtUnixMs);
  else missing(date, NO_DATE);
  tr.append(date);

  // Its Visit History opens under the row, and the count says how many it holds.
  const controls = cell(null, "controls");
  const open = opened.has(`${shownKey()} ${m.id}`);
  const toggle = document.createElement("button");
  toggle.type = "button";
  toggle.className = "visits-toggle";
  toggle.dataset.id = m.id;
  toggle.setAttribute("aria-expanded", String(open));
  toggle.title = open ? "Hide its visits" : "Show its visits";
  const count = m.visits.length === 1 ? "1 visit" : `${m.visits.length} visits`;
  toggle.append(count, span("sr-only", ` on ${mapName(m)}`), chevron());
  controls.append(toggle);
  tr.append(controls);
  return tr;
}

/** The disclosure's chevron: pointing right while the map is closed, down while open. */
function chevron() {
  const svg = svgEl("svg", { class: "chevron", viewBox: "0 0 8 8", "aria-hidden": "true" });
  svg.append(svgEl("path", { d: "M3 1.5L5.5 4L3 6.5" }));
  return svg;
}

/** Each of a map's Visits by a key of its own: whose it is, its map, its best (with the
 *  level and job that set it) and, for Visits whose bests are the same, which of them it
 *  is. */
function visitKeys(m: BestsMap): string[] {
  const seen = new Map<string, number>();
  return m.visits.map((v) => {
    const best = [shownKey(), m.id, v.expPerHour, v.setAtUnixMs ?? "-", v.level ?? "-", v.job ?? "-"].join(" ");
    const n = seen.get(best) ?? 0;
    seen.set(best, n + 1);
    return `${best} ${n}`;
  });
}

/** The level and job a Visit was set at: "Lv 23 Magician", or as much as was read. */
function setBy(v: BestsVisit) {
  return [v.level === null ? null : `Lv ${v.level}`, v.job].filter(Boolean).join(" ");
}

/** One of an opened map's Visits, under its row: the level that set it, its best's EXP/h,
 *  share and date under the row's, then its Move (with another Character to move it to), or
 *  for the Visit under way's "This visit", and its ×. */
function visitRow(m: BestsMap, v: BestsVisit, key: string, place: number, last: boolean) {
  const tr = document.createElement("tr");
  tr.className = last ? "visit last" : "visit";

  const first = cell(null, "text blank");
  first.append(span("sr-only", `${mapName(m)}, visit ${place} of ${m.visits.length}`));
  const level = setBy(v);
  if (level) first.append(span("visit-level", level));
  tr.append(first, cell(null, "blank"));

  const exp = cell(null, "visit-exp");
  exp.textContent = rate(v.expPerHour);
  tr.append(exp, visitLane(v));

  const share = cell(null, "visit-share");
  if (v.ceilingShareHundredths !== null) share.textContent = sharePercent(v.ceilingShareHundredths);
  else missing(share, noShare(m));
  tr.append(share);

  const date = cell(null, "date visit-date");
  if (v.setAtUnixMs !== null) date.textContent = day(v.setAtUnixMs);
  else missing(date, NO_DATE);
  tr.append(date);

  const controls = cell(null, "controls");
  listed.set(key, { map: m.id, visit: v });
  // While its × asks "Remove?", that takes Move's room too, so the column never widens. The
  // Visit under way's can't be moved until it ends: its tag stands in Move's place.
  if (armed !== `remove:${key}`) {
    if (v.thisVisit) controls.append(span("this-visit", "This visit"));
    else if (moveTargets().length > 0) controls.append(moveButton(key));
  }
  controls.append(removeButton(key));
  tr.append(controls);
  return tr;
}

/** The last line of an opened map's Visit History: its Map Leaderboard on the site, for a
 *  Character's map that has one. */
function boardRow(m: BestsMap) {
  const tr = document.createElement("tr");
  tr.className = "visit board-row last";
  const td = cell(null, "text");
  td.colSpan = 7;
  const board = document.createElement("button");
  board.type = "button";
  board.className = "row-leaderboard";
  board.dataset.id = m.id;
  board.title = "Open its Map Leaderboard on owlofminerva.app";
  board.append("Map Leaderboard", span("sr-only", ` for ${mapName(m)} (opens owlofminerva.app in your browser)`));
  td.append(board);
  tr.append(td);
  return tr;
}

/** A button that arms `arm` when clicked; if it is `final`, a second click once armed does
 *  what it says. */
function armButton(arm: string, final: boolean) {
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.arm = arm;
  if (final) button.dataset.final = "";
  return button;
}

/** A light text button that arms `arm`: `words`, or `armedWords` once armed. Armed, a final
 *  one takes the problem palette: the next click does what it asks, with no undo. */
function textButton(arm: string, final: boolean, words: string, armedWords: string, title: string) {
  const button = armButton(arm, final);
  const on = armed === arm;
  button.className = on && final ? "text-button armed" : "text-button";
  button.textContent = on ? armedWords : words;
  button.title = title;
  return button;
}

/** A Visit's ×, as the Grind Tracker's Best row has; once clicked, "Remove?", which
 *  removes it with a second click. */
function removeButton(key: string) {
  const arm = `remove:${key}`;
  if (armed === arm) {
    const button = textButton(arm, true, "Remove?", "Remove?", "Click again to remove this visit from your history");
    button.classList.add("remove");
    return button;
  }
  const button = armButton(arm, true);
  button.className = "icon-button light remove";
  button.setAttribute("aria-label", "Remove this visit from your history");
  button.title = "Remove this visit from your history";
  button.innerHTML = '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" /></svg>';
  return button;
}

/** A Visit's Move, which lists the Characters to move it to under its line. */
function moveButton(key: string) {
  const button = textButton(`move:${key}`, false, "Move", "Move", "Move this visit to another of your characters");
  button.classList.add("move");
  button.setAttribute("aria-expanded", String(picking("move", key)));
  return button;
}

/** Whether the list of `what`'s targets (a Visit's Move, a Character's Merge, or Unassigned's
 *  Assign all) is open, for `key`: its button, or one of the targets, is armed. */
function picking(what: "move" | "merge" | "assign", key: string) {
  return armed === `${what}:${key}` || !!armed?.startsWith(`${what}:${key}>`);
}

/** Under a Visit whose Move is open: a button for each other Character to move it to, each
 *  asking again once clicked. */
function moveRow(key: string) {
  const tr = document.createElement("tr");
  tr.className = "visit move-row";
  const td = cell(null, "text");
  td.colSpan = 7;
  const to = document.createElement("div");
  to.className = "move-to";
  to.append(span("move-label", "Move this visit to"));
  for (const name of moveTargets()) {
    const arm = `move:${key}>${name}`;
    to.append(textButton(arm, true, name, `Move to ${name}?`, `Move this visit to ${name}'s bests`));
  }
  td.append(to);
  tr.append(td);
  return tr;
}

/** The choices as last sent, and how the player sorted them: kept until Minerva closes,
 *  so the window opens again as they left it. */
let characters: BestsCharacter[] = [];
/** The shown choice's rows. */
let maps: BestsMap[] = [];
let marks: MapMarks = {};
/** Linking to Discord, as the core last said it. */
let discord: DiscordLink = { linkedAs: null, offered: false, linking: false, failed: null, prompt: false };
let sort: { column: Column; way: Way } = { column: "exp", way: "descending" };
let scale: Scale = { top: 1, lines: [] };
/** The maps opened to their Visit History, by whose they are and their ID, kept as the sort
 *  is. */
const opened = new Set<string>();
/** The choice the player picked, by its key; `null` until they pick one, and after it goes:
 *  the picker shows the Character playing. */
let picked: string | null = null;
/** The shown Character's Manage is open: their Merge and Remove. Kept as the player
 *  picks another, until they close it. */
let managing = false;
/** What a click armed, until a second click does it, a click anywhere else, the window
 *  hiding or its button going: one at a time. A Visit's × (`remove:<visit>`), its Move
 *  (`move:<visit>`, and `move:<visit>><name>` once a Character is clicked); a Character's
 *  Remove (`remove-character:<name>`) and Merge (`merge:<name>`, and `merge:<name>><into>`),
 *  and held uploads' Send (`send:<name>`); and Unassigned's Assign all (`assign:`, and
 *  `assign:><name>`). */
let armed: string | null = null;
/** When the button now armed was armed (`performance.now()`): a second click within
 *  [`SECOND_CLICK_MS`] of it is a double-click's, and is ignored, so a double-click never
 *  does what a confirmed click does. */
let armedAt = 0;
const SECOND_CLICK_MS = 400;
/** The Visits listed with a × now, by their keys: what their buttons act on. */
const listed = new Map<string, { map: string; visit: BestsVisit }>();

/** The Unassigned choice's key; a Character's is their name, which is never empty. */
const UNASSIGNED = "";
const keyOf = (c: BestsCharacter) => c.name ?? UNASSIGNED;
const nameOf = (c: BestsCharacter) => c.name ?? "Unassigned";

/** The choice shown: the one picked, or else the Character playing. */
function shown(): BestsCharacter | null {
  return characters.find((c) => keyOf(c) === picked) ?? characters.find((c) => c.current) ?? characters[0] ?? null;
}

function shownKey() {
  const c = shown();
  return c ? keyOf(c) : UNASSIGNED;
}

/** Every Character's name, the one shown included. */
function names() {
  return characters.flatMap((c) => (c.name === null ? [] : [c.name]));
}

/** The Characters a shown Visit can be moved to: every one but whose it is. */
function moveTargets() {
  const from = shown()?.name ?? null;
  return names().filter((name) => name !== from);
}

/** "1 map", "14 maps". */
const mapsText = (n: number) => (n === 1 ? "1 map" : `${n} maps`);

function renderRows() {
  const keys = new Map(maps.map((m) => [m.id, visitKeys(m)]));
  scale = scaleOf(maps);
  renderScale();
  listed.clear();
  rows.replaceChildren(
    ...sorted(maps, sort).flatMap((m) => {
      if (!opened.has(`${shownKey()} ${m.id}`)) return [row(m, marks)];
      const mapKeys = keys.get(m.id)!;
      const board = m.leaderboardUrl !== null;
      const visits = m.visits.flatMap((v, i) => {
        const key = mapKeys[i];
        const line = visitRow(m, v, key, i + 1, !board && i === m.visits.length - 1);
        return picking("move", key) ? [line, moveRow(key)] : [line];
      });
      return [row(m, marks), ...visits, ...(board ? [boardRow(m)] : [])];
    }),
  );
  for (const th of headers) {
    if (th.dataset.column === sort.column) th.setAttribute("aria-sort", sort.way);
    else th.removeAttribute("aria-sort");
  }
}

/** The picker, with a button for each choice, the shown one pressed and the Character
 *  playing marked "playing": always, so the window names whose bests these are, even with one
 *  choice. Then an empty-state or Unassigned note, and the Character's Manage, or Unassigned's
 *  Assign all to… */
function renderWhose() {
  const current = shown();
  whose.hidden = characters.length === 0;
  whose.replaceChildren(
    ...characters.map((c) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "pick";
      button.dataset.pick = keyOf(c);
      button.setAttribute("aria-pressed", String(c === current));
      button.append(nameOf(c));
      if (c.current && c.name !== null) button.append(span("playing", "playing"));
      return button;
    }),
  );
  let words = "";
  if (maps.length === 0) words = `No bests yet for ${current ? nameOf(current) : "you"}`;
  else if (current?.name === null) words = "Set before Minerva read your characters' names, or while it couldn't";
  count.textContent = words;
  count.hidden = words === "";

  countActions.replaceChildren();
  if (!current) return;
  if (current.name === null) {
    // Unassigned: give every Visit to one Character at once.
    if (maps.length > 0 && names().length > 0) {
      const assign = textButton("assign:", false, "Assign all to…", "Assign all to…", "Give every Unassigned visit to one of your characters");
      assign.setAttribute("aria-expanded", String(picking("assign", "")));
      countActions.append(assign);
    }
  } else if (manageable(current)) {
    // The Character's Merge and Remove, folded under one quiet line so the bars lead.
    const manage = document.createElement("button");
    manage.type = "button";
    manage.className = "manage-toggle";
    manage.setAttribute("aria-expanded", String(managing));
    manage.setAttribute("aria-controls", "manage");
    manage.title = managing ? `Hide what you can do with ${current.name}` : `Merge or remove ${current.name}`;
    manage.append("Manage", span("sr-only", ` ${current.name}`), chevron());
    countActions.append(manage);
  }
}

/** Whether a Character has anything to manage: bests to merge or remove. */
function manageable(c: BestsCharacter) {
  return c.maps.length > 0;
}

/** A day and time in the player's local time: "3 Oct, 14:02". */
function when(unixMs: number) {
  const date = new Date(unixMs);
  const time = date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  return `${day(unixMs)}, ${time}`;
}

/** What the switch says beside it of where the Character stands with the Map Leaderboards,
 *  when it isn't a problem (those are notes of their own). */
function standing(t: Toggle): string {
  switch (t.status) {
    case "sending":
      return t.on ? "Uploading…" : "Taking them off…";
    case "upToDate":
      return t.uploadedAtUnixMs === null ? "Up to date." : `Up to date: uploaded ${when(t.uploadedAtUnixMs)}.`;
    default:
      return "";
  }
}

/** Link Discord, in words a player can act on: a button to link, or whose Discord this
 *  Minerva is linked to. */
function linkButton(words = "Link Discord") {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "text-button";
  button.dataset.linkDiscord = "";
  button.textContent = words;
  button.title = "Link this PC to your Discord on owlofminerva.app, so your characters' names on the Map Leaderboards follow you to a new PC";
  return button;
}

/** Beside the Share: whose Discord this Minerva is linked to, that linking is under way, or
 *  Link Discord. Nothing while the site hasn't said. */
function discordLine(): HTMLElement | null {
  if (discord.linkedAs !== null) return span("leaderboard-status discord-linked", `Linked to Discord as ${discord.linkedAs}`);
  if (discord.linking) return span("leaderboard-status", "Link Discord: finish in your browser…");
  if (discord.offered) {
    const line = span("discord-offer", "");
    line.append(linkButton());
    return line;
  }
  return null;
}

/** A line of the Manage panel. */
function line(...parts: (string | HTMLElement)[]) {
  const div = document.createElement("div");
  div.className = "manage-line";
  div.append(...parts);
  return div;
}

/** The shown Character's Share, under their name: the switch and where they stand on the Map
 *  Leaderboards, or why nothing of theirs can go there yet. Unassigned has none. */
function renderShare() {
  const c = shown();
  const t = c?.leaderboard;
  shareEl.replaceChildren();
  if (c?.name && t?.offered) {
    shareEl.append(leaderboardSwitch(c.name, t), span("leaderboard-status", standing(t)));
    const link = discordLine();
    if (link) shareEl.append(link);
  } else if (c?.name && t && c.maps.length > 0) {
    shareEl.append(
      span("move-label", `Nothing of ${c.name}'s can go on the Map Leaderboards yet: a best goes up once Minerva has read the level and job that set it, and its date.`),
    );
  }
  shareEl.hidden = shareEl.childElementCount === 0;
}

/** The shown Character's Manage, while open: Merge into… (for a Character the game renamed)
 *  and Remove, each asking again before it does anything. Unassigned's Assign all to… opens
 *  its Characters here too. */
function renderManage() {
  const c = shown();
  manageEl.replaceChildren();
  if (c && c.name === null && picking("assign", "")) {
    const n = maps.length;
    manageEl.append(
      line(
        span("move-label", "Give every Unassigned visit to"),
        ...names().map((name) =>
          textButton(`assign:>${name}`, true, name, `Assign all to ${name}?`, `Move every Unassigned visit, on ${mapsText(n)}, to ${name}'s bests`),
        ),
      ),
    );
  }
  if (c && c.name !== null && managing && manageable(c)) {
    const name = c.name;
    const t = c.leaderboard;
    const public_ = !!t && (t.on || t.status !== "off");
    if (c.maps.length > 0) {
      const into = names().filter((other) => other !== name);
      const site = public_ ? " and their Map Leaderboard entries" : "";
      const parts: HTMLElement[] = [];
      if (into.length > 0) {
        const merge = textButton(`merge:${name}`, false, "Merge into…", "Merge into…", `For a character renamed in the game: move ${name}'s bests to their new name`);
        merge.setAttribute("aria-expanded", String(picking("merge", name)));
        parts.push(span("move-label", "Renamed in the game?"), merge);
      }
      const remove = textButton(
        `remove-character:${name}`,
        true,
        `Remove ${name}`,
        `Remove ${name}, their bests on ${mapsText(c.maps.length)}${site}?`,
        `Remove ${name}, with all their bests${site}`,
      );
      remove.classList.add("remove-character");
      parts.push(remove);
      manageEl.append(line(...parts));
      if (picking("merge", name)) {
        const off = public_ ? `, and take ${name} off the Map Leaderboards?` : "?";
        manageEl.append(
          line(
            span("move-label", `Merge ${name}'s bests into`),
            ...into.map((other) =>
              textButton(`merge:${name}>${other}`, true, other, `Merge ${name} into ${other}${off}`, `Move ${name}'s bests to ${other}, ${name}'s new name`),
            ),
          ),
        );
      }
    }
  }
  manageEl.hidden = manageEl.childElementCount === 0;
}

/** A Character's Share: a switch, which flips at a click. Its tooltip says what that does: on,
 *  how many maps go up and that the site is public; off, that it deletes them there. */
function leaderboardSwitch(name: string, t: Toggle) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "switch";
  button.dataset.share = name;
  button.setAttribute("role", "switch");
  button.setAttribute("aria-checked", String(t.on));
  button.setAttribute("aria-label", `Share ${name}'s bests on the Map Leaderboards`);
  button.title = t.on
    ? `${name}'s bests are on the Map Leaderboards at owlofminerva.app, for anyone to see. Turn off to delete them from the site.`
    : `Share ${name}'s bests on ${mapsText(t.maps)} on the Map Leaderboards at owlofminerva.app, for anyone to see.`;
  const knob = span("knob", "");
  knob.setAttribute("aria-hidden", "true");
  button.append(knob, "Share");
  return button;
}

/** An entry the site kept off, in words: "Sunset Sky, Lv 20–29: over 5× its solo EXP/h". */
function rejectedWords(r: Rejected) {
  const map = maps.find((m) => m.id === r.map);
  // Joined around the dash and after "Lv", so a band never breaks across lines.
  const levels = `Lv ${r.lowestLevel}⁠–⁠${r.highestLevel}`;
  const why = "overCap" in r.reason ? `above ${whole(r.reason.overCap.cap)} EXP/h, 5× its solo EXP/h` : "taken down by the site";
  return `${map ? mapName(map) : `Map ${r.map}`}, ${levels}: ${why}`;
}

/** A problem note, as the window's others: the orange dot, then its words, wrapping. */
function problemNote(words: (string | HTMLElement)[], action?: HTMLElement) {
  const note = document.createElement("div");
  note.className = "note problem";
  const text = document.createElement("p");
  text.className = "problem-words";
  text.append(...words);
  note.append(text);
  if (action) note.append(action);
  return note;
}

/** The prompt to link Discord, once, quietly: not a problem, so no dot, with Link Discord
 *  and Not now. */
function linkPrompt() {
  const note = document.createElement("div");
  note.className = "note link-prompt";
  const text = document.createElement("p");
  text.className = "problem-words";
  text.textContent = "Link Discord, and your characters' names on the Map Leaderboards follow you to a new PC.";
  const later = document.createElement("button");
  later.type = "button";
  later.className = "text-button";
  later.dataset.dismissLink = "";
  later.textContent = "Not now";
  note.append(text, linkButton(), later);
  return note;
}

/** The shown Character's upload problems, whether Manage is open or not: held, claimed or
 *  tried again, each with its Send again, and the entries the site kept off. */
function renderProblems() {
  const c = shown();
  const t = c?.leaderboard ?? null;
  problemsEl.replaceChildren();
  if (c && c.name !== null && t) {
    const name = c.name;
    const again = document.createElement("button");
    again.type = "button";
    again.className = "text-button";
    again.dataset.sendAgain = name;
    again.textContent = "Send again";
    again.title = t.on ? `Upload ${name}'s bests now, rather than waiting` : `Take ${name}'s bests off the site now, rather than waiting`;
    if (t.status === "held") {
      const send = textButton(
        `send:${name}`,
        true,
        "Send what's here",
        `Replace ${name}'s entries on the site with ${mapsText(t.maps)}?`,
        `Upload ${name}'s bests as they are here, in place of what the site holds`,
      );
      problemsEl.append(
        problemNote(
          [
            `Minerva couldn't read your bests file at a launch, so the site may hold more of ${name}'s bests than are here. `,
            "Nothing of theirs is sent until you send what's here, which replaces what the site holds.",
            ...(discord.offered ? [" To keep your characters' names yours if this PC's files are ever lost, ", linkButton(), "."] : []),
          ],
          send,
        ),
      );
    } else if (t.status === "claimed") {
      // To the panel's Discord link, as About & Credits' is.
      const link = document.createElement("button");
      link.type = "button";
      link.className = "text-link";
      link.dataset.control = "openDiscord";
      link.textContent = "ask on Minerva's Discord";
      const words: (string | HTMLElement)[] = [`Not on the Map Leaderboards: another Minerva holds the name ${name} there. If it's yours (a new PC, say), `];
      // Linked already, the name would have gone through if the old PC was: only Discord can help.
      if (discord.offered) words.push("and your old PC was linked to Discord, ", linkButton("link this one"), "; otherwise ");
      words.push(link, ", then send again.");
      problemsEl.append(problemNote(words, again));
    } else if (t.status === "retrying") {
      const last = t.uploadedAtUnixMs === null ? "" : ` Last uploaded ${when(t.uploadedAtUnixMs)}.`;
      const words = t.on
        ? `Couldn't upload ${name}'s bests: Minerva tries again by itself.${last}`
        : `Couldn't take ${name}'s bests off the site yet: Minerva tries again by itself.`;
      problemsEl.append(problemNote([words], again));
    }
    if (t.rejected.length > 0) {
      problemsEl.append(problemNote([`Kept off the Map Leaderboards: ${t.rejected.map(rejectedWords).join("; ")}.`]));
    }
  }
  if (discord.failed !== null) {
    problemsEl.append(problemNote([`Couldn't open the page to link Discord: ${discord.failed}.`], linkButton("Try again")));
  }
  if (discord.prompt) problemsEl.append(linkPrompt());
  problemsEl.hidden = problemsEl.childElementCount === 0;
}

/** Draw the shown choice, and disarm whatever went with its button. */
function renderAll() {
  const c = shown();
  maps = c?.maps ?? [];
  noteMissing.hidden = maps.every((m) => m.mobLevels !== null && m.ceilingShareHundredths !== null);
  noteDot.hidden = !maps.some((m) => m.visits.some((v) => v.thisVisit));
  const draw = () => {
    renderWhose();
    renderShare();
    renderManage();
    renderProblems();
    renderRows();
  };
  draw();
  if (armed !== null && !document.querySelector(`[data-arm="${CSS.escape(armed)}"]`)) {
    armed = null;
    draw();
  }
}

function render(view: BestsView) {
  // Not showing, the window draws nothing (the shell lets clicks through it).
  document.body.classList.toggle("hidden", !view.shown);
  characters = view.characters;
  marks = view.mapMarks;
  discord = view.discord;
  // Hidden, nothing stays armed. Closed, the window has no rows, and the maps opened stay
  // opened for when it opens again; open, a map whose row went is no longer opened, and a
  // choice that went is no longer picked.
  if (!view.shown) armed = null;
  if (characters.length > 0) {
    if (!characters.some((c) => keyOf(c) === picked)) picked = null;
    const rowsNow = new Set(characters.flatMap((c) => c.maps.map((m) => `${keyOf(c)} ${m.id}`)));
    for (const key of opened) if (!rowsNow.has(key)) opened.delete(key);
  }
  renderAll();
  reportHeight();
}

for (const th of headers) {
  th.querySelector("button")!.addEventListener("click", () => {
    const column = th.dataset.column as Column;
    const flipped: Way = sort.way === "ascending" ? "descending" : "ascending";
    const way = column === sort.column ? flipped : COLUMNS[column].first;
    sort = { column, way };
    renderRows();
    announce.textContent = `Sorted by ${COLUMNS[column].name}, ${COLUMNS[column].ways[way === "ascending" ? 0 : 1]}.`;
  });
}

// The shell sizes the window to the content, as the Grind Tracker's is, up to its most;
// past that the table scrolls.
const reportHeight = heightReporter(panelEl, "page_height");
// Narrowed to a smaller game, the rows lay out again, as cards at the narrowest, and the
// ruler spaces its figures for the lane's new length.
window.addEventListener("resize", () => {
  spaceScale();
  reportHeight();
});
// The height and the ruler's figures depend on the font: measure again once it has loaded.
document.fonts.ready.then(() => {
  spaceScale();
  reportHeight();
});

// Dragged by its header, never by its button. A click leaves the keyboard with the game:
// the window can't be activated.
dragByHandles();

/** A Visit as the shell takes it back: by its best, with the level and job that set it. */
function visitArgs(v: BestsVisit) {
  return { expPerHour: v.expPerHour, setAtUnixMs: v.setAtUnixMs, level: v.level, job: v.job };
}

/** Do what the armed button `arm` says, through the core, once clicked again. */
function act(arm: string) {
  const [what, rest] = [arm.slice(0, arm.indexOf(":")), arm.slice(arm.indexOf(":") + 1)];
  const [key, target] = rest.split(">");
  const c = shown();
  if (what === "remove") {
    const visit = listed.get(key);
    if (visit && c) {
      invoke("personal_bests_remove_visit", { character: c.name, map: visit.map, visit: visitArgs(visit.visit) });
    }
  } else if (what === "move") {
    const visit = listed.get(key);
    if (visit && c && target) {
      invoke("personal_bests_move_visit", { from: c.name, map: visit.map, visit: visitArgs(visit.visit), to: target });
    }
  } else if (what === "assign" && target) {
    invoke("personal_bests_assign_unassigned", { to: target });
  } else if (what === "remove-character") {
    invoke("personal_bests_remove_character", { name: key });
  } else if (what === "merge" && target) {
    invoke("personal_bests_merge_characters", { from: key, into: target });
  } else if (what === "send") {
    invoke("personal_bests_send_again", { name: key });
  }
}

/** What a second click on an armed button does, for screen readers. */
function armedWords(arm: string) {
  const final = arm.includes(">") || !/^(move|merge|assign):/.test(arm);
  if (!final) return "";
  const asks = document.querySelector<HTMLElement>(`[data-arm="${CSS.escape(arm)}"]`)?.textContent ?? "";
  return `${asks} Click again to confirm. It can't be undone.`;
}

// Its ×, through the core like the panel's icons.
pressControls(document);

// A map's name, to its MeowDB page, and its Map Leaderboard to the site; a map's count,
// opening or closing its Visit History; a choice
// in the picker, showing it; a Character's Share, flipping it through the core; Manage, opening
// or closing the Character's. A button that asks first (a Visit's × or Move, a Character's
// Remove or Merge, Assign all, held uploads' Send) arms it, and asks again: a second click
// does it through the core, once the double-click it may be part of is over. Any other click
// disarms it.
document.addEventListener("click", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const map = target?.closest<HTMLButtonElement>("button.row-map");
  if (map) invoke("personal_bests_open_map", { id: map.dataset.id });
  const board = target?.closest<HTMLButtonElement>("button.row-leaderboard");
  if (board) invoke("personal_bests_open_leaderboard", { character: shown()?.name ?? null, id: board.dataset.id });
  const again = target?.closest<HTMLButtonElement>("button[data-send-again]");
  if (again) invoke("personal_bests_send_again", { name: again.dataset.sendAgain });
  if (target?.closest("button[data-link-discord]")) invoke("personal_bests_link_discord");
  if (target?.closest("button[data-dismiss-link]")) invoke("personal_bests_dismiss_link_prompt");
  const share = target?.closest<HTMLButtonElement>("button[data-share]");
  if (share) invoke("personal_bests_show_on_leaderboards", { name: share.dataset.share, on: share.getAttribute("aria-checked") !== "true" });
  const toggle = target?.closest<HTMLButtonElement>("button.visits-toggle");
  const pick = target?.closest<HTMLButtonElement>("button[data-pick]");
  const manage = target?.closest<HTMLButtonElement>("button.manage-toggle");
  const arm = target?.closest<HTMLButtonElement>("button[data-arm]");
  const before = armed;
  // The second click of a double-click on what it just armed: not a confirmation.
  if (arm && arm.dataset.arm === before && performance.now() - armedAt < SECOND_CLICK_MS) return;
  armed = null;
  if (arm) {
    const key = arm.dataset.arm!;
    if (key !== before) {
      armed = key;
      armedAt = performance.now();
    } else if (arm.dataset.final !== undefined) {
      act(key);
    }
  }
  if (pick) picked = pick.dataset.pick!;
  if (manage) managing = !managing;
  if (toggle) {
    const id = `${shownKey()} ${toggle.dataset.id!}`;
    if (!opened.delete(id)) opened.add(id);
  }
  if (toggle || pick || manage || armed !== before) {
    renderAll();
    reportHeight();
  }
  // Said once the armed button is drawn asking.
  if (armed !== null && armed !== before) announce.textContent = armedWords(armed);
});

followView<BestsView>("personal-bests-state", "page_view", render);
reportHeight();
