// A new #1 on a Map Leaderboard, told to Discord (ADR 0022): when an upload takes a map and
// Level Band's top spot from another Character, the Worker posts a line about it to a Discord
// webhook. Only a spot *taken* is told, so a first entry in an empty band, a Character
// beating its own best, and a top spot that fell to someone because an entry was deleted all
// stay quiet.
import mapNameRows from "../../crates/minerva-core/tests/fixtures/osms-map-names.json";
import { rate } from "../../src/figures";
import { levelsOf } from "../level-bands";
import { type MapNames, mapNames, placeOf } from "../map-names";

/** The maps the fixture names, built once per isolate. */
const PLACES: MapNames = mapNames(mapNameRows);

/** How many taken spots one post names. A Character that mirrors a whole Visit History for
 *  the first time can take several at once; the rest are counted, not listed, so the channel
 *  stays readable. */
export const MOST_TOLD = 3;

/** A map and Level Band's top entry. */
export interface Top {
  map: string;
  band: number;
  nameKey: string;
  name: string;
  expPerHour: number;
}

/** A top spot an upload took from another Character. */
export interface Taken {
  map: string;
  band: number;
  /** The Character that holds it now. */
  name: string;
  expPerHour: number;
  /** The Character it was taken from, and what it held. */
  from: string;
  fromExpPerHour: number;
}

/** The top spots `nameKey` took in this write: the ones it holds now that another Character
 *  held before. A band nobody held before (its first entry) isn't one, nor is a band the
 *  Character already held, so improving its own best says nothing. Best first. */
export function takenTops(before: Top[], after: Top[], nameKey: string): Taken[] {
  const held = new Map(before.map((t) => [`${t.map} ${t.band}`, t]));
  const taken: Taken[] = [];
  for (const now of after) {
    if (now.nameKey !== nameKey) continue;
    const was = held.get(`${now.map} ${now.band}`);
    if (was === undefined || was.nameKey === nameKey) continue;
    taken.push({
      map: now.map,
      band: now.band,
      name: now.name,
      expPerHour: now.expPerHour,
      from: was.name,
      fromExpPerHour: was.expPerHour,
    });
  }
  return taken.sort((a, b) => b.expPerHour - a.expPerHour || a.map.localeCompare(b.map) || a.band - b.band);
}

/** The Map Leaderboards link a line points at: the map's page, opened at the band with the
 *  Character's entry marked, as the app's deep links do (ADR 0015). */
function linkTo(taken: Taken): string {
  const query = new URLSearchParams({ map: taken.map, band: String(taken.band), name: taken.name });
  return `https://owlofminerva.app/leaderboards/map/?${query}`;
}

/** A map's name as Discord markdown, so a name holding one of its characters is read as the
 *  name and not as formatting: "G* Coconut Season" is one in the map names fixture, and a
 *  "]" would end the link early. A Character's name is letters and digits only (ADR 0015),
 *  so it never needs this. */
function escaped(name: string): string {
  return name.replace(/[\\*_~`[\]|]/g, "\\$&");
}

/** One taken spot as a line of the post. The map's name is the link, so Discord shows no
 *  preview under a post that names several. */
function lineFor(taken: Taken): string {
  const place = placeOf(PLACES, taken.map);
  const [first, last] = levelsOf(taken.band);
  return (
    `**${taken.name}** took #1 on [${escaped(place.name)}](${linkTo(taken)}), levels ${first}–${last}, ` +
    `with ${rate(taken.expPerHour)} EXP/h, past ${taken.from}'s ${rate(taken.fromExpPerHour)}.`
  );
}

/** What a Discord webhook is posted. */
interface Post {
  content: string;
  /** Nothing in a post ever pings: a Character's name is its own, not a mention. */
  allowed_mentions: { parse: [] };
}

/** The post for the spots an upload took, or `null` with none. */
export function postFor(taken: Taken[]): Post | null {
  if (taken.length === 0) return null;
  const lines = taken.slice(0, MOST_TOLD).map(lineFor);
  const rest = taken.length - lines.length;
  if (rest > 0) lines.push(`…and ${rest} more top ${rest === 1 ? "spot" : "spots"} in the same upload.`);
  return { content: lines.join("\n"), allowed_mentions: { parse: [] } };
}

/** Post the taken spots to the Discord webhook, if one is set up (the DISCORD_WEBHOOK_URL
 *  secret). Nothing is retried and no failure reaches the upload: the Map Leaderboards are
 *  the record, and a post is only a nicety. */
export async function tell(env: Env, taken: Taken[]): Promise<void> {
  const url = env.DISCORD_WEBHOOK_URL;
  const post = postFor(taken);
  if (!url || post === null) return;
  try {
    await fetch(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(post),
    });
  } catch {
    // A webhook that is gone, rate-limited or slow never fails an upload.
  }
}
