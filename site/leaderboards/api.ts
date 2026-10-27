// The Map Leaderboards pages' reads of the site's API (ADR 0015), and the few DOM helpers
// both pages draw with.
import fixture from "../../crates/minerva-core/tests/fixtures/osms-map-names.json";
import { make, svgEl } from "../../src/dom.ts";
import ceilings from "../worker/osms-ceilings.json";
import {
  type JobClass,
  type MapMarks,
  mapMarks,
  mapNames,
  type MapNames,
  type Medal,
  type PortraitSlot,
  type Said,
  type Words,
} from "./leaderboards.ts";

/** The maps the committed map names fixture names, as the panel names them. */
export const names: MapNames = mapNames(fixture);

/** Each map's Map Mark, from the committed ceilings table, as Where to train shows it. */
export const marks: MapMarks = mapMarks(ceilings);

/** What the pages say when the API can't be reached, or answers wrong, before Try again. */
export const UNREACHABLE = "Can't reach the Map Leaderboards right now.";

/** The reads that couldn't be had, each by the line that would say so, to try again
 *  together. */
const failed = new Map<HTMLElement, () => void>();
/** Whether Try again had the focus when pressed, so the one that takes its place takes it. */
let refocus = false;

/** Say once on the page, on the first of the lines that would say so, that the API can't be
 *  reached, with Try again, which runs each read that couldn't be had again. */
export function unreachable(line: HTMLElement, retry: () => void) {
  failed.set(line, retry);
  const lines = [...failed.keys()].sort((a, b) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1));
  for (const [i, each] of lines.entries()) {
    each.hidden = i > 0;
    if (i > 0) each.replaceChildren();
    else if (each.querySelector(".lb-retry") === null) each.replaceChildren(`${UNREACHABLE} `, tryAgain());
  }
}

function tryAgain(): HTMLButtonElement {
  const again = make("button", "lb-retry", "Try again");
  again.type = "button";
  again.addEventListener("click", () => {
    refocus = true;
    const retries = [...failed.values()];
    failed.clear();
    for (const read of retries) read();
  });
  // Pressed, Try again goes with its line: the one in its place takes the focus, unless
  // the player has moved on.
  if (refocus && (document.activeElement === null || document.activeElement === document.body)) queueMicrotask(() => again.focus());
  return again;
}

/** The API's answer to a GET of `path`, or `null` when it can't be had. */
export async function read<T>(path: string): Promise<T | null> {
  try {
    const response = await fetch(path, { headers: { accept: "application/json" } });
    return response.ok ? ((await response.json()) as T) : null;
  } catch {
    return null;
  }
}

export function byId<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

/** A map in a list of maps: its Map Mark (or the mark's room, without one) and name in bold
 *  over its street, a link to its page, and what's beside it (how many Characters have
 *  entries there). */
export function mapItem(
  map: { name: string; street: string; href: string; aside?: string | null; mark?: { name: string; picture: string } | null },
  enter = false,
): HTMLLIElement {
  const item = make("li");
  // The map Enter opens is the selected one, as the panel marks the match Enter picks.
  const link = make("a", enter ? "lb-map lb-enter" : "lb-map");
  link.href = map.href;
  const words = make("span", "lb-map-words");
  const title = make("span", "lb-map-title");
  title.append(markSlot(map.mark ?? null), make("span", "lb-map-name", map.name));
  words.append(title, make("span", "lb-map-street", map.street));
  link.append(words);
  if (map.aside != null) link.append(make("span", "lb-map-aside", map.aside));
  item.append(link);
  return item;
}

/** A Map Mark's picture, the game's own emblem for an area, at half the Mini Map's size;
 *  `named`, it names the area to assistive tech and under the pointer, as a map's mark does. */
export function markImage(mark: { name: string; picture: string }, named = false): HTMLImageElement {
  const img = make("img", "lb-mark");
  img.src = mark.picture;
  img.alt = named ? mark.name : "";
  if (named) img.title = mark.name;
  img.width = 19;
  img.height = 19;
  img.decoding = "async";
  img.referrerPolicy = "no-referrer";
  return img;
}

/** A map's Map Mark before its name in a list of maps, named; a map without one keeps the
 *  mark's room, empty and hidden from assistive tech, so the names line up down the list,
 *  as the panel keeps it. */
export function markSlot(mark: { name: string; picture: string } | null): HTMLElement {
  if (mark !== null) return markImage(mark, true);
  const slot = make("span", "lb-mark");
  slot.setAttribute("aria-hidden", "true");
  return slot;
}

/** A Character's name as a link to their page (ADR 0027). At rest it reads as the name around
 *  it does; under the pointer it is the link it is. Followed, it glides into that page's
 *  heading (site/seams.html). */
export function characterLink(name: string, href: string, className = ""): HTMLAnchorElement {
  const link = make("a", className ? `lb-character ${className}` : "lb-character", name);
  link.href = href;
  link.dataset.seam = "seam-name";
  return link;
}

/** How big a portrait is in a band's table (ADR 0029): ClassicRank's photos' own size, so
 *  each shows pixel for pixel. */
export const BAND_PORTRAIT = 96;

/** The question-mark figure (site/art), for a Character ClassicRank has no photo of. */
const NO_PORTRAIT = new URL("../art/portrait-none.png", import.meta.url).href;

/** A Character's portrait (ADR 0029), `size` pixels square: their photo, loaded from
 *  classicrank.com and drawn pixel for pixel, fitted to the square whatever its own size; the
 *  question-mark figure for none, or for a photo that won't load; or, until ClassicRank has
 *  been asked, the square, empty. Decoration: the name beside it says whose it is. */
export function portraitImage(slot: PortraitSlot, size: number): HTMLElement {
  if (slot === "waiting") {
    const room = make("span", "lb-portrait");
    room.setAttribute("aria-hidden", "true");
    room.style.setProperty("--size", `${size}px`);
    return room;
  }
  const img = make("img", "lb-portrait");
  img.alt = "";
  img.width = size;
  img.height = size;
  img.decoding = "async";
  img.loading = "lazy";
  const none = () => {
    img.src = NO_PORTRAIT;
    img.classList.remove("lb-portrait-photo");
    img.classList.add("lb-portrait-none");
  };
  if (slot === "none") {
    none();
  } else {
    img.src = slot.photo;
    img.classList.add("lb-portrait-photo");
    img.referrerPolicy = "no-referrer";
    img.addEventListener("error", none, { once: true });
  }
  return img;
}

/** Each class's icon (site/art/jobs), drawn pixel for pixel, and its size. A pirate's has none
 *  yet, and shows no icon. */
const JOB_ICONS: Partial<Record<JobClass, { src: string; size: number }>> = {
  beginner: { src: new URL("../art/jobs/beginner.png", import.meta.url).href, size: 26 },
  warrior: { src: new URL("../art/jobs/warrior.png", import.meta.url).href, size: 27 },
  magician: { src: new URL("../art/jobs/magician.png", import.meta.url).href, size: 26 },
  bowman: { src: new URL("../art/jobs/bowman.png", import.meta.url).href, size: 26 },
  thief: { src: new URL("../art/jobs/thief.png", import.meta.url).href, size: 27 },
};

/** A job as the pages write it: its class's icon, then its name ("Cleric"), or its name alone
 *  for a job without one; `inColumn`, where jobs stand one under another, a job without one
 *  keeps the icon's room, so the names line up. The icon is decoration: the name says the
 *  job. */
export function jobWithIcon(job: string, jobClass: JobClass | null, className = "", inColumn = true): HTMLSpanElement {
  const line = make("span", className ? `lb-job-line ${className}` : "lb-job-line");
  const icon = jobClass === null ? undefined : JOB_ICONS[jobClass];
  if (icon === undefined && inColumn) {
    const slot = make("span", "lb-job-slot");
    slot.setAttribute("aria-hidden", "true");
    line.append(slot);
  }
  if (icon !== undefined) {
    const img = make("img", "lb-job-icon");
    img.src = icon.src;
    img.alt = "";
    img.width = icon.size;
    img.height = icon.size;
    img.decoding = "async";
    line.append(img);
  }
  line.append(job);
  return line;
}

/** Each crown's metal: lit at its top as a medal is, ringed in a darker edge (DESIGN.md, Crown
 *  Gold, Crown Silver and Crown Bronze). */
const CROWNS: Record<Medal, { stops: [string, string, string]; edge: string }> = {
  1: { stops: ["#ffe58a", "#f7c33a", "#d99a00"], edge: "#6e4a00" },
  2: { stops: ["#f4f6fa", "#b9c1cd", "#7e8898"], edge: "#3b4352" },
  3: { stops: ["#f7d6b6", "#d8925a", "#a5602b"], edge: "#5a2f10" },
};

/** How many crowns are drawn, so each gradient's ID is its own. */
let crowns = 0;

/** A crown on a place in a band's top 3: gold, silver or bronze. Never You Yellow, which marks
 *  the player's own best. Decoration: the rank beside it says the place. */
export function crown(medal: Medal): SVGSVGElement {
  const metal = CROWNS[medal];
  const id = `lb-crown-${++crowns}`;
  const svg = svgEl("svg", { viewBox: "0 0 16 16", class: `lb-crown lb-crown-${medal}`, "aria-hidden": "true" });
  const gradient = svgEl("linearGradient", { id, x1: 0, y1: 0, x2: 0, y2: 1 });
  metal.stops.forEach((color, i) => gradient.append(svgEl("stop", { offset: [0, 0.45, 1][i], "stop-color": color })));
  const defs = svgEl("defs");
  defs.append(gradient);
  svg.append(defs, svgEl("path", { d: "M2 12.5 1 4.5l4 3 3-5 3 5 4-3-1 8Z", fill: `url(#${id})`, stroke: metal.edge }));
  return svg;
}

/** `words` drawn, a link for each that has one. */
export function wordsOf(words: Words): (string | HTMLAnchorElement)[] {
  return words.map((w) => (typeof w === "string" ? w : characterLink(w.text, w.href)));
}

/** Put what a page says in `line`: its words, and the link after them, if any. */
export function said(line: HTMLElement, what: Said) {
  const more: (string | HTMLElement)[] = [];
  if (what.link !== undefined) {
    const link = make("a", "", what.link.text);
    link.href = what.link.href;
    more.push(" ", link);
  }
  line.replaceChildren(what.text, ...more);
}
