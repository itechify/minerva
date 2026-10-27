// A map's page (leaderboards/map/index.html): its Level Bands, one band's top 50, each
// entry's EXP/h on its lane, the linked Character's entry marked with the You Yellow star
// and its rank, and a report link on each other entry. A click on a Level Band switches the
// page to it in place, the Characters in both bands sliding to their new rank. Where it
// opens and what it shows are worked out in leaderboards.ts.
import { make, reducedMotion, svgEl } from "../../src/dom.ts";
import { STAR } from "../star.ts";
import { byId, characterLink, crown, jobWithIcon, names, portraitImage, read, said as saidWith, unreachable } from "./api.ts";
import {
  type BandAnswer,
  bandTable,
  type Link,
  linkTo,
  type MapAnswer,
  type MapPlace,
  mapPage,
  mapPicture,
  morphNames,
  NO_MAP,
  placeOf,
  readLink,
  reportOutcome,
  type Row,
  type RulerStep,
  type Said,
} from "./leaderboards.ts";

/** What the page's address names: the map, the band, the Character. */
let link = readLink(location.search);
const said = byId("said");
const rows = byId<HTMLTableSectionElement>("rows");
const ruler = byId("ruler");
/** The map's answer, once read: its bands. */
let mapAnswer: MapAnswer | null = null;
/** The rows the table shows, the linked Character's outside the top 50 among them, so a
 *  switch knows which Characters stay. */
let shown: Row[] = [];
/** The band the page shows, or is switching to. */
let wanted: number | null = null;

/** What the page says in place of the leaderboard: why there isn't one, or what's loading. */
function say(text: Said | string) {
  byId("board").classList.remove("lb-waiting");
  saidWith(said, typeof text === "string" ? { text } : text);
  said.hidden = false;
}

/** The API can't be reached: the page says so, and Try again reads the map's bands and
 *  band again, in the room kept for them. */
function down() {
  byId("board").classList.remove("lb-waiting");
  unreachable(said, () => {
    say("Loading the Map Leaderboard…");
    byId("board").classList.add("lb-waiting");
    openBoard(place!);
  });
}

const place = link.map === null ? null : placeOf(names, link.map);
/** Whether the page is titled with the map yet. */
let titled = false;

/** The page as its address opens it: the map, then its bands and band together, scrolled
 *  to the linked Character's entry. What's drawn as it comes is drawn into room kept for it
 *  (the street's line, the picture's height, a screen's room for the board), so arriving
 *  never moves anything in view. A map the map list doesn't name is titled only once the
 *  API says it has bests (a map newer than the list), and otherwise is no map: the page
 *  keeps only its title, as for a link naming none, so neither keeps room for a map. */
async function open() {
  if (place === null) return noMap(NO_MAP);
  if (place.known) drawHead(place);
  else {
    byId("map-street").hidden = true;
    byId("map-picture").hidden = true;
  }
  openBoard(place);
}

/** The map's name under its street, as the panel titles a map, and its picture. */
function drawHead(place: MapPlace) {
  titled = true;
  byId("map-name").textContent = place.name;
  const street = byId("map-street");
  street.textContent = place.region ? `${place.street} · ${place.region}` : place.street;
  street.hidden = false;
  byId("map-picture").hidden = false;
  document.title = `${place.name} · Map Leaderboard · Minerva`;
  // Pictures load independently: a missing picture never holds up the rankings.
  showPicture(place);
}

/** There's no map to show: the page's title alone, the owl lost, and why, with the way to
 *  find one. */
function noMap(why: Said) {
  byId("map-street").hidden = true;
  byId("map-picture").hidden = true;
  byId("lost").hidden = false;
  say(why);
}

/** The map's bands, then the band the link opens at, drawn together. */
async function openBoard(place: MapPlace) {
  mapAnswer = await read<MapAnswer>(`/api/maps/${place.id}`);
  if (mapAnswer === null) return down();
  const page = mapPage(link, names, mapAnswer);
  if (!page.found) return noMap(page.empty!);
  if (!titled) drawHead(place);
  if (page.open === null) {
    drawBands(page.bands);
    return say(page.empty!);
  }
  wanted = page.open;

  const band = await readBand(page.open, link.name);
  if (band === null) return down();
  drawBands(page.bands);
  drawBand(band, null, false);
  byId("board").classList.remove("lb-waiting");
  // The linked Character's entry is what the player came to see.
  rows.querySelector<HTMLElement>(".lb-you")?.scrollIntoView({ block: "center" });
}

/** OSMS's live map picture, fetched at runtime rather than bundled with the site, in a box
 *  that narrows to it once it's in; with none, a line says so in the box's place. */
async function showPicture(place: MapPlace) {
  const figure = byId("map-picture");
  const image = byId<HTMLImageElement>("map-image");
  const status = byId("map-picture-status");
  const none = (text: string) => {
    figure.classList.add("lb-none");
    image.hidden = true;
    status.textContent = text;
  };
  const manifest = await read<unknown>("https://osmsdataexplorer.com/data/current/map_manifest.json");
  if (manifest === null) return none("Couldn't load the map picture.");
  const url = mapPicture(manifest, place.id);
  if (url === null) return none("No map picture for this map.");
  image.alt = `Map layout of ${place.name}`;
  image.onload = () => {
    figure.style.setProperty("--ratio", String(image.naturalWidth / image.naturalHeight));
    figure.style.setProperty("--wide", `${image.naturalWidth}px`);
    figure.classList.add("lb-in");
    status.hidden = true;
  };
  image.onerror = () => none("Couldn't load the map picture.");
  image.src = url;
}

/** The bands read so far, by their address, so Back and Forward don't wait on the API. */
const bandsRead = new Map<string, BandAnswer>();

async function readBand(band: number, name: string | null): Promise<BandAnswer | null> {
  const query = name === null ? "" : `?name=${encodeURIComponent(name)}`;
  const path = `/api/maps/${place!.id}/bands/${band}${query}`;
  const answer = bandsRead.get(path) ?? (await read<BandAnswer>(path));
  if (answer !== null) bandsRead.set(path, answer);
  return answer;
}

/** The band a page shows, its title, the line about the linked Character, and its table,
 *  each row named by `morph` if it's to slide; `grow` regrows the lanes from the left. */
function drawBand(band: BandAnswer, morph: Map<string, string> | null, grow: boolean) {
  const table = bandTable(band, link.name, new Date());
  const choice = mapPage(link, names, mapAnswer!).bands.find((b) => b.open)!;
  const title = byId("band-title");
  title.textContent = choice.levels;
  title.hidden = false;
  const note = byId("note");
  note.textContent = table.note ?? "";
  note.hidden = table.note === null;
  note.classList.toggle("gone", band.character === null);

  shown = table.yours === null ? table.rows : [...table.rows, table.yours];
  if (table.empty !== null) {
    byId("table").hidden = true;
    byId("credit").hidden = true;
    rows.replaceChildren();
    return say(table.empty);
  }
  said.hidden = true;
  byId("caption").textContent = table.caption ?? "";
  drawRuler(table.ruler);
  const at = { place: place!, band: band.band, ruler: table.ruler, morph };
  rows.replaceChildren();
  for (const row of table.rows) rows.append(drawRow(row, at));
  if (table.yours !== null) {
    if (table.gap !== null) rows.append(lineRow("lb-gap", table.gap));
    rows.append(drawRow(table.yours, at));
  }
  rows.classList.toggle("lb-grow", grow);
  rovingReports(rows);
  byId("table").hidden = false;
  byId("credit").hidden = !table.portraits;
}

/** The band choices, each a link to its band on the page, the open one marked. The links
 *  stay when the same bands are drawn again, so the one clicked keeps the focus. */
function drawBands(bands: ReturnType<typeof mapPage>["bands"]) {
  const nav = byId("bands");
  const had = [...nav.querySelectorAll<HTMLAnchorElement>("a.lb-band")];
  const same = had.length === bands.length && had.every((a, i) => a.dataset.band === String(bands[i].band));
  if (same) {
    had.forEach((a, i) => (bands[i].open ? a.setAttribute("aria-current", "page") : a.removeAttribute("aria-current")));
    return;
  }
  const focused = nav.contains(document.activeElement);
  nav.replaceChildren(
    ...bands.map((band) => {
      const choice = make("a", "lb-band");
      choice.href = band.href;
      choice.dataset.band = String(band.band);
      if (band.open) choice.setAttribute("aria-current", "page");
      choice.append(make("span", "lb-band-levels", band.levels), make("span", "lb-band-entries", band.entries));
      return choice;
    }),
  );
  // A map with one band has it named over its table: no row of one.
  nav.hidden = bands.length < 2;
  if (focused) nav.querySelector<HTMLElement>("[aria-current]")?.focus();
}

/** Clicking a Level Band switches the page to it in place, and the address with it, so
 *  the link still shares and Back comes back. A click meant for a new tab or window, or
 *  with script off, is the link it always was. */
byId("bands").addEventListener("click", (event) => {
  const choice = (event.target as Element).closest<HTMLAnchorElement>("a.lb-band");
  if (choice === null || event.defaultPrevented || event.button !== 0) return;
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
  event.preventDefault();
  const to = readLink(new URL(choice.href).search);
  if (to.band === wanted) return;
  switchTo(to, choice.href, true);
});

/** Back and Forward between bands switch in place too, sliding as a click does, unless
 *  the browser draws its own (a swipe back). */
addEventListener("popstate", (event) => {
  const to = readLink(location.search);
  if (place === null || mapAnswer === null || to.map !== link.map) return location.reload();
  switchTo(to, null, !(event as PopStateEvent & { hasUAVisualTransition?: boolean }).hasUAVisualTransition);
});

/** Which switch is the latest, so a slower answer to an earlier click is dropped. */
let switches = 0;
/** The switch moving now, cut short by another click. */
let moving: ViewTransition | null = null;

/** Switch the page to the band `to` names, pushing `href` to the history when a click asked
 *  for it, sliding the rows when `animate` and the browser can, and motion is welcome. */
async function switchTo(to: Link, href: string | null, animate: boolean) {
  const turn = ++switches;
  const open = mapPage(to, names, mapAnswer!).open;
  if (open === null) return;
  wanted = open;
  byId("board").setAttribute("aria-busy", "true");
  const band = await readBand(open, to.name);
  if (turn !== switches) return;
  byId("board").removeAttribute("aria-busy");
  // The API not answering: the link, loaded as a page, says so.
  if (band === null) return href === null ? location.reload() : location.assign(href);
  const before = shown;
  // The address changes with what the page shows, never ahead of it. A switch cut short by
  // a later one still runs its update, maybe after the later one's: only the latest draws.
  const update = (morph: Map<string, string> | null, grow: boolean) => {
    if (turn !== switches) return;
    const address = linkTo(to);
    if (href !== null && address !== location.pathname + location.search) history.pushState(null, "", address);
    link = to;
    drawBands(mapPage(link, names, mapAnswer!).bands);
    drawBand(band, morph, grow);
    byId("switched").textContent = byId("caption").textContent || said.textContent || "";
  };
  moving?.skipTransition();
  if (!animate || reducedMotion().matches || !document.startViewTransition) return update(null, false);

  const next = bandTable(band, to.name, new Date());
  const morph = morphNames(before, next.yours === null ? next.rows : [...next.rows, next.yours]);
  nameRows(morph);
  const root = document.documentElement;
  root.classList.add("lb-morph");
  const transition = document.startViewTransition(() => update(morph, true));
  moving = transition;
  transition.finished.finally(() => {
    if (moving !== transition) return;
    moving = null;
    root.classList.remove("lb-morph");
    // Done growing: kept, the lanes a resize shows (under the figure, or in their column)
    // would grow again.
    rows.classList.remove("lb-grow");
    nameRows(null);
  });
}

/** Name the shown rows of the Characters in `morph` for the switch, or no row (`null`). */
function nameRows(morph: Map<string, string> | null) {
  for (const tr of rows.querySelectorAll<HTMLTableRowElement>("tr[data-name]")) {
    tr.style.viewTransitionName = morph?.get(tr.dataset.name!) ?? "";
  }
}

/** The ruler over the lanes: each step's figure over a notch, at its place along them. */
function drawRuler(steps: RulerStep[]) {
  ruler.replaceChildren(
    ...steps.map((step, i) => {
      const mark = make("span", i === 0 ? "lb-step lb-step-zero" : "lb-step", step.text);
      mark.style.setProperty("--at", String(step.at));
      return mark;
    }),
  );
  spaceRuler();
}

/** The ruler's figures far enough apart to read: where the lanes are too short for every
 *  step's, every other one keeps its notch alone, as the Personal Bests window's do. */
function spaceRuler() {
  const steps = [...ruler.children] as HTMLElement[];
  if (steps.length < 2) return;
  const apart = ruler.clientWidth * Number(steps[1].style.getPropertyValue("--at"));
  const widest = Math.max(0, ...steps.map((el) => el.offsetWidth));
  const every = apart > 0 ? Math.ceil((widest + 8) / apart) : 1;
  for (const [i, el] of steps.entries()) el.classList.toggle("lb-quiet", i % every !== 0);
}
new ResizeObserver(spaceRuler).observe(ruler);

/** A row of one cell across the table, holding a line: how many entries are between the top
 *  50 and the linked Character's, or where the linked Character stands. */
function lineRow(className: string, text: string): HTMLTableRowElement {
  const tr = make("tr", className);
  tr.setAttribute("role", "row");
  const cell = make("td", "", text);
  cell.setAttribute("role", "cell");
  cell.colSpan = 8;
  tr.append(cell);
  return tr;
}

/** An entry's lane: its EXP/h as the game's EXP gauge draws the bar, lime in its grey
 *  frame, on the band's one scale, over the ruler's lines. It draws the figure beside it
 *  again, so it's hidden from assistive tech. */
function laneCell(row: Row, steps: RulerStep[]): HTMLTableCellElement {
  const td = make("td", "lb-lane");
  td.setAttribute("aria-hidden", "true");
  for (const step of steps.slice(1)) {
    const line = make("span", "lane-grid");
    line.style.setProperty("--at", String(step.at));
    td.append(line);
  }
  td.append(lane(row, "lb-lane-track"));
  return td;
}

/** The lane itself: the track, and the bar as far along it as the entry's EXP/h. */
function lane(row: Row, className: string): HTMLSpanElement {
  const track = make("span", className);
  track.setAttribute("aria-hidden", "true");
  const bar = make("span", "lane-bar");
  bar.style.setProperty("--at", String(row.lane));
  track.append(bar);
  return track;
}

/** Report's flag, a 2-stroke line icon as the game's small buttons draw theirs. Report's own
 *  name says what it is. */
function flag(): SVGSVGElement {
  const svg = svgEl("svg", { viewBox: "0 0 16 16", class: "lb-flag", "aria-hidden": "true" });
  svg.append(svgEl("path", { d: "M4 14.5v-12M4 3h8.5l-2 3.25 2 3.25H4" }));
  return svg;
}

/** The You Yellow star, the Personal Bests window's mark for a best of the player's. */
function star(): SVGSVGElement {
  const svg = svgEl("svg", { viewBox: "0 0 16 16", class: "lb-star", "aria-hidden": "true" });
  svg.append(svgEl("path", { d: STAR }));
  return svg;
}

/** Where an entry's row is drawn: the map and band (for its report), the band's ruler, and
 *  the rows that slide in a switch. */
interface RowsAt {
  place: MapPlace;
  band: number;
  ruler: RulerStep[];
  morph: Map<string, string> | null;
}

/** An entry's row: its rank (crowned in the top 3), name (a link to the Character's page), job
 *  (beside its class's icon), level, EXP/h, lane, date (under the pointer, with a mouse) and
 *  Report's flag; or, for the linked Character's, the You Yellow star before the name and no
 *  Report (the player takes their own entries off from Minerva). A row that slides in a
 *  switch takes its Character's name for it. */
/** How big a portrait is beside a name in a band's table (ADR 0028). */
const PORTRAIT_SIZE = 32;

function drawRow(row: Row, { place, band, ruler, morph }: RowsAt): HTMLTableRowElement {
  const tr = make("tr", row.you ? "lb-you" : "");
  tr.setAttribute("role", "row");
  tr.dataset.name = row.name;
  tr.style.viewTransitionName = morph?.get(row.name) ?? "";
  if (row.you) tr.setAttribute("aria-current", "true");
  // With a mouse, the date is the row's under the pointer; its cell is for a touch screen and
  // assistive tech (site.css).
  tr.title = `Set ${row.date}`;
  const name = make("th", "lb-name");
  name.scope = "row";
  name.setAttribute("role", "rowheader");
  if (row.you) name.append(star());
  if (row.portrait !== null) name.append(portraitImage(row.portrait, PORTRAIT_SIZE));
  name.append(characterLink(row.name, row.nameHref));
  const reportCell = make("td", "lb-report");
  if (!row.you) {
    const report = make("button", "lb-report-button");
    report.type = "button";
    report.title = "Report";
    report.append(flag());
    report.setAttribute("aria-label", `Report ${row.name}'s entry`);
    report.setAttribute("aria-describedby", "report-keys");
    report.addEventListener("click", () => openReport(row, place, band));
    reportCell.append(report);
  }
  const rank = make("td", "lb-rank");
  if (row.medal !== null) rank.append(crown(row.medal));
  rank.append(row.rank);
  const job = make("td", "lb-job");
  job.append(jobWithIcon(row.job, row.jobClass));
  const cells = [rank, job, make("td", "lb-level", row.level)];
  const exp = make("td", "lb-exp", row.expPerHour);
  // Its lane again under the figure, for a page too narrow for the lanes' own column.
  exp.append(lane(row, "lb-lane-track lb-under"));
  const figures = [exp, make("td", "lb-date", row.date), reportCell];
  for (const cell of [...cells, ...figures]) cell.setAttribute("role", "cell");
  tr.append(cells[0], name, cells[1], cells[2], figures[0], laneCell(row, ruler), figures[1], figures[2]);
  return tr;
}

/** The table's Report buttons as one Tab stop: the up and down arrows (Home and End too)
 *  move between them, and Tab goes on past the table. */
function rovingReports(rows: HTMLElement) {
  const reports = [...rows.querySelectorAll<HTMLButtonElement>(".lb-report-button")];
  reports.forEach((report, i) => (report.tabIndex = i === 0 ? 0 : -1));
  rows.onkeydown = (event) => {
    const at = reports.indexOf(event.target as HTMLButtonElement);
    if (at === -1) return;
    const to = { ArrowDown: at + 1, ArrowUp: at - 1, Home: 0, End: reports.length - 1 }[event.key];
    if (to === undefined || to < 0 || to >= reports.length) return;
    event.preventDefault();
    reports[at].tabIndex = -1;
    reports[to].tabIndex = 0;
    reports[to].focus();
  };
}

const dialog = byId<HTMLDialogElement>("report");
const reason = byId<HTMLTextAreaElement>("report-reason");
const reportSaid = byId("report-said");
const send = byId<HTMLButtonElement>("report-send");
const cancel = byId<HTMLButtonElement>("report-cancel");
/** The entry the report window is open for. */
let reporting: { name: string; map: string; band: number } | null = null;
/** Whether a report is on its way, so a second press of Send doesn't send it twice. */
let sending = false;
/** A touch screen, where focusing the box for why would raise the keyboard over the window. */
const touch = matchMedia("(hover: none) and (pointer: coarse)");

function openReport(row: Row, place: MapPlace, band: number) {
  reporting = { name: row.name, map: place.id, band };
  byId("report-what").textContent = `${row.name}'s ${row.expPerHour} EXP/h at level ${row.level}, #${row.rank} on ${place.name}.`;
  reason.value = "";
  reason.disabled = false;
  reportSaid.hidden = true;
  send.hidden = false;
  send.removeAttribute("aria-disabled");
  cancel.textContent = "Cancel";
  // The window opens on the box for why, but on a touch screen on Cancel: the box is
  // optional, and its keyboard would cover the window.
  cancel.autofocus = touch.matches;
  dialog.showModal();
}

dialog.addEventListener("submit", async (event) => {
  if (event.submitter !== send || reporting === null) return;
  event.preventDefault();
  // Send keeps the focus while the report goes (disabling it would drop the focus out of
  // the window), and says it's busy.
  if (sending) return;
  sending = true;
  send.setAttribute("aria-disabled", "true");
  let status: number | null = null;
  try {
    const response = await fetch("/api/reports", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ ...reporting, reason: reason.value.trim() || undefined }),
    });
    status = response.status;
  } catch {
    status = null;
  }
  sending = false;
  send.removeAttribute("aria-disabled");
  const outcome = reportOutcome(status);
  reportSaid.textContent = outcome.said;
  reportSaid.hidden = false;
  // Taken, or gone already: nothing more to send, and the focus goes to Close. Otherwise
  // the player may send it again, from where they are.
  const done = outcome.taken || status === 404;
  reason.disabled = done;
  cancel.textContent = done ? "Close" : "Cancel";
  if (done) {
    send.hidden = true;
    cancel.focus();
  }
});
// A click on the veil around the window closes it, as Esc does.
dialog.addEventListener("click", (event) => {
  if (event.target === dialog) dialog.close();
});

open();
