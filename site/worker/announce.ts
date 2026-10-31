// A new #1 on a Map Leaderboard, told to Discord (ADR 0022): when an upload takes a map and
// Level Band's top spot from another Character, the Worker posts it to a Discord webhook, an
// embed for each spot showing the Character's portrait from ClassicRank, or the question-mark
// figure for none (ADR 0029), and the map's Map Mark. Only a spot *taken* is told, so a first
// entry in an empty band, a Character beating its own best, and a top spot that fell to
// someone because an entry was deleted all stay quiet.
import mapNameRows from "../../crates/minerva-core/tests/fixtures/osms-map-names.json";
import noPortrait from "../art/portrait-none.png";
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
  /** The picture beside the spot: the post's file that holds it. */
  thumbnail?: { url: string };
  /** The map's Map Mark beside the area it stands for, when the map has one, then a portrait's
   *  credit to ClassicRank. */
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

/** The picture beside each spot in a post (ADR 0029): the Character's portrait from
 *  ClassicRank, or the site's question-mark figure (site/art) for a Character with none, as a
 *  band's table shows. It goes with the post as a file: given a photo's address, Discord shows
 *  nothing, as it can't load one from ClassicRank. */
export interface Picture {
  bytes: ArrayBuffer;
  /** Its type, and its file's extension for it. */
  type: string;
  extension: string;
  /** It is the Character's portrait from ClassicRank, which the post credits. */
  portrait: boolean;
}

/** The question-mark figure, for a Character ClassicRank has no photo of, or whose photo
 *  couldn't be had. */
export const NO_PORTRAIT: Picture = { bytes: noPortrait, type: "image/png", extension: "png", portrait: false };

/** The name of the file a post's `index`th embed shows its picture from. Each embed has a file
 *  of its own, though the pictures are the same. */
function fileName(index: number, picture: Picture): string {
  return `portrait-${index + 1}.${picture.extension}`;
}

/** One taken spot as an embed: titled with the map's name, linked to its Map Leaderboard, and
 *  showing `picture` (the post's `index`th file) as its thumbnail, with the map's Map Mark beside
 *  its area and a portrait's credit in the footer. With no picture (no ClassicRank key), there
 *  is no thumbnail. Each Character it names links to their page. The title isn't markdown, so
 *  the map's name goes in as the site writes it ("G* Coconut Season" is one in the map names
 *  fixture); a Character's name is letters and digits only (ADR 0015), so the description
 *  needs no escaping either, in its words or its links. */
function embedFor(taken: Taken, mark: PostedMark | null, picture: Picture | null, index: number): Embed {
  const [first, last] = levelsOf(taken.band);
  const embed: Embed = {
    title: placeOf(PLACES, taken.map).name,
    url: linkTo(taken),
    description:
      `${nameOf(taken.name, true)} took #1 for levels ${first}–${last} ` +
      `with ${rate(taken.expPerHour)} EXP/h${taken.party ? " in a party" : ""}. ${nameOf(taken.from)} held it with ${rate(taken.fromExpPerHour)}.`,
  };
  if (picture !== null) embed.thumbnail = { url: `attachment://${fileName(index, picture)}` };
  const credit = picture?.portrait === true;
  if (mark !== null) {
    embed.footer = { text: credit ? `${mark.name} · ${PORTRAIT_CREDIT}` : mark.name, icon_url: mark.picture };
  } else if (credit) {
    embed.footer = { text: PORTRAIT_CREDIT };
  }
  return embed;
}

/** What a Discord webhook is posted, as its `payload_json`, with its files beside it. */
export interface Post {
  embeds: Embed[];
  /** The post's files: each embed's picture. */
  attachments: { id: number; filename: string }[];
  /** Nothing in a post ever pings: a Character's name is its own, not a mention. */
  allowed_mentions: { parse: [] };
}

/** The posts for the spots an upload took, in order, none with none: an embed a spot and no
 *  text, `EMBEDS_PER_POST` to a post. `markOf` gives a map's Map Mark (`null` for a map without
 *  one), and `picture` is what shows beside each spot (`null` for nothing). */
export function postsFor(taken: Taken[], markOf: (map: string) => PostedMark | null, picture: Picture | null = null): Post[] {
  const posts: Post[] = [];
  for (let at = 0; at < taken.length; at += EMBEDS_PER_POST) {
    const told = taken.slice(at, at + EMBEDS_PER_POST);
    posts.push({
      embeds: told.map((t, i) => embedFor(t, markOf(t.map), picture, i)),
      attachments: picture === null ? [] : told.map((_, i) => ({ id: i, filename: fileName(i, picture) })),
      allowed_mentions: { parse: [] },
    });
  }
  return posts;
}

/** The types a portrait may be, and their files' extensions. */
const PICTURE_TYPES: Record<string, string> = { "image/png": "png", "image/jpeg": "jpg", "image/gif": "gif", "image/webp": "webp" };

/** The most a portrait may weigh. ClassicRank's photos are a few KB, and Discord takes 10 MB
 *  of files in a post. */
const MOST_BYTES = 500_000;

/** The picture for a Character whose portrait is at `photo` (`null` for none): their photo,
 *  where ClassicRank's address for it leads, or the question-mark figure when they have none or
 *  it can't be had. Never throws. */
export async function pictureOf(photo: string | null): Promise<Picture> {
  if (photo === null) return NO_PORTRAIT;
  try {
    const response = await fetch(photo, { headers: { "user-agent": "Minerva (https://owlofminerva.app)" } });
    const type = (response.headers.get("content-type") ?? "").split(";")[0].trim().toLowerCase();
    const extension = PICTURE_TYPES[type];
    if (!response.ok || extension === undefined) return NO_PORTRAIT;
    const bytes = await response.arrayBuffer();
    return bytes.byteLength > 0 && bytes.byteLength <= MOST_BYTES ? { bytes, extension, type, portrait: true } : NO_PORTRAIT;
  } catch {
    return NO_PORTRAIT;
  }
}

/** `post` as Discord's webhook takes one with files: its `payload_json`, and each embed's
 *  picture as `files[n]`. */
function formOf(post: Post, picture: Picture | null): FormData {
  const form = new FormData();
  form.append("payload_json", JSON.stringify(post));
  if (picture === null) return form;
  for (const { id, filename } of post.attachments) {
    form.append(`files[${id}]`, new File([picture.bytes], filename, { type: picture.type }));
  }
  return form;
}

/** Post the taken spots to the Discord webhook, if one is set up (the DISCORD_WEBHOOK_URL
 *  secret), with the portrait of the Character that took them (`photo`, its address, `null`
 *  for none) or the question-mark figure; with no ClassicRank key (ADR 0029), with neither.
 *  Several posts go one after another, so they show in order.
 *  Nothing is retried and no failure reaches the upload: the Map Leaderboards are the record,
 *  and a post is only a nicety. A post that can't be sent at all leaves the ones after it unsent;
 *  one Discord turns down (a 429) doesn't. */
export async function tell(env: Env, taken: Taken[], photo: string | null = null): Promise<void> {
  const url = env.DISCORD_WEBHOOK_URL;
  if (!url || taken.length === 0) return;
  const picture = env.CLASSICRANK_KEY ? await pictureOf(photo) : null;
  try {
    for (const post of postsFor(taken, (map) => markOf(env, map), picture)) {
      await fetch(url, { method: "POST", body: formOf(post, picture) });
    }
  } catch {
    // A webhook that is gone, rate-limited or slow never fails an upload.
  }
}
