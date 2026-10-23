import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { byId, span } from "./dom";
import { keyLabel } from "./hotkeys";
import { followView, heightReporter } from "./shell";

/** Mirror of `minerva_core::ItemChoice`: one of several MeowDB items sharing a name. */
type ItemChoice = { id: string; name: string; category: string | null; url: string };

/** Mirror of `minerva_core::ItemLookupLine`: what the latest Item Lookup came to, or what
 * kept an Item Card from showing (`hotkey` says which the player pressed). */
type ItemLookupLine = { lookup: number; hotkey: "itemCard" | "itemLookup" } & (
  | { status: "lookingUp"; read: string }
  | { status: "opened"; name: string; url: string }
  | { status: "choose"; read: string; items: ItemChoice[] }
  | { status: "noTooltip" }
  | { status: "unreadable" }
  | { status: "covered"; by: keyof typeof COVERS }
  | { status: "notFound"; read: string }
  | { status: "unreachable" }
  | { status: "formatChanged" }
  | { status: "noSuchItem"; read: string }
  | { status: "itemsLoading" }
  | { status: "itemsUnavailable" }
  | { status: "itemsFormatChanged" }
);

/** What the shell sends: the alert, whether it shows at all, and the hotkey the player
 * pressed, to name. */
type AlertView = { shown: boolean; line: ItemLookupLine | null; hotkey: string };

/**
 * How long the core keeps an alert (ms; `LINE_MS` and `CHOICES_MS` in
 * `crates/minerva-core/src/item_lookup.rs`): it fades out over its last second.
 */
const LINE_MS = 6_000;
const CHOICES_MS = 30_000;

const alertEl = byId("alert");
const lineEl = byId("line");
const choices = byId("choices");

/** Mirror of `minerva_core::OurWindow`: our windows, which show in our captures. */
const COVERS = {
  panel: "The panel",
  grindTracker: "The Grind Tracker",
  alert: "The last lookup's alert",
  personalBests: "The Personal Bests window",
};

/** What the latest Item Lookup came to, in a sentence; and whether it's a problem. */
function sentence(line: ItemLookupLine, key: string): [string, boolean] {
  switch (line.status) {
    case "lookingUp":
      return [`Looking up ${line.read} on MeowDB…`, false];
    case "opened":
      return [`Opened ${line.name} on MeowDB.`, false];
    case "choose":
      return [`${line.items.length} MeowDB items are named ${line.read}. Pick yours:`, false];
    case "noTooltip":
      return [`No item tooltip by the pointer. Hover an item until its tooltip shows, then press ${key}.`, true];
    case "unreadable":
      return [`Couldn't read the item's name. Press ${key} again with its tooltip in clear view.`, true];
    case "covered":
      return [`${COVERS[line.by]} is covering the item's name. Move it out of the way, then press ${key} again.`, true];
    case "notFound":
      return [`No MeowDB item is named “${line.read}”.`, true];
    case "unreachable":
      return ["MeowDB can't be reached right now.", true];
    case "formatChanged":
      return ["Item Lookup is unavailable: MeowDB's search has changed.", true];
    case "noSuchItem":
      return [`No item is named “${line.read}”.`, true];
    case "itemsLoading":
      return ["Loading the item list from OSMS Data Explorer…", false];
    case "itemsUnavailable":
      return ["The item list can't be loaded: OSMS Data Explorer can't be reached right now.", true];
    case "itemsFormatChanged":
      return ["The Item Card is unavailable: OSMS Data Explorer's item list has changed. Minerva needs an update.", true];
  }
}

/** One of the items sharing the name read: a link to its MeowDB page, with its category. */
function choiceRow(item: ItemChoice, label: string): HTMLLIElement {
  const row = document.createElement("li");
  const button = document.createElement("button");
  button.type = "button";
  button.className = "choice";
  button.dataset.id = item.id;
  button.title = `Open ${item.name} on MeowDB`;
  button.append(span("choice-name", item.name), span("choice-category", label));
  row.append(button);
  return row;
}

/** The alert: shown afresh for each lookup, it fades out as the core lets it go. */
let shownLine = "";
function render({ shown, line, hotkey }: AlertView) {
  // Not showing, the window draws nothing (and the shell lets clicks through it).
  document.body.classList.toggle("hidden", !shown || !line);
  if (!shown || !line) {
    shownLine = "";
    return;
  }
  const key = JSON.stringify([line, hotkey]);
  if (key === shownLine) return;
  shownLine = key;
  const [text, problem] = sentence(line, keyLabel(hotkey));
  lineEl.textContent = text;
  alertEl.classList.toggle("problem", problem);
  const items = line.status === "choose" ? line.items : [];
  // Items sharing a category too (a male and a female version) are told apart by number.
  const label = (item: ItemChoice) => item.category ?? "Item";
  const shared = (item: ItemChoice) => items.filter((other) => label(other) === label(item)).length > 1;
  choices.replaceChildren(...items.map((item) => choiceRow(item, shared(item) ? `${label(item)} · No. ${item.id}` : label(item))));
  choices.hidden = items.length === 0;
  // Waiting on MeowDB, it stays; otherwise it fades out over its last second.
  alertEl.classList.toggle("stays", line.status === "lookingUp");
  alertEl.style.setProperty("--alert-ms", `${line.status === "choose" ? CHOICES_MS : LINE_MS}ms`);
  alertEl.style.animation = "none";
  void alertEl.offsetWidth;
  alertEl.style.animation = "";
  reportHeight();
}

// The shell sizes the window to the alert, as the overlay does the panel.
const reportHeight = heightReporter(alertEl, "item_lookup_height");
window.addEventListener("resize", reportHeight);
// The height depends on the font: measure again once it has loaded.
document.fonts.ready.then(reportHeight);

// A click on an item opens its MeowDB page. The window takes clicks only while it lists
// items, and never the keyboard: it can't be activated.
choices.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLElement>(".choice") : null;
  if (button?.dataset.id) invoke("pick_item", { id: button.dataset.id });
});

followView<AlertView>("item-lookup-state", "item_lookup_view", render);
