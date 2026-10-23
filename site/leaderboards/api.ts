// The Map Leaderboards pages' reads of the site's API (ADR 0015), and the few DOM helpers
// both pages draw with.
import fixture from "../../crates/minerva-core/tests/fixtures/osms-map-names.json";
import { make } from "../../src/dom.ts";
import { mapNames, type MapNames, type Said } from "./leaderboards.ts";

/** The maps the committed map names fixture names, as the panel names them. */
export const names: MapNames = mapNames(fixture);

/** What the pages say when the API can't be reached, or answers wrong. */
export const UNREACHABLE = "Can't reach the Map Leaderboards right now. Try again in a minute.";

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

/** A map in a list of maps: its name in bold over its street, a link to its page, and
 *  what's beside it (how many Characters have entries there). */
export function mapItem(map: { name: string; street: string; href: string; aside?: string | null }, enter = false): HTMLLIElement {
  const item = make("li");
  // The map Enter opens is the selected one, as the panel marks the match Enter picks.
  const link = make("a", enter ? "lb-map lb-enter" : "lb-map");
  link.href = map.href;
  const words = make("span", "lb-map-words");
  words.append(make("span", "lb-map-name", map.name), make("span", "lb-map-street", map.street));
  link.append(words);
  if (map.aside != null) link.append(make("span", "lb-map-aside", map.aside));
  item.append(link);
  return item;
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
