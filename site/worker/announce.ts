// A new #1 on a Map Leaderboard, told to Discord (ADR 0022): when an upload takes a map and
// Level Band's top spot from another Character, the Worker posts it to a Discord webhook, an
// embed for each spot showing the map's Map Mark, and the Character's portrait from
// ClassicRank when they have one (ADR 0029). Only a spot *taken* is told, so a first
// entry in an empty band, a Character beating its own best, and a top spot that fell to
// someone because an entry was deleted all stay quiet.
import mapNameRows from "../../crates/minerva-core/tests/fixtures/osms-map-names.json";
import { rate } from "../../src/figures";
import { characterLinkTo } from "../entries";
import { levelsOf } from "../level-bands";
import { type MapNames, mapNames, placeOf } from "../map-names";
import { markOf } from "./ceilings";

/** The maps the fixture names, built once per isolate. */
const PLACES: MapNames = mapNames(mapNameRows);

/** How many taken spots one post names: Discord's most embeds in one message. A Character that
 *  mirrors a whole Visit History for the first time can take more at once, and the rest go in
 *  the posts after it, so every spot is told. */
export const EMBEDS_PER_POST = 10;

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
  const held = new Map(before.map((t) => [`${t.map} ${t.band} ${t.party}`, t]));
  const taken: Taken[] = [];
  for (const now of after) {
    if (now.nameKey !== nameKey) continue;
    const was = held.get(`${now.map} ${now.band} ${now.party}`);
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
  const query = new URLSearchParams({ map: taken.map, band: String(taken.band), name: taken.name, kind: taken.party ? "party" : "solo" });
  return `https://owlofminerva.app/leaderboards/map/?${query}`;
}

/** A Character's name in a post: bold if `bold`, and a link to their page (ADR 0027). A masked
 *  link in an embed's description shows no preview, so a post naming several stays as it is. */
function nameOf(name: string, bold = false): string {
  const linked = `[${name}](https://owlofminerva.app${characterLinkTo(name)})`;
  return bold ? `**${linked}**` : linked;
}

/** A spot's embed in a post. */
interface Embed {
  title: string;
  url: string;
  description: string;
  /** The Character's portrait, or with none the map's Map Mark; none for neither. */
  thumbnail?: { url: string };
  /** With a portrait, its credit to ClassicRank, after the map's Map Mark beside the area it
   *  stands for when the map has one. */
  footer?: { text: string; icon_url?: string };
}

/** A Map Mark as a post shows it: the area it stands for, and its picture. */
export interface PostedMark {
  name: string;
  picture: string;
}

/** What a post with a portrait says of it, as ClassicRank asks wherever its data shows: its
 *  address, visible. It goes in the footer, at the bottom of the embed, where Discord shows no
 *  links, so it is written out rather than linked. */
export const PORTRAIT_CREDIT = "Portrait: classicrank.com";

/** One taken spot as an embed: titled with the map's name, linked to its Map Leaderboard, and
 *  showing the Character's portrait (`photo`, its address) as its thumbnail, with the map's
 *  Map Mark and the portrait's credit in the footer; with no portrait, the Map Mark is the
 *  thumbnail, as before portraits. Each Character it names links to their page. The title isn't markdown, so the
 *  map's name goes in as the site writes it ("G* Coconut Season" is one in the map names
 *  fixture); a Character's name is letters and digits only (ADR 0015), so the description
 *  needs no escaping either, in its words or its links. */
function embedFor(taken: Taken, mark: PostedMark | null, photo: string | null): Embed {
  const [first, last] = levelsOf(taken.band);
  const embed: Embed = {
    title: placeOf(PLACES, taken.map).name,
    url: linkTo(taken),
    description:
      `${nameOf(taken.name, true)} took #1 for levels ${first}–${last} ` +
      `with ${rate(taken.expPerHour)} EXP/h${taken.party ? " in a party" : ""}. ${nameOf(taken.from)} held it with ${rate(taken.fromExpPerHour)}.`,
  };
  if (photo !== null) {
    embed.thumbnail = { url: photo };
    embed.footer = mark !== null ? { text: `${mark.name} · ${PORTRAIT_CREDIT}`, icon_url: mark.picture } : { text: PORTRAIT_CREDIT };
  } else if (mark !== null) {
    embed.thumbnail = { url: mark.picture };
  }
  return embed;
}

/** What a Discord webhook is posted. */
export interface Post {
  embeds: Embed[];
  /** Nothing in a post ever pings: a Character's name is its own, not a mention. */
  allowed_mentions: { parse: [] };
}

/** The posts for the spots an upload took, in order, none with none: an embed a spot and no
 *  text, `EMBEDS_PER_POST` to a post. `markOf` gives a map's Map Mark (`null` for a map without
 *  one), and `photo` is the address of the portrait of the Character that took them (`null` for
 *  none). */
export function postsFor(taken: Taken[], markOf: (map: string) => PostedMark | null, photo: string | null = null): Post[] {
  const posts: Post[] = [];
  for (let at = 0; at < taken.length; at += EMBEDS_PER_POST) {
    const told = taken.slice(at, at + EMBEDS_PER_POST);
    posts.push({ embeds: told.map((t) => embedFor(t, markOf(t.map), photo)), allowed_mentions: { parse: [] } });
  }
  return posts;
}

/** Post the taken spots to the Discord webhook, if one is set up (the DISCORD_WEBHOOK_URL
 *  secret), with the portrait of the Character that took them (`photo`, its address, `null`
 *  for none). Several posts go one after another, so they show in order.
 *  Nothing is retried and no failure reaches the upload: the Map Leaderboards are the record,
 *  and a post is only a nicety. A post that can't be sent at all leaves the ones after it unsent;
 *  one Discord turns down (a 429) doesn't. */
export async function tell(env: Env, taken: Taken[], photo: string | null = null): Promise<void> {
  const url = env.DISCORD_WEBHOOK_URL;
  if (!url) return;
  try {
    for (const post of postsFor(taken, (map) => markOf(env, map), photo)) {
      await fetch(url, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(post),
      });
    }
  } catch {
    // A webhook that is gone, rate-limited or slow never fails an upload.
  }
}
