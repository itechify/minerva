import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { byId, span } from "./dom";
import { signed, whole } from "./figures";
import { followView } from "./shell";

/** Mirror of `minerva_core::ListedPrice`: `quantity` of the item for `mesos`. */
type ListedPrice = { quantity: number; mesos: number };

/** Mirror of `minerva_core::DealGrade`: how good a deal is, graded as the game grades its items. */
type DealGrade = "good" | "great" | "steal";

/** Mirror of `minerva_core::DealItem`: `rechargeable` for a throwing star or a bullet, whose
 * listing is one set, its number how many the set holds. */
type DealItem = { id: string; name: string; rechargeable: boolean };

/** Mirror of `minerva_core::DealBadge`: one listing's badge, level with it (physical px
 * from the window's top), and what it says; `covered`, what was read there before the
 * pointer or a tooltip covered it. */
export type DealBadge = { row: number; top: number; height: number; covered: boolean; open: boolean } & (
  | { verdict: "deal"; item: DealItem; listed: ListedPrice; percent: number; lineMesos: number; grade: DealGrade }
  | { verdict: "noDeal"; item: DealItem; listed: ListedPrice; percent: number; lineMesos: number }
  | { verdict: "noPrice"; item: DealItem; listed: ListedPrice }
  | { verdict: "unknown" }
);

/** A deal's grade, as its accessible name says it. */
const GRADE_WORDS: Record<DealGrade, string> = { good: "a good deal", great: "a great deal", steal: "a steal" };

/** A listing's band at least this tall (CSS px) holds a roomy badge: 38px, the gauge 5px. */
const ROOMY_BAND = 44;

/** What the shell sends: the badges (none while the window draws nothing), and the Deal
 * Line's name for their words, and whether it is one of maplestory.market's figures. */
type BadgesView = { badges: DealBadge[]; dealLine: string | null; fromMarket: boolean };

const list = byId("badges");

/** How far off the Deal Line a price is, as the badge says it: "−32%", "+12%", "0%"; a deal
 * under a percent off is "−<1%". */
function off(percent: number, deal: boolean): string {
  if (percent === 0) return deal ? "−<1%" : "0%";
  return percent > 0 ? `+${percent}%` : signed(`${percent}%`);
}

/** What was compared, as the game writes prices: the price of one, in whole mesos or to a
 * tenth; for a throwing star or a bullet, its set's. */
function each(item: DealItem, { quantity, mesos }: ListedPrice): string {
  if (item.rechargeable) return `${whole(mesos)} mesos for the set of ${whole(quantity)}`;
  const one = mesos / quantity;
  return `${Number.isInteger(one) ? whole(one) : one.toLocaleString("en-US", { maximumFractionDigits: 1 })} mesos each`;
}

/**
 * Where the listing's price sits against the Deal Line, on a gauge from nothing to twice the
 * line, the line at its middle: the share filled, and whether it runs past the gauge's end.
 * The price compared is the one the verdict judged: of one item, or a set's.
 */
function gauge(item: DealItem, { quantity, mesos }: ListedPrice, lineMesos: number): { fill: number; past: boolean } {
  const compared = item.rechargeable ? mesos : mesos / quantity;
  const ratio = lineMesos > 0 ? compared / lineMesos : 2;
  return { fill: Math.min(Math.max(ratio / 2, 0.02), 1), past: ratio > 2 };
}

/** A price on a badge, as players write it: in full under a million ("188,888"), past it in
 * millions to two places ("1.25m", "2m"); its accessible name says it in full. */
function short(mesos: number): string {
  if (mesos < 1_000_000) return whole(mesos);
  return `${Number((mesos / 1_000_000).toFixed(2))}m`;
}

/** The badge's words, and what it says in full for its accessible name. */
function words(badge: DealBadge, line: string, fromMarket: boolean): [string, string] {
  switch (badge.verdict) {
    case "deal":
      return [
        off(badge.percent, true),
        `${badge.item.name}: ${each(badge.item, badge.listed)}, ${GRADE_WORDS[badge.grade]} against ${line} (${whole(badge.lineMesos)}). Click to open its card.`,
      ];
    case "noDeal":
      return [
        off(badge.percent, false),
        `${badge.item.name}: ${each(badge.item, badge.listed)}, not a deal against ${line} (${whole(badge.lineMesos)}). Click to open its card.`,
      ];
    case "noPrice":
      return [
        "no price",
        fromMarket
          ? `${badge.item.name}: ${line} has no price for it. Click to open its card.`
          : `${badge.item.name}: ${line} has no price for it. Click to set one on its card.`,
      ];
    case "unknown":
      return ["?", "Minerva couldn't read this listing reliably, so it doesn't judge it."];
  }
}

function badgeItem(badge: DealBadge, line: string, fromMarket: boolean): HTMLLIElement {
  const row = document.createElement("li");
  row.className = "deal-slot";
  // The core places badges in physical px; the page lays out in CSS px.
  const scale = window.devicePixelRatio || 1;
  row.style.top = `${badge.top / scale}px`;
  row.style.height = `${badge.height / scale}px`;
  // Where the band has room (54 CSS px at 100%), the badge grows into it with a fuller gauge;
  // at 200% its 27 keep the compact one.
  row.classList.toggle("roomy", badge.height / scale >= ROOMY_BAND);
  const [text, said] = words(badge, line, fromMarket);
  const full = !badge.covered
    ? said
    : badge.verdict === "unknown"
      ? "The pointer or a tooltip is covering this listing, and Minerva hadn't read it before that."
      : `The pointer or a tooltip is covering this listing. This is what Minerva read before it was covered. ${said}`;
  const opens = badge.verdict !== "unknown";
  const el = document.createElement(opens ? "button" : "span");
  el.className = `deal-badge ${badge.verdict}`;
  el.classList.toggle("covered", badge.covered);
  el.classList.toggle("open", badge.open);
  if (badge.verdict === "deal") el.classList.add(badge.grade);
  // Said to screen readers only: a tooltip would pop up over the shop's listings.
  el.setAttribute("aria-label", full);
  if (el instanceof HTMLButtonElement) {
    el.type = "button";
    el.dataset.row = String(badge.row);
    el.setAttribute("aria-pressed", String(badge.open));
  }
  const head = document.createElement("span");
  head.className = "deal-head";
  // Covered, its verdict stays, with the doubt dot: it isn't being read now.
  if (badge.covered && badge.verdict !== "unknown") {
    const dot = span("deal-covered", "");
    dot.setAttribute("aria-hidden", "true");
    head.append(dot);
  }
  const figure = span("deal-words", text);
  // A deal names the line's price too, a step quieter than how far below it the deal is.
  if (badge.verdict === "deal") figure.append(span("deal-at", short(badge.lineMesos)));
  head.append(figure);
  el.append(head);
  // A judged price's gauge, so the column beside the shop reads as a chart.
  if (badge.verdict === "deal" || badge.verdict === "noDeal") {
    const { fill, past } = gauge(badge.item, badge.listed, badge.lineMesos);
    const meter = span("deal-gauge", "");
    meter.setAttribute("aria-hidden", "true");
    meter.style.setProperty("--fill", String(fill));
    meter.classList.toggle("past", past);
    el.append(meter);
  }
  row.append(el);
  return row;
}

let shown = "";
function render(view: BadgesView) {
  // Without badges the window draws nothing (and the shell lets clicks through it).
  document.body.classList.toggle("hidden", view.badges.length === 0);
  const key = JSON.stringify([view, window.devicePixelRatio]);
  if (key === shown) return;
  shown = key;
  const line = view.dealLine === null ? "the Deal Line" : view.fromMarket ? `maplestory.market's ${view.dealLine}` : view.dealLine;
  list.replaceChildren(...view.badges.map((badge) => badgeItem(badge, line, view.fromMarket)));
}

// A click on a badge shows its item's card. The window never takes the keyboard: it can't
// be activated.
list.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLElement>(".deal-badge[data-row]") : null;
  if (button?.dataset.row) invoke("act", { action: { openDealItem: { row: Number(button.dataset.row) } } });
});

let last: BadgesView | null = null;
followView<BadgesView>("deal-badges-state", "page_view", (view) => {
  last = view;
  render(view);
});
// A move to a display of another scale lays the badges out afresh.
window.addEventListener("resize", () => last && render(last));
