// The Pedestals, on the Map Leaderboards page (leaderboards/index.html): the three Characters
// with the most places in a Level Band's top 3, or the three with the highest EXP/h on any map,
// one at a time, a tab for each (ADR 0024, amended); the "?" says what the one shown ranks. Both
// are drawn, one over the other, so a switch moves nothing around them, and a tab clicked
// restacks the steps (DESIGN.md, Motion). What they show is worked out in
// leaderboards.ts.
import { byId, drawPedestal, names, read, restack, unreachable } from "./api.ts";
import { type PedestalChoice, type PedestalKind, type PedestalsAnswer, pedestalChoices } from "./leaderboards.ts";

const tabs = [...byId("pedestal-tabs").querySelectorAll<HTMLButtonElement>("[role=tab]")];
/** Each Pedestal's panel and steps, by its kind. */
const panels: Record<PedestalKind, HTMLElement> = { podiums: byId("pedestal-panel-podiums"), exp: byId("pedestal-panel-exp") };
const stepsOf: Record<PedestalKind, HTMLOListElement> = { podiums: byId("pedestal-podiums"), exp: byId("pedestal-exp") };
const said = byId("pedestal-said");
const creditOf: Record<PedestalKind, HTMLElement> = { podiums: byId("pedestal-credit-podiums"), exp: byId("pedestal-credit-exp") };
const help = byId<HTMLButtonElement>("pedestal-help");
const about = byId("pedestal-about");

/** The Pedestals, once read. */
let choices: PedestalChoice[] | null = null;
/** The Pedestal shown. */
let shown: PedestalKind = "podiums";

/** Mark the tab shown, show its panel and say what it ranks. */
function mark() {
  for (const tab of tabs) {
    const kind = tab.dataset.pedestal as PedestalKind;
    const open = kind === shown;
    tab.setAttribute("aria-selected", String(open));
    tab.tabIndex = open ? 0 : -1;
    panels[kind].toggleAttribute("data-off", !open);
  }
  const choice = choices?.find((c) => c.kind === shown);
  if (choice !== undefined) about.textContent = choice.about;
}

/** Show the Pedestal `tab` names, its Characters restacking from the other's steps. */
function pick(tab: HTMLButtonElement) {
  const kind = tab.dataset.pedestal as PedestalKind;
  if (kind === shown) return;
  const from = stepsOf[shown];
  shown = kind;
  mark();
  if (choices !== null) restack(from, stepsOf[kind]);
}

for (const tab of tabs) tab.addEventListener("click", () => pick(tab));

// The arrow keys move between the tabs, as a tab list's do, each showing its Pedestal.
byId("pedestal-tabs").addEventListener("keydown", (event) => {
  const at = tabs.indexOf(document.activeElement as HTMLButtonElement);
  if (at < 0) return;
  const to = { ArrowLeft: at - 1, ArrowRight: at + 1, Home: 0, End: tabs.length - 1 }[event.key];
  if (to === undefined) return;
  event.preventDefault();
  const tab = tabs[(to + tabs.length) % tabs.length];
  tab.focus();
  pick(tab);
});

// The "?" says what the Pedestal ranks under the pointer and at the keyboard's focus (site.css),
// and stays open at a tap, for a touch screen, until tapped again or away from. Escape hushes
// it however it opened, until the pointer leaves it or it is pressed again.
const helpBox = help.parentElement!;
const tip = (open: boolean) => {
  help.setAttribute("aria-expanded", String(open));
  helpBox.classList.remove("lb-hushed");
};
help.addEventListener("click", () => tip(help.getAttribute("aria-expanded") !== "true"));
help.addEventListener("blur", () => tip(false));
helpBox.addEventListener("mouseleave", () => helpBox.classList.remove("lb-hushed"));
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  help.setAttribute("aria-expanded", "false");
  helpBox.classList.add("lb-hushed");
});

function load() {
  read<PedestalsAnswer>("/api/pedestals").then((answer) => {
    stepsOf.podiums.classList.remove("lb-waiting");
    if (answer === null) {
      stepsOf.podiums.hidden = true;
      return unreachable(said, () => {
        said.hidden = true;
        stepsOf.podiums.classList.add("lb-waiting");
        stepsOf.podiums.hidden = false;
        load();
      });
    }
    choices = pedestalChoices(answer, names);
    for (const choice of choices) {
      drawPedestal(stepsOf[choice.kind], choice.pedestal);
      creditOf[choice.kind].hidden = !choice.pedestal.portraits;
    }
    mark();
  });
}
load();
