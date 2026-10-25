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

/** Mirror of `minerva_core::OwlLine`: what the owl says. */
type OwlLine = ({ says: "itemLookup" } & ItemLookupLine) | { says: "update"; version: string };

/** Mirror of `minerva_core::OwlBubble`, as the shell sends it (its placement is the
 * window's). `mood` picks the bubble's look and the owl's pose. */
type OwlBubble = { line: OwlLine; mood: "news" | "problem" | "waiting"; clickable: boolean; lastsMs: number | null };

/** What the shell sends: what the owl says, if anything, and the hotkey the player pressed,
 * for Item Lookup's line to name. */
type OwlView = { bubble: OwlBubble | null; hotkey: string };

const owlEl = byId("owl-bubble");
const bubbleEl = byId("bubble");
const lineEl = byId("line");
const choices = byId("choices");
const dismiss = byId("dismiss");

/** Mirror of `minerva_core::OurWindow`: our windows, which show in our captures. */
const COVERS = {
  panel: "The panel",
  grindTracker: "The Grind Tracker",
  owlBubble: "The owl",
  personalBests: "The Personal Bests window",
  settings: "The Settings window",
};

/** What the latest Item Lookup came to, in a sentence. */
function lookupSentence(line: ItemLookupLine, key: string): string {
  switch (line.status) {
    case "lookingUp":
      return `Looking up ${line.read} on MeowDB…`;
    case "opened":
      return `Opened ${line.name} on MeowDB.`;
    case "choose":
      return `${line.items.length} MeowDB items are named ${line.read}. Pick yours:`;
    case "noTooltip":
      return `No item tooltip by the pointer. Hover an item until its tooltip shows, then press ${key}.`;
    case "unreadable":
      return `Couldn't read the item's name. Press ${key} again with its tooltip in clear view.`;
    case "covered":
      return `${COVERS[line.by]} is covering the item's name. Move it out of the way, then press ${key} again.`;
    case "notFound":
      return `No MeowDB item is named “${line.read}”.`;
    case "unreachable":
      return "MeowDB can't be reached right now.";
    case "formatChanged":
      return "Item Lookup is unavailable: MeowDB's search has changed.";
    case "noSuchItem":
      return `No item is named “${line.read}”.`;
    case "itemsLoading":
      return "Loading the item list from OSMS Data Explorer…";
    case "itemsUnavailable":
      return "The item list can't be loaded: OSMS Data Explorer can't be reached right now.";
    case "itemsFormatChanged":
      return "The Item Card is unavailable: OSMS Data Explorer's item list has changed. Minerva needs an update.";
  }
}

/** What the owl says, in a sentence. */
function sentence(line: OwlLine, key: string): string {
  return line.says === "update" ? `A newer version of Minerva is available! (${line.version})` : lookupSentence(line, key);
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

/** The owl's line: shown afresh for each, Item Lookup's fading out as the core lets it go,
 *  the update's staying until the player acts on it. */
let shownLine = "";
let update = false;
function render({ bubble, hotkey }: OwlView) {
  // With nothing to say, the window draws nothing (and the shell lets clicks through it).
  document.body.classList.toggle("hidden", !bubble);
  if (!bubble) {
    shownLine = "";
    return;
  }
  const { line, mood, lastsMs } = bubble;
  const key = JSON.stringify([line, mood, hotkey]);
  if (key === shownLine) return;
  shownLine = key;
  update = line.says === "update";
  owlEl.dataset.mood = mood;
  lineEl.textContent = sentence(line, keyLabel(hotkey));
  dismiss.hidden = !update;
  bubbleEl.classList.toggle("opens", update);
  bubbleEl.title = update ? "Open About & Credits to update" : "";
  const items = line.says === "itemLookup" && line.status === "choose" ? line.items : [];
  // Items sharing a category too (a male and a female version) are told apart by number.
  const label = (item: ItemChoice) => item.category ?? "Item";
  const shared = (item: ItemChoice) => items.filter((other) => label(other) === label(item)).length > 1;
  choices.replaceChildren(...items.map((item) => choiceRow(item, shared(item) ? `${label(item)} · No. ${item.id}` : label(item))));
  choices.hidden = items.length === 0;
  // A line that lasts fades out over its last second; one that stays holds still.
  owlEl.classList.toggle("stays", lastsMs === null);
  owlEl.style.setProperty("--line-ms", `${lastsMs ?? 0}ms`);
  owlEl.style.animation = "none";
  void owlEl.offsetWidth;
  owlEl.style.animation = "";
  reportHeight();
}

// The shell sizes the window to the owl and its bubble, as the overlay does the panel, its
// width as well as its height: a window wider than them would take the game's clicks beside
// them. Measured as wide as they would be (up to the widest the window can be), not as
// the window is now, or it could never grow.
const heightOnly = heightReporter(owlEl, "page_height");
let reportedWidth = 0;
function reportHeight() {
  owlEl.classList.add("measuring-width");
  const width = Math.ceil(owlEl.getBoundingClientRect().width);
  owlEl.classList.remove("measuring-width");
  if (width > 0 && width !== reportedWidth) {
    reportedWidth = width;
    invoke("page_width", { width });
  }
  heightOnly();
}
window.addEventListener("resize", heightOnly);
// The height depends on the font: measure again once it has loaded.
document.fonts.ready.then(reportHeight);

// A click on an item opens its MeowDB page; on the update's line, About & Credits, where
// updating is; on its ×, the owl says no more of it. The window takes clicks only while the
// core says so, and never the keyboard: it can't be activated.
choices.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLElement>(".choice") : null;
  if (button?.dataset.id) invoke("pick_item", { id: button.dataset.id });
});
dismiss.addEventListener("click", () => invoke("act", { action: "dismissUpdateLine" }));
bubbleEl.addEventListener("click", (e) => {
  if (update && !(e.target instanceof Element && e.target.closest("#dismiss"))) {
    invoke("act", { action: "openUpdateLine" });
  }
});

followView<OwlView>("owl-bubble-state", "page_view", render);
