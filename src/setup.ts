import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hotkeyAvailable, keyLabel, type Hotkeys } from "./hotkeys";
import { mapMark, type MapMarks } from "./map-marks";
import { momentText } from "./dates";
import { LOADING_MAP_LIST, TITLE_LOST, recognitionProblem, type Recognition } from "./recognition";

/** A map, as `minerva_core::CurrentMap` names it. */
type FoundMap = { id: string; streetName: string; mapName: string; mark: string | null };

type SetupReason = "firstRun" | "requested";

/** Mirror of `minerva_core::UpdateStatus`. */
type UpdateStatus = "notChecked" | "checking" | "upToDate" | "newerVersion" | "failed";

/** Mirror of `minerva_core::UpdateStage`. */
type UpdateStage =
  | "notifyOnly"
  | "out"
  | "asking"
  | "downloading"
  | "downloadingForQuit"
  | "installsWhenYouQuit"
  | "finishingBeforeQuit"
  | "installing"
  | "downloadFailed"
  | "installFailed"
  | "changed";

/** Mirror of `minerva_core::UpdateOffer`. */
type UpdateOffer = "nothing" | "releasePage" | "update" | "choices" | "updateNow" | "tryAgain";

/** Mirror of `minerva_core::UpdateNotice`: decided by the core, only put into words here. */
type UpdateNotice = { version: string; releaseUrl: string; stage: UpdateStage; offer: UpdateOffer };

/** Mirror of `minerva_core::Updates`. */
type Updates = {
  /** "Check for updates" is on (the panel's Settings). */
  checksOn: boolean;
  status: UpdateStatus;
  lastCheckedUnixMs: number | null;
  notice: UpdateNotice | null;
};

/** Mirror of the shell's `SetupView`. */
type SetupView = {
  setup: {
    needed: SetupReason | null;
    /** The map the minimap title shows, once one is found. */
    map: FoundMap | null;
    credit: string | null;
  };
  /** No minimap title has been found for a while: the minimap may be closed. */
  titleLost: boolean;
  /** The Grind Tracker is on (the panel's Settings). */
  grindTrackerOn: boolean;
  unavailableHotkeys: string[];
  /** Those of them that don't name a key combination; the rest another app has. */
  invalidHotkeys: string[];
  recognition: Recognition;
  game: { width: number; height: number } | null;
  gameTitle: string;
  hotkeys: Hotkeys;
  /** Every Map Mark, by OSMS's name for it, which a recognised map names its own by. */
  mapMarks: MapMarks;
  /** Whether a newer version is out, for About & Credits. */
  updates: Updates;
};

const STEPS = ["warning", "window", "requirements", "find", "exp"] as const;
/** Pages opened from the window bar, outside the walk-through: back returns to the step left. */
const ASIDES = ["guide", "about"] as const;
type Aside = (typeof ASIDES)[number];
type Step = (typeof STEPS)[number] | Aside;
const isAside = (s: Step): s is Aside => (ASIDES as readonly Step[]).includes(s);

const $ = (id: string) => document.getElementById(id)!;
let step: Step = "warning";
let view: SetupView | null = null;
/** Why setup was last opened; a new reason restarts the walk-through. */
let openedFor: SetupReason | null = null;
let picking = false;
let returnStep: Step = "window";
let renderedRead = "";

function show(next: Step) {
  if (isAside(next) && !isAside(step)) returnStep = step;
  step = next;
  for (const section of document.querySelectorAll<HTMLElement>("[data-step]")) {
    section.hidden = section.dataset.step !== step;
  }
  // A re-setup skips the warning, so it has one step fewer.
  const steps = openedFor === "firstRun" ? STEPS : STEPS.slice(1);
  $("progress").textContent =
    step === "guide" ? "Guide" : step === "about" ? "About & Credits" : `Step ${steps.indexOf(step) + 1} of ${steps.length}`;
  // On its own page, its button would only repeat the bar's label (and wrap the bar).
  $("about-open").hidden = step === "about";
  render();
  document.querySelector<HTMLElement>(`[data-step="${step}"] h2`)?.focus();
}

function move(by: number) {
  if (isAside(step)) return;
  show(STEPS[Math.min(Math.max(STEPS.indexOf(step) + by, 0), STEPS.length - 1)]);
}

function shortcut(name: keyof Hotkeys): string {
  if (!view) return "";
  const binding = view.hotkeys[name];
  if (binding === "") return "Off";
  return keyLabel(binding) + (hotkeyAvailable(binding, view.unavailableHotkeys) ? "" : " (unavailable)");
}

const NOTICES: Record<SetupReason, string> = {
  firstRun: "",
  requested: "Here is setup again. Minerva finds the minimap title by itself, so there's nothing to redo.",
};

function render() {
  if (!view) return;
  const { setup, game, gameTitle, recognition } = view;
  // About & Credits is about the app, not this setup: no notice over it.
  const notice = openedFor && step !== "about" ? NOTICES[openedFor] : "";
  $("notice").hidden = !notice;
  $("notice").textContent = notice;

  const windowStatus = game
    ? `Found the “${gameTitle}” window (${game.width}×${game.height}).`
    : `Can't find a window titled “${gameTitle}”. Start MapleStory, or pick its window by hand.`;
  if ($("window-status").textContent !== windowStatus) $("window-status").textContent = windowStatus;
  ($("window-next") as HTMLButtonElement).disabled = !game;
  ($("pick") as HTMLButtonElement).disabled = picking;
  const found = setup.map;
  const box = $("test-read");
  const shownMark = found?.mark ? view.mapMarks[found.mark] : undefined;
  const readKey = JSON.stringify([found, recognition.status, shownMark, view.titleLost]);
  if (readKey !== renderedRead) {
    renderedRead = readKey;
    // Found, the map shows as the overlay's title bar will show it.
    let kind: "reading" | "recognised" | "problem" = "reading";
    if (found) {
      kind = "recognised";
      const title = document.createElement("div");
      title.className = "title";
      const street = document.createElement("p");
      street.className = "street";
      street.textContent = found.streetName;
      const map = document.createElement("p");
      map.className = "map";
      map.textContent = found.mapName;
      title.append(street, map);
      // Its Map Mark beside both lines, as the overlay's title bar draws it; with none, no
      // room for it.
      const mark = found.mark && view.mapMarks[found.mark] ? [mapMark(view.mapMarks, found.mark)] : [];
      box.replaceChildren(...mark, title, tag("Found"));
    } else if (recognition.status === "starting") {
      box.textContent = LOADING_MAP_LIST;
    } else if (recognitionProblem(recognition)) {
      kind = "problem";
      box.textContent = recognitionProblem(recognition);
    } else if (view.titleLost) {
      kind = "problem";
      box.textContent = `${TITLE_LOST} Open it in the game, or carry on: Minerva keeps looking.`;
    } else {
      box.textContent = "Looking for the minimap title…";
    }
    box.className = `test-read ${kind}`;
  }
  $("test-credit").textContent = setup.credit ?? "";

  // The Grind Tracker needs no mark: it is on, or the player turned it off in Settings.
  $("exp-status").textContent = view.grindTrackerOn
    ? "It's on. To turn it off, open Settings, the gear at the top of the panel."
    : "It's off. To turn it on, open Settings, the gear at the top of the panel.";

  $("handoff").hidden = !found;
  $("resetup-hint").textContent = hotkeyAvailable(view.hotkeys.setup, view.unavailableHotkeys)
    ? `${shortcut("setup")} reopens setup and this guide while you play.`
    : view.hotkeys.setup === ""
      ? "Help & Setup, the ? at the top of the panel, reopens setup and this guide while you play."
      : "The setup shortcut is unavailable. Resolve the shortcut warning before closing this window.";
  for (const label of document.querySelectorAll<HTMLElement>("[data-shortcut]")) {
    label.textContent = shortcut(label.dataset.shortcut as keyof Hotkeys);
  }
  // The hotkeys are changed on the panel, which shows only with the game.
  ($("change-hotkeys") as HTMLButtonElement).disabled = !game;
  $("change-hotkeys-note").textContent = game ? "" : "Start the game first: the hotkeys are changed on its panel.";
  $("change-hotkeys-note").hidden = !!game;
  $("hotkey-problem").textContent = hotkeyProblems(view);
  $("hotkey-problem").hidden = $("hotkey-problem").textContent === "";

  const { updates } = view;
  const updateState = updateText(updates);
  if ($("update-state").textContent !== updateState) $("update-state").textContent = updateState;
  $("update-state").hidden = updateState === "";
  const newer = updates.notice;
  $("update-newer").hidden = !newer;
  $("update-version").textContent = newer?.version ?? "";
  ($("update-check") as HTMLButtonElement).disabled = updates.status === "checking";

  const progress = newer ? updateProgress(newer) : "";
  if ($("update-progress").textContent !== progress) $("update-progress").textContent = progress;
  $("update-progress").hidden = progress === "";
  const failed = newer ? updateFailure(newer) : "";
  $("update-failed-text").textContent = failed;
  $("update-failed").hidden = failed === "";
  $("update-ask").hidden = newer?.offer !== "choices";
  const install = newer ? updateButton(newer.offer) : "";
  $("update-install").textContent = install;
  $("update-install").hidden = install === "";
}

/** About & Credits' line on how updating is going, or, in a build that can't install, where
 *  to get the newer version. */
function updateProgress({ stage, version }: UpdateNotice): string {
  switch (stage) {
    case "notifyOnly":
      return "This copy of Minerva doesn't update itself: download the new version from its release page.";
    case "downloading":
      return `Downloading Minerva ${version}…`;
    case "downloadingForQuit":
      return `Downloading Minerva ${version}. It installs when you quit Minerva.`;
    case "installsWhenYouQuit":
      return `Minerva ${version} installs when you choose Quit Minerva in the tray.`;
    case "finishingBeforeQuit":
      return `Finishing Minerva ${version}'s download, then quitting to install it.`;
    case "installing":
      return `Starting Minerva ${version}'s installer. Minerva closes while it installs.`;
    case "out":
    case "asking":
    case "downloadFailed":
    case "installFailed":
    case "changed":
      return "";
  }
}

/** What went wrong updating, if it did: nothing was installed. */
function updateFailure({ stage, version }: UpdateNotice): string {
  switch (stage) {
    case "downloadFailed":
      return `Couldn't download Minerva ${version}, or it wasn't signed as Minerva's own, so nothing was installed.`;
    case "installFailed":
      return `Couldn't start Minerva ${version}'s installer, so this version stays.`;
    case "changed":
      return `The update changed since Minerva ${version} was found, so nothing was installed.`;
    default:
      return "";
  }
}

/** The Update button's label, or "" while it offers nothing. */
function updateButton(offer: UpdateOffer): string {
  switch (offer) {
    case "update":
      return "Update";
    case "updateNow":
      return "Update now";
    case "tryAgain":
      return "Try again";
    case "nothing":
    case "releasePage":
    case "choices":
      return "";
  }
}

/** About & Credits' line on updates: what the last check came to, or one under way. A newer
 *  version has a line of its own, which a failed check after it leaves showing. */
function updateText({ checksOn, status, lastCheckedUnixMs }: Updates): string {
  const lastChecked = lastCheckedUnixMs === null ? "" : ` (last checked ${momentText(lastCheckedUnixMs)})`;
  switch (status) {
    case "notChecked":
      return checksOn ? "" : "Check for updates is off in Settings: Minerva checks only when you ask.";
    case "checking":
      return "Checking for updates…";
    case "upToDate":
      return `Minerva is up to date${lastChecked}.`;
    case "newerVersion":
      return lastCheckedUnixMs === null ? "" : `Last checked ${momentText(lastCheckedUnixMs)}.`;
    case "failed":
      return `Couldn't check for updates${lastChecked}.`;
  }
}

/** "Recognised", in the overlay's tag vocabulary. */
function tag(text: string): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = "tag";
  el.textContent = text;
  return el;
}

/** Each hotkey that does nothing, with its real cause, then how to fix it. */
function hotkeyProblems(view: SetupView): string {
  const invalid = view.invalidHotkeys;
  const taken = view.unavailableHotkeys.filter((k) => !invalid.includes(k));
  if (taken.length + invalid.length === 0) return "";
  const list = (keys: string[]) => keys.map(keyLabel).join(", ");
  const sentences = [];
  if (taken.length > 0) {
    sentences.push(
      taken.length === 1
        ? `${list(taken)} is taken by another app, so it does nothing.`
        : `${list(taken)} are taken by another app, so they do nothing.`,
    );
  }
  if (invalid.length > 0) {
    sentences.push(
      invalid.length === 1 ? `${list(invalid)} isn't a key combination.` : `${list(invalid)} aren't key combinations.`,
    );
  }
  // One way to recover, in one sentence.
  const them = taken.length + invalid.length === 1 ? "it" : "them";
  sentences.push(
    `${taken.length > 0 ? "Close the other app, or change" : "Change"} ${them} in the panel's Settings (the gear at its top).`,
  );
  return sentences.join(" ");
}

function update(next: SetupView) {
  const before = view?.updates.notice?.stage;
  view = next;
  const needed = view.setup.needed;
  if (needed && needed !== openedFor) {
    openedFor = needed;
    show(needed === "firstRun" ? "warning" : "window");
  } else {
    if (!needed) openedFor = null;
    render();
  }
  // Choosing to update (from the tray, say) shows how it goes, or asks first, in About.
  const stage = view.updates.notice?.stage;
  const shows = stage === "asking" || stage === "downloading" || stage === "downloadingForQuit";
  if (shows && stage !== before && step !== "about") show("about");
}

listen<SetupView>("setup-state", (e) => update(e.payload));
// The shell may have sent the state before this page was listening.
invoke<SetupView | null>("page_view").then((v) => {
  if (v && !view) update(v);
});

$("guide-open").addEventListener("click", () => show("guide"));
// The panel's settings view comes in front with the keyboard, its first Change focused.
$("change-hotkeys").addEventListener("click", () => {
  $("action-error").hidden = true;
  invoke("panel_press", { control: "settings" }).catch(actionError);
});
$("guide-back").addEventListener("click", () => show(returnStep));
$("about-open").addEventListener("click", () => show("about"));
$("about-back").addEventListener("click", () => show(returnStep));
$("site-open").addEventListener("click", () => {
  $("action-error").hidden = true;
  invoke("panel_press", { control: "openSite" }).catch(actionError);
});
$("github-open").addEventListener("click", () => {
  $("action-error").hidden = true;
  invoke("panel_press", { control: "openGithub" }).catch(actionError);
});
$("discord-open").addEventListener("click", () => {
  $("action-error").hidden = true;
  invoke("panel_press", { control: "openDiscord" }).catch(actionError);
});
$("update-check").addEventListener("click", () => {
  $("action-error").hidden = true;
  invoke("act", { action: "checkForUpdates" }).catch(actionError);
});
$("update-whats-new").addEventListener("click", () => {
  $("action-error").hidden = true;
  invoke("panel_press", { control: "openRelease" }).catch(actionError);
});
$("update-release").addEventListener("click", () => {
  $("action-error").hidden = true;
  invoke("panel_press", { control: "openRelease" }).catch(actionError);
});
$("update-install").addEventListener("click", () => {
  $("action-error").hidden = true;
  invoke("panel_press", { control: "update" }).catch(actionError);
});
for (const [id, action] of [
  ["update-now", "updateNow"],
  ["update-when-i-quit", "updateWhenIQuit"],
  ["update-later", "updateLater"],
] as const) {
  $(id).addEventListener("click", () => {
    $("action-error").hidden = true;
    invoke("act", { action }).catch(actionError);
  });
}
getVersion().then((version) => ($("app-version").textContent = version));

function actionError(error: unknown) {
  $("action-error").textContent = `Couldn't complete that action: ${String(error)}`;
  $("action-error").hidden = false;
}

for (const button of document.querySelectorAll<HTMLButtonElement>("[data-action]")) {
  button.addEventListener("click", () => {
    const action = button.dataset.action;
    if (action === "next") move(1);
    else if (action === "back") move(-1);
    else if (action === "close") getCurrentWindow().close();
  });
}

$("finish").addEventListener("click", () => invoke("act", { action: "finishSetup" }).catch(actionError));

// Picking by hand: the player has a few seconds to put the mouse over the game window.
// Nothing is clicked or sent to the game; only the window under the pointer is read.
$("pick").addEventListener("click", async () => {
  picking = true;
  render();
  const status = $("pick-status");
  for (let s = 3; s > 0; s--) {
    status.textContent = `Hover the mouse over the game window… ${s}`;
    await new Promise((r) => setTimeout(r, 1000));
  }
  try {
    const title = await invoke<string>("pick_game_window");
    status.textContent = `Picked “${title}”.`;
  } catch (e) {
    status.textContent = `Couldn't pick a window: ${e}. Try again.`;
  }
  picking = false;
  render();
});

show("warning");
