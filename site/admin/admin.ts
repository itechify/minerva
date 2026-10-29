// The maintainer's page (admin/index.html, ADR 0023 and ADR 0031): Overview, the counts and
// two buttons; Reports, each opened, deleted or dismissed; Characters, a name looked up, its
// entries taken down, reassigned to the refused key or an Account, or blocked, and every Block;
// Accounts, one looked up, a PC unlinked or the Account deleted; and Log. What it says is
// worked out in account/account.ts.
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
  type FoundAccount,
  foundLine,
  holds,
  type LogLine,
  logLine,
  marketLine,
  type Overview,
  overviewLines,
  pcDates,
  portraitsLine,
  type Removed,
  removedLine,
  type Report,
  reportLine,
  signInFailed,
} from "../account/account.ts";
import { armed, call, send, UNREACHABLE } from "../account/api.ts";
import { names } from "../leaderboards/api.ts";
import { linkTo, placeOf } from "../leaderboards/leaderboards.ts";

const byId = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const said = byId("said");

/** A row of a list: its words at the left, its buttons at the right. */
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

// ---- The five views ----

const views = {
  overview: byId("show-overview"),
  reports: byId("show-reports"),
  characters: byId("show-characters"),
  accounts: byId("show-accounts"),
  log: byId("show-log"),
};
type View = keyof typeof views;
const loads: Record<View, () => Promise<void>> = {
  overview: () => loadOverview(),
  reports: () => loadReports(),
  characters: () => loadBlocks(),
  accounts: async () => {},
  log: () => loadLog(),
};

/** Open `view`, and read what it shows afresh. */
function show(view: View) {
  for (const [name, tab] of Object.entries(views)) {
    tab.setAttribute("aria-selected", String(name === view));
    byId(name).hidden = name !== view;
  }
  loads[view]();
}
for (const view of Object.keys(views) as View[]) views[view].addEventListener("click", () => show(view));

// ---- Overview ----

async function loadOverview() {
  const line = byId("overview-said");
  const read = await call<Overview>("/api/admin/overview");
  if (read.status !== 200 || read.body === null) {
    line.textContent = UNREACHABLE;
    return;
  }
  const overview = read.body;
  line.textContent = "";
  byId("overview-lines").replaceChildren(...overviewLines(overview).map((text) => make("li", "", text)));
  byId("portraits-said").textContent = portraitsLine(overview);
  byId("market-said").textContent = marketLine(overview);
  byId<HTMLButtonElement>("refresh-portraits").disabled = !overview.portraits.on;
  byId<HTMLButtonElement>("pull-market").disabled = !overview.market.on;
}

/** A button that runs one of the Worker's jobs at once, then reads the Overview again. */
function job(id: string, path: string, busy: string) {
  const b = byId<HTMLButtonElement>(id);
  const label = b.textContent ?? "";
  b.addEventListener("click", async () => {
    b.disabled = true;
    b.textContent = busy;
    const done = await send(path, "POST");
    b.textContent = label;
    if (done.status !== 204) byId("overview-said").textContent = UNREACHABLE;
    await loadOverview();
  });
}
job("refresh-portraits", "/api/admin/portraits", "Refreshing…");
job("pull-market", "/api/admin/market", "Pulling…");

// ---- Reports ----

/** Draw every report, newest first, each with Open, Dismiss (the report goes) and Take entry
 *  down (the entry goes too, and the same Visit stays off until restored). */
async function loadReports() {
  const line = byId("reports-said");
  const read = await call<{ reports: Report[] }>("/api/admin/reports");
  if (read.status !== 200 || read.body === null) {
    line.textContent = UNREACHABLE;
    return;
  }
  const reports = read.body.reports;
  line.textContent = reports.length === 0 ? "No reports." : reports.length === 1 ? "1 report." : `${reports.length} reports.`;
  byId("report-list").replaceChildren(
    ...reports.map((report) => {
      const { what, when } = reportLine(report, placeOf(names, report.map).name);
      const item = make("li", "account-item");
      const words = make("div", "account-words");
      words.append(make("strong", "", what), make("span", "", when));
      if (report.reason !== "") words.append(make("span", "account-reason", `“${report.reason}”`));
      const open = make("a", "button small secondary", "Open");
      open.href = linkTo({ map: report.map, band: report.band, name: report.name, kind: report.party ? "party" : "solo" });
      const dismiss = button("Dismiss", "Removes the report. The entry stays on the Map Leaderboard.");
      dismiss.addEventListener("click", async () => {
        if ((await send(`/api/admin/reports/${report.id}`, "DELETE")).status !== 204) line.textContent = UNREACHABLE;
        else await loadReports();
      });
      const takeDown = button("Take entry down", "Removes the entry and its reports. The same visit stays off until you restore it under Taken down.");
      armed(takeDown, `Take ${report.name}'s entry down? Click again`, async () => {
        const kind = report.party ? "party" : "solo";
        const path = `/api/admin/entries/${encodeURIComponent(report.name)}/${report.map}/${report.band}?kind=${kind}`;
        if ((await send(path, "DELETE")).status !== 204) line.textContent = UNREACHABLE;
        else await loadReports();
      });
      const actions = make("div", "account-actions");
      actions.append(open, dismiss, takeDown);
      item.append(words, actions);
      return item;
    }),
  );
  await loadRemoved();
}

/** Draw every entry taken down, each with Restore: the same Visit goes up again at its next
 *  upload. */
async function loadRemoved() {
  const line = byId("removed-said");
  const read = await call<{ removed: Removed[] }>("/api/admin/removed");
  if (read.status !== 200 || read.body === null) {
    line.textContent = UNREACHABLE;
    return;
  }
  const removed = read.body.removed;
  line.textContent =
    removed.length === 0 ? "None. An entry you take down stays off until you restore it here." : "Each stays off the Map Leaderboards until you restore it.";
  byId("removed-list").replaceChildren(
    ...removed.map((entry) => {
      const item = make("li", "account-item");
      item.append(make("span", "account-words", removedLine(entry, placeOf(names, entry.map).name)));
      const restore = button("Restore", `${entry.name}'s next upload of this visit puts it back`);
      restore.addEventListener("click", async () => {
        const kind = entry.party ? "party" : "solo";
        const path = `/api/admin/removed/${encodeURIComponent(entry.name)}/${entry.map}/${entry.band}/${entry.expPerHour}?kind=${kind}`;
        if ((await send(path, "DELETE")).status !== 204) line.textContent = UNREACHABLE;
        else await loadRemoved();
      });
      item.append(restore);
      return item;
    }),
  );
}

// ---- Characters ----

const characterSaid = byId("character-said");
const characterInput = byId<HTMLInputElement>("character-name");
let looked: CharacterAnswer | null = null;

/** Look the name in the field up, and draw what the maintainer can do with it. */
async function lookUp() {
  const name = characterInput.value.trim();
  byId("character").hidden = true;
  byId("account-list").replaceChildren();
  looked = null;
  if (name === "") return;
  const read = await call<CharacterAnswer>(`/api/admin/characters/${encodeURIComponent(name)}`);
  if (read.status === 404) {
    characterSaid.textContent = `No Character named ${name} has uploaded.`;
    return;
  }
  if (read.status !== 200 || read.body === null) {
    characterSaid.textContent = UNREACHABLE;
    return;
  }
  looked = read.body;
  characterSaid.textContent = "";
  byId("character-lines").replaceChildren(...characterLines(looked).map((text) => make("li", "", text)));
  byId<HTMLButtonElement>("reassign-refused").disabled = looked.refusedAt === null;
  byId("open-account").hidden = looked.account === null;
  drawEntries(looked);
  drawBlock(looked);
  byId("character-log").replaceChildren(...looked.log.map((l) => logRow(l)));
  byId("character").hidden = false;
}
byId("character-form").addEventListener("submit", (event) => {
  event.preventDefault();
  lookUp();
});

/** Open the Characters view on `name`, looked up. */
function lookUpName(name: string) {
  characterInput.value = name;
  show("characters");
  lookUp();
}

/** The Character's entries, each with Open and Take entry down. */
function drawEntries(character: CharacterAnswer) {
  const hidden = character.blocked !== null;
  byId("entries-said").textContent =
    character.entries.length === 0 ? "No entries." : hidden ? "Hidden from the Map Leaderboards while the Block stands." : "";
  byId<HTMLButtonElement>("take-all-down").disabled = character.entries.length === 0;
  byId("entry-list").replaceChildren(
    ...character.entries.map((entry) => {
      const { what, when } = entryLine(entry, placeOf(names, entry.map).name);
      const kind = entry.party ? "party" : "solo";
      const open = make("a", "button small secondary", "Open");
      open.href = linkTo({ map: entry.map, band: entry.band, name: character.name, kind });
      const takeDown = button("Take entry down", "Removes the entry. The same visit stays off until you restore it under Taken down.");
      armed(takeDown, "Take it down? Click again", async () => {
        const path = `/api/admin/entries/${encodeURIComponent(character.name)}/${entry.map}/${entry.band}?kind=${kind}`;
        if ((await send(path, "DELETE")).status !== 204) characterSaid.textContent = UNREACHABLE;
        await lookUp();
      });
      return row(what, when, open, takeDown);
    }),
  );
}

armed(byId<HTMLButtonElement>("take-all-down"), "Take every entry down? Click again", async () => {
  if (looked === null) return;
  const name = looked.name;
  const done = await send(`/api/admin/entries/${encodeURIComponent(name)}`, "DELETE");
  await lookUp();
  characterSaid.textContent = done.status === 204 ? `Every entry of ${name}'s taken down.` : UNREACHABLE;
});

/** Block, for a Character no Block reaches, or Unblock, for one that does. */
function drawBlock(character: CharacterAnswer) {
  const blocked = character.blocked !== null;
  byId("block-said").textContent = blocked
    ? "Its uploads are refused, and its entries are hidden. Unblock puts them back where they were."
    : "Refuses uploads from its Account, the PCs linked to it and the key that holds the name, and hides every Character they hold. Its entries stay, for Unblock.";
  byId("block-form").hidden = blocked;
  byId("block").hidden = blocked;
  byId("unblock").hidden = !blocked;
}

armed(byId<HTMLButtonElement>("block"), "Block? Click again", async () => {
  if (looked === null) return;
  const name = looked.name;
  const done = await send(`/api/admin/characters/${encodeURIComponent(name)}/block`, "POST", {
    name: byId<HTMLInputElement>("block-name").checked,
    note: byId<HTMLInputElement>("block-note").value,
  });
  byId<HTMLInputElement>("block-note").value = "";
  byId<HTMLInputElement>("block-name").checked = false;
  await lookUp();
  characterSaid.textContent = done.status === 201 ? `${name} blocked.` : done.status === 409 ? "A Block reaches it already." : UNREACHABLE;
  await loadBlocks();
});

armed(byId<HTMLButtonElement>("unblock"), "Unblock? Click again", async () => {
  if (looked?.blocked == null) return;
  const name = looked.name;
  const done = await send(`/api/admin/blocks/${looked.blocked.id}`, "DELETE");
  await lookUp();
  characterSaid.textContent = done.status === 204 ? `${name} unblocked.` : UNREACHABLE;
  await loadBlocks();
});

/** Every Block, the newest first, each with Look up and Unblock. */
async function loadBlocks() {
  const line = byId("blocks-said");
  const read = await call<{ blocks: Block[] }>("/api/admin/blocks");
  if (read.status !== 200 || read.body === null) {
    line.textContent = UNREACHABLE;
    return;
  }
  const blocks = read.body.blocks;
  line.textContent = blocks.length === 0 ? "Nobody's blocked." : "";
  byId("block-list").replaceChildren(
    ...blocks.map((block) => {
      const { what, when } = blockListLine(block);
      const open = button("Look up");
      open.addEventListener("click", () => lookUpName(block.name));
      const lift = button("Unblock");
      armed(lift, "Unblock? Click again", async () => {
        if ((await send(`/api/admin/blocks/${block.id}`, "DELETE")).status !== 204) line.textContent = UNREACHABLE;
        await loadBlocks();
        if (looked?.name === block.name) await lookUp();
      });
      return row(what, when, open, lift);
    }),
  );
}

/** Reassign the name looked up: to the last key refused for it, or to `account`. */
async function reassign(account?: number) {
  if (looked === null) return;
  const name = looked.name;
  const done = await send(`/api/admin/characters/${encodeURIComponent(name)}/reassign`, "POST", account === undefined ? undefined : { account });
  await lookUp();
  characterSaid.textContent =
    done.status === 200
      ? `${name} reassigned.`
      : done.status === 409
        ? account === undefined
          ? "No other key has been refused for that name since it was last reassigned."
          : "That Account has no PC linked to reassign to."
        : UNREACHABLE;
}
armed(byId<HTMLButtonElement>("reassign-refused"), "Reassign? Click again", () => reassign());

byId("open-account").addEventListener("click", () => {
  if (looked?.account == null) return;
  show("accounts");
  openAccount(looked.account.id);
});

/** The Accounts whose username holds what's in `input`, drawn in `list`, each with `act`. */
async function findAccounts(input: string, list: HTMLElement, act: (account: FoundAccount) => HTMLButtonElement) {
  const query = byId<HTMLInputElement>(input).value.trim();
  if (query === "") return;
  const found = await call<{ accounts: FoundAccount[] }>(`/api/admin/accounts?username=${encodeURIComponent(query)}`);
  if (found.status !== 200 || found.body === null) {
    list.replaceChildren(make("li", "", UNREACHABLE));
    return;
  }
  if (found.body.accounts.length === 0) {
    list.replaceChildren(make("li", "", "No Account has that in its username."));
    return;
  }
  list.replaceChildren(
    ...found.body.accounts.map((account) => {
      const item = make("li", "account-item");
      item.append(make("span", "account-words", foundLine(account)), act(account));
      return item;
    }),
  );
}

byId("account-form").addEventListener("submit", (event) => {
  event.preventDefault();
  findAccounts("account-query", byId("account-list"), (account) => {
    const to = button("Reassign here");
    to.disabled = account.pcs === 0;
    armed(to, "Reassign? Click again", () => reassign(account.id));
    return to;
  });
});

// ---- Accounts ----

const openedSaid = byId("opened-said");

byId("find-form").addEventListener("submit", (event) => {
  event.preventDefault();
  findAccounts("find-query", byId("found-list"), (account) => {
    const open = button("Open");
    open.addEventListener("click", () => openAccount(account.id));
    return open;
  });
});

let opened: AccountDetail | null = null;

/** Open Account `id`: its lines, its PCs, each with Unlink, and its Characters. */
async function openAccount(id: number) {
  byId("opened").hidden = true;
  opened = null;
  const read = await call<AccountDetail>(`/api/admin/accounts/${id}`);
  if (read.status === 404) {
    openedSaid.textContent = "That Account is gone.";
    return;
  }
  if (read.status !== 200 || read.body === null) {
    openedSaid.textContent = UNREACHABLE;
    return;
  }
  const account = read.body;
  opened = account;
  openedSaid.textContent = "";
  byId("opened-lines").replaceChildren(...accountLines(account).map((text) => make("li", "", text)));
  byId("pc-list").replaceChildren(
    ...account.pcs.map((pc) => {
      const unlink = button("Unlink", "As the player's own Unlink: a name it claimed moves to the Account's most recently used other PC.");
      armed(unlink, "Unlink? Click again", async () => {
        if ((await send(`/api/admin/accounts/${account.id}/pcs/${pc.id}`, "DELETE")).status !== 204) openedSaid.textContent = UNREACHABLE;
        await openAccount(account.id);
      });
      return row(holds(pc.characters), pcDates(pc), unlink);
    }),
  );
  byId("opened-characters").replaceChildren(
    ...account.characters.map((name) => {
      const open = button("Look up");
      open.addEventListener("click", () => lookUpName(name));
      return row(name, null, open);
    }),
  );
  byId("opened").hidden = false;
}

armed(byId<HTMLButtonElement>("delete-account"), "Delete this Account? Click again", async () => {
  if (opened === null) return;
  const username = opened.username ?? "The Account";
  const done = await send(`/api/admin/accounts/${opened.id}`, "DELETE");
  byId("opened").hidden = true;
  byId("found-list").replaceChildren();
  opened = null;
  openedSaid.textContent = done.status === 204 ? `${username} deleted. Each name stays with the key that claimed it.` : UNREACHABLE;
});

// ---- Log ----

/** A line of the log, as a row. */
function logRow(line: LogLine): HTMLLIElement {
  const { what, when } = logLine(line);
  return row(what, when);
}

let oldest: number | null = null;

/** Draw the log's newest 50 lines, or, `older`, add the 50 before the last drawn. */
async function loadLog(older = false) {
  const line = byId("log-said");
  const read = await call<{ lines: LogLine[]; more: boolean }>(`/api/admin/log${older && oldest !== null ? `?before=${oldest}` : ""}`);
  if (read.status !== 200 || read.body === null) {
    line.textContent = UNREACHABLE;
    return;
  }
  const { lines, more } = read.body;
  const list = byId("log-list");
  if (older) list.append(...lines.map(logRow));
  else list.replaceChildren(...lines.map(logRow));
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
  show("overview");
}

load();
