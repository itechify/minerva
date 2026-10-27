// Characters' portraits from ClassicRank (ADR 0029): the Worker asks ClassicRank's API for a
// Character's when they upload, and for every Character with entries once a day (the cron
// trigger in site/wrangler.jsonc), and keeps what it answers for the API's reads to give
// beside their name. Only the Worker asks, with its key (the CLASSICRANK_KEY secret); without
// one it never asks, and the reads give no portraits at all. Visitors' browsers load the
// photos straight from classicrank.com.
import type { Portrait, PortraitAnswer } from "../entries";

/** ClassicRank's API, asked for Characters by name. */
const API = "https://classicrank.com/api/v1/characters";

/** How many names one ask holds: as many as a Partner key may. */
export const NAMES_PER_ASK = 300;

/** Where a photo and a profile may be: ClassicRank's own pages. Anything else is taken as
 *  none, since the pages show it as a picture and a link. */
const CLASSICRANK = "https://classicrank.com/";

/** A Character ClassicRank doesn't show: not known there, hidden, or with a name it cuts
 *  short, which might be another's. */
const NOT_SHOWN: Portrait = { photo: null, profile: null };

/** One Character in ClassicRank's answer, as far as it's read. */
interface Found {
  name?: unknown;
  photo?: unknown;
  profile_url?: unknown;
  name_cut_off?: unknown;
}

function onClassicRank(value: unknown): string | null {
  return typeof value === "string" && value.startsWith(CLASSICRANK) ? value : null;
}

/** What ClassicRank shows of `names` (at most NAMES_PER_ASK), by name in lower case, a name it
 *  leaves out as not shown; `null` when it couldn't be asked: busy (a 429), refusing the key,
 *  unreachable, or answering in a shape it doesn't say it does. */
async function ask(key: string, names: string[]): Promise<Map<string, Portrait> | null> {
  const query = new URLSearchParams({ names: names.join(","), fields: "name,photo,profile_url,name_cut_off", limit: String(names.length) });
  let answer: { data?: unknown; meta?: { next?: unknown } };
  try {
    const response = await fetch(`${API}?${query}`, {
      headers: { authorization: `Bearer ${key}`, accept: "application/json", "user-agent": "Minerva (https://owlofminerva.app)" },
    });
    if (!response.ok) return null;
    answer = await response.json();
  } catch {
    return null;
  }
  // Every name's answer fits on the one page, so a page after it is an answer to something
  // else.
  if (!Array.isArray(answer.data) || (answer.meta?.next ?? null) !== null) return null;
  const shown = new Map(names.map((n) => [n.toLowerCase(), NOT_SHOWN]));
  for (const found of answer.data as Found[]) {
    if (typeof found?.name !== "string" || found.name_cut_off === true) continue;
    const nameKey = found.name.toLowerCase();
    if (!shown.has(nameKey)) continue;
    shown.set(nameKey, { photo: onClassicRank(found.photo), profile: onClassicRank(found.profile_url) });
  }
  return shown;
}

/** Keep `shown`, each for a Character that still has entries: one who opted out while
 *  ClassicRank was being asked keeps none. */
async function keep(env: Env, shown: Map<string, Portrait>, now: number): Promise<void> {
  await env.DB.batch(
    [...shown].map(([nameKey, p]) =>
      env.DB.prepare(
        `INSERT INTO portraits (name_key, photo, profile, checked_at)
         SELECT ?1, ?2, ?3, ?4 WHERE EXISTS (SELECT 1 FROM entries WHERE name_key = ?1)
         ON CONFLICT (name_key) DO UPDATE SET photo = excluded.photo, profile = excluded.profile, checked_at = excluded.checked_at`,
      ).bind(nameKey, p.photo, p.profile, now),
    ),
  );
}

/** Ask ClassicRank for `name`'s portrait as they upload, and keep it; `null` when it couldn't
 *  be had (the daily refresh asks again), or with no key. Never throws: an upload never waits
 *  on ClassicRank, nor fails for it. */
export async function askOnUpload(env: Env, name: string, now: number): Promise<Portrait | null> {
  const key = env.CLASSICRANK_KEY;
  if (!key) return null;
  try {
    const shown = await ask(key, [name]);
    if (shown === null) return null;
    await keep(env, shown, now);
    return shown.get(name.toLowerCase()) ?? null;
  } catch {
    return null;
  }
}

/** The daily refresh: ask ClassicRank about every Character with entries, NAMES_PER_ASK at a
 *  time, and keep what it answers, so a changed photo shows within a day, and so does a
 *  Character ClassicRank stops showing. When ClassicRank can't be asked, the rest wait for
 *  tomorrow's, as they were. */
export async function refreshPortraits(env: Env, now: number): Promise<void> {
  const key = env.CLASSICRANK_KEY;
  if (!key) return;
  const rows = await env.DB.prepare(
    "SELECT c.name FROM characters c WHERE EXISTS (SELECT 1 FROM entries e WHERE e.name_key = c.name_key) ORDER BY c.name_key",
  ).all<{ name: string }>();
  const names = rows.results.map((r) => r.name);
  for (let from = 0; from < names.length; from += NAMES_PER_ASK) {
    const shown = await ask(key, names.slice(from, from + NAMES_PER_ASK));
    if (shown === null) return;
    await keep(env, shown, now);
  }
}

/** Forget `nameKey`'s portrait, as they opt out. */
export function forgetPortrait(env: Env, nameKey: string): D1PreparedStatement {
  return env.DB.prepare("DELETE FROM portraits WHERE name_key = ?").bind(nameKey);
}

/** The columns a read takes a Character's portrait from, joined to `characters c` as `p`. */
export const PORTRAIT_COLUMNS = "p.photo AS portrait_photo, p.profile AS portrait_profile, p.checked_at AS portrait_checked_at";
export const PORTRAIT_JOIN = "LEFT JOIN portraits p ON p.name_key = c.name_key";

/** A read's row's portrait columns. */
export interface PortraitColumns {
  portrait_photo: string | null;
  portrait_profile: string | null;
  portrait_checked_at: number | null;
}

/** A Character's portrait as an answer gives it, from a read's row: `{ portrait }`, `null`
 *  until ClassicRank has been asked; nothing at all while the Worker has no key. */
export function portraitIn(env: Env, row: PortraitColumns): { portrait?: PortraitAnswer } {
  if (!env.CLASSICRANK_KEY) return {};
  if (row.portrait_checked_at === null) return { portrait: null };
  return { portrait: { photo: row.portrait_photo, profile: row.portrait_profile } };
}
