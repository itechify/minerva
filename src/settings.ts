import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { momentText } from "./dates";
import { byId, span } from "./dom";
import {
  HOTKEY_NAMES,
  KEY_ONLY,
  codeLabel,
  hotkeyAvailable,
  hotkeyName,
  keyLabel,
  sameHotkey,
  type Hotkey,
  type Hotkeys,
  type RebindProblem,
  type SettingsNotice,
  type SettingsWindow,
} from "./hotkeys";
import { dragByHandles, followView, heightReporter } from "./shell";

/** Mirror of `minerva_core::Refresh`. */
type Refresh = "upToDate" | "unreachable" | "needsUpdate" | "refreshing";

/** Mirror of `minerva_core::GameData`: when the map set was fetched, and the player's Refresh. */
type GameData = { fetchedAtUnixMs: number | null; refresh: Refresh | null };

/** What the shell sends: the window while it is open (`null`: closed, and the page draws
 *  nothing), and the settings it changes as they are now. */
export type SettingsPage = {
  window: SettingsWindow | null;
  hotkeys: Hotkeys;
  unavailableHotkeys: string[];
  /** Those of them that don't name a key combination; the rest another app has. */
  invalidHotkeys: string[];
  /** The Deal Margin, in percent. */
  marginPercent: number;
  /** The Grind Tracker's switch: the EXP figures are read. */
  grindTrackerOn: boolean;
  /** "Check for updates". */
  checksOn: boolean;
  /** "Prices from maplestory.market". */
  marketPrices: boolean;
  /** What the Deal Line may be, the chosen one marked: maplestory.market's figures while
   *  they're on, then the player's Market Lines. */
  dealLines: DealLineChoice[];
  gameData: GameData;
};

/** Mirror of `minerva_core::DealLineChoice`. */
type DealLineChoice = { id: string; name: string; chosen: boolean };

const panelEl = byId("panel");
const settingsEl = byId("settings");
const settingsClose = byId("settings-close");
const hotkeyRows = byId("hotkey-rows");
const settingsNotice = byId("settings-notice");
const settingsAnnounce = byId("settings-announce");
const hotkeysUndo = byId("hotkeys-undo") as HTMLButtonElement;
const hotkeysReset = byId("hotkeys-reset") as HTMLButtonElement;
const dealMargin = byId("deal-margin") as HTMLInputElement;
const grindTrackerOn = byId("grind-tracker-on") as HTMLInputElement;
const checkForUpdatesOn = byId("check-for-updates-on") as HTMLInputElement;
const marketPricesOn = byId("market-prices-on") as HTMLInputElement;
const dealLineChoices = byId("deal-line-choices");
const gameDataState = byId("game-data-state");
const gameDataRefresh = byId("game-data-refresh") as HTMLButtonElement;

/**
 * What each part of the window was last rendered from. The view comes again whenever
 * anything in it changes: a part whose own inputs didn't change keeps its rows, and the
 * focus and hover on them.
 */
const renderedFrom = new Map<string, string>();
function changed(part: string, ...inputs: unknown[]): boolean {
  const key = JSON.stringify(inputs);
  if (renderedFrom.get(part) === key) return false;
  renderedFrom.set(part, key);
  return true;
}

/** The view as last rendered; null until the first. */
let shown: SettingsPage | null = null;
/** The window, as last rendered: null while it is closed. */
let settings: SettingsWindow | null = null;
/** The player clicked the Deal Margin field while the game had the keyboard: once the window
 *  takes it, the keys go there. */
let marginClicked = false;

/** A combination as read aloud or in a sentence: "Ctrl+Shift+F", or "off". */
const spokenKeys = (binding: string) => (binding === "" ? "off" : keyLabel(binding));

/**
 * A combination as the window draws it: "Ctrl+Shift+F", breaking only after a "+" so a long
 * one wraps rather than squeezing what the hotkey does; "Off" with none.
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
      return `Add Ctrl, Alt or Win. If ${keyLabel(problem.pressed)} alone were a hotkey, the game would never get that key, even in chat.`;
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
      return "Every hotkey is back to its default and turned on.";
    case "restoreUndone":
      return "Saved: your hotkeys are back as they were.";
  }
}

/** Game Data's line: how the player's Refresh is going or what it came to, or else when
 *  the map set was fetched. */
function gameDataText({ fetchedAtUnixMs, refresh }: GameData): string {
  switch (refresh) {
    case "refreshing":
      return "Refreshing…";
    case "upToDate":
      return "Up to date";
    case "unreachable":
      return "Couldn't reach OSMS Data Explorer";
    case "needsUpdate":
      return "OSMS's data changed: update Minerva to read it";
    case null:
      return fetchedAtUnixMs === null ? "Not fetched yet" : `Fetched ${momentText(fetchedAtUnixMs)}`;
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
function hotkeyRow(hotkey: Hotkey, name: string, page: SettingsPage, view: SettingsWindow): HTMLLIElement {
  const row = document.createElement("li");
  row.dataset.focus = `row:${hotkey}`;
  row.tabIndex = -1;
  const binding = page.hotkeys[hotkey];
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
    swap?.addEventListener("click", () => invoke("act", { action: "swapHotkeys" }));
    row.append(rowNote(rebindProblemText(problem), "problem", undefined, swap));
  } else if (waiting) {
    const turnOff =
      binding === "" ? undefined : textButton("Turn off", `turn-off:${hotkey}`, `Turn off ${name}`);
    if (turnOff) turnOff.title = "Turn off (Backspace)";
    turnOff?.addEventListener("click", () => invoke("act", { action: "turnOffHotkey" }));
    row.append(rowNote("Hold Ctrl+Shift and press a key · Esc cancels", "help", undefined, turnOff));
  } else if (binding === "" && KEY_ONLY.includes(hotkey)) {
    row.append(rowNote("It has no button, so it can't be used while it is off.", "help"));
  } else if (binding !== "" && !hotkeyAvailable(binding, page.unavailableHotkeys)) {
    const invalid = page.invalidHotkeys.some((key) => sameHotkey(key, binding));
    const why = invalid ? "These aren't keys a hotkey can use." : "Windows or another app has these keys.";
    row.append(rowNote(why, "broken", invalid ? "Invalid" : "Taken"));
    change.setAttribute("aria-label", `Change ${name}, now ${spokenKeys(binding)}, which does nothing: ${why}`);
  }
  return row;
}

/** What to announce for the window's latest change: the problem, the wait, or the notice. */
function settingsAnnouncement(view: SettingsWindow, hotkeys: Hotkeys): string {
  if (view.problem) return rebindProblemText(view.problem);
  if (view.rebinding) {
    return `Press the new keys for ${hotkeyName(view.rebinding)}, holding Ctrl and Shift. Escape cancels; Backspace turns it off.`;
  }
  return view.notice ? settingsNoticeText(view.notice, hotkeys) : "";
}

/** Where the keys go as the window takes the keyboard: the row waiting for its keys, or the
 *  first Change. */
function keyboardTarget(view: SettingsWindow): string {
  return view.rebinding ? `row:${view.rebinding}` : `change:${HOTKEY_NAMES[0][0]}`;
}

function focusOn(target: string) {
  settingsEl.querySelector<HTMLElement>(`[data-focus="${target}"]`)?.focus({ preventScroll: true });
}

/** The window took the keyboard: the keys go to the Deal Margin field if the player clicked
 *  it, or else where they go as it opens. */
function takeKeys(view: SettingsWindow) {
  if (marginClicked) {
    marginClicked = false;
    dealMargin.focus({ preventScroll: true });
    dealMargin.select();
  } else {
    focusOn(keyboardTarget(view));
  }
}

/**
 * The Settings window: each hotkey with what it does, its keys and a Change button.
 * Changing one, its row waits for the new keys, which the keys pressed now are (see the
 * keydown handler below), with its instruction, or why the keys pressed were refused, under
 * it. Every change is saved at once, and the line by Restore defaults says so. The rows are
 * drawn anew on every change, so the focus is put back where it was. Then maplestory.market's
 * switch, the Deal Line and the Deal Margin, the Grind Tracker's switch, Game Data and Updates.
 */
function render(page: SettingsPage) {
  shown = page;
  const view = page.window;
  document.body.classList.toggle("hidden", !view?.placement);
  const was = settings;
  settings = view;
  if (view === null) return;
  // The keyboard went back to the game: a margin typed in the field is saved, or put back,
  // as leaving the field does.
  if (was?.takesKeyboard && !view.takesKeyboard && document.activeElement === dealMargin) dealMargin.blur();
  const focused = (document.activeElement as HTMLElement | null)?.dataset?.focus;
  const { hotkeys, unavailableHotkeys, invalidHotkeys } = page;
  const redrawn = changed("hotkey-rows", hotkeys, unavailableHotkeys, invalidHotkeys, view.rebinding, view.problem);
  if (redrawn) {
    hotkeyRows.replaceChildren(...HOTKEY_NAMES.map(([hotkey, name]) => hotkeyRow(hotkey, name, page, view)));
  }
  // The Deal Margin as saved, unless the player is typing one.
  if (document.activeElement !== dealMargin) dealMargin.value = String(page.marginPercent);
  grindTrackerOn.checked = page.grindTrackerOn;
  checkForUpdatesOn.checked = page.checksOn;
  marketPricesOn.checked = page.marketPrices;
  const linesRedrawn = changed("deal-lines", page.dealLines);
  if (linesRedrawn) dealLineChoices.replaceChildren(...page.dealLines.map(dealLineChoice));
  const gameData = gameDataText(page.gameData);
  if (changed("game-data", gameData)) gameDataState.textContent = gameData;
  settingsNotice.textContent = view.notice ? settingsNoticeText(view.notice, hotkeys) : "";
  hotkeysUndo.hidden = view.notice?.kind !== "restored";
  // Nothing to restore, or a row waiting: one change at a time.
  hotkeysReset.disabled = view.atDefaults || view.rebinding !== null;
  const announcement = settingsAnnouncement(view, hotkeys);
  if (changed("settings-announce", announcement)) settingsAnnounce.textContent = announcement;
  // Opened, or opened again: the first Change. Waiting: its row, so no key presses a button.
  // Done: that row's Change. Restored: Undo, and undone: Restore defaults. Taking the
  // keyboard back: the Deal Margin field, if that was clicked. Otherwise where it was.
  if (was !== null && !was.takesKeyboard && view.takesKeyboard && !view.rebinding && was.opened === view.opened) {
    takeKeys(view);
    reportHeight();
    return;
  }
  const target =
    was === null || was.opened !== view.opened
      ? keyboardTarget(view)
      : view.rebinding && view.rebinding !== was.rebinding
        ? `row:${view.rebinding}`
        : !view.rebinding && was.rebinding
          ? `change:${was.rebinding}`
          : view.notice?.kind !== was.notice?.kind && view.notice?.kind === "restored"
            ? "undo"
            : view.notice?.kind !== was.notice?.kind && view.notice?.kind === "restoreUndone"
              ? "reset"
              : redrawn || linesRedrawn
                ? focused
                : undefined;
  if (target) focusOn(target);
  reportHeight();
}

// The shell sizes the window to the content, up to its most; past that the settings scroll
// under the window bar.
const reportHeight = heightReporter(panelEl, "page_height", true);
window.addEventListener("resize", reportHeight);
// The height depends on the font: measure again once it has loaded.
document.fonts.ready.then(reportHeight);

// Dragged by its header, never by its button.
dragByHandles();

// Whenever the window takes the keyboard, the keys go to the row waiting for them, or else
// the first Change; unless the player's click put them in the window already (the Deal
// Margin field, say).
window.addEventListener("focus", () => {
  if (settings && (marginClicked || !settingsEl.contains(document.activeElement))) takeKeys(settings);
});

settingsClose.addEventListener("click", () => invoke("act", { action: "closeSettings" }));
// A click on the Deal Margin field while the game has the keyboard: the window takes it, so
// the player can type the margin there.
dealMargin.addEventListener("pointerdown", () => {
  if (settings && !settings.takesKeyboard) {
    marginClicked = true;
    invoke("act", { action: "focusSettings" });
  }
});
// The Deal Margin is saved once the player is done with it (Enter, or leaving the field): a
// whole percent from 0 to 99. What isn't one goes back to the margin saved.
dealMargin.addEventListener("change", () => {
  const percent = Math.round(Number(dealMargin.value));
  if (dealMargin.value.trim() !== "" && Number.isFinite(percent) && percent >= 0) {
    invoke("act", { action: { setDealMargin: { percent: Math.min(percent, 99) } } });
  } else if (shown) {
    dealMargin.value = String(shown.marginPercent);
  }
});
// Enter keeps the margin typed; Esc puts the saved one back, and a second Esc closes the
// window.
dealMargin.addEventListener("keydown", (e) => {
  if (e.key === "Enter") dealMargin.blur();
  if (e.key === "Escape" && shown) {
    e.preventDefault();
    e.stopPropagation();
    dealMargin.value = String(shown.marginPercent);
    dealMargin.blur();
    settingsEl.focus({ preventScroll: true });
  }
});
grindTrackerOn.addEventListener("change", () => invoke("act", { action: { setGrindTracker: { on: grindTrackerOn.checked } } }));
/** One thing the Deal Line may be: a radio and its name, choosing it with a click. */
function dealLineChoice(choice: DealLineChoice): HTMLLabelElement {
  const label = document.createElement("label");
  label.className = "deal-line-choice";
  const radio = document.createElement("input");
  radio.type = "radio";
  radio.name = "deal-line";
  radio.checked = choice.chosen;
  radio.dataset.focus = `deal-line:${choice.id}`;
  radio.addEventListener("change", () => invoke("choose_deal_line", { line: choice.id }));
  label.append(radio, span("name", choice.name));
  return label;
}

marketPricesOn.addEventListener("change", () => invoke("act", { action: { setMarketPrices: { on: marketPricesOn.checked } } }));

checkForUpdatesOn.addEventListener("change", () =>
  invoke("act", { action: { setCheckForUpdates: { on: checkForUpdatesOn.checked } } }),
);
hotkeysReset.addEventListener("click", () => invoke("act", { action: "resetHotkeys" }));
gameDataRefresh.addEventListener("click", () => invoke("act", { action: "refreshGameData" }));
hotkeysUndo.addEventListener("click", () => invoke("act", { action: "undoResetHotkeys" }));
// A row's Change button waits for its new combination, with the keyboard; while it waits,
// it cancels.
hotkeyRows.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLButtonElement>("button[data-hotkey]") : null;
  const hotkey = button?.dataset.hotkey as Hotkey | undefined;
  if (!hotkey) return;
  if (settings?.rebinding === hotkey) invoke("act", { action: "cancelRebind" });
  else invoke("act", { action: { beginRebind: hotkey } });
});
// The window's keys. While a hotkey waits for its new combination, every key goes to the
// core as part of it (a modifier on its own waits for the key, Esc cancels, and Backspace
// turns it off), and none does what it would do here, but for two that no hotkey could be:
// Tab alone moves to the waiting row's buttons, and Enter or Space alone presses the one
// focused. Otherwise Esc closes the window.
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
    invoke("act", { action: "closeSettings" });
  }
});

followView<SettingsPage>("settings-state", "page_view", render);
reportHeight();
