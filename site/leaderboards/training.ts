// Where to train, on the Map Leaderboards page (leaderboards/index.html): the Level Bands,
// and the open band's maps, each with its mob levels and the band's #1 there, on every
// street or one (ADR 0024). A click on a band, or a street picked, switches the table in
// place and the address with it, so the link still shares and Back comes back. What it
// shows is worked out in leaderboards.ts.
import { make } from "../../src/dom.ts";
import { byId, names, read, said as saidWith, UNREACHABLE } from "./api.ts";
import {
  type BandsAnswer,
  PICK_A_BAND,
  readTrainingLink,
  type Said,
  type TrainingAnswer,
  type TrainingLink,
  type TrainingRow,
  trainingBands,
  trainingLinkTo,
  trainingTable,
} from "./leaderboards.ts";

/** What the page's address names: the band and the street. */
let link = readTrainingLink(location.search);
const section = byId("where-to-train");
const said = byId("train-said");
const bandsNav = byId("train-bands");
const tools = byId("train-tools");
const street = byId<HTMLSelectElement>("train-street");
const table = byId("train-table");
const rows = byId<HTMLTableSectionElement>("train-rows");
/** The Level Bands, once read. */
let bands: BandsAnswer | null = null;

/** What the section says in place of the table: why there isn't one, or what's loading. */
function say(text: Said | string) {
  saidWith(said, typeof text === "string" ? { text } : text);
  said.hidden = false;
  table.hidden = true;
}

/** The bands read so far, so Back, Forward and a street don't wait on the API. */
const bandsRead = new Map<number, TrainingAnswer>();

async function readBand(band: number): Promise<TrainingAnswer | null> {
  const answer = bandsRead.get(band) ?? (await read<TrainingAnswer>(`/api/bands/${band}`));
  if (answer !== null) bandsRead.set(band, answer);
  return answer;
}

/** Which draw is the latest, so a slower answer to an earlier click is dropped. */
let draws = 0;

/** Draw what `link` names: the bands, and the open band's table. */
async function show() {
  const turn = ++draws;
  if (bands === null) return;
  drawBands();
  if (link.band === null) {
    tools.hidden = true;
    return say(PICK_A_BAND);
  }
  section.setAttribute("aria-busy", "true");
  const answer = await readBand(link.band);
  if (turn !== draws) return;
  section.removeAttribute("aria-busy");
  if (answer === null) return say(UNREACHABLE);
  const shown = trainingTable(answer, names, link, new Date());
  // A street the band has no maps on shows every street, and the address says so.
  if (shown.street !== link.street) {
    link = { ...link, street: shown.street };
    history.replaceState(null, "", trainingLinkTo(link));
    drawBands();
  }
  drawStreets(shown.streets, shown.street);
  tools.hidden = shown.rows.length === 0 && shown.streets.length === 0;
  if (shown.empty !== null) return say(shown.empty);
  said.hidden = true;
  byId("train-caption").textContent = shown.caption ?? "";
  rows.replaceChildren(...shown.rows.map(drawRow));
  table.hidden = false;
}

/** The band choices, each a link to its band, the open one marked. */
function drawBands() {
  const choices = trainingBands(bands!, link);
  const focused = bandsNav.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.band : undefined;
  bandsNav.replaceChildren(
    ...choices.map((band) => {
      const choice = make("a", "lb-band");
      choice.href = band.href;
      choice.dataset.band = String(band.band);
      if (band.open) choice.setAttribute("aria-current", "page");
      choice.append(make("span", "lb-band-levels", band.levels), make("span", "lb-band-entries", band.maps));
      return choice;
    }),
  );
  bandsNav.hidden = choices.length === 0;
  // The one clicked keeps the focus.
  if (focused !== undefined) bandsNav.querySelector<HTMLElement>(`[data-band="${focused}"]`)?.focus();
}

/** The street filter: every street, then each street the band's maps are on. */
function drawStreets(streets: string[], shown: string | null) {
  const all = make("option", "", "All streets");
  all.value = "";
  street.replaceChildren(
    all,
    ...streets.map((name) => {
      const option = make("option", "", name);
      option.value = name;
      return option;
    }),
  );
  street.value = shown ?? "";
}

/** A map's row: its name (its Map Leaderboard at the band) over its street, its mob levels,
 *  the band's #1 there, and its entries. */
function drawRow(row: TrainingRow): HTMLTableRowElement {
  const tr = make("tr");
  tr.setAttribute("role", "row");
  const cell = (className: string, ...content: (string | Node)[]) => {
    const td = make("td", className);
    td.setAttribute("role", "cell");
    td.append(...content);
    tr.append(td);
    return td;
  };
  const name = make("a", "lb-map-name", row.name);
  name.href = row.href;
  cell("lb-train-map", name, make("span", "lb-map-street", row.street));
  cell("lb-train-mobs", row.mobs ?? "—");
  if (row.best === null) cell("lb-train-best lb-train-none", "—");
  else {
    const who = make("span", "lb-train-who", `${row.best.who} · `);
    who.append(make("span", "lb-train-date", row.best.date));
    cell("lb-train-best", make("span", "lb-train-exp", row.best.expPerHour), who);
  }
  cell("lb-train-entries", row.entries);
  return tr;
}

/** Switch to `to` in place, pushing it to the history. */
function go(to: TrainingLink) {
  link = to;
  history.pushState(null, "", trainingLinkTo(link));
  show();
}

/** A click on a band switches in place. A click meant for a new tab or window, or
 *  with script off, is the link it always was. */
bandsNav.addEventListener("click", (event) => {
  const choice = (event.target as Element).closest<HTMLAnchorElement>("a.lb-band");
  if (choice === null || event.defaultPrevented || event.button !== 0) return;
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
  event.preventDefault();
  go(readTrainingLink(new URL(choice.href).search));
});
street.addEventListener("change", () => go({ ...link, street: street.value || null }));
addEventListener("popstate", () => {
  link = readTrainingLink(location.search);
  show();
});

read<BandsAnswer>("/api/bands").then((answer) => {
  if (answer === null) return say(UNREACHABLE);
  bands = answer;
  show();
});
