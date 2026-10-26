// A new #1 on a Map Leaderboard, told to Discord (ADR 0022): when an upload takes a map and
// Level Band's top spot from another Character, the Worker posts it to a Discord webhook, an
// embed for each spot showing the map's Map Mark. Only a spot *taken* is told, so a first
// entry in an empty band, a Character beating its own best, and a top spot that fell to
// someone because an entry was deleted all stay quiet.
import mapNameRows from "../../crates/minerva-core/tests/fixtures/osms-map-names.json";
import { rate } from "../../src/figures";
import { levelsOf } from "../level-bands";
import { type MapNames, mapNames, placeOf } from "../map-names";
import { markOf } from "./ceilings";

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
  /** It is the Character's Party Best, not its Solo Best. */
  party: boolean;
}

/** A top spot an upload took from another Character. */
export interface Taken {
  map: string;
  band: number;
  /** The Character that holds it now. */
  name: string;
  expPerHour: number;
  /** It took the spot with its Party Best, not its Solo Best. */
  party: boolean;
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
      party: now.party,
      from: was.name,
      fromExpPerHour: was.expPerHour,
    });
  }
  return taken.sort((a, b) => b.expPerHour - a.expPerHour || a.map.localeCompare(b.map) || a.band - b.band);
}

/** The Map Leaderboards link a spot's embed points at: the map's page, opened at the band with
 *  the Character's entry marked, as the app's deep links do (ADR 0015). Two spots never share
 *  one, which matters: Discord folds embeds with the same link into one. */
function linkTo(taken: Taken): string {
  const query = new URLSearchParams({ map: taken.map, band: String(taken.band), name: taken.name });
  return `https://owlofminerva.app/leaderboards/map/?${query}`;
}

/** A spot's embed in a post. */
interface Embed {
  title: string;
  url: string;
  description: string;
  /** The map's Map Mark; none for a map without one. */
  thumbnail?: { url: string };
}

/** One taken spot as an embed: titled with the map's name, linked to its Map Leaderboard, and
 *  showing its Map Mark when it has one. The link is the title's, never the description's, so
 *  Discord shows no preview under a post that names several. The title isn't markdown, so the
 *  map's name goes in as the site writes it ("G* Coconut Season" is one in the map names
 *  fixture); a Character's name is letters and digits only (ADR 0015), so the description
 *  needs no escaping either. */
function embedFor(taken: Taken, picture: string | null): Embed {
  const [first, last] = levelsOf(taken.band);
  const embed: Embed = {
    title: placeOf(PLACES, taken.map).name,
    url: linkTo(taken),
    description:
      `**${taken.name}** took #1, levels ${first}–${last}, ` +
      `with ${rate(taken.expPerHour)} EXP/h${taken.party ? " in a party" : ""}, past ${taken.from}'s ${rate(taken.fromExpPerHour)}.`,
  };
  if (picture !== null) embed.thumbnail = { url: picture };
  return embed;
}

/** What a Discord webhook is posted. */
export interface Post {
  /** How many more spots the upload took than it names; none when it names them all. */
  content?: string;
  embeds: Embed[];
  /** Nothing in a post ever pings: a Character's name is its own, not a mention. */
  allowed_mentions: { parse: [] };
}

/** The post for the spots an upload took, or `null` with none, with `pictureOf` giving a
 *  map's Map Mark picture (`null` for a map without one). */
export function postFor(taken: Taken[], pictureOf: (map: string) => string | null): Post | null {
  if (taken.length === 0) return null;
  const told = taken.slice(0, MOST_TOLD);
  const post: Post = { embeds: told.map((t) => embedFor(t, pictureOf(t.map))), allowed_mentions: { parse: [] } };
  const rest = taken.length - told.length;
  if (rest > 0) post.content = `…and ${rest} more top ${rest === 1 ? "spot" : "spots"} in the same upload.`;
  return post;
}

/** Post the taken spots to the Discord webhook, if one is set up (the DISCORD_WEBHOOK_URL
 *  secret). Nothing is retried and no failure reaches the upload: the Map Leaderboards are
 *  the record, and a post is only a nicety. */
export async function tell(env: Env, taken: Taken[]): Promise<void> {
  const url = env.DISCORD_WEBHOOK_URL;
  if (!url) return;
  const post = postFor(taken, (map) => markOf(env, map)?.picture ?? null);
  if (post === null) return;
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
