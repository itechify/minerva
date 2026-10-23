// The account pages (site/account, site/admin), worked out as plain data: what they say of
// an Account, a linked PC, a link token and a report, from the API's answers (ADR 0023). The
// pages' scripts only fetch and draw what this gives them.
import { dayText } from "../../src/dates.ts";
import { whole } from "../../src/figures.ts";
import { levelsOf } from "../level-bands.ts";

/** A PC linked to an Account, as GET /api/account lists it. */
export interface Pc {
  id: number;
  linkedAt: number;
  lastUsedAt: number;
  characters: string[];
}

/** GET /api/account's answer. */
export interface AccountAnswer {
  username: string;
  maintainer: boolean;
  pcs: Pc[];
}

/** GET /api/link/tokens/<token>'s answer: the Characters the token's key holds, and whether
 *  it's linked already, to this Account or another. */
export interface TokenAnswer {
  characters: string[];
  linked: "this" | "other" | null;
}

/** A report, as GET /api/admin/reports lists it. */
export interface Report {
  id: number;
  name: string;
  map: string;
  band: number;
  expPerHour: number;
  reason: string;
  reportedAt: number;
}

/** GET /api/admin/characters/<name>'s answer. */
export interface CharacterAnswer {
  name: string;
  claimedAt: number;
  refusedAt: number | null;
  account: { id: number; username: string | null } | null;
  entries: number;
}

/** An Account GET /api/admin/accounts finds. */
export interface FoundAccount {
  id: number;
  username: string;
  pcs: number;
  lastUsedAt: number | null;
}

/** Names as a sentence lists them: "Egbert", "Egbert and Bob", "Egbert, Bob and Cid". */
export function listed(names: string[]): string {
  if (names.length <= 1) return names.join("");
  return `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}

/** What a PC holds: "Holds Egbert and Bob.", or "Holds no Characters yet." */
export function holds(characters: string[]): string {
  return characters.length === 0 ? "Holds no Characters yet." : `Holds ${listed(characters)}.`;
}

/** A day as the pages write it: "3 Oct 2026". */
function day(unixMs: number): string {
  return dayText(new Date(unixMs), true);
}

/** When a PC was linked and last uploaded: "Linked 3 Oct 2026 · last used 21 Oct 2026". */
export function pcDates(pc: Pc): string {
  return `Linked ${day(pc.linkedAt)} · last used ${day(pc.lastUsedAt)}`;
}

/** The address that signs in with Discord and comes back to `back`, a path on the site. */
export function signInHref(back: string): string {
  return `/api/auth/discord?return=${encodeURIComponent(back)}`;
}

/** The link token a confirmation page's address (its `location.search`) carries, if any. */
export function tokenIn(search: string): string | null {
  const token = new URLSearchParams(search).get("t");
  return token !== null && /^[A-Za-z0-9_-]{20,}$/.test(token) ? token : null;
}

/** Whether a page's address says a sign-in didn't go through. */
export function signInFailed(search: string): boolean {
  return new URLSearchParams(search).get("signin") === "failed";
}

/** What the confirmation page asks, for the Discord account `username` and the token's key. */
export function linkQuestion(username: string, token: TokenAnswer): { ask: string; holds: string; warn: string } {
  return {
    ask: `Link this Minerva to your Discord account ${username}?`,
    holds:
      token.characters.length === 0
        ? "It holds no Characters on the Map Leaderboards yet."
        : `It holds ${listed(token.characters)}.`,
    warn:
      token.linked === "other"
        ? "It's linked to another Discord account now, and moves to this one. Only link it if you just clicked Link Discord in Minerva."
        : "Only link it if you just clicked Link Discord in Minerva.",
  };
}

/** A report as the admin page lists it: what was reported, and when. */
export function reportLine(report: Report, mapName: string): { what: string; when: string } {
  const [first, last] = levelsOf(report.band);
  return {
    what: `${report.name} on ${mapName}, levels ${first}–${last}: ${whole(report.expPerHour)} EXP/h`,
    when: `Reported ${day(report.reportedAt)}`,
  };
}

/** A name as the admin page shows it, looked up: who holds it, and when another key was
 *  last refused for it (the key "Reassign to refused key" moves it to). */
export function characterLines(character: CharacterAnswer): string[] {
  const entries = character.entries === 1 ? "1 entry" : `${character.entries} entries`;
  return [
    `${character.name}: ${entries}, claimed ${day(character.claimedAt)}.`,
    character.account === null ? "No Account." : `Account: ${character.account.username ?? "(no username)"}.`,
    character.refusedAt === null ? "No other key refused." : `Another key was last refused ${day(character.refusedAt)}.`,
  ];
}

/** An Account the admin page found: its username, and how many PCs it linked. */
export function foundLine(account: FoundAccount): string {
  const pcs = account.pcs === 1 ? "1 PC" : `${account.pcs} PCs`;
  return account.lastUsedAt === null ? `${account.username}: no PC linked` : `${account.username}: ${pcs}, last used ${day(account.lastUsedAt)}`;
}
