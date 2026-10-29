// The maintainer's page (admin/index.html, ADR 0023 and ADR 0031): Overview, the open reports,
// the counts, two buttons and every Block; Reports, each opened, looked up, dismissed or taken
// down; Characters, a name looked up, reassigned, its entries taken down, or blocked; Accounts,
// one looked up, a PC unlinked or the Account deleted; and Log. The view, and the Character or
// Account open in it, is the address's (#/characters/Egbert), so a reload, Back or a link keeps
// the place. What it says is worked out in account/account.ts.
import { make } from "../../src/dom.ts";
import {
  type AccountAnswer,
  type AccountDetail,
  accountLines,
  type Block,
  blockListLine,
  type CharacterAnswer,
  characterLines,
  entryLine,
  failureText,
  type FoundAccount,
  foundLine,
  holds,
  type LogLine,
  logLine,
  marketLines,
  type Overview,
  overviewLines,
  pcDates,
  portraitsLine,
  type Removed,
  removedLine,
  type Report,
  reportLine,
  reportsWaiting,
  signInFailed,
  unblockAsk,
} from "../account/account.ts";
import { armed, type Called, call, send, UNREACHABLE } from "../account/api.ts";
import { names } from "../leaderboards/api.ts";
import { linkTo, placeOf } from "../leaderboards/leaderboards.ts";

const byId = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const said = byId("said");
const mapName = (map: string) => placeOf(names, map).name;

/** What the page says of a call that didn't go through. */
function failed(called: Called<unknown>): string {
  const error = (called.body as { error?: unknown } | null)?.error;
  return failureText(called.status, typeof error === "string" ? error : null) ?? UNREACHABLE;
}

/** Say `words` on `line`, or what went wrong with `called` if it isn't `ok`. */
function tell(line: HTMLElement, called: Called<unknown>, ok: number, words: string) {
  line.textContent = called.status === ok ? words : failed(called);
}

/** A row of a list: its words at the left, its buttons and links at the right. */
function row(what: string, when: string | null, ...buttons: HTMLElement[]): HTMLLIElement {
  const item = make("li", "account-item");
  const words = make("div", "account-words");
  words.append(make("strong", "", what));
  if (when !== null) words.append(make("span", "", when));
  item.append(words);
  if (buttons.length > 0) {
    const actions = make("div", "account-actions");
    actions.append(...buttons);
    item.append(actions);
  }
  return item;
}

function button(label: string, title?: string): HTMLButtonElement {
  const b = make("button", "button small secondary", label);
  b.type = "button";
  if (title !== undefined) b.title = title;
  return b;
}

/** A small button-styled link: to a place on this page, or, `away`, a page in a new tab. */
function link(label: string, href: string, away = false): HTMLAnchorElement {
  const a = make("a", "button small secondary", label);
  a.href = href;
  if (away) {
    a.target = "_blank";
    a.rel = "noopener";
  }
  return a;
}

// ---- Where the page is: the address ----

type View = "overview" | "reports" | "characters" | "accounts" | "log";
const VIEWS: View[] = ["overview", "reports", "characters", "accounts", "log"];

/** A place on the page: a view, and the Character or Account open in it. */
interface Place {
  view: View;
  name?: string;
  account?: number;
}

function placeIn(hash: string): Place {
  const [view, rest] = decodeURIComponent(hash.replace(/^#\/?/, "")).split("/");
  if (view === "characters" && rest) return { view, name: rest };
  if (view === "accounts" && /^\d+$/.test(rest ?? "")) return { view, account: Number(rest) };
  return { view: VIEWS.includes(view as View) ? (view as View) : "overview" };
}

function hashOf(place: Place): string {
  if (place.name !== undefined) return `#/characters/${encodeURIComponent(place.name)}`;
  if (place.account !== undefined) return `#/accounts/${place.account}`;
  return `#/${place.view}`;
}

const characterHref = (name: string) => hashOf({ view: "characters", name });
const accountHref = (id: number) => hashOf({ view: "accounts", account: id });

/** Go to `place`: the address changes, and `render` draws it. Going where the page is already
 *  reads it again. */
function go(place: Place) {
  const hash = hashOf(place);
  if (location.hash === hash) render();
  else location.hash = hash;
}

const tabs = Object.fromEntries(VIEWS.map((view) => [view, byId<HTMLButtonElement>(`show-${view}`)])) as Record<View, HTMLButtonElement>;
/** The Character and Account last open, so their tabs open on them again. */
let lastName: string | undefined;
let lastAccount: number | undefined;

for (const view of VIEWS) {
  tabs[view].addEventListener("click", () =>
    go({ view, name: view === "characters" ? lastName : undefined, account: view === "accounts" ? lastAccount : undefined }),
  );
}
// The tabs as a tab list: the arrows, Home and End move between them, and Tab leaves them.
byId("views").addEventListener("keydown", (event) => {
  const at = VIEWS.findIndex((view) => tabs[view] === document.activeElement);
  if (at < 0) return;
  const to = { ArrowRight: at + 1, ArrowLeft: at - 1, Home: 0, End: VIEWS.length - 1 }[event.key];
  if (to === undefined) return;
  event.preventDefault();
  const view = VIEWS[(to + VIEWS.length) % VIEWS.length];
  tabs[view].focus();
  tabs[view].click();
});

/** Draw the place the address names: its tab pressed, its view shown and read afresh. */
async function render() {
  const place = placeIn(location.hash);
  for (const view of VIEWS) {
    const on = view === place.view;
    tabs[view].setAttribute("aria-selected", String(on));
    tabs[view].tabIndex = on ? 0 : -1;
    byId(view).hidden = !on;
  }
  switch (place.view) {
    case "overview":
      return loadOverview();
    case "reports":
      return loadReports();
    case "characters":
      if (place.name === undefined) {
        byId("character").hidden = true;
        characterSaid.textContent = "";
        return;
      }
      lastName = place.name;
      characterInput.value = place.name;
      return lookUp(place.name, true);
    case "accounts":
      if (place.account === undefined) {
        byId("opened").hidden = true;
        return;
      }
      lastAccount = place.account;
      return openAccount(place.account, true);
    case "log":
      return loadLog();
  }
}
addEventListener("hashchange", render);

// ---- Overview ----

async function loadOverview() {
  const line = byId("overview-said");
  const read = await call<Overview>("/api/admin/overview");
  if (read.status !== 200 || read.body === null) {
    line.textContent = failed(read);
    return;
  }
  const overview = read.body;
  line.textContent = "";
  const waiting = byId("waiting");
  waiting.replaceChildren(reportsWaiting(overview));
  if (overview.reports > 0) waiting.append(link("Read them", hashOf({ view: "reports" })));
  byId("overview-lines").replaceChildren(...overviewLines(overview).map((text) => make("li", "", text)));
  byId("portraits-said").textContent = portraitsLine(overview);
  byId("market-words").replaceChildren(make("strong", "", "Market prices"), ...marketLines(overview).map((text) => make("span", "", text)));
  byId<HTMLButtonElement>("refresh-portraits").disabled = !overview.portraits.on;
  byId<HTMLButtonElement>("pull-market").disabled = !overview.market.on;
  await loadBlocks();
}

/** A button that runs one of the Worker's jobs at once, then reads the Overview again. */
function job(id: string, path: string, busy: string, done: string) {
  const b = byId<HTMLButtonElement>(id);
  const label = b.textContent ?? "";
  b.addEventListener("click", async () => {
    b.disabled = true;
    b.textContent = busy;
    const ran = await send(path, "POST");
    b.textContent = label;
    await loadOverview();
    tell(byId("overview-said"), ran, 204, done);
  });
}
job("refresh-portraits", "/api/admin/portraits", "Refreshing…", "Portraits refreshed.");
job("pull-market", "/api/admin/market", "Pulling…", "Market prices pulled.");

/** Every Block, the newest first, each with Look up and Unblock. */
async function loadBlocks() {
  const line = byId("blocks-said");
  const read = await call<{ blocks: Block[] }>("/api/admin/blocks");
  if (read.status !== 200 || read.body === null) {
    line.textContent = failed(read);
    return;
  }
  const blocks = read.body.blocks;
  line.textContent = blocks.length === 0 ? "Nobody's blocked. Block a player from their Character, under Characters." : "";
  byId("block-list").replaceChildren(
    ...blocks.map((block) => {
      const { what, when } = blockListLine(block);
      const lift = button("Unblock");
      armed(lift, unblockAsk({ ...block }, block.name), async () => {
        const done = await send(`/api/admin/blocks/${block.id}`, "DELETE");
        await loadBlocks();
        tell(line, done, 204, `${block.name} unblocked.`);
      });
      return row(what, when, link("Look up", characterHref(block.name)), lift);
    }),
  );
}

// ---- Reports ----

/** Draw every report, newest first, each with Open (its board, in a new tab), Look up,
 *  Dismiss (the report goes) and Take entry down (the entry goes too, and the same Visit stays
 *  off until restored); and, on its own, every entry taken down. */
async function loadReports() {
  const line = byId("reports-said");
  if (byId("report-list").childElementCount === 0) line.textContent = "Loading…";
  await Promise.all([drawReports(line), loadRemoved()]);
}

async function drawReports(line: HTMLElement) {
  const read = await call<{ reports: Report[] }>("/api/admin/reports");
  if (read.status !== 200 || read.body === null) {
    line.textContent = failed(read);
    return;
  }
  const reports = read.body.reports;
  line.textContent = reports.length === 0 ? "No reports." : reports.length === 1 ? "1 report." : `${reports.length} reports.`;
  byId("report-list").replaceChildren(
    ...reports.map((report) => {
      const { what, when } = reportLine(report, mapName(report.map));
      const item = make("li", "account-item");
      const words = make("div", "account-words");
      words.append(make("strong", "", what), make("span", "", when));
      if (report.reason !== "") words.append(make("span", "account-reason", `“${report.reason}”`));
      const open = link("Open", linkTo({ map: report.map, band: report.band, name: report.name, kind: report.party ? "party" : "solo" }), true);
      open.title = "Its Map Leaderboard, in a new tab";
      const dismiss = button("Dismiss", "Removes the report. The entry stays on the Map Leaderboard.");
      dismiss.addEventListener("click", async () => {
        const done = await send(`/api/admin/reports/${report.id}`, "DELETE");
        await drawReports(line);
        if (done.status !== 204) line.textContent = failed(done);
      });
      const takeDown = button("Take entry down", "Removes the entry and its reports. The same visit stays off until you restore it under Taken down.");
      armed(takeDown, `Take ${report.name}'s entry down? Click again`, async () => {
        const kind = report.party ? "party" : "solo";
        const path = `/api/admin/entries/${encodeURIComponent(report.name)}/${report.map}/${report.band}?kind=${kind}`;
        const done = await send(path, "DELETE");
        await loadReports();
        tell(line, done, 204, `${report.name}'s entry taken down. It's under Taken down.`);
      });
      const actions = make("div", "account-actions");
      actions.append(open, link("Look up", characterHref(report.name)), dismiss, takeDown);
      item.append(words, actions);
      return item;
    }),
  );
}

/** Draw every entry taken down, each with Restore: the same Visit goes up again at its next
 *  upload. */
async function loadRemoved() {
  const line = byId("removed-said");
  const read = await call<{ removed: Removed[] }>("/api/admin/removed");
  if (read.status !== 200 || read.body === null) {
    line.textContent = failed(read);
    return;
  }
  const removed = read.body.removed;
  line.textContent =
    removed.length === 0 ? "None. An entry you take down stays off until you restore it here." : "Each stays off the Map Leaderboards until you restore it.";
  byId("removed-list").replaceChildren(
    ...removed.map((entry) => {
      const restore = button("Restore", `${entry.name}'s next upload of this visit puts it back`);
      restore.addEventListener("click", async () => {
        const kind = entry.party ? "party" : "solo";
        const path = `/api/admin/removed/${encodeURIComponent(entry.name)}/${entry.map}/${entry.band}/${entry.expPerHour}?kind=${kind}`;
        const done = await send(path, "DELETE");
        await loadRemoved();
        tell(line, done, 204, `Restored: ${entry.name}'s next upload of that visit puts it back.`);
      });
      return row(removedLine(entry, mapName(entry.map)), null, link("Look up", characterHref(entry.name)), restore);
    }),
  );
}

// ---- Characters ----

const characterSaid = byId("character-said");
const characterInput = byId<HTMLInputElement>("character-name");
/** The Character open, as last read. */
let looked: CharacterAnswer | null = null;

/** Each group's line, where what its buttons did is said, beside them. */
const groupSaid = { name: byId("name-said"), entries: byId("entries-said"), block: byId("block-said") };

byId("character-form").addEventListener("submit", (event) => {
  event.preventDefault();
  const name = characterInput.value.trim();
  if (name !== "") go({ view: "characters", name });
});

/** Read `name` and draw what the maintainer can do with it, each group's line saying where it
 *  stands until an action says what it did. `arriving` (a new name, from the address) brings
 *  its heading into focus; after an action, the Character stays drawn while it's read again,
 *  so the page doesn't jump. */
async function lookUp(name: string, arriving = false) {
  const same = looked?.name.toLowerCase() === name.toLowerCase();
  if (arriving && !same) {
    byId("character").hidden = true;
    groupSaid.name.textContent = "";
    byId("account-list").replaceChildren();
    closeBlockFields();
    characterSaid.textContent = "Loading…";
  }
  const read = await call<CharacterAnswer>(`/api/admin/characters/${encodeURIComponent(name)}`);
  if (read.status === 404) {
    looked = null;
    byId("character").hidden = true;
    characterSaid.textContent = `No Character named ${name} has uploaded.`;
    return;
  }
  if (read.status !== 200 || read.body === null) {
    characterSaid.textContent = failed(read);
    return;
  }
  looked = read.body;
  characterSaid.textContent = "";
  drawCharacter(looked);
  byId("character").hidden = false;
  if (arriving && !same) byId("character-heading").focus();
}

function drawCharacter(character: CharacterAnswer) {
  byId("character-title").textContent = character.name;
  const tag = byId("character-tag");
  tag.hidden = character.blocked === null;
  tag.className = "tag blocked";
  tag.textContent = "Blocked";
  byId("character-lines").replaceChildren(...characterLines(character).map((text) => make("li", "", text)));
  byId<HTMLButtonElement>("reassign-refused").disabled = character.refusedAt === null;
  byId("open-account").hidden = character.account === null;
  drawEntries(character);
  drawBlock(character);
  byId("character-log-said").textContent = character.log.length === 0 ? "Nothing done to it here yet." : "";
  byId("character-log").replaceChildren(...character.log.map((l) => logRow(l, false)));
}

/** The Character's entries, each with Open and Take entry down. */
function drawEntries(character: CharacterAnswer) {
  const n = character.entries.length;
  const take = byId<HTMLButtonElement>("take-all-down");
  take.disabled = n === 0;
  take.title = "Each comes back with Restore, under Reports, Taken down.";
  groupSaid.entries.textContent =
    n === 0 ? "No entries on the Map Leaderboards." : character.blocked !== null ? "Hidden from the Map Leaderboards while the Block stands." : "";
  byId("entry-list").replaceChildren(
    ...character.entries.map((entry) => {
      const { what, when } = entryLine(entry, mapName(entry.map));
      const kind = entry.party ? "party" : "solo";
      const open = link("Open", linkTo({ map: entry.map, band: entry.band, name: character.name, kind }), true);
      open.title = "Its Map Leaderboard, in a new tab";
      const takeDown = button("Take entry down", "Removes the entry. The same visit stays off until you restore it under Reports, Taken down.");
      armed(takeDown, `Take ${character.name}'s entry on ${mapName(entry.map)} down? Click again`, async () => {
        const path = `/api/admin/entries/${encodeURIComponent(character.name)}/${entry.map}/${entry.band}?kind=${kind}`;
        const done = await send(path, "DELETE");
        await lookUp(character.name);
        tell(groupSaid.entries, done, 204, "Taken down. It's under Reports, Taken down, to restore.");
      });
      return row(what, when, open, takeDown);
    }),
  );
}

armed(
  byId<HTMLButtonElement>("take-all-down"),
  () => {
    const n = looked?.entries.length ?? 0;
    return n === 1 ? `Take ${looked?.name}'s only entry down? Click again` : `Take all ${n} of ${looked?.name}'s entries down? Click again`;
  },
  async () => {
    if (looked === null) return;
    const name = looked.name;
    const done = await send(`/api/admin/entries/${encodeURIComponent(name)}`, "DELETE");
    await lookUp(name);
    tell(groupSaid.entries, done, 204, `Every entry of ${name}'s taken down. Each is under Reports, Taken down, to restore.`);
  },
);

/** "Block…", which opens the note and the name's checkbox, for a Character no Block reaches;
 *  Unblock for one that does. */
function drawBlock(character: CharacterAnswer) {
  const blocked = character.blocked !== null;
  groupSaid.block.textContent = blocked
    ? "Its uploads are refused and its entries are hidden. Unblock puts them back where they were."
    : `Blocking refuses uploads from ${character.account === null ? "the PC that holds the name" : `${character.account.username}'s Account, every PC linked to it,`} and hides every Character they hold. The entries stay, for Unblock.`;
  byId("block-open").hidden = blocked;
  byId("unblock").hidden = !blocked;
  if (blocked) closeBlockFields();
}

function closeBlockFields() {
  byId("block-fields").hidden = true;
  byId("block-open").setAttribute("aria-expanded", "false");
}

byId("block-open").addEventListener("click", () => {
  const open = byId("block-fields").hidden;
  byId("block-fields").hidden = !open;
  byId("block-open").setAttribute("aria-expanded", String(open));
  if (open) byId("block-note").focus();
});

armed(
  byId<HTMLButtonElement>("block"),
  () => {
    const account = looked?.account?.username;
    const nameToo = byId<HTMLInputElement>("block-name").checked ? ", and the name itself" : "";
    return `Block ${looked?.name}${account ? ` and ${account}'s Account` : ""}${nameToo}? Click again`;
  },
  async () => {
    if (looked === null) return;
    const name = looked.name;
    const done = await send(`/api/admin/characters/${encodeURIComponent(name)}/block`, "POST", {
      name: byId<HTMLInputElement>("block-name").checked,
      note: byId<HTMLTextAreaElement>("block-note").value,
    });
    if (done.status === 201) {
      byId<HTMLTextAreaElement>("block-note").value = "";
      byId<HTMLInputElement>("block-name").checked = false;
    }
    await lookUp(name);
    tell(groupSaid.block, done, 201, `${name} blocked. Its entries are hidden.`);
  },
);

armed(
  byId<HTMLButtonElement>("unblock"),
  () => (looked?.blocked ? unblockAsk(looked.blocked, looked.name) : "Unblock? Click again"),
  async () => {
    if (looked?.blocked == null) return;
    const name = looked.name;
    const done = await send(`/api/admin/blocks/${looked.blocked.id}`, "DELETE");
    await lookUp(name);
    tell(groupSaid.block, done, 204, `Unblocked. ${name}'s entries are back.`);
  },
);

/** Reassign the name open: to the last key refused for it, or to `account`. */
async function reassign(account?: number) {
  if (looked === null) return;
  const name = looked.name;
  const done = await send(`/api/admin/characters/${encodeURIComponent(name)}/reassign`, "POST", account === undefined ? undefined : { account });
  await lookUp(name);
  tell(groupSaid.name, done, 200, `${name} reassigned. Tell the player to press Send again in Minerva.`);
}
armed(
  byId<HTMLButtonElement>("reassign-refused"),
  () => `Give ${looked?.name} to the PC refused it? Click again`,
  () => reassign(),
);

byId("open-account").addEventListener("click", () => {
  if (looked?.account) go({ view: "accounts", account: looked.account.id });
});

/** The Accounts whose username holds what's in `input`, drawn in `list`, each with `act`. */
async function findAccounts(input: string, list: HTMLElement, act: (account: FoundAccount) => HTMLElement) {
  const query = byId<HTMLInputElement>(input).value.trim();
  if (query === "") return;
  const found = await call<{ accounts: FoundAccount[] }>(`/api/admin/accounts?username=${encodeURIComponent(query)}`);
  if (found.status !== 200 || found.body === null) {
    list.replaceChildren(make("li", "", failed(found)));
    return;
  }
  if (found.body.accounts.length === 0) {
    list.replaceChildren(make("li", "", "No Account has that in its username."));
    return;
  }
  list.replaceChildren(...found.body.accounts.map((account) => row(foundLine(account), null, act(account))));
}

byId("account-form").addEventListener("submit", (event) => {
  event.preventDefault();
  findAccounts("account-query", byId("account-list"), (account) => {
    const to = button("Reassign here");
    to.disabled = account.pcs === 0;
    if (to.disabled) to.title = "It has no PC linked to reassign to";
    armed(to, () => `Give ${looked?.name} to ${account.username}? Click again`, () => reassign(account.id));
    return to;
  });
});

// ---- Accounts ----

const openedSaid = byId("opened-said");

byId("find-form").addEventListener("submit", (event) => {
  event.preventDefault();
  findAccounts("find-query", byId("found-list"), (account) => link("Open", accountHref(account.id)));
});

let opened: AccountDetail | null = null;

/** Open Account `id`: its heading and lines, its Block's Unblock, its PCs, each with Unlink,
 *  its Characters, and Delete. */
async function openAccount(id: number, arriving = false) {
  const same = opened?.id === id;
  if (arriving && !same) {
    byId("opened").hidden = true;
    openedSaid.textContent = "Loading…";
  }
  const read = await call<AccountDetail>(`/api/admin/accounts/${id}`);
  if (read.status !== 200 || read.body === null) {
    opened = null;
    byId("opened").hidden = true;
    openedSaid.textContent = read.status === 404 ? "There's no such Account. It may have been deleted." : failed(read);
    return;
  }
  const account = read.body;
  opened = account;
  if (arriving) openedSaid.textContent = "";
  byId("found-list").replaceChildren();
  byId("opened-title").textContent = account.username ?? "(no username)";
  byId("opened-tag").hidden = account.blocked === null;
  byId("opened-tag").className = "tag blocked";
  byId("opened-lines").replaceChildren(...accountLines(account).map((text) => make("li", "", text)));
  byId("opened-block").hidden = account.blocked === null;
  byId("pc-list").replaceChildren(
    ...account.pcs.map((pc) => {
      const unlink = button("Unlink", "As the player's own Unlink: a name it claimed moves to the Account's most recently used other PC.");
      armed(unlink, `Unlink this PC from ${account.username}? Click again`, async () => {
        const done = await send(`/api/admin/accounts/${account.id}/pcs/${pc.id}`, "DELETE");
        await openAccount(account.id);
        tell(openedSaid, done, 204, "PC unlinked.");
      });
      return row(holds(pc.characters), pcDates(pc), unlink);
    }),
  );
  byId("opened-characters-said").textContent = account.characters.length === 0 ? "No Characters." : "";
  byId("opened-characters").replaceChildren(...account.characters.map((name) => row(name, null, link("Look up", characterHref(name)))));
  byId("opened").hidden = false;
  if (arriving && !same) byId("opened-heading").focus();
}

armed(
  byId<HTMLButtonElement>("opened-unblock"),
  () => (opened?.blocked ? unblockAsk(opened.blocked, null) : "Unblock? Click again"),
  async () => {
    if (opened?.blocked == null) return;
    const id = opened.id;
    const done = await send(`/api/admin/blocks/${opened.blocked.id}`, "DELETE");
    await openAccount(id);
    tell(openedSaid, done, 204, "Unblocked. Its Characters' entries are back.");
  },
);

armed(
  byId<HTMLButtonElement>("delete-account"),
  () => `Delete ${opened?.username ?? "this Account"}? It can't be undone. Click again`,
  async () => {
    if (opened === null) return;
    const username = opened.username ?? "The Account";
    const done = await send(`/api/admin/accounts/${opened.id}`, "DELETE");
    if (done.status === 204) {
      byId("opened").hidden = true;
      opened = null;
      lastAccount = undefined;
      history.replaceState(null, "", hashOf({ view: "accounts" }));
    }
    tell(openedSaid, done, 204, `${username} deleted. Each name stays with the key that claimed it.`);
  },
);

// ---- Log ----

/** A line of the log, as a row: Look up for its Character, Open for its Account. `links`
 *  leaves them out, in the Character's own log. */
function logRow(line: LogLine, links = true): HTMLLIElement {
  const { what, when } = logLine(line, mapName);
  const to: HTMLElement[] = [];
  if (links && line.name !== null) to.push(link("Look up", characterHref(line.name)));
  if (line.account !== null && line.account.username !== null) to.push(link("Open Account", accountHref(line.account.id)));
  return row(what, when, ...to);
}

let oldest: number | null = null;

/** Draw the log's newest 50 lines, or, `older`, add the 50 before the last drawn. */
async function loadLog(older = false) {
  const line = byId("log-said");
  const list = byId("log-list");
  if (!older && list.childElementCount === 0) line.textContent = "Loading…";
  const read = await call<{ lines: LogLine[]; more: boolean }>(`/api/admin/log${older && oldest !== null ? `?before=${oldest}` : ""}`);
  if (read.status !== 200 || read.body === null) {
    line.textContent = failed(read);
    return;
  }
  const { lines, more } = read.body;
  if (older) list.append(...lines.map((l) => logRow(l)));
  else list.replaceChildren(...lines.map((l) => logRow(l)));
  oldest = lines.length === 0 ? oldest : lines[lines.length - 1].id;
  line.textContent = list.childElementCount === 0 ? "Nothing yet. Every change you make here is kept in the log." : "";
  byId("log-older").hidden = !more;
}
byId("log-older").addEventListener("click", () => loadLog(true));

// ---- Signed in? ----

async function load() {
  const account = await call<AccountAnswer>("/api/account");
  if (account.status === 401) {
    said.textContent = signInFailed(location.search) ? "Signing in with Discord didn't go through. Try again." : "This page is for the maintainer. Sign in with Discord.";
    byId("signed-out").hidden = false;
    return;
  }
  if (account.status !== 200 || account.body === null) {
    said.textContent = UNREACHABLE;
    return;
  }
  if (!account.body.maintainer) {
    said.textContent = `This page is for the maintainer. You're signed in as ${account.body.username}.`;
    return;
  }
  said.textContent = `Signed in as ${account.body.username}.`;
  byId("views").hidden = false;
  await render();
}

load();
