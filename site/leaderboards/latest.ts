// Latest, on the Map Leaderboards page (leaderboards/index.html): the newest entries on every
// map, each a link to its Map Leaderboard with the entry marked, saying when it took a top
// spot (ADR 0024). What it shows is worked out in leaderboards.ts.
import { make } from "../../src/dom.ts";
import { byId, markSlot, names, partyMark, read, said as saidWith, unreachable } from "./api.ts";
import { type Kind, type LatestAnswer, type LatestLine, latestKinds, latestLines } from "./leaderboards.ts";

const said = byId("latest-said");
const list = byId<HTMLOListElement>("latest-entries");
const kindsGroup = byId("latest-kinds");
/** The entries Latest lists: both kinds, or only the Solo Bests' or the Party Bests'. */
let kind: Kind | null = null;
/** What the list holds while the entries come: its room kept, as index.html draws it. */
const waiting = [...list.children];

/** An entry, two lines beside two figures: the Character and their job and level beside
 *  what they got, over the map (its Map Mark before it, or the mark's room, so the maps'
 *  names line up) and band beside the day; then, if
 *  it did, that it took the map's top spot. */
function drawLine(line: LatestLine): HTMLLIElement {
  const item = make("li");
  const link = make("a", "lb-map lb-entry");
  link.href = line.href;
  const who = make("span", "lb-entry-who");
  who.append(make("span", "lb-map-name", line.name), " ", make("span", "lb-entry-job", line.who));
  const exp = make("span", "lb-entry-exp", line.expPerHour);
  exp.append(make("span", "lb-entry-unit", " EXP/h"));
  if (line.party) exp.append(partyMark());
  const where = make("span", "lb-entry-where");
  const place = make("span", "lb-entry-place");
  place.append(make("span", "lb-entry-map", line.map), " ", make("span", "lb-entry-band", `· ${line.levels}`));
  where.append(markSlot(line.mark), place);
  link.append(who, exp, where, make("span", "lb-entry-date", line.date));
  if (line.took !== null) link.append(make("span", "lb-took", line.took));
  item.append(link);
  return item;
}

/** The choice of kinds, both, solo or party, as buttons: pressed, Latest lists that kind. */
function drawKinds() {
  const focused = kindsGroup.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.kind : undefined;
  kindsGroup.replaceChildren(
    ...latestKinds(kind).map((choice) => {
      const button = make("button", "lb-kind", choice.label);
      button.type = "button";
      button.dataset.kind = choice.kind ?? "";
      button.setAttribute("aria-pressed", String(choice.open));
      return button;
    }),
  );
  kindsGroup.hidden = false;
  if (focused !== undefined) kindsGroup.querySelector<HTMLElement>(`[data-kind="${focused}"]`)?.focus();
}

kindsGroup.addEventListener("click", (event) => {
  const button = (event.target as Element).closest<HTMLButtonElement>("button.lb-kind");
  if (button === null) return;
  const to = button.dataset.kind === "solo" || button.dataset.kind === "party" ? button.dataset.kind : null;
  if (to === kind) return;
  kind = to;
  drawKinds();
  list.classList.add("lb-waiting");
  load();
});

/** Which load is the latest, so a slower answer to an earlier press is dropped. */
let loads = 0;

function load() {
  const turn = ++loads;
  read<LatestAnswer>(`/api/latest${kind === null ? "" : `?kind=${kind}`}`).then((answer) => {
    if (turn !== loads) return;
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
    const shown = latestLines(answer, names, new Date(), kind);
    if (shown.empty !== null) saidWith(said, shown.empty);
    said.hidden = shown.empty === null;
    list.replaceChildren(...shown.lines.map(drawLine));
    list.hidden = shown.lines.length === 0;
  });
}
drawKinds();
load();
