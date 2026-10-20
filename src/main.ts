import "./errors";
import { soloExpHelp } from "./solo-exp-help";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { dayText } from "./dates";
import { byId, cardTop, span } from "./dom";
import { LOADING_MAP_LIST, recognitionProblem, type Recognition } from "./recognition";
import type { ExpTracking } from "./exp-tracking";
import { levels } from "./figures";
import { mapMark, showMark, type MapMarks } from "./map-marks";
import {
  initMapPreview,
  layOutMapPreview,
  lightMob,
  litMob,
  renderMapPreview,
  renderPortalRows,
  type HiddenPortal,
  type MapPreview,
} from "./map-preview";
import {
  HOTKEY_NAMES,
  KEY_ONLY,
  codeLabel,
  hotkeyAvailable,
  keyLabel,
  sameHotkey,
  type Hotkey,
  type Hotkeys,
  type RebindProblem,
  type SettingsNotice,
  type SettingsView,
} from "./hotkeys";
import {
  TIER_NAMES,
  density,
  distanceText,
  distanceWithTravel,
  nearestLine,
  resultGroup,
  resultsHead,
  resultsNote,
  routeStops,
  tripLine,
  type SearchState,
} from "./search-state";

type PlacementMode = "inside" | "dockedRight" | "dockedLeft" | "detached";

/** Mirror of `minerva_core::PerParty`. */
type PerParty = { solo: number; fullParty: number };

/** Mirror of `minerva_core::Mob`. */
type Mob = {
  id: string;
  name: string;
  level: number;
  hp: number;
  exp: number;
  spawnCount: number;
  spawnSharePercent: number;
};


/** Mirror of `minerva_core::MapData`. */
type MapData = {
  summary: { mobsPerHour: PerParty; expPerHour: PerParty } | null;
  /** Every mob on the map, all listed, by level (then name). */
  mobs: Mob[];
  hiddenPortals: HiddenPortal[];
};

/**
 * Mirror of `minerva_core::Section`: a section the player can collapse to its heading. The
 * Map section is `hiddenPortals`, as the settings have always named it.
 */
type Section = "hiddenPortals" | "mobs" | "navigation";

/** Mirror of `minerva_core::MapDataStatus`. */
type MapDataStatus =
  | { status: "noMap" }
  | ({ status: "ready" } & MapData)
  | ({ status: "savedCopy"; savedAtUnixMs: number } & MapData);

/** Mirror of `minerva_core::MobDetails`: a figure shown as "—" is null. */
type MobDetails = {
  elements: { weak: string[]; strong: string[]; immune: string[] };
  touchDamage: { min: number; max: number } | null;
  mp: number | null;
  accuracy: number | null;
  avoid: number | null;
  physicalDefense: number | null;
  magicDefense: number | null;
  speed: string;
  hpPerExp: { hundredths: number; rank: number; ranked: number } | null;
};

/** Mirror of `minerva_core::MobCard`: one listed mob's Mob Card. */
type MobCard = { id: string; url: string; image: string | null } & MobDetails;

/** Mirror of `minerva_core::Outline`. */
type Outline = {
  mobLevels: { lowest: number; highest: number } | null;
  firstMobs: string[];
  hiddenPortals: number;
};

/** Mirror of `minerva_core::Candidates`. */
type Candidates = {
  maps: { id: string; outline: Outline }[];
  shown: number;
  chosen: boolean;
};

/** Mirror of `minerva_core::ExpBar`. */
type ExpBar = { exp: number; percentHundredths: number };

/** Mirror of `minerva_core::PanelState`. */
type PanelState = {
  recognition: Recognition;
  /** The shown map; its `mark` names its Map Mark in `mapMarks`. */
  map: { id: string; streetName: string; mapName: string; url: string; mark: string | null } | null;
  confidence: number;
  notSure: boolean;
  candidates: Candidates | null;
  data: MapDataStatus;
  /** The shown map's Map Preview; null while there is none, or it isn't settled which map it is. */
  mapPreview: MapPreview | null;
  credit: string | null;
  /** The Mob Card of each mob the panel lists, in its order. */
  mobCards: MobCard[];
  /** Every Map Mark, by OSMS's name for it, which each map shown names its own by. */
  mapMarks: MapMarks;
  expanded: boolean;
  /** Mirror of `minerva_core::CollapsedSections`: the sections collapsed to their heading. */
  collapsedSections: Record<Section, boolean>;
  gameFound: boolean;
  placementMode: PlacementMode;
  layer: "onTop" | "hidden";
  /** The player can drag the panel by its header (inside or detached). */
  draggable: boolean;
  setup: { needed: "firstRun" | "resolutionChanged" | "requested" | null };
  hotkeys: Hotkeys;
  unavailableHotkeys: string[];
  /** Those of them that don't name a key combination; the rest another app has. */
  invalidHotkeys: string[];
  /** The shell has the hotkeys registered: the game or one of our windows is in front. */
  hotkeysLive: boolean;
  /** The settings view, which takes over the panel; null while it is closed. */
  settingsView: SettingsView | null;
  gameWindowTitle: string;
  expBar: ExpBar | null;
  expTracking: ExpTracking;
  /** Mirror of `minerva_core::GrindTrackerWindow`: it is on (the settings view's switch),
   * and the player wants it open. */
  grindTracker: { on: boolean; open: boolean };
  search: SearchState;
  /** Mirror of `minerva_core::DealFinderView`. */
  dealFinder: { on: boolean; dealLine: string | null; marginPercent: number; shopFound: boolean };
};

const panelEl = byId("panel");
const street = byId("street");
const map = byId("map");
const note = byId("note");
const candidatesEl = byId("candidates");
const candidatesNote = byId("candidates-note");
const candidateRows = byId("candidate-rows");
const summary = byId("summary");
summary.querySelector(".unit")!.append(soloExpHelp());
const mobsTable = byId("mobs");
const mobRows = byId("mob-rows");
const portals = byId("portals");
const footer = byId("footer");
const credit = byId("credit");
const hotkeyProblem = byId("hotkey-problem");
const grindTracker = byId("grind-tracker");
const dealFinderButton = byId("deal-finder-button");
const badge = byId("badge");
const mapMobs = byId("map-mobs");
const noMobs = byId("no-mobs");
const portalsToggle = byId("portals-toggle") as HTMLButtonElement;
const mobsToggle = byId("mobs-toggle") as HTMLButtonElement;
const navigationToggle = byId("navigation-toggle") as HTMLButtonElement;
const mobCardEl = byId("mob-card");
const iconCount = byId("icon-count");
const iconAlert = byId("icon-alert");
const recovery = byId("recovery");
const uncertainty = byId("uncertainty");
const normalView = byId("normal-view");
const toolbar = byId("toolbar");
const icon = document.querySelector<HTMLElement>(".icon")!;
const nextCandidate = byId("next-candidate");
const titlebar = byId("titlebar");
const titleMark = byId("title-mark") as HTMLImageElement;
const navigation = byId("navigation");
const huntingSprite = byId("hunting-sprite");
const huntingTitle = byId("hunting-title");
const huntingReopen = byId("hunting-reopen");
const huntingClear = byId("hunting-clear");
const huntingRouteTitle = byId("hunting-route-title");
const huntingRoute = byId("hunting-route");
const huntingRouteNone = byId("hunting-route-none");
const huntingArrival = byId("hunting-arrival");
const huntingTrip = byId("hunting-trip");
const searchView = byId("search-view");
const searchBack = byId("search-back");
const searchTitle = byId("search-title");
const matchesStep = byId("matches-step");
const mapsStep = byId("maps-step");
const query = byId("query") as HTMLInputElement;
const suggestions = byId("suggestions");
const noMatch = byId("no-match");
const taxis = byId("taxis") as HTMLInputElement;
const teleportScrolls = byId("teleport-scrolls") as HTMLInputElement;
const searchNearest = byId("search-nearest");
const searchNote = byId("search-note");
const resultsBox = byId("results-box");
const resultsTable = byId("search-results") as HTMLTableElement;
const searchCredit = byId("search-credit");
const settingsView = byId("settings-view");
const settingsBack = byId("settings-back");
const hotkeyRows = byId("hotkey-rows");
const settingsNotice = byId("settings-notice");
const settingsAnnounce = byId("settings-announce");
const hotkeysUndo = byId("hotkeys-undo") as HTMLButtonElement;
const hotkeysReset = byId("hotkeys-reset") as HTMLButtonElement;
const dealFinderEl = byId("deal-finder");
const dealState = byId("deal-state");
const dealDetail = byId("deal-detail");
const dealMargin = byId("deal-margin") as HTMLInputElement;
const grindTrackerOn = byId("grind-tracker-on") as HTMLInputElement;
const iconArt = byId("icon-art") as HTMLImageElement;
const resultsHeader = resultsHead();
resultsTable.append(resultsHeader);
let draggable = false;

/**
 * What each part of the panel was last rendered from. The panel state comes again
 * whenever anything in it changes (the EXP tracking's figures, say): a part whose own
 * inputs didn't change keeps its rows, and the scroll and hover on them.
 */
const renderedFrom = new Map<string, string>();
function changed(part: string, ...inputs: unknown[]): boolean {
  const key = JSON.stringify(inputs);
  if (renderedFrom.get(part) === key) return false;
  renderedFrom.set(part, key);
  return true;
}
/** The search view's open step, as last rendered. */
let step: SearchState["view"] = null;
/** The settings view, as last rendered: null while it is closed. */
let settings: SettingsView | null = null;

/**
 * "Ctrl+Shift+N is taken by another app, so it does nothing here." and "Ctrl+Shift+Banana
 * isn't a key combination.": each hotkey that does nothing, with its real cause.
 */
function hotkeyCauses(panel: PanelState): string {
  const invalid = panel.invalidHotkeys;
  const taken = panel.unavailableHotkeys.filter((k) => !invalid.includes(k));
  const list = (keys: string[]) => keys.map(keyLabel).join(", ");
  const sentences = [];
  if (taken.length > 0) {
    sentences.push(
      taken.length === 1
        ? `${list(taken)} is taken by another app, so it does nothing here.`
        : `${list(taken)} are taken by another app, so they do nothing here.`,
    );
  }
  if (invalid.length > 0) {
    sentences.push(
      invalid.length === 1 ? `${list(invalid)} isn't a key combination.` : `${list(invalid)} aren't key combinations.`,
    );
  }
  return sentences.join(" ");
}

const number = (n: number) => n.toLocaleString("en-US");


function cell(text: string): HTMLTableCellElement {
  const td = document.createElement("td");
  td.textContent = text;
  return td;
}

/** Keep player-recognizable clues ahead of the secondary map ID. */
function candidateRow(id: string, outline: Outline): HTMLLIElement {
  const row = document.createElement("li");
  const facts = document.createElement("div");
  facts.className = "candidate-facts";
  const { mobLevels, firstMobs, hiddenPortals } = outline;
  row.append(span("candidate-mobs", firstMobs.length ? firstMobs.join(", ") : "No mobs"));
  if (mobLevels) facts.append(span("candidate-levels", levels(mobLevels)));
  facts.append(span("candidate-portals", `${hiddenPortals} hidden portal${hiddenPortals === 1 ? "" : "s"}`));
  facts.append(span("candidate-id", `Map ${id}`));
  row.append(facts);
  return row;
}

/** "7 Oct 2026", in the player's time zone. */
const savedOn = (unixMs: number) => dayText(new Date(unixMs), true);

/** The note shown above the map's data: only that it comes from a saved copy over a week old. */
function dataNote(data: MapDataStatus): string {
  // A map without mobs says so in its Mobs section, not here.
  switch (data.status) {
    case "savedCopy":
      return `OSMS Data Explorer unreachable — showing a saved copy from ${savedOn(data.savedAtUnixMs)}`;
    case "ready":
    case "noMap":
      return "";
  }
}

/** Whether the panel shows a map, and so its notes and its Map and Mobs sections. */
const mapShown = (panel: PanelState) => panel.gameFound && !!panel.map;

function renderData(panel: PanelState) {
  const { data } = panel;
  if (!mapShown(panel)) {
    for (const part of [candidatesEl, note, portals, mapMobs]) part.hidden = true;
    renderMapPreview(null, "");
    renderedFrom.delete("data");
    return;
  }
  const inputs = [data, panel.mapPreview, panel.candidates, panel.collapsedSections, panel.recognition, panel.mapMarks];
  if (!changed("data", ...inputs, panel.hotkeys, panel.unavailableHotkeys)) return;

  const { candidates } = panel;
  candidatesEl.hidden = !candidates;
  if (candidates) {
    const nextKey = panel.hotkeys.chooseCandidate;
    const next = hotkeyAvailable(nextKey, panel.unavailableHotkeys)
      ? `${keyLabel(nextKey)} for the next`
      : nextKey === ""
        ? "the two arrows at the top for the next"
        : "Candidate shortcut unavailable; change it in Settings";
    candidatesNote.textContent = candidates.chosen
      ? `Your pick of ${candidates.maps.length} maps with this title · ${next}`
      : `${candidates.maps.length} maps share this title; not sure which you're on · ${next}`;
    candidatesNote.classList.toggle("problem", !candidates.chosen);
    candidateRows.replaceChildren(
      ...candidates.maps.map((candidate, i) => {
        const row = candidateRow(candidate.id, candidate.outline);
        row.classList.toggle("current", i === candidates.shown);
        if (i === candidates.shown) row.setAttribute("aria-current", "true");
        return row;
      }),
    );
  }

  note.textContent = dataNote(data);
  note.hidden = note.textContent === "";
  note.classList.toggle("problem", data.status === "savedCopy");

  const ready = data.status === "ready" || data.status === "savedCopy" ? data : null;

  // The Map first: the map with every portal on it, then the Hidden Portals, what the game
  // won't show the player. With neither picture (nor why it's missing) nor Hidden Portals,
  // it has nothing to show and goes.
  const hiddenPortals: HiddenPortal[] = ready?.hiddenPortals ?? [];
  const pictured = renderMapPreview(ready ? panel.mapPreview : null, panel.map?.mapName ?? "", ready?.mobs ?? []);
  portals.hidden = !ready || (!pictured && hiddenPortals.length === 0);
  renderPortalRows(hiddenPortals, panel.mapPreview?.marks ?? [], panel.mapMarks);

  mapMobs.hidden = !ready;
  noMobs.hidden = !ready || ready.mobs.length > 0;
  summary.hidden = !ready?.summary;
  if (ready?.summary) {
    const { mobsPerHour, expPerHour } = ready.summary;
    byId("mobs-solo").textContent = number(mobsPerHour.solo);
    byId("mobs-party").textContent = number(mobsPerHour.fullParty);
    byId("exp-solo").textContent = number(expPerHour.solo);
    byId("exp-party").textContent = number(expPerHour.fullParty);
  }
  mobsTable.hidden = !ready || ready.mobs.length === 0;
  mobRows.replaceChildren(
    ...(ready?.mobs ?? []).map((mob) => {
      const row = document.createElement("tr");
      // Lit while its spawn points are, on the Map Preview.
      row.dataset.mob = mob.id;
      row.classList.toggle("lit", mob.id === litMob());
      // The name opens the mob's MeowDB page, and resting on it shows its Mob Card.
      const name = document.createElement("td");
      name.className = "mob";
      const button = document.createElement("button");
      button.type = "button";
      button.className = "mob-name";
      button.textContent = mob.name;
      button.dataset.id = mob.id;
      button.setAttribute("aria-describedby", "mob-card");
      button.classList.toggle("hovered", mob.id === hoveredMob);
      name.append(button);
      row.append(
        name,
        cell(String(mob.level)),
        cell(number(mob.hp)),
        cell(number(mob.exp)),
        cell(`${mob.spawnSharePercent}%`),
      );
      ["Mob", "Lv", "HP", "EXP", "Share"].forEach((label, i) => {
        row.cells[i].dataset.label = label;
      });
      return row;
    }),
  );

  // Each section collapses to its heading; Mobs then says how many it holds. With nothing
  // to list, Mobs has nothing to collapse: its heading and "No mobs on this map" show as
  // ever. The Map shows only with something in it.
  const mobCount = ready?.mobs.length ?? 0;
  foldSection(portals, portalsToggle, panel.collapsedSections.hiddenPortals, true);
  foldSection(mapMobs, mobsToggle, panel.collapsedSections.mobs, mobCount > 0, mobCount);
  // Opened again, the picture is fitted to the section.
  layOutMapPreview();
}

/**
 * The footer's info icon, whose tooltip is the credit, so it takes one small line: the
 * panel's credit while a map is shown, or else search's while there is a Hunting Target or
 * a Destination. With nothing to credit, no footer.
 */
function renderCredit(panel: PanelState) {
  const navigating = panel.search.hunting || panel.search.destination;
  const text = (mapShown(panel) ? panel.credit : null) ?? (navigating ? panel.search.credit : null);
  footer.hidden = !text;
  credit.title = text ?? "";
  credit.setAttribute("aria-label", text ?? "");
}

/**
 * A section collapsed to its heading, or open. `holds` says whether it has anything to
 * collapse; `count`, if given, is how many it lists, which its heading says while collapsed.
 */
function foldSection(section: HTMLElement, toggle: HTMLButtonElement, collapsed: boolean, holds: boolean, count?: number) {
  const folded = collapsed && holds;
  section.classList.toggle("collapsed", folded);
  toggle.disabled = !holds;
  toggle.setAttribute("aria-expanded", String(!folded));
  toggle.title = !holds ? "" : folded ? "Show this section" : "Collapse to the heading";
  if (count === undefined) return;
  const countEl = byId(`${toggle.id}-count`);
  countEl.hidden = !folded;
  countEl.textContent = String(count);
}

/** Which hotkey each icon at the top of the panel stands for. */
const CONTROL_HOTKEYS: Record<string, keyof Hotkeys> = {
  search: "search",
  chooseCandidate: "chooseCandidate",
  cyclePlacement: "cyclePlacement",
  setup: "setup",
  togglePanel: "togglePanel",
  toggleGrindTracker: "grindTracker",
  toggleDealFinder: "dealFinder",
};

/**
 * The icons at the top of the panel, one per hotkey action. Each tooltip names its
 * hotkey (unless another app has it). The next-candidate icon shows only while maps share
 * its title, the Grind Tracker's only while
 * the EXP bar is tracked (pressed while its window is open).
 */
function renderToolbar(panel: PanelState) {
  for (const button of toolbar.querySelectorAll<HTMLButtonElement>("button[data-control]")) {
    const label = button.getAttribute("aria-label") ?? "";
    const hotkey = panel.hotkeys[CONTROL_HOTKEYS[button.dataset.control ?? ""]];
    button.title = hotkey && hotkeyAvailable(hotkey, panel.unavailableHotkeys) ? `${label} (${keyLabel(hotkey)})` : label;
  }
  // The map's title opens its MeowDB page, while a map is shown.
  titlebar.classList.toggle("linked", !!panel.map);
  const pageKey = panel.hotkeys.openMeowdbPage;
  titlebar.title = !panel.map
    ? ""
    : hotkeyAvailable(pageKey, panel.unavailableHotkeys)
      ? `Open this map on MeowDB (${keyLabel(pageKey)})`
      : "Open this map on MeowDB";
  nextCandidate.hidden = !panel.candidates;
  grindTracker.hidden = panel.expTracking.state === "off";
  const open = panel.grindTracker.open;
  grindTracker.setAttribute("aria-pressed", String(open));
  grindTracker.setAttribute("aria-label", open ? "Hide the Grind Tracker" : "Show the Grind Tracker");
  const trackerKey = panel.hotkeys.grindTracker;
  grindTracker.title = hotkeyAvailable(trackerKey, panel.unavailableHotkeys)
    ? `${grindTracker.getAttribute("aria-label")} (${keyLabel(trackerKey)})`
    : grindTracker.getAttribute("aria-label") ?? "";
  // Pressed while the Deal Finder is on, as the Grind Tracker's is while its window shows.
  const dealing = panel.dealFinder.on;
  dealFinderButton.setAttribute("aria-pressed", String(dealing));
  dealFinderButton.setAttribute("aria-label", dealing ? "Turn the Deal Finder off" : "Turn the Deal Finder on");
  const dealKey = panel.hotkeys.dealFinder;
  dealFinderButton.title = hotkeyAvailable(dealKey, panel.unavailableHotkeys)
    ? `${dealFinderButton.getAttribute("aria-label")} (${keyLabel(dealKey)})`
    : dealFinderButton.getAttribute("aria-label") ?? "";
  icon.title = hotkeyAvailable(panel.hotkeys.togglePanel, panel.unavailableHotkeys)
    ? `Click to expand (${keyLabel(panel.hotkeys.togglePanel)})` : "Click to expand";
}

/**
 * The mob's animation in a square of its own, standing on its floor at its own size (drawn
 * pixelated), or scaled down smoothly to fit, never up, as the Mob Card draws it. The
 * square keeps its room while the animation loads, or when there is none.
 */
function fillSprite(square: HTMLElement, url: string | null, scaleAbove: number) {
  const img = square.querySelector("img");
  if ((img?.getAttribute("src") ?? null) === url) return;
  if (!url) {
    square.replaceChildren();
    return;
  }
  const next = document.createElement("img");
  next.alt = "";
  // Shrunk to fit, a sprite is smoothed: pixel doubling only works upward.
  next.addEventListener("load", () =>
    next.classList.toggle("scaled", next.naturalWidth > scaleAbove || next.naturalHeight > scaleAbove),
  );
  // No saved image after all: the square stays empty.
  next.addEventListener("error", () => next.remove());
  next.src = url;
  square.replaceChildren(next);
}

/** The Navigation section's square for the Hunting Target mob's animation, in CSS px. */
const TARGET_SPRITE = 40;

/**
 * The Navigation section, between the Map and the Mobs, while there is a Hunting Target or
 * a Destination: the Hunting Target's mob (as OSMS animates it) and map, with the map's
 * figures, or the Destination's map and street; then the route there from the map you're
 * on, which scrolls when the panel runs out of room; once you're there, that you've
 * arrived. A Hunting Target's 🔍 reopens the mob's map list; the × clears either. It
 * collapses to its heading like the map's sections.
 */
function renderNavigation(panel: PanelState) {
  const { hunting, destination } = panel.search;
  const target = hunting ?? destination;
  // The map, with its way there.
  const map = hunting?.map ?? destination;
  navigation.hidden = !target;
  if (!target || !map || !changed("navigation", target, panel.collapsedSections.navigation, panel.mapMarks)) return;
  foldSection(navigation, navigationToggle, panel.collapsedSections.navigation, true);
  const line = (className: string, ...parts: (Node | string)[]) => {
    const el = document.createElement("span");
    el.className = className;
    el.append(...parts);
    return el;
  };
  huntingSprite.hidden = !hunting;
  huntingReopen.hidden = !hunting;
  for (const figure of navigation.querySelectorAll<HTMLElement>("[data-hunting]")) figure.hidden = !hunting;
  const clear = hunting ? "Clear the Hunting Target" : "Clear the Destination";
  huntingClear.title = clear;
  huntingClear.setAttribute("aria-label", clear);
  if (hunting) {
    // The mob on the first line, the map on the next.
    const { mob } = hunting;
    fillSprite(huntingSprite, hunting.image, TARGET_SPRITE);
    huntingTitle.replaceChildren(
      line("hunting-mob", mob.name),
      line("hunting-map", mapMark(panel.mapMarks, map.mark), map.mapName),
    );
    huntingTitle.title = `${mob.name} (Lv ${mob.level}) on ${map.streetName}: ${map.mapName}`;
    const tier = byId("hunting-tier");
    tier.textContent = TIER_NAMES[hunting.map.tier];
    tier.className = `tier ${hunting.map.tier}`;
    byId("hunting-density").textContent = density(hunting.map.densityHundredths);
    byId("hunting-share").textContent = `${hunting.map.spawnSharePercent}%`;
  } else {
    // The map on the first line, its street on the next.
    fillSprite(huntingSprite, null, TARGET_SPRITE);
    huntingTitle.replaceChildren(
      line("hunting-mob", mapMark(panel.mapMarks, map.mark), map.mapName),
      line("hunting-map destination-street", map.streetName),
    );
    huntingTitle.title = `${map.streetName}: ${map.mapName}`;
  }
  const distance = distanceWithTravel(map);
  const distanceEl = byId("hunting-distance");
  distanceEl.textContent = distance.text;
  distanceEl.title = distance.title;
  distanceEl.className = typeof map.distance === "string" ? "distance none" : "distance";

  // The route from the map you're on (its first stop); without one, why not. Once you've
  // arrived the route has done its job: in its place, that you're here and the way you
  // came, until you leave the map.
  const { arrived, trip } = target;
  const routed = !arrived && map.route.length > 0;
  huntingRouteTitle.textContent = routed ? `Route from ${map.route[0].mapName}` : "";
  huntingRouteTitle.hidden = !routed;
  const stops = routed ? routeStops(map, panel.mapMarks) : [];
  // A Destination's map is ringed where its route ends: there's no mob there.
  if (!hunting) stops[stops.length - 1]?.classList.add("destination");
  huntingRoute.replaceChildren(...stops);
  huntingRoute.hidden = !routed;
  huntingRouteNone.textContent = routed || arrived ? "" : distanceText(map.distance).title;
  huntingRouteNone.hidden = routed || arrived;
  huntingArrival.hidden = !arrived;
  huntingTrip.textContent = trip ? tripLine(trip) : "";
  huntingTrip.hidden = !trip;
}

/**
 * The search view, which takes over the whole panel: first the query box and what it
 * matches (mobs, maps and items), then a picked mob's ranked map list. Picking a map from
 * that list makes it the Hunting Target, a matched map the Destination, and matched items
 * show their Item Card; the back arrow or Esc goes back a step with nothing changed.
 */
function renderSearchView(panel: PanelState) {
  const { search } = panel;
  const view = search.view;
  normalView.hidden = view !== null || panel.settingsView !== null;
  searchView.hidden = view === null;
  const opened = view !== step;
  step = view;
  if (view === null) return;
  matchesStep.hidden = view !== "matches";
  mapsStep.hidden = view !== "maps";
  // Before the lists, which scroll their highlight into what room is left.
  searchCredit.textContent = search.credit ?? "";
  searchCredit.hidden = !search.credit;
  if (view === "matches") {
    searchTitle.textContent = "Search";
    renderMatches(search, panel.recognition, panel.mapMarks);
    // A fresh query box, and the typing goes to it.
    if (opened) {
      query.value = search.query;
      query.focus({ preventScroll: true });
    }
  } else {
    searchTitle.textContent = search.picked?.name ?? "";
    if (search.picked) searchTitle.append(span("level", `Lv ${search.picked.level}`));
    renderMaps(search, panel.mapMarks);
    if (opened) resultsBox.focus({ preventScroll: true });
  }
}

/** What a hotkey does, as the settings view and the setup guide name it. */
const hotkeyName = (hotkey: Hotkey) => HOTKEY_NAMES.find(([h]) => h === hotkey)?.[1] ?? hotkey;

/** A combination as read aloud or in a sentence: "Ctrl+Shift+F", or "off". */
const spokenKeys = (binding: string) => (binding === "" ? "off" : keyLabel(binding));

/**
 * A combination as the settings view draws it: "Ctrl+Shift+F", breaking only after a "+"
 * so a long one wraps rather than squeezing what the hotkey does; "Off" with none.
 */
function keysElement(binding: string, kind: "set" | "refused" = "set"): HTMLElement {
  const keys = document.createElement("kbd");
  keys.className = `keys ${kind}`;
  if (binding === "") {
    keys.classList.add("off");
    keys.textContent = "Off";
    return keys;
  }
  const parts = keyLabel(binding).split("+");
  parts.forEach((key, i) => {
    keys.append(key);
    if (i < parts.length - 1) keys.append("+", document.createElement("wbr"));
  });
  return keys;
}

/** "Ctrl+Shift+F is already “Search”. …": why the combination pressed wasn't taken. */
function rebindProblemText(problem: RebindProblem): string {
  switch (problem.kind) {
    case "noModifier":
      return `Add Ctrl, Alt or Win: as a hotkey, ${keyLabel(problem.pressed)} would never reach the game, not even its chat.`;
    case "unsupportedKey":
      return `${codeLabel(problem.code)} can't be part of a hotkey. Press another key.`;
    case "inUse":
      return `${keyLabel(problem.pressed)} is already “${hotkeyName(problem.by)}”. Swap them, or press other keys.`;
  }
}

/** "Saved: Search is now Ctrl+Alt+F.": what the player's last change came to. */
function settingsNoticeText(notice: SettingsNotice, hotkeys: Hotkeys): string {
  switch (notice.kind) {
    case "saved": {
      const now = (hotkey: Hotkey) => `${hotkeyName(hotkey)} is now ${spokenKeys(hotkeys[hotkey])}`;
      const swapped = notice.swappedWith ? `, and ${now(notice.swappedWith)}` : "";
      return `Saved: ${now(notice.hotkey)}${swapped}.`;
    }
    case "restored":
      return "Every hotkey is back to its default, and on.";
    case "restoreUndone":
      return "Saved: your hotkeys are back as they were.";
  }
}

function textButton(text: string, focusKey: string, label: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "text-button";
  button.textContent = text;
  button.dataset.focus = focusKey;
  button.setAttribute("aria-label", label);
  return button;
}

/** A row's second line: what to press while it waits, why its keys were refused, or why they do nothing. */
function rowNote(text: string, kind: "help" | "problem" | "broken", tag?: string, button?: HTMLButtonElement): HTMLElement {
  const note = document.createElement("p");
  note.className = `row-note ${kind}`;
  const words = span("words", text);
  if (tag) words.prepend(span("tag", tag), " ");
  note.append(words);
  if (button) note.append(button);
  return note;
}

/** One hotkey's row: what it does, its keys and Change; waiting, its instruction or problem under it. */
function hotkeyRow(hotkey: Hotkey, name: string, panel: PanelState, view: SettingsView): HTMLLIElement {
  const row = document.createElement("li");
  row.dataset.focus = `row:${hotkey}`;
  row.tabIndex = -1;
  const binding = panel.hotkeys[hotkey];
  const waiting = view.rebinding === hotkey;
  row.classList.toggle("rebinding", waiting);
  const { problem } = view;
  let keys: HTMLElement;
  if (!waiting) keys = keysElement(binding);
  else if (problem?.kind === "unsupportedKey") keys = keysElement(codeLabel(problem.code), "refused");
  else if (problem) keys = keysElement(problem.pressed, "refused");
  else keys = span("keys pending", "Press keys…");
  const change = textButton(
    waiting ? "Cancel" : "Change",
    `change:${hotkey}`,
    waiting ? `Cancel changing ${name}` : `Change ${name}, now ${spokenKeys(binding)}`,
  );
  change.dataset.hotkey = hotkey;
  row.append(span("action", name), keys, change);
  if (waiting && problem) {
    const swap =
      problem.kind === "inUse" ? textButton("Swap", `swap:${hotkey}`, `Swap with ${hotkeyName(problem.by)}`) : undefined;
    swap?.addEventListener("click", () => invoke("rebind_swap"));
    row.append(rowNote(rebindProblemText(problem), "problem", undefined, swap));
  } else if (waiting) {
    const turnOff =
      binding === "" ? undefined : textButton("Turn off", `turn-off:${hotkey}`, `Turn off ${name}`);
    if (turnOff) turnOff.title = "Turn off (Backspace)";
    turnOff?.addEventListener("click", () => invoke("rebind_turn_off"));
    row.append(rowNote("Hold Ctrl+Shift and press a key · Esc cancels", "help", undefined, turnOff));
  } else if (binding === "" && KEY_ONLY.includes(hotkey)) {
    row.append(rowNote("It has no button, so it can't be used while it is off.", "help"));
  } else if (binding !== "" && !hotkeyAvailable(binding, panel.unavailableHotkeys)) {
    const invalid = panel.invalidHotkeys.some((key) => sameHotkey(key, binding));
    const why = invalid ? "These aren't keys a hotkey can use." : "Windows or another app has these keys.";
    row.append(rowNote(why, "broken", invalid ? "Invalid" : "Taken"));
    change.setAttribute("aria-label", `Change ${name}, now ${spokenKeys(binding)}, which does nothing: ${why}`);
  }
  return row;
}

/** What to announce for the settings view's latest change: the problem, the wait, or the notice. */
function settingsAnnouncement(view: SettingsView, hotkeys: Hotkeys): string {
  if (view.problem) return rebindProblemText(view.problem);
  if (view.rebinding) {
    return `Press the new keys for ${hotkeyName(view.rebinding)}, holding Ctrl and Shift. Escape cancels; Backspace turns it off.`;
  }
  return view.notice ? settingsNoticeText(view.notice, hotkeys) : "";
}

/**
 * The settings view, in the search view's place: each hotkey with what it does, its keys and
 * a Change button. Changing one, its row waits for the new keys, which the keys pressed now
 * are (see the keydown handler below), with its instruction, or why the keys pressed were
 * refused, under it. Every change is saved at once, and the line by Restore defaults says
 * so. The rows are drawn anew on every change, so the focus is put back where it was.
 */
function renderSettingsView(panel: PanelState) {
  const view = panel.settingsView;
  settingsView.hidden = view === null;
  const was = settings;
  settings = view;
  if (view === null) return;
  const focused = (document.activeElement as HTMLElement | null)?.dataset?.focus;
  const { hotkeys, unavailableHotkeys, invalidHotkeys } = panel;
  const redrawn = changed("hotkey-rows", hotkeys, unavailableHotkeys, invalidHotkeys, view.rebinding, view.problem);
  if (redrawn) {
    hotkeyRows.replaceChildren(...HOTKEY_NAMES.map(([hotkey, name]) => hotkeyRow(hotkey, name, panel, view)));
  }
  // The Deal Margin as saved, unless the player is typing one.
  if (document.activeElement !== dealMargin) dealMargin.value = String(panel.dealFinder.marginPercent);
  grindTrackerOn.checked = panel.grindTracker.on;
  settingsNotice.textContent = view.notice ? settingsNoticeText(view.notice, hotkeys) : "";
  hotkeysUndo.hidden = view.notice?.kind !== "restored";
  // Nothing to restore, or a row waiting: one change at a time.
  hotkeysReset.disabled = view.atDefaults || view.rebinding !== null;
  const announcement = settingsAnnouncement(view, hotkeys);
  if (changed("settings-announce", announcement)) settingsAnnounce.textContent = announcement;
  // Opened: the first Change. Waiting: its row, so no key presses a button. Done: that
  // row's Change. Restored: Undo, and undone: Restore defaults. Otherwise where it was.
  const target =
    was === null
      ? `change:${HOTKEY_NAMES[0][0]}`
      : view.rebinding && view.rebinding !== was.rebinding
        ? `row:${view.rebinding}`
        : !view.rebinding && was.rebinding
          ? `change:${was.rebinding}`
          : view.notice?.kind !== was.notice?.kind && view.notice?.kind === "restored"
            ? "undo"
            : view.notice?.kind !== was.notice?.kind && view.notice?.kind === "restoreUndone"
              ? "reset"
              : redrawn
                ? focused
                : undefined;
  if (target) settingsView.querySelector<HTMLElement>(`[data-focus="${target}"]`)?.focus({ preventScroll: true });
}

/** Each kind of match: its section's heading, and how a click or Enter on one picks it. */
const MATCH_KINDS = {
  mob: { heading: "Mobs", pick: "search_pick" },
  map: { heading: "Maps", pick: "search_pick_destination" },
  item: { heading: "Items", pick: "search_pick_item" },
} as const;
type MatchKind = keyof typeof MATCH_KINDS;

/** The square a matched mob's animation or item's picture stands in, in CSS px. */
const MATCH_PICTURE = 32;

/** Why no items are matched, in the Items section's place. */
const ITEMS_MISSING = {
  loading: "Items aren't loaded yet",
  unavailable: "Items couldn't be loaded",
  formatChanged: "Items couldn't be loaded",
} as const;

/**
 * The first step: what the query matches, a section for each kind (mobs, maps, then items)
 * with its best five, and a line counting the rest. The highlighted match is what Enter
 * picks. Search knows no mob or map until OSMS's file set is in, which `recognition` says,
 * and no item until its item files are, which the Items section says in its place.
 */
function renderMatches(search: SearchState, recognition: Recognition, marks: MapMarks) {
  const loaded = recognition.status === "ready" || recognition.status === "savedCopy";
  const { mobs, maps, items, itemsMissing, highlighted } = search;
  if (!changed("matches", mobs, maps, items, itemsMissing, highlighted, search.query, loaded, marks)) return;
  let at = 0;
  let current: HTMLElement | null = null;
  const option = (kind: MatchKind, id: string, title: string, ...parts: Node[]) => {
    const row = document.createElement("li");
    row.id = `match-${kind}-${id}`;
    row.dataset.kind = kind;
    row.dataset.id = id;
    row.title = title;
    row.setAttribute("role", "option");
    row.append(...parts);
    const isCurrent = at++ === highlighted;
    row.classList.toggle("current", isCurrent);
    row.setAttribute("aria-selected", String(isCurrent));
    if (isCurrent) current = row;
    return row;
  };
  // A mob's animation or an item's picture, in a square of its own.
  const picture = (url: string | null, pixelArt: boolean) => {
    const square = span("match-picture", "");
    if (pixelArt && url) {
      const img = document.createElement("img");
      img.alt = "";
      img.src = url;
      square.append(img);
    } else {
      fillSprite(square, url, MATCH_PICTURE);
    }
    return square;
  };
  const section = (kind: MatchKind, rows: HTMLLIElement[], more: number, note?: string) => {
    if (rows.length === 0 && !note) return [];
    const holder = document.createElement("li");
    holder.className = `matches matches-${kind}`;
    holder.setAttribute("role", "none");
    const heading = span("matches-head", MATCH_KINDS[kind].heading);
    heading.id = `matches-head-${kind}`;
    const group = document.createElement("ul");
    group.setAttribute("role", "group");
    group.setAttribute("aria-labelledby", heading.id);
    group.append(...rows);
    holder.append(heading, group);
    if (note) holder.append(span("note", note));
    if (more > 0) holder.append(span("matches-more", `+${more} more, keep typing`));
    return [holder];
  };
  const sections = [
    ...section(
      "mob",
      mobs.shown.map((mob) =>
        option("mob", mob.id, `${mob.name} (Lv ${mob.level})`, picture(mob.image, false), span("name", mob.name), span("match-detail", `Lv ${mob.level}`)),
      ),
      mobs.more,
    ),
    ...section(
      "map",
      maps.shown.map((map) =>
        option("map", map.mapId, `${map.streetName}: ${map.mapName}`, mapMark(marks, map.mark), span("name", map.mapName), span("match-detail", map.streetName)),
      ),
      maps.more,
    ),
    ...section(
      "item",
      items.shown.map((item) =>
        option("item", item.id, `${item.name} (${item.kind})`, picture(item.icon, true), span("name", item.name), span("match-detail", item.kind)),
      ),
      items.more,
      itemsMissing ? ITEMS_MISSING[itemsMissing] : undefined,
    ),
  ];
  suggestions.replaceChildren(...sections);
  const listed = sections.length > 0;
  suggestions.hidden = !listed;
  query.setAttribute("aria-expanded", String(listed));
  if (current) {
    query.setAttribute("aria-activedescendant", (current as HTMLElement).id);
    scrollWithin(suggestions, current);
  } else {
    query.removeAttribute("aria-activedescendant");
  }
  noMatch.textContent = loaded ? "Nothing known has that in its name." : "Loading the map list…";
  noMatch.hidden = listed || search.query.trim() === "";
}

/** The second step: the picked mob's maps, the highlighted one first to Enter, and the travel toggles. */
function renderMaps(search: SearchState, marks: MapMarks) {
  // The core keeps the toggles (and saves them); the checkboxes follow it.
  taxis.checked = search.travel.taxis;
  teleportScrolls.checked = search.travel.teleportScrolls;
  searchNearest.textContent = nearestLine(search);
  searchNearest.hidden = searchNearest.textContent === "";
  searchNote.textContent = resultsNote(search);
  searchNote.hidden = searchNote.textContent === "";
  if (!changed("results", search.results, search.highlightedResult, marks)) return;
  const groups = search.results.map((result, i) => {
    const group = resultGroup(result, marks);
    const current = i === search.highlightedResult;
    group.classList.toggle("current", current);
    group.setAttribute("aria-selected", String(current));
    return group;
  });
  resultsBox.hidden = groups.length === 0;
  resultsTable.replaceChildren(resultsHeader, ...groups);
  const current = resultsTable.querySelector<HTMLElement>("tbody.current");
  if (current) scrollWithin(resultsBox, current);
}

/** Scroll `box` just enough to show `item`. Not `scrollIntoView`, which would scroll the overlay's page too. */
function scrollWithin(box: HTMLElement, item: HTMLElement) {
  const outer = box.getBoundingClientRect();
  const inner = item.getBoundingClientRect();
  // Below a sticky table header, if there is one.
  const top = outer.top + (box.querySelector("thead")?.getBoundingClientRect().height ?? 0);
  if (inner.top < top) box.scrollTop -= top - inner.top;
  else if (inner.bottom > outer.bottom) box.scrollTop += inner.bottom - outer.bottom;
}

/** Time to Level on the collapsed icon, while there is one: in the light weight while it
 *  is still settling or held, as the Grind Tracker shows it. */
function renderBadge(panel: PanelState) {
  const text = panel.expTracking.timeToLevelBadge;
  badge.hidden = !text;
  badge.textContent = text ?? "";
  badge.classList.toggle("held", panel.expTracking.state !== "active");
}

// The Map Preview's card shows over the expanded panel's normal view only, and the panel
// is measured again once its picture is in.
initMapPreview({
  cardAllowed: () => !!shownPanel?.expanded && step === null,
  laidOut: () => reportHeight(),
  mobLit: (mob) => {
    for (const row of mobRows.querySelectorAll<HTMLElement>("tr")) row.classList.toggle("lit", row.dataset.mob === mob);
  },
});

/** How long the pointer rests on a mob's name before its Mob Card shows. */
const MOB_CARD_DELAY_MS = 250;
/** The listed mob whose name the pointer is on, if any, and whether its card is due. */
let hoveredMob: string | null = null;
let mobCardDue = false;
let mobCardTimer: number | undefined;
/** The panel state last rendered, for the card to follow. */
let shownPanel: PanelState | null = null;
/** What the card last showed, so a render that changes nothing on it doesn't rebuild it. */
let mobCardShown = "";

function hoverMob(id: string | null) {
  if (id === hoveredMob) return;
  hoveredMob = id;
  mobCardDue = false;
  window.clearTimeout(mobCardTimer);
  for (const button of mobRows.querySelectorAll<HTMLElement>(".mob-name")) {
    button.classList.toggle("hovered", button.dataset.id === id);
  }
  if (id) {
    mobCardTimer = window.setTimeout(() => {
      mobCardDue = true;
      renderMobCard();
    }, MOB_CARD_DELAY_MS);
  }
  renderMobCard();
}

/** "—" for a figure the mob doesn't have. */
const figure = (n: number | null) => (n === null ? "—" : number(n));

/** A tag per element: "Weak to Fire", "Resists Ice", "Immune to Poison". */
function elementTags({ weak, strong, immune }: MobDetails["elements"]): HTMLElement {
  const tags = document.createElement("span");
  tags.className = "mob-card-elements";
  const all = [
    ...weak.map((e) => `Weak to ${e}`),
    ...strong.map((e) => `Resists ${e}`),
    ...immune.map((e) => `Immune to ${e}`),
  ];
  tags.append(...(all.length ? all.map((text) => span("tag", text)) : [span("none", "No element")]));
  return tags;
}

/** Touch and MP, ACC and AVOID, the defences in pairs, then HP per EXP and its rank. */
function mobFigures(details: MobDetails): HTMLDListElement {
  const list = document.createElement("dl");
  list.className = "mob-card-figures";
  const add = (label: string, ...value: (string | Node)[]) => {
    const pair = document.createElement("div");
    const dt = document.createElement("dt");
    dt.textContent = label;
    const dd = document.createElement("dd");
    dd.append(...value);
    pair.append(dt, dd);
    list.append(pair);
    return pair;
  };
  const touch = details.touchDamage;
  add("Touch", touch ? `${number(touch.min)}–${number(touch.max)}` : "—");
  add("MP", figure(details.mp));
  add("ACC", figure(details.accuracy));
  add("AVOID", figure(details.avoid));
  add("P.DEF", figure(details.physicalDefense));
  add("M.DEF", figure(details.magicDefense));
  const perExp = details.hpPerExp;
  const hpPerExp = perExp
    ? [(perExp.hundredths / 100).toFixed(2), span("rank", ` · #${perExp.rank} of ${perExp.ranked}`)]
    : ["—"];
  add("HP per EXP", ...hpPerExp).className = "wide";
  return list;
}

/** The Mob Card's content: the mob, then its figures. */
function mobCardContent(mob: Mob, card: MobCard): Node[] {
  const sprite = document.createElement("div");
  sprite.className = "mob-card-sprite";
  fillSprite(sprite, card.image, 64);
  const who = document.createElement("div");
  who.className = "mob-card-who";
  who.append(span("mob-card-name", mob.name), span("mob-card-gait", `Lv ${mob.level} · ${card.speed}`));
  who.append(elementTags(card.elements));
  const head = document.createElement("div");
  head.className = "mob-card-head";
  head.append(sprite, who);
  return [head, mobFigures(card)];
}

/** The space kept between the card and the hovered row, and the panel's edges. */
const MOB_CARD_GAP = 4;

/**
 * The hovered mob's Mob Card, once the pointer has rested on its name: below its row if it
 * fits in the panel, otherwise above it, never over the window bar. Where it fits on
 * neither side, it takes the roomier one. Only over the normal view of the expanded
 * panel, and gone once the mob isn't listed.
 */
function renderMobCard() {
  const panel = shownPanel;
  const button = hoveredMob
    ? mobRows.querySelector<HTMLElement>(`.mob-name[data-id="${CSS.escape(hoveredMob)}"]`)
    : null;
  const ready = panel?.data.status === "ready" || panel?.data.status === "savedCopy" ? panel.data : null;
  const mob = ready?.mobs.find((m) => m.id === hoveredMob);
  const card = panel?.mobCards.find((c) => c.id === hoveredMob);
  if (!mobCardDue || !panel?.expanded || step !== null || !button || !mob || !card) {
    mobCardEl.hidden = true;
    mobCardShown = "";
    return;
  }
  const shown = JSON.stringify([mob, card]);
  if (shown !== mobCardShown) {
    mobCardEl.replaceChildren(...mobCardContent(mob, card));
    mobCardShown = shown;
  }
  mobCardEl.hidden = false;
  const row = (button.closest("tr") ?? button).getBoundingClientRect();
  const bar = normalView.querySelector(".window-bar");
  const top = cardTop(panelEl, bar, row, mobCardEl.offsetHeight, MOB_CARD_GAP, "below");
  mobCardEl.style.top = `${top}px`;
}

// Resting on a listed mob's name shows its Mob Card; clicking it opens its MeowDB page.
// Resting anywhere on a mob's row lights its spawn points on the Map Preview; on its name,
// its Mob Card shows too. Its name taking the keyboard's focus lights them as well.
mobRows.addEventListener("mouseover", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const button = target?.closest<HTMLElement>(".mob-name");
  hoverMob(button?.dataset.id ?? null);
  lightMob(target?.closest<HTMLElement>("tr")?.dataset.mob ?? null);
});
mobRows.addEventListener("mouseleave", () => {
  hoverMob(null);
  lightMob(null);
});
mobRows.addEventListener("focusin", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLElement>(".mob-name") : null;
  lightMob(button?.dataset.id ?? null);
});
mobRows.addEventListener("focusout", () => lightMob(null));
mobRows.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLElement>(".mob-name") : null;
  if (!button?.dataset.id) return;
  invoke("open_mob_page", { id: button.dataset.id });
  hoverMob(null);
});

/**
 * The Deal Finder's line, while it is on: whether it has found a Player Shop, the Deal Line
 * it compares listings with, and the Deal Margin when there is one.
 */
function renderDealFinder(panel: PanelState) {
  const deal = panel.dealFinder;
  dealFinderEl.hidden = !deal.on;
  if (!deal.on) return;
  dealState.textContent = deal.shopFound ? "· reading the shop" : "· open a Player Shop";
  const line = deal.dealLine ? `Comparing with ${deal.dealLine}` : "No Market Line to compare with";
  dealDetail.textContent = deal.marginPercent > 0 ? `${line}, ${deal.marginPercent}% below` : line;
  const key = panel.hotkeys.dealFinder;
  const off = byId("deal-off");
  off.title = key && hotkeyAvailable(key, panel.unavailableHotkeys) ? `Turn off (${keyLabel(key)})` : "Turn off";
}

/** The collapsed icon: the owl (with its magnifier while the Deal Finder is on), the shown
 * map's Hidden Portal count, and a dot when something needs a look. */
function renderIcon(panel: PanelState) {
  const detective = panel.dealFinder.on;
  if (iconArt.classList.contains("detective") !== detective) {
    iconArt.classList.toggle("detective", detective);
    iconArt.src = detective ? "/assets/deal-finder-owl.png" : "/assets/icon.png";
    iconArt.alt = detective ? "Minerva, Deal Finder on" : "Minerva";
  }
  const data = panel.data.status === "ready" || panel.data.status === "savedCopy" ? panel.data : null;
  iconCount.textContent = panel.map && data ? String(data.hiddenPortals.length) : "";
  iconAlert.hidden = !(
    panel.notSure ||
    (panel.candidates && !panel.candidates.chosen) ||
    panel.data.status === "savedCopy" ||
    panel.setup.needed === "firstRun" ||
    panel.setup.needed === "resolutionChanged" ||
    panel.expTracking.state === "unreadable"
  );
}

function render(panel: PanelState) {
  const { hotkeys } = panel;
  const { recognition, map: current, notSure } = panel;
  draggable = panel.draggable;
  // Hidden while the game is minimised or gone, or another app is in front: the window
  // stays, but draws nothing.
  document.body.classList.toggle("hidden", panel.layer === "hidden");
  document.body.classList.toggle("collapsed", !panel.expanded);
  document.body.classList.toggle("draggable", panel.draggable);
  document.body.classList.toggle("unreadable", !current);
  // Not sure of the map, or of which map sharing its title: the facts shown are a guess.
  const doubtful = !!current && (notSure || (!!panel.candidates && !panel.candidates.chosen));
  document.body.classList.toggle("not-sure", doubtful);
  uncertainty.hidden = !doubtful;
  const setupHint = hotkeyAvailable(hotkeys.setup, panel.unavailableHotkeys)
    ? `Re-mark with ${keyLabel(hotkeys.setup)}.`
    : hotkeys.setup === ""
      ? "Re-mark from Help & Setup, the ? at the top of the panel."
      : "Setup shortcut unavailable. Close the app using it, or change it in Settings (the gear at the top of the panel).";
  // Why it isn't sure is the chip's tooltip, not a note taking room under the title.
  const doubt = !doubtful
    ? ""
    : panel.candidates && !panel.candidates.chosen
      ? `${panel.candidates.maps.length} maps share this title; not sure which you're on.`
      : `Title not confirmed. Keep both minimap title lines visible. ${setupHint}`;
  uncertainty.title = doubt;
  uncertainty.setAttribute("aria-label", doubt ? `Not sure: ${doubt}` : "Current Map is uncertain");
  // Without a map, the title bar holds the status and the note under it what to do; the
  // Street Name slot stays the Street Name's.
  recovery.textContent = "";
  let problem = false;
  let marked = false;
  street.textContent = "";
  const unrecognisable = recognitionProblem(recognition);
  if (!panel.gameFound) {
    map.textContent = `Looking for ${panel.gameWindowTitle}…`;
  } else if (panel.setup.needed === "firstRun" || panel.setup.needed === "resolutionChanged") {
    // Nothing is read until the Title Region is marked (again).
    map.textContent =
      panel.setup.needed === "firstRun" ? "Finish setup to start" : "The game window changed size";
    recovery.textContent = `Mark the minimap title. ${setupHint}`;
    problem = panel.setup.needed === "resolutionChanged";
  } else if (unrecognisable) {
    map.textContent = "Map recognition unavailable";
    recovery.textContent = unrecognisable;
    problem = true;
  } else if (current) {
    street.textContent = current.streetName;
    map.textContent = current.mapName;
    marked = showMark(titleMark, panel.mapMarks, current.mark);
  } else {
    map.textContent = "Reading the minimap title…";
    recovery.textContent = recognition.status === "starting"
      ? LOADING_MAP_LIST
      : `Keep the minimap open with both title lines visible. If this persists, ${setupHint.charAt(0).toLowerCase() + setupHint.slice(1)}`;
  }
  // The map's Map Mark beside its title, as the Mini Map draws it; with none, no room for it.
  titleMark.hidden = !marked;
  recovery.hidden = recovery.textContent === "";
  recovery.classList.toggle("problem", problem);
  renderToolbar(panel);
  // Each hotkey that does nothing, its real cause, then one way to fix it.
  const causes = hotkeyCauses(panel);
  const them = panel.unavailableHotkeys.length === 1 ? "it" : "them";
  const taken = panel.unavailableHotkeys.some((key) => !panel.invalidHotkeys.includes(key));
  hotkeyProblem.textContent = causes &&
    `${causes} ${taken ? "Close the other app, or change" : "Change"} ${them} in Settings (the gear at the top of the panel).`;
  hotkeyProblem.hidden = hotkeyProblem.textContent === "";
  renderDealFinder(panel);
  renderData(panel);
  renderNavigation(panel);
  renderCredit(panel);
  renderSearchView(panel);
  renderSettingsView(panel);
  shownPanel = panel;
  renderMobCard();
  renderBadge(panel);
  renderIcon(panel);
  expanded = panel.expanded;
  reportHeight();
}

// The shell sizes the window to the expanded panel's content, and the core keeps it
// within the space where the panel is placed. The panel is then no taller than the
// window, and its long part (the route, the suggestions, the map list) scrolls. So the
// core always learns the content's full height, borders and bottom padding included,
// the panel is measured without the window's limit: measured within it, the cut-off
// padding would not count, and the window would shrink a little with every report.
// Lifting the limit scrolls the long parts back to the top, so they are put back.
let expanded = true;
let reportedHeight = 0;
function reportHeight() {
  if (!expanded) return;
  const scrolls = [...panelEl.querySelectorAll<HTMLElement>(".scroll")];
  const scrolledTo = scrolls.map((el) => el.scrollTop);
  panelEl.classList.add("measuring");
  const height = Math.ceil(panelEl.getBoundingClientRect().height);
  panelEl.classList.remove("measuring");
  scrolls.forEach((el, i) => (el.scrollTop = scrolledTo[i]));
  if (height > 0 && height !== reportedHeight) {
    reportedHeight = height;
    invoke("panel_height", { height });
  }
}
window.addEventListener("resize", reportHeight);
// The panel's height depends on its font: measure again once it has loaded.
document.fonts.ready.then(reportHeight);

// The panel is dragged by its header, or collapsed by its icon (inside the game, or
// detached), never by its buttons. A click anywhere leaves the keyboard with the game:
// the overlay can't be activated, except while the search view is open.
function startDrag() {
  getCurrentWindow().startDragging();
}

document.addEventListener("mousedown", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const onHeader = !!target?.closest(".drag-handle") && !target?.closest("button, input");
  // The map's title is also a link: it drags only once the mouse moves (below).
  const onLink = !!target?.closest(".titlebar.linked");
  if (draggable && e.button === 0 && onHeader && !onLink) startDrag();
});

// The expanded panel's bottom corners resize it, whatever its placement, as its edges
// do: its width only, as the height follows the content. Once the player lets go the
// core keeps the width (never under its minimum).
for (const grip of document.querySelectorAll<HTMLElement>(".grip")) {
  grip.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    getCurrentWindow().startResizeDragging(grip.dataset.resize === "West" ? "West" : "East");
  });
}

/** How far the mouse moves, pressed, before a press on something clickable drags. */
const DRAG_AFTER_PX = 4;

/**
 * `el` is clicked to `press` its control, and dragged to move the panel where it can be:
 * a press becomes a drag once the mouse moves (and never a click), and a click if it is let
 * go first. Only
 * while `clickable` says so; otherwise it is left to the header's own drag.
 */
function clickOrDrag(el: HTMLElement, control: string, clickable: () => boolean) {
  let pressed: { x: number; y: number } | null = null;
  el.addEventListener("mousedown", (e) => {
    pressed = e.button === 0 && clickable() ? { x: e.screenX, y: e.screenY } : null;
  });
  el.addEventListener("mousemove", (e) => {
    if (!pressed) return;
    // Moved: not a click, and a drag where the panel can be dragged.
    if (Math.abs(e.screenX - pressed.x) + Math.abs(e.screenY - pressed.y) >= DRAG_AFTER_PX) {
      pressed = null;
      if (draggable) startDrag();
    }
  });
  el.addEventListener("mouseup", (e) => {
    if (pressed && e.button === 0) invoke("panel_press", { control });
    pressed = null;
  });
  el.addEventListener("mouseleave", () => {
    pressed = null;
  });
}

// The collapsed icon expands the panel again (like the collapse hotkey); the map's title
// opens its MeowDB page (like Ctrl+Shift+M).
clickOrDrag(icon, "togglePanel", () => true);
clickOrDrag(titlebar, "openMeowdbPage", () => titlebar.classList.contains("linked"));

// Each icon at the top of the panel does what its hotkey does.
toolbar.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLButtonElement>("button[data-control]") : null;
  if (button) invoke("panel_press", { control: button.dataset.control });
});
// The Deal Finder's Turn off, like its hotkey.
dealFinderEl.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLButtonElement>("button[data-control]") : null;
  if (button) invoke("panel_press", { control: button.dataset.control });
});
searchBack.addEventListener("click", () => invoke("search_escape"));
huntingReopen.addEventListener("click", () => invoke("hunting_reopen"));
huntingClear.addEventListener("click", () => invoke("hunting_clear"));
// A click on a section's heading collapses the section to it, or opens it again.
for (const toggle of [portalsToggle, navigationToggle, mobsToggle]) {
  toggle.addEventListener("click", () => invoke("panel_toggle_section", { section: toggle.dataset.section }));
}

// A click on a match, or on a map in a mob's map list, picks it, like Enter on it.
function pickOnClick(list: HTMLElement, item: string) {
  list.addEventListener("click", (e) => {
    const picked = e.target instanceof Element ? e.target.closest<HTMLElement>(item) : null;
    const kind = picked?.dataset.kind as MatchKind | undefined;
    if (picked?.dataset.id) invoke(kind ? MATCH_KINDS[kind].pick : "search_pick", { id: picked.dataset.id });
  });
}
pickOnClick(suggestions, "li[data-id]");
pickOnClick(resultsTable, "tbody[data-id]");

/** The search view's keys: the arrows move the highlight, Enter picks, Esc goes back. */
function searchKey(e: KeyboardEvent): boolean {
  switch (e.key) {
    case "ArrowDown":
    case "ArrowUp":
      invoke("search_move", { down: e.key === "ArrowDown" });
      break;
    case "Enter":
      invoke("search_enter");
      break;
    case "Escape":
      invoke("search_escape");
      break;
    default:
      return false;
  }
  e.preventDefault();
  return true;
}
query.addEventListener("input", () => invoke("search_query", { query: query.value }));
query.addEventListener("keydown", (e) => {
  if (!e.isComposing && searchKey(e)) e.stopPropagation();
});
// The map list has no text box: its keys come from wherever the focus is in the view.
document.addEventListener("keydown", (e) => {
  if (step === null || e.isComposing) return;
  // Buttons and checkboxes keep their own Enter.
  if (e.key === "Enter" && e.target instanceof Element && e.target.closest("button, input")) return;
  searchKey(e);
});
// After a click on a checkbox, the keys go back to the map list.
taxis.addEventListener("change", () => {
  invoke("search_toggle_taxis");
  resultsBox.focus({ preventScroll: true });
});
teleportScrolls.addEventListener("change", () => {
  invoke("search_toggle_teleport_scrolls");
  resultsBox.focus({ preventScroll: true });
});
// Whenever the overlay comes in front with the search view or the settings view open, the
// keys go to it.
window.addEventListener("focus", () => {
  if (step === "matches") query.focus({ preventScroll: true });
  else if (step === "maps") resultsBox.focus({ preventScroll: true });
  else if (settings && !settingsView.contains(document.activeElement)) {
    const target = settings.rebinding ? `row:${settings.rebinding}` : `change:${HOTKEY_NAMES[0][0]}`;
    settingsView.querySelector<HTMLElement>(`[data-focus="${target}"]`)?.focus({ preventScroll: true });
  }
});

settingsBack.addEventListener("click", () => invoke("settings_close"));
// The Deal Margin is saved once the player is done with it (Enter, or leaving the field): a
// whole percent from 0 to 99. What isn't one goes back to the margin saved.
dealMargin.addEventListener("change", () => {
  const percent = Math.round(Number(dealMargin.value));
  if (dealMargin.value.trim() !== "" && Number.isFinite(percent) && percent >= 0) {
    invoke("deal_margin_set", { percent: Math.min(percent, 99) });
  } else if (shownPanel) {
    dealMargin.value = String(shownPanel.dealFinder.marginPercent);
  }
});
dealMargin.addEventListener("keydown", (e) => {
  if (e.key === "Enter") dealMargin.blur();
});
grindTrackerOn.addEventListener("change", () => invoke("grind_tracker_set", { on: grindTrackerOn.checked }));
hotkeysReset.addEventListener("click", () => invoke("hotkeys_reset"));
hotkeysUndo.addEventListener("click", () => invoke("hotkeys_undo_reset"));
// A row's Change button waits for its new combination; while it waits, it cancels.
hotkeyRows.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLButtonElement>("button[data-hotkey]") : null;
  const hotkey = button?.dataset.hotkey as Hotkey | undefined;
  if (!hotkey) return;
  if (settings?.rebinding === hotkey) invoke("rebind_cancel");
  else invoke("rebind_begin", { hotkey });
});
// The settings view's keys. While a hotkey waits for its new combination, every key goes
// to the core as part of it (a modifier on its own waits for the key, Esc cancels, and
// Backspace turns it off), and none does what it would do here, but for two that no hotkey
// could be: Tab alone moves to the waiting row's buttons, and Enter or Space alone presses
// the one focused. Otherwise Esc goes back.
document.addEventListener("keydown", (e) => {
  if (!settings || e.isComposing) return;
  if (settings.rebinding) {
    const plain = !(e.ctrlKey || e.altKey || e.metaKey);
    const onButton = e.target instanceof HTMLButtonElement && (e.key === "Enter" || e.key === " ");
    if (plain && (e.key === "Tab" || onButton)) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.repeat) return;
    const press = { code: e.code, ctrl: e.ctrlKey, alt: e.altKey, shift: e.shiftKey, win: e.metaKey };
    invoke("rebind_key", { press });
  } else if (e.key === "Escape") {
    e.preventDefault();
    invoke("settings_close");
  }
});

listen<PanelState>("overlay-state", (e) => render(e.payload));

