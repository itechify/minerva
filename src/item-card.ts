import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { dayText } from "./dates";
import { byId, reducedMotion, span, svgEl } from "./dom";
import { whole } from "./figures";
import { dragByHandles, followView, heightReporter } from "./shell";

/** Mirror of `minerva_core::ShopPrice`. */
type ShopPrice = { status: "sells"; mesos: number } | { status: "notSold" } | { status: "cash" } | { status: "unknown" };

/** Mirror of `minerva_core::MarketPrice`: a price the player keeps, and when they set it. */
type MarketPrice = { mesos: number; setAtUnixMs?: number };

/** Mirror of `minerva_core::MarketLine`: one of the player's Market Lines, with the item's
 * price on it, and whether it is the Deal Line the Deal Finder compares listings with. */
type MarketLine = { id: string; name: string; price: MarketPrice | null; dealLine: boolean };

/** Mirror of `minerva_core::NameProblem`. */
type NameProblem = { kind: "empty" } | { kind: "taken"; name: string };

/** Mirror of `minerva_core::MarketEdit`: what the market section has open. */
type MarketEdit =
  | { field: "price"; line: string; notAPrice: string | null }
  | { field: "name"; line: string | null; problem: NameProblem | null }
  | { field: "remove"; line: string; prices: number };

/** Mirror of `minerva_core::ItemCard`. */
type ItemCard = {
  id: string;
  name: string;
  /** Where its picture loads from, once it's in. */
  icon: string | null;
  shop: ShopPrice;
  market: MarketLine[];
  marketEdit: MarketEdit | null;
  savedCopyFromUnixMs: number | null;
  credit: string;
};

/** Mirror of `minerva_core::ItemCardChoice`. */
type ItemCardChoice = { id: string; kind: string; icon: string | null };

/** Mirror of `minerva_core::ShownItemCard`: `unfoldFrom`, the edge facing the Deal Badge it
 * opened from. */
type ShownItemCard = { card: number; unfoldFrom?: "left" | "right" } & (
  | ({ status: "card" } & ItemCard)
  | { status: "choose"; name: string; items: ItemCardChoice[] }
);

/** What the shell sends: the card, and whether it shows at all. */
type CardView = { shown: boolean; card: ShownItemCard | null };

const cardEl = byId("card");
const title = byId("title");
const body = byId("body");

const mesos = (amount: number) => `${whole(amount)} mesos`;

/** One figure: its label at the left, its value right-aligned, as the Mob Card sets them. */
function figure(label: string, value: Node | string, quiet = false): HTMLDivElement {
  const row = document.createElement("div");
  const dt = document.createElement("dt");
  dt.textContent = label;
  const dd = document.createElement("dd");
  dd.append(value);
  dd.classList.toggle("quiet", quiet);
  row.append(dt, dd);
  return row;
}

/** What a shop pays, or why it says nothing. */
function shopFigure(shop: ShopPrice): HTMLDivElement {
  switch (shop.status) {
    case "sells":
      return figure("Shop pays", mesos(shop.mesos));
    case "notSold":
      return figure("Shop pays", "Can't be sold", true);
    case "cash":
      return figure("Shop pays", "Cash item", true);
    case "unknown":
      return figure("Shop pays", "Not known", true);
  }
}

/** A link on the card: Portal Blue under the pointer, as a mob's name in the panel. */
function link(text: string, label: string, onClick: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "item-card-link";
  button.textContent = text;
  button.title = label;
  button.addEventListener("click", onClick);
  return button;
}

/** A light icon button in the card's body: `path` drawn as the panel's icons are. */
function iconButton(path: string, label: string, onClick: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button light";
  button.setAttribute("aria-label", label);
  button.title = label;
  const svg = svgEl("svg", { viewBox: "0 0 16 16", "aria-hidden": "true" });
  svg.append(svgEl("path", { d: path }));
  button.append(svg);
  button.addEventListener("click", onClick);
  return button;
}

/** What opens the item's page on maplestory.market, where its listings and prices are. */
const MARKET_LOOK_UP = "Look it up on maplestory.market";
const openMarket = () => invoke("item_card_open_market");
const marketButton = () => iconButton("M9 3h4v4M13 3L7.5 8.5M11.5 9.5V13h-8.5V4.5H6.5", MARKET_LOOK_UP, openMarket);

/** A field in the market section, open: what is there selected, Enter to keep what is
 * typed, Esc to leave everything as it was. Leaving it keeps what is typed, unless `onLeave`
 * says otherwise. */
let fieldDone = false;
function field(
  className: string,
  value: string,
  placeholder: string,
  label: string,
  enter: (typed: string) => void,
  onLeave: (typed: string) => void = enter,
): HTMLInputElement {
  const input = document.createElement("input");
  input.type = "text";
  input.className = `item-card-input ${className}`;
  input.autocomplete = "off";
  input.spellcheck = false;
  input.placeholder = placeholder;
  input.setAttribute("aria-label", label);
  input.value = value;
  fieldDone = false;
  const finish = (done: () => void) => {
    if (fieldDone) return;
    fieldDone = true;
    done();
  };
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") finish(() => enter(input.value));
    if (e.key === "Escape") finish(() => invoke("act", { action: "cancelCardMarket" }));
  });
  input.addEventListener("blur", () => {
    if (input.isConnected) finish(() => onLeave(input.value));
  });
  queueMicrotask(() => {
    input.focus();
    input.select();
  });
  return input;
}

/** What a field does, or why what was entered can't be kept. */
function hint(problem: string | null, otherwise: string): HTMLParagraphElement {
  const note = document.createElement("p");
  note.className = problem ? "item-card-hint problem" : "item-card-hint";
  note.textContent = problem ?? otherwise;
  return note;
}

const quoted = (name: string) => `“${name}”`;

/** The Deal Line's mark before a line's name: filled on the one the Deal Finder compares
 * listings with, hollow on the others, where a click makes it the Deal Line. */
function dealMark(line: MarketLine): HTMLButtonElement {
  const mark = document.createElement("button");
  mark.type = "button";
  mark.className = "deal-mark";
  mark.setAttribute("role", "radio");
  mark.setAttribute("aria-checked", String(line.dealLine));
  const label = line.dealLine
    ? `${quoted(line.name)} is the Deal Line: the Deal Finder compares Player Shop listings with it`
    : `Make ${quoted(line.name)} the Deal Line, which the Deal Finder compares Player Shop listings with`;
  mark.title = label;
  mark.setAttribute("aria-label", label);
  if (!line.dealLine) mark.addEventListener("click", () => invoke("item_card_choose_deal_line", { line: line.id }));
  return mark;
}

/** A line: the Deal Line's mark and its name at the left (a link that opens its name field)
 * and the item's price on it right-aligned (a link that opens its price field, the day it
 * was set in its tooltip), or "Not set". */
function lineRow(line: MarketLine): HTMLElement {
  const row = document.createElement("div");
  row.className = "item-card-line";
  const name = link(line.name, `Rename or remove ${quoted(line.name)}`, () =>
    invoke("item_card_edit_market_line", { line: line.id }),
  );
  name.classList.add("name");
  const editPrice = () => invoke("item_card_edit_market", { line: line.id });
  const setOn = line.price?.setAtUnixMs;
  const value = line.price
    ? link(
        mesos(line.price.mesos),
        `${line.name}${setOn === undefined ? "" : `, set ${dayText(new Date(setOn), false)}`}. Click to change it`,
        editPrice,
      )
    : link("Not set", `Set your price on ${quoted(line.name)}`, editPrice);
  value.classList.toggle("quiet", !line.price);
  value.classList.add("value");
  row.append(dealMark(line), name, value);
  return row;
}

/** A line's price field, open in its price's place. */
function priceField(card: ItemCard, line: MarketLine, notAPrice: string | null): HTMLElement[] {
  const row = document.createElement("div");
  row.className = "item-card-line editing";
  const input = field(
    "price",
    notAPrice ?? (line.price ? whole(line.price.mesos) : ""),
    "450,000 or 450k",
    `Your ${line.name} for ${card.name}`,
    (typed) => invoke("item_card_enter_market", { typed }),
  );
  input.inputMode = "decimal";
  row.append(span("name", line.name), input);
  const problem = notAPrice === null ? null : `${quoted(notAPrice)} isn't a price. Try 450,000, 450k or 1.2m.`;
  return [row, hint(problem, "Enter keeps it · Esc cancels · empty clears")];
}

/** A line's name field, open in its row (`line` null: a line being added at the end), with
 * the button that asks to remove the line. Left empty, it closes as it was. */
function nameField(line: MarketLine | null, problem: NameProblem | null): HTMLElement[] {
  const row = document.createElement("div");
  row.className = "item-card-line editing";
  const typed = problem?.kind === "taken" ? problem.name : problem ? "" : (line?.name ?? "");
  const enter = (typed: string) => invoke("item_card_enter_market_line", { typed });
  const input = field(
    "name",
    typed,
    "Name it, e.g. Lowest ask",
    line ? `Rename ${quoted(line.name)}` : "Name the new market line",
    enter,
    (typed) => (typed.trim() ? enter(typed) : invoke("act", { action: "cancelCardMarket" })),
  );
  row.append(input);
  if (line) {
    // Pressed, the field loses the keyboard: what it holds isn't kept, the question asked.
    const remove = iconButton(
      "M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5",
      `Remove ${quoted(line.name)}`,
      () => invoke("act", { action: "askRemoveCardMarketLine" }),
    );
    remove.addEventListener("mousedown", () => (fieldDone = true));
    row.append(remove);
  }
  const says =
    problem?.kind === "empty"
      ? "Give the line a name, or Esc to cancel."
      : problem?.kind === "taken"
        ? `There's a line named ${quoted(problem.name)} already.`
        : null;
  return [row, hint(says, line ? "Enter renames it · Esc cancels" : "Enter adds it to every card · Esc cancels")];
}

/** The question before a line goes, with its prices on every item, in the line's place. */
function removeQuestion(line: MarketLine, prices: number): HTMLElement {
  const box = document.createElement("div");
  box.className = "item-card-remove";
  const what =
    prices === 0
      ? `Remove ${quoted(line.name)}? It has no prices yet.`
      : `Remove ${quoted(line.name)} and its prices on ${whole(prices)} ${prices === 1 ? "item" : "items"}?`;
  const question = document.createElement("p");
  question.textContent = what;
  const buttons = document.createElement("div");
  buttons.className = "item-card-remove-buttons";
  const button = (text: string, action: string) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "text-button";
    b.textContent = text;
    b.addEventListener("click", () => invoke("act", { action }));
    return b;
  };
  buttons.append(button("Remove", "removeCardMarketLine"), button("Keep", "cancelCardMarket"));
  box.append(question, buttons);
  return box;
}

/** The Market Lines under a rule: the heading, with the buttons that add a line and look
 * the item up on maplestory.market, then each line, or what the market section has open in
 * its place, and at the foot the look-up again, in words. */
function marketSection(card: ItemCard): HTMLElement {
  const section = document.createElement("div");
  section.className = "item-card-market";
  const head = document.createElement("div");
  head.className = "item-card-market-head";
  const add = iconButton("M8 3.5v9M3.5 8h9", "Add a market line to every card", () =>
    invoke("act", { action: "addCardMarketLine" }),
  );
  head.append(span("label", "Market"), add, marketButton());
  section.append(head);
  const edit = card.marketEdit;
  for (const line of card.market) {
    if (edit?.line !== line.id) section.append(lineRow(line));
    else if (edit.field === "price") section.append(...priceField(card, line, edit.notAPrice));
    else if (edit.field === "name") section.append(...nameField(line, edit.problem));
    else section.append(removeQuestion(line, edit.prices));
  }
  if (edit?.field === "name" && edit.line === null) section.append(...nameField(null, edit.problem));
  else if (card.market.length === 0) section.append(hint(null, "No market lines. + adds one to every card."));
  const foot = link(MARKET_LOOK_UP, "Open the item's page on maplestory.market", openMarket);
  foot.classList.add("item-card-market-foot");
  section.append(foot);
  return section;
}

/** An item's picture as the game draws it, in a box that holds its place until it's in. */
function itemIcon(url: string | null): HTMLSpanElement {
  const icon = document.createElement("span");
  icon.className = "item-card-icon";
  if (url) {
    const img = document.createElement("img");
    img.src = url;
    img.alt = "";
    icon.append(img);
  }
  return icon;
}

/** The credit as the main window's info icon at the card's foot, its words in its tooltip. */
function creditFooter(text: string) {
  const footer = document.createElement("footer");
  const credit = document.createElement("span");
  credit.className = "credit";
  credit.setAttribute("role", "img");
  credit.title = text;
  credit.setAttribute("aria-label", text);
  const svg = svgEl("svg", { viewBox: "0 0 16 16", "aria-hidden": "true" });
  const ring = svgEl("circle", { cx: "8", cy: "8", r: "6.25" });
  const stem = svgEl("path", { d: "M8 7.4v3.9" });
  const dot = svgEl("circle", { class: "dot", cx: "8", cy: "4.9", r: ".95" });
  svg.append(ring, stem, dot);
  credit.append(svg);
  footer.append(credit);
  return footer;
}

/** One item's card: its picture beside its name. */
function renderCard(card: ItemCard) {
  title.replaceChildren(
    itemIcon(card.icon),
    link(card.name, `Open ${card.name} on MeowDB`, () => invoke("item_card_open_meowdb")),
  );
  const prices = document.createElement("dl");
  prices.className = "item-card-prices";
  prices.append(shopFigure(card.shop));
  const box = document.createElement("div");
  box.className = "item-card-box";
  box.append(prices, marketSection(card));
  const parts: Node[] = [box];
  if (card.savedCopyFromUnixMs !== null) {
    const note = document.createElement("p");
    note.className = "item-card-note";
    note.textContent = `OSMS Data Explorer unreachable — item data from ${dayText(new Date(card.savedCopyFromUnixMs), true)}`;
    parts.push(note);
  }
  parts.push(creditFooter(card.credit));
  body.replaceChildren(...parts);
}

/** The items sharing the name read, for the player to pick theirs by its picture. */
function renderChoice(name: string, items: ItemCardChoice[]) {
  title.replaceChildren(span("item-card-ask", `Which ${name}?`));
  const choices = document.createElement("ol");
  choices.className = "item-card-choices";
  choices.append(
    ...items.map((item) => {
      const row = document.createElement("li");
      const button = document.createElement("button");
      button.type = "button";
      button.className = "item-card-choice";
      button.title = `The ${item.kind} ${name}`;
      button.append(itemIcon(item.icon), span("", item.kind));
      button.addEventListener("click", () => invoke("item_card_pick", { id: item.id }));
      row.append(button);
      return row;
    }),
  );
  const hint = document.createElement("p");
  hint.className = "item-card-hint";
  hint.textContent = "Pick the one whose picture the game's tooltip shows.";
  const box = document.createElement("div");
  box.className = "item-card-box";
  box.append(choices);
  body.replaceChildren(hint, box);
}

let shownKey = "";
function render({ shown, card }: CardView) {
  // Not showing, the window draws nothing (and the shell lets clicks through it).
  document.body.classList.toggle("hidden", !shown || !card);
  if (!shown || !card) {
    shownKey = "";
    return;
  }
  const key = JSON.stringify(card);
  if (key === shownKey) return;
  shownKey = key;
  if (card.status === "card") renderCard(card);
  else renderChoice(card.name, card.items);
  if (card.card !== unfoldedCard) {
    unfoldedCard = card.card;
    unfold(card.unfoldFrom);
  }
  reportHeight();
}

/**
 * A card opened from a Deal Badge unfolds out of it, from the edge facing it: the window's one
 * authored motion, answering the click. Its content keeps its size and is uncovered, never
 * scaled, from a first frame already showing the third nearest the badge. Any other card,
 * or with reduced motion, shows at once.
 */
let unfoldedCard = -1;
function unfold(from: "left" | "right" | undefined) {
  cardEl.classList.remove("unfold-left", "unfold-right");
  if (!from || reducedMotion().matches) return;
  void cardEl.offsetWidth;
  cardEl.classList.add(`unfold-${from}`);
}
cardEl.addEventListener("animationend", () => cardEl.classList.remove("unfold-left", "unfold-right"));

// The shell sizes the window to the card, up to its most; past that the card scrolls under
// its bar.
const reportHeight = heightReporter(cardEl, "page_height");
window.addEventListener("resize", reportHeight);
// The height depends on the font, and on pictures arriving: measure again once in.
document.fonts.ready.then(reportHeight);
new ResizeObserver(reportHeight).observe(cardEl);

// Dragged by its bar, never by its button. The window takes clicks only while a card
// shows, and the keyboard only while a price or line name field is open.
dragByHandles();
byId("close").addEventListener("click", () => invoke("item_card_close"));

followView<CardView>("item-card-state", "page_view", render);
