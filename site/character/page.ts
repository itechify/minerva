// A Character's page (character/index.html, ADR 0027): their name, who their newest entry
// says they are, their #1s, podiums, maps and best EXP/h, then a table of their entries for
// each Level Band, highest first, each row's map a link to the entry on its Map Leaderboard.
// What it shows is worked out in character.ts.
import { make, svgEl } from "../../src/dom.ts";
import { byId, markSlot, names, said as saidWith, unreachable } from "../leaderboards/api.ts";
import type { Said } from "../leaderboards/leaderboards.ts";
import { type CharacterAnswer, type CharacterBand, type CharacterRow, characterPage, noCharacter, readName, type Stat } from "./character.ts";

const name = readName(location.search);
const board = byId("board");
const said = byId("said");

/** What the page says in place of the bests: the owl lost over it, in Ink, when there's no
 *  Character to show, with where to go from there. */
function say(text: Said, lost: boolean) {
  board.classList.remove("lb-waiting");
  saidWith(said, text);
  said.hidden = false;
  said.classList.toggle("ch-lost-said", lost);
  byId("lost").hidden = !lost;
  // With no Character, their line's room under the name goes, so the owl sits close under it.
  byId("who").hidden = lost;
}

/** The figures under the name: each over what it counts, and what more it says under that. */
function drawStats(stats: Stat[]) {
  const list = byId("stats");
  list.replaceChildren(
    ...stats.map((stat) => {
      const item = make("div", "ch-stat");
      const value = make("dd", "ch-stat-value", stat.value);
      const label = make("dt", "ch-stat-label", stat.label);
      item.append(label, value);
      if (stat.detail !== null) item.append(make("dd", "ch-stat-detail", stat.detail));
      return item;
    }),
  );
  list.hidden = false;
}

/** A crown, a #1's mark, in Crown Gold (index.html's gradient), never the You Yellow that
 *  marks the player's own. */
function crown(): SVGSVGElement {
  const svg = svgEl("svg", { viewBox: "0 0 16 16", class: "ch-crown", "aria-hidden": "true" });
  svg.append(svgEl("path", { d: "M2 12.5 1 4.5l4 3 3-5 3 5 4-3-1 8Z" }));
  return svg;
}

/** An entry's row: its rank of how many (crowned for a #1), its map (a link to the entry on
 *  its Map Leaderboard) over its street, the map's mob levels, the job and level that set it,
 *  its EXP/h and its date. */
function drawRow(row: CharacterRow): HTMLTableRowElement {
  const tr = make("tr", row.top ? "ch-top" : "");
  tr.setAttribute("role", "row");
  const cell = (tag: "td" | "th", className: string, ...content: (string | Node)[]) => {
    const el = make(tag, className);
    el.setAttribute("role", tag === "th" ? "rowheader" : "cell");
    el.append(...content);
    tr.append(el);
    return el;
  };
  const rank = make("span", "ch-rank-of", ` ${row.of}`);
  const rankCell = cell("td", "ch-rank", ...(row.top ? [crown()] : []), row.rank, rank);
  if (row.top) rankCell.setAttribute("aria-label", `${row.rank} ${row.of}, the band's top spot`);
  const map = make("a", "lb-map-name", row.map);
  map.href = row.href;
  const title = make("span", "lb-map-title");
  title.append(markSlot(row.mark), map);
  const mapCell = cell("th", "ch-map", title, make("span", "lb-map-street", row.street));
  (mapCell as HTMLTableCellElement).scope = "row";
  cell("td", row.mobs === null ? "ch-mobs ch-none" : "ch-mobs", row.mobs ?? "—");
  cell("td", "ch-job", row.job);
  cell("td", "ch-level", row.level);
  cell("td", "ch-exp", row.expPerHour);
  cell("td", "ch-date", row.date);
  return tr;
}

/** A Level Band's section: its levels as its heading, then its table. */
function drawBand(band: CharacterBand): HTMLElement {
  const section = make("section", "ch-band");
  const id = `band-${band.band}`;
  section.setAttribute("aria-labelledby", id);
  const title = make("h2", "", band.levels);
  title.id = id;
  const table = make("table", "ch-table");
  table.setAttribute("role", "table");
  const caption = make("caption", "lb-caption", band.caption);
  const head = make("thead");
  head.setAttribute("role", "rowgroup");
  const headRow = make("tr");
  headRow.setAttribute("role", "row");
  for (const [className, text] of [
    ["ch-rank", "Rank"],
    ["ch-map", "Map"],
    ["ch-mobs", "Mob levels"],
    ["ch-job", "Job"],
    ["ch-level", "Level"],
    ["ch-exp", "EXP/h"],
    ["ch-date", "Date"],
  ]) {
    const th = make("th", className, text);
    th.scope = "col";
    th.setAttribute("role", "columnheader");
    headRow.append(th);
  }
  head.append(headRow);
  const body = make("tbody");
  body.setAttribute("role", "rowgroup");
  body.append(...band.rows.map(drawRow));
  table.append(caption, head, body);
  section.append(title, table);
  return section;
}

/** The Character's answer from the API; "none" for a name with no entries (a 404, whatever
 *  the reason), or `null` when it can't be had. */
async function readCharacter(name: string): Promise<CharacterAnswer | "none" | null> {
  try {
    const response = await fetch(`/api/characters/${encodeURIComponent(name)}`, { headers: { accept: "application/json" } });
    if (response.status === 404) return "none";
    return response.ok ? ((await response.json()) as CharacterAnswer) : null;
  } catch {
    return null;
  }
}

async function load() {
  if (name === null) return say(noCharacter(null), true);
  const answer = await readCharacter(name);
  if (answer === "none") {
    // The name as asked: the page can't know it as last uploaded. A page for a name with no
    // entries is the site's 404 in all but status, so search engines keep it out, as they do
    // the 404 itself.
    const robots = make("meta");
    robots.name = "robots";
    robots.content = "noindex";
    document.head.append(robots);
    document.title = `${name} · Minerva`;
    byId("name").textContent = name;
    return say(noCharacter(name), true);
  }
  if (answer === null) {
    board.classList.remove("lb-waiting");
    return unreachable(said, () => {
      saidWith(said, { text: "Loading their bests…" });
      board.classList.add("lb-waiting");
      load();
    });
  }
  const page = characterPage(answer, names, new Date());
  document.title = `${page.name} · Minerva`;
  byId("name").textContent = page.name;
  byId("who").textContent = page.who;
  drawStats(page.stats);
  said.hidden = true;
  byId("loaded").textContent = `${page.name}: ${page.stats.map((s) => `${s.value} ${s.label}`).join(", ")}.`;
  byId("bands").replaceChildren(...page.bands.map(drawBand));
  board.classList.remove("lb-waiting");
}

load();
