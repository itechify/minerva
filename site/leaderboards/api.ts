// The Map Leaderboards pages' reads of the site's API (ADR 0015), and the few DOM helpers
// both pages draw with.
import fixture from "../../crates/minerva-core/tests/fixtures/osms-map-names.json";
import { make } from "../../src/dom.ts";
import ceilings from "../worker/osms-ceilings.json";
import { type MapMarks, mapMarks, mapNames, type MapNames, type Said, type Words } from "./leaderboards.ts";

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
 *  it does; under the pointer it is the link it is. */
export function characterLink(name: string, href: string, className = ""): HTMLAnchorElement {
  const link = make("a", className ? `lb-character ${className}` : "lb-character", name);
  link.href = href;
  return link;
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
