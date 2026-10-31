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
  /** The Worker made it at upload (ADR 0032), not a player. */
  automatic: boolean;
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

/** The Block that reaches a Character or an Account, as its lookup gives it: `name` is the
 *  Character it was made on. */
export interface BlockReaching {
  id: number;
  blockedAt: number;
  note: string;
  name: string;
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

/** A report as the admin page lists it: what was reported, when and by whom, and why: a
 *  player's words quoted, Minerva's as they are (ADR 0032); "" for no reason given. */
export function reportLine(report: Report, mapName: string): { what: string; when: string; reason: string } {
  const [first, last] = levelsOf(report.band);
  return {
    what: `${report.name} on ${mapName}, levels ${first}–${last}: ${whole(report.expPerHour)} EXP/h${report.party ? " (Party Best)" : ""}`,
    when: `${report.automatic ? "Flagged by Minerva" : "Reported"} ${day(report.reportedAt)}`,
    reason: report.reason === "" || report.automatic ? report.reason : `“${report.reason}”`,
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
    `${count(character.entries.length, "entry", "entries")}, claimed ${day(character.claimedAt)}.`,
    character.account === null ? "No Account." : `Account: ${character.account.username ?? "(no username)"}.`,
    character.refusedAt === null
      ? "No other PC has been refused the name."
      : `Another PC tried to upload as ${character.name} on ${day(character.refusedAt)} and was refused. Reassign to refused key gives that PC the name.`,
  ];
  if (character.blocked !== null) lines.push(blockedLine(character.blocked, character.name));
  return lines;
}

/** A Block that reaches a Character or an Account, `subject` the one looked up: "Blocked 3 Oct
 *  2026: Botting.", or, made on another Character, "Blocked 3 Oct 2026 by the Block on
 *  Speedhax: Botting." */
export function blockedLine(block: BlockReaching, subject: string | null): string {
  const on = block.name === subject ? "" : ` by the Block on ${block.name}`;
  return `Blocked ${day(block.blockedAt)}${on}${block.note === "" ? "" : `: ${block.note}`}.`;
}

/** What Unblock asks before it lifts `block`, from `subject`: it says so when the Block was
 *  made on another Character, since lifting it frees every Character it reaches. */
export function unblockAsk(block: BlockReaching, subject: string | null): string {
  return block.name === subject
    ? `Unblock ${block.name}? Click again`
    : `Lift the Block on ${block.name}? It frees every Character it reaches. Click again`;
}

/** What a call the page made came to, when it didn't go through: words for the codes the API
 *  answers with, or `null` for the site not answering (the page says it can't reach it). */
export function failureText(status: number, error: string | null): string | null {
  if (status === 401) return "You've been signed out. Reload the page to sign in again.";
  if (status === 404) return "That's gone already. The page shows what's there now.";
  switch (error) {
    case "market-off":
      return "Off: the Worker has no maplestory.market key.";
    case "blocked":
      return "A Block reaches it already.";
    case "no-linked-key":
      return "That Account has no PC linked to reassign to.";
    case "no-refused-key":
      return "No other PC has been refused the name since it was last reassigned.";
    case "wrong-origin":
      return "That change has to come from owlofminerva.app itself. Reload the page and try again.";
    default:
      return null;
  }
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

/** A log line's detail in words: an entry's map and band ("100000000 30 solo") as the page
 *  names them, anything else as it is. */
export function detailWords(detail: string, mapName: (map: string) => string): string {
  const entry = /^(\d{9}) (\d+) (solo and party|solo|party)$/.exec(detail);
  if (entry === null) return detail;
  const [first, last] = levelsOf(Number(entry[2]));
  const kind = entry[3] === "solo" ? "Solo Best" : entry[3] === "party" ? "Party Best" : "both bests";
  return `${mapName(entry[1])}, levels ${first}–${last}, ${kind}`;
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
export function logLine(line: LogLine, mapName: (map: string) => string): { what: string; when: string } {
  const whom = [line.name, line.account === null ? null : `Account ${line.account.username ?? line.account.id}`].filter((w) => w !== null);
  const what = [ACTIONS[line.action] ?? line.action, ...whom].join(" · ");
  const detail = detailWords(line.detail, mapName);
  return { what: detail === "" ? what : `${what} (${detail})`, when: `${momentText(line.at)} by ${line.by}` };
}

/** An Account as the Accounts view shows it, opened. */
export function accountLines(account: AccountDetail): string[] {
  const lines = [`${count(account.pcs.length, "PC", "PCs")} linked, signed in with Discord since ${day(account.createdAt)}.`];
  if (account.blocked !== null) lines.push(blockedLine(account.blocked, null));
  return lines;
}

/** What the Overview opens on: the open reports, if there are any. */
export function reportsWaiting(o: Overview): string {
  return o.reports === 0 ? "No open reports." : `${count(o.reports, "open report", "open reports")} to read.`;
}

/** The Overview's counts, a line each. */
export function overviewLines(o: Overview): string[] {
  return [
    `${count(o.characters, "Character", "Characters")} with ${count(o.entries, "entry", "entries")}.`,
    `${count(o.accounts, "Account", "Accounts")}, with ${count(o.pcs, "PC", "PCs")} linked.`,
    `${count(o.removed, "entry", "entries")} taken down, ${count(o.blocks, "Block", "Blocks")}.`,
    `New bests: ${whole(o.newBests.day)} in the last day, ${whole(o.newBests.week)} in the last week.`,
    `Top spots taken: ${whole(o.topsTaken.day)} in the last day, ${whole(o.topsTaken.week)} in the last week.`,
  ];
}

/** When the portraits were last had: off without ClassicRank's key. */
export function portraitsLine(o: Overview): string {
  if (!o.portraits.on) return "Off: the Worker has no ClassicRank key.";
  return o.portraits.checkedAt === null ? "Never asked ClassicRank yet." : `Last asked ClassicRank ${momentText(o.portraits.checkedAt)}.`;
}

/** When each of maplestory.market's tables was last pulled, a line each: off without its key. */
export function marketLines(o: Overview): string[] {
  if (!o.market.on) return ["Off: the Worker has no maplestory.market key."];
  const table = (label: string, pull: Pull | null) =>
    pull === null
      ? `${label}: never asked for.`
      : pull.pulledAt === null
        ? `${label}: never pulled. Last asked for ${momentText(pull.triedAt)}.`
        : `${label}: pulled ${momentText(pull.pulledAt)}.${pull.triedAt > pull.pulledAt ? ` Last asked for ${momentText(pull.triedAt)}.` : ""}`;
  return [table("Going rates", o.market.prices), table("Listings", o.market.listed)];
}

/** An Account the admin page found: its username, and how many PCs it linked. */
export function foundLine(account: FoundAccount): string {
  const pcs = account.pcs === 1 ? "1 PC" : `${account.pcs} PCs`;
  return account.lastUsedAt === null ? `${account.username}: no PC linked` : `${account.username}: ${pcs}, last used ${day(account.lastUsedAt)}`;
}
