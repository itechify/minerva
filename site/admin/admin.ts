// The maintainer's page (admin/index.html): Reports, each opened, deleted or dismissed, and
// Characters, a name looked up and reassigned to the refused key or an Account. What it says
// is worked out in account/account.ts.
import { make } from "../../src/dom.ts";
import {
  type AccountAnswer,
  type CharacterAnswer,
  characterLines,
  type FoundAccount,
  foundLine,
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

// ---- The two views ----

const views = { reports: byId("show-reports"), characters: byId("show-characters") };

function show(view: keyof typeof views) {
  for (const [name, tab] of Object.entries(views)) {
    tab.setAttribute("aria-selected", String(name === view));
    byId(name).hidden = name !== view;
  }
}
views.reports.addEventListener("click", () => show("reports"));
views.characters.addEventListener("click", () => show("characters"));

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
      open.href = linkTo({ map: report.map, band: report.band, name: report.name });
      const dismiss = make("button", "button small secondary", "Dismiss");
      dismiss.type = "button";
      dismiss.title = "The report goes; the entry stays on the Map Leaderboard";
      dismiss.addEventListener("click", async () => {
        if ((await send(`/api/admin/reports/${report.id}`, "DELETE")).status !== 204) line.textContent = UNREACHABLE;
        else await loadReports();
      });
      const takeDown = make("button", "button small secondary", "Take entry down");
      takeDown.type = "button";
      takeDown.title = "The entry and its reports go, and the same visit stays off until restored under Taken down";
      armed(takeDown, `Take ${report.name}'s entry down? Click again`, async () => {
        const path = `/api/admin/entries/${encodeURIComponent(report.name)}/${report.map}/${report.band}`;
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
    removed.length === 0 ? "None. An entry taken down stays off until restored here." : "Each stays off the Map Leaderboards until restored.";
  byId("removed-list").replaceChildren(
    ...removed.map((entry) => {
      const item = make("li", "account-item");
      item.append(make("span", "account-words", removedLine(entry, placeOf(names, entry.map).name)));
      const restore = make("button", "button small secondary", "Restore");
      restore.type = "button";
      restore.title = `${entry.name}'s next upload of this visit puts it back`;
      restore.addEventListener("click", async () => {
        const path = `/api/admin/removed/${encodeURIComponent(entry.name)}/${entry.map}/${entry.band}/${entry.expPerHour}`;
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
  byId("character").hidden = false;
}
byId("character-form").addEventListener("submit", (event) => {
  event.preventDefault();
  lookUp();
});

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

byId("account-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const query = byId<HTMLInputElement>("account-query").value.trim();
  if (query === "") return;
  const found = await call<{ accounts: FoundAccount[] }>(`/api/admin/accounts?username=${encodeURIComponent(query)}`);
  const list = byId("account-list");
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
      const to = make("button", "button small secondary", "Reassign here");
      to.type = "button";
      to.disabled = account.pcs === 0;
      armed(to, "Reassign? Click again", () => reassign(account.id));
      item.append(make("span", "account-words", foundLine(account)), to);
      return item;
    }),
  );
});

// ---- Signed in? ----

async function load() {
  const account = await call<AccountAnswer>("/api/account");
  if (account.status === 401) {
    said.textContent = signInFailed(location.search) ? "Signing in with Discord didn't go through. Try again." : "This page is the maintainer's. Sign in with Discord.";
    byId("signed-out").hidden = false;
    return;
  }
  if (account.status !== 200 || account.body === null) {
    said.textContent = UNREACHABLE;
    return;
  }
  if (!account.body.maintainer) {
    said.textContent = `This page is the maintainer's, and you're signed in as ${account.body.username}.`;
    return;
  }
  said.textContent = `Signed in as ${account.body.username}.`;
  byId("views").hidden = false;
  show("reports");
  await loadReports();
}

load();
