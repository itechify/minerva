// Latest, on the Map Leaderboards page (leaderboards/index.html): the newest entries on every
// map, each a link to its Map Leaderboard with the entry marked, saying when it took a top
// spot (ADR 0024). What it shows is worked out in leaderboards.ts.
import { make } from "../../src/dom.ts";
import { byId, names, read, said as saidWith, UNREACHABLE } from "./api.ts";
import { type LatestAnswer, type LatestLine, latestLines } from "./leaderboards.ts";

const said = byId("latest-said");
const list = byId<HTMLOListElement>("latest-entries");

/** An entry: the Character and what they got, over where and when, as a list of maps
 *  writes a map's name over its street. */
function drawLine(line: LatestLine): HTMLLIElement {
  const item = make("li");
  const link = make("a", "lb-map");
  link.href = line.href;
  const words = make("span", "lb-map-words");
  const first = make("span", "lb-map-name", `${line.name} · ${line.expPerHour}`);
  if (line.took !== null) first.append(" ", make("span", "lb-took", line.took));
  words.append(first, make("span", "lb-map-street", `${line.who} · ${line.map}, ${line.levels}`));
  link.append(words, make("span", "lb-map-aside", line.date));
  item.append(link);
  return item;
}

read<LatestAnswer>("/api/latest").then((answer) => {
  if (answer === null) {
    said.textContent = UNREACHABLE;
    return;
  }
  const shown = latestLines(answer, names, new Date());
  if (shown.empty !== null) saidWith(said, shown.empty);
  said.hidden = shown.empty === null;
  list.replaceChildren(...shown.lines.map(drawLine));
  list.hidden = shown.lines.length === 0;
});
