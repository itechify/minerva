// The account pages (site/account, site/admin), worked out as plain data: what they say of
// an Account, a linked PC, a link token and a report, from the API's answers (ADR 0023), and
// of the maintainer's tools (ADR 0031). The pages' scripts only fetch and draw what this gives
// them.
import { dayText, momentText } from "../../src/dates.ts";
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
  /** It is against a Party Best's entry, not a Solo Best's. */
  party: boolean;
}

/** An entry taken down, as GET /api/admin/removed lists it. */
export interface Removed {
  name: string;
  map: string;
  band: number;
  expPerHour: number;
  /** It was a Party Best's entry, not a Solo Best's. */
  party: boolean;
}

/** An entry of a Character's, as the maintainer's lookup lists it, hidden by a Block or not. */
export interface CharacterEntry {
  map: string;
  band: number;
  party: boolean;
  level: number;
  job: string;
  expPerHour: number;
  setAt: number;
}

/** A line of the maintainer's log, as GET /api/admin/log lists it. */
export interface LogLine {
  id: number;
  at: number;
  /** The maintainer's Discord username, or "token". */
  by: string;
  action: string;
  name: string | null;
  account: { id: number; username: string | null } | null;
  detail: string;
}

/** The Block that reaches a Character, as its lookup gives it. */
export interface BlockReaching {
  id: number;
  blockedAt: number;
  note: string;
}

/** GET /api/admin/characters/<name>'s answer. */
export interface CharacterAnswer {
  name: string;
  claimedAt: number;
  refusedAt: number | null;
  account: { id: number; username: string | null } | null;
  entries: CharacterEntry[];
  blocked: BlockReaching | null;
  log: LogLine[];
}

/** A Block, as GET /api/admin/blocks lists it. */
export interface Block {
  id: number;
  name: string;
  blocksName: boolean;
  account: { id: number; username: string | null } | null;
  keys: number;
  note: string;
  blockedAt: number;
}

/** GET /api/admin/accounts/<id>'s answer. */
export interface AccountDetail {
  id: number;
  username: string | null;
  createdAt: number;
  pcs: Pc[];
  characters: string[];
  blocked: BlockReaching | null;
}

/** When one of maplestory.market's tables was last pulled, and last asked for. */
export interface Pull {
  pulledAt: number | null;
  triedAt: number;
}

/** GET /api/admin/overview's answer. */
export interface Overview {
  characters: number;
  entries: number;
  accounts: number;
  pcs: number;
  reports: number;
  removed: number;
  blocks: number;
  newBests: { day: number; week: number };
  topsTaken: { day: number; week: number };
  portraits: { on: boolean; checkedAt: number | null };
  market: { on: boolean; prices: Pull | null; listed: Pull | null };
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
        ? "It's linked to another Discord account now. Linking moves it to this one. Only link it if you just clicked Link Discord in Minerva."
        : "Only link it if you just clicked Link Discord in Minerva.",
  };
}

/** A report as the admin page lists it: what was reported, and when. */
export function reportLine(report: Report, mapName: string): { what: string; when: string } {
  const [first, last] = levelsOf(report.band);
  return {
    what: `${report.name} on ${mapName}, levels ${first}–${last}: ${whole(report.expPerHour)} EXP/h${report.party ? " (Party Best)" : ""}`,
    when: `Reported ${day(report.reportedAt)}`,
  };
}

/** An entry taken down as the admin page lists it: whose, where, and the Visit kept off. */
export function removedLine(removed: Removed, mapName: string): string {
  const [first, last] = levelsOf(removed.band);
  return `${removed.name} on ${mapName}, levels ${first}–${last}: ${whole(removed.expPerHour)} EXP/h${removed.party ? " (Party Best)" : ""}`;
}

/** How many of something: "1 entry", "3 entries". */
function count(n: number, one: string, many: string): string {
  return `${whole(n)} ${n === 1 ? one : many}`;
}

/** A name as the admin page shows it, looked up: who holds it, when another key was last
 *  refused for it (the key "Reassign to refused key" moves it to), and its Block. */
export function characterLines(character: CharacterAnswer): string[] {
  const lines = [
    `${character.name}: ${count(character.entries.length, "entry", "entries")}, claimed ${day(character.claimedAt)}.`,
    character.account === null ? "No Account." : `Account: ${character.account.username ?? "(no username)"}.`,
    character.refusedAt === null ? "No other key refused." : `Another key was last refused ${day(character.refusedAt)}.`,
  ];
  if (character.blocked !== null) lines.push(blockedLine(character.blocked));
  return lines;
}

/** A Block that reaches a Character or an Account: "Blocked 3 Oct 2026: Botting." Its entries
 *  are hidden while it stands. */
export function blockedLine(block: BlockReaching): string {
  return `Blocked ${day(block.blockedAt)}${block.note === "" ? "" : `: ${block.note}`}. Its entries are hidden.`;
}

/** An entry as the maintainer's lookup lists it: where, how fast, and when it was set. */
export function entryLine(entry: CharacterEntry, mapName: string): { what: string; when: string } {
  const [first, last] = levelsOf(entry.band);
  return {
    what: `${mapName}, levels ${first}–${last}: ${whole(entry.expPerHour)} EXP/h${entry.party ? " (Party Best)" : ""}`,
    when: `Lv ${entry.level} ${entry.job}, set ${day(entry.setAt)}`,
  };
}

/** A Block as the Characters view lists it: whose, what it reaches, and when it was made. */
export function blockListLine(block: Block): { what: string; when: string } {
  const reaches = [
    block.account === null ? "no Account" : `Account ${block.account.username ?? "(deleted)"}`,
    count(block.keys, "key", "keys"),
    ...(block.blocksName ? ["the name too"] : []),
  ];
  return {
    what: `${block.name}: ${reaches.join(", ")}`,
    when: `Blocked ${day(block.blockedAt)}${block.note === "" ? "" : `: ${block.note}`}`,
  };
}

/** What each of the log's actions reads as. */
const ACTIONS: Record<string, string> = {
  dismiss: "Dismissed a report",
  "take-down": "Took an entry down",
  "take-all-down": "Took all entries down",
  restore: "Restored an entry",
  reassign: "Reassigned",
  block: "Blocked",
  unblock: "Unblocked",
  "unlink-pc": "Unlinked a PC",
  "delete-account": "Deleted an Account",
  "refresh-portraits": "Refreshed portraits",
  "pull-market": "Pulled market prices",
};

/** A line of the log as the Log view lists it: what was done to whom, and by whom when. */
export function logLine(line: LogLine): { what: string; when: string } {
  const whom = [line.name, line.account === null ? null : `Account ${line.account.username ?? line.account.id}`].filter((w) => w !== null);
  const what = [ACTIONS[line.action] ?? line.action, ...whom].join(" · ");
  return { what: line.detail === "" ? what : `${what} (${line.detail})`, when: `${momentText(line.at)} by ${line.by}` };
}

/** An Account as the Accounts view shows it, opened. */
export function accountLines(account: AccountDetail): string[] {
  const lines = [
    `${account.username ?? "(no username)"}: ${count(account.pcs.length, "PC", "PCs")}, since ${day(account.createdAt)}.`,
    account.characters.length === 0 ? "No Characters." : `Characters: ${listed(account.characters)}.`,
  ];
  if (account.blocked !== null) lines.push(blockedLine(account.blocked));
  return lines;
}

/** The Overview's counts, a line each. */
export function overviewLines(o: Overview): string[] {
  return [
    `${count(o.characters, "Character", "Characters")} with ${count(o.entries, "entry", "entries")}.`,
    `${count(o.accounts, "Account", "Accounts")}, with ${count(o.pcs, "PC", "PCs")} linked.`,
    `${count(o.reports, "open report", "open reports")}, ${count(o.removed, "entry", "entries")} taken down, ${count(o.blocks, "Block", "Blocks")}.`,
    `New bests: ${whole(o.newBests.day)} in the last day, ${whole(o.newBests.week)} in the last week.`,
    `Top spots taken: ${whole(o.topsTaken.day)} in the last day, ${whole(o.topsTaken.week)} in the last week.`,
  ];
}

/** When the portraits were last had: off without ClassicRank's key. */
export function portraitsLine(o: Overview): string {
  if (!o.portraits.on) return "Off: the Worker has no ClassicRank key.";
  return o.portraits.checkedAt === null ? "Never asked yet." : `Last asked ${momentText(o.portraits.checkedAt)}.`;
}

/** When each of maplestory.market's tables was last pulled: off without its key. */
export function marketLine(o: Overview): string {
  if (!o.market.on) return "Off: the Worker has no maplestory.market key.";
  const table = (label: string, pull: Pull | null) =>
    pull === null
      ? `${label} never asked`
      : `${label} pulled ${pull.pulledAt === null ? "never" : momentText(pull.pulledAt)}, last asked ${momentText(pull.triedAt)}`;
  return `${table("Going rates", o.market.prices)}. ${table("Listings", o.market.listed)}.`;
}

/** An Account the admin page found: its username, and how many PCs it linked. */
export function foundLine(account: FoundAccount): string {
  const pcs = account.pcs === 1 ? "1 PC" : `${account.pcs} PCs`;
  return account.lastUsedAt === null ? `${account.username}: no PC linked` : `${account.username}: ${pcs}, last used ${day(account.lastUsedAt)}`;
}
