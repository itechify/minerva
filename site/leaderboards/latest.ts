// Latest, on the Map Leaderboards page (leaderboards/index.html): the newest entries on every
// map, each a link to its Map Leaderboard with the entry marked, saying when it took a top
// spot (ADR 0024). What it shows is worked out in leaderboards.ts.
import { make } from "../../src/dom.ts";
import { byId, characterLink, markSlot, names, read, said as saidWith, unreachable, wordsOf } from "./api.ts";
import { type LatestAnswer, type LatestLine, latestLines } from "./leaderboards.ts";

const said = byId("latest-said");
const list = byId<HTMLOListElement>("latest-entries");
/** What the list holds while the entries come: its room kept, as index.html draws it. */
const waiting = [...list.children];

/** An entry, two lines beside two figures: the Character (a link to their page) and their
 *  job and level beside what they got, over the map (its Map Mark before it, or the mark's
 *  room, so the maps' names line up) and band beside the day; then, if it did, that it took
 *  the map's top spot, and from whom. The map's name is the link to the entry on its Map
 *  Leaderboard, and stretches over the whole entry, as a card's does; the Characters' names
 *  sit over it, so each is its own link. */
function drawLine(line: LatestLine): HTMLLIElement {
  const item = make("li");
  const entry = make("div", "lb-map lb-entry");
  const who = make("span", "lb-entry-who");
  who.append(characterLink(line.name, line.nameHref, "lb-map-name"), " ", make("span", "lb-entry-job", line.who));
  const exp = make("span", "lb-entry-exp", line.expPerHour);
  exp.append(make("span", "lb-entry-unit", " EXP/h"));
  const where = make("span", "lb-entry-where");
  const place = make("span", "lb-entry-place");
  const map = make("a", "lb-entry-map", line.map);
  map.href = line.href;
  place.append(map, " ", make("span", "lb-entry-band", `· ${line.levels}`));
  where.append(markSlot(line.mark), place);
  entry.append(who, exp, where, make("span", "lb-entry-date", line.date));
  if (line.took !== null) {
    const took = make("span", "lb-took");
    took.append(...wordsOf(line.took));
    entry.append(took);
  }
  item.append(entry);
  return item;
}

function load() {
  read<LatestAnswer>("/api/latest").then((answer) => {
    list.classList.remove("lb-waiting");
    if (answer === null) {
      list.hidden = true;
      return unreachable(said, () => {
        said.hidden = true;
        list.replaceChildren(...waiting);
        list.classList.add("lb-waiting");
        list.hidden = false;
        load();
      });
    }
    const shown = latestLines(answer, names, new Date());
    if (shown.empty !== null) saidWith(said, shown.empty);
    said.hidden = shown.empty === null;
    list.replaceChildren(...shown.lines.map(drawLine));
    list.hidden = shown.lines.length === 0;
  });
}
load();
