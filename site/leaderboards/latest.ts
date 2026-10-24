// Latest, on the Map Leaderboards page (leaderboards/index.html): the newest entries on every
// map, each a link to its Map Leaderboard with the entry marked, saying when it took a top
// spot (ADR 0024). What it shows is worked out in leaderboards.ts.
import { make } from "../../src/dom.ts";
import { byId, markImage, names, read, said as saidWith, unreachable } from "./api.ts";
import { type LatestAnswer, type LatestLine, latestLines } from "./leaderboards.ts";

const said = byId("latest-said");
const list = byId<HTMLOListElement>("latest-entries");
/** What the list holds while the entries come: its room kept, as index.html draws it. */
const waiting = [...list.children];

/** An entry, two lines beside two figures: the Character and their job and level beside
 *  what they got, over the map (its Map Mark before it) and band beside the day; then, if
 *  it did, that it took the map's top spot. */
function drawLine(line: LatestLine): HTMLLIElement {
  const item = make("li");
  const link = make("a", "lb-map lb-entry");
  link.href = line.href;
  const who = make("span", "lb-entry-who");
  who.append(make("span", "lb-map-name", line.name), " ", make("span", "lb-entry-job", line.who));
  const exp = make("span", "lb-entry-exp", line.expPerHour);
  exp.append(make("span", "lb-entry-unit", " EXP/h"));
  const where = make("span", "lb-entry-where");
  if (line.mark !== null) where.append(markImage(line.mark, true));
  const place = make("span", "lb-entry-place");
  place.append(make("span", "lb-entry-map", line.map), " ", make("span", "lb-entry-band", `· ${line.levels}`));
  where.append(place);
  link.append(who, exp, where, make("span", "lb-entry-date", line.date));
  if (line.took !== null) link.append(make("span", "lb-took", line.took));
  item.append(link);
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
