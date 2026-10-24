// The FAQ's search: a field in the owl's words, as an NPC asks you to type an answer, that
// keeps only the questions holding every word typed, in their question or their answer, each
// opened, and says how many there are. Groups and headings left with none go, and come back
// as the words do. Escape, or emptying the field, shows every question as it was, folding
// again those it opened. An address naming a question or a group it hides shows everything
// first, so the address always lands, and a printed page shows every answer, as ever. Without the script there is no field: the owl's list
// is the index, and the browser's own find opens a folded answer.

import { make } from "../src/dom.ts";

const words = document.querySelector<HTMLElement>(".faq-head .npc-words");
const questions = [...document.querySelectorAll<HTMLDetailsElement>("details.qa")];

if (words && questions.length) {
  const form = make("form", "faq-filter");
  form.setAttribute("role", "search");
  form.action = "javascript:void 0";
  const label = make("label", "", "Look for a word");
  label.htmlFor = "faq-query";
  const field = make("input");
  field.id = "faq-query";
  field.type = "search";
  field.autocomplete = "off";
  field.spellcheck = false;
  field.setAttribute("aria-describedby", "faq-said");
  form.append(label, field);
  const said = make("p", "faq-said");
  said.id = "faq-said";
  said.setAttribute("aria-live", "polite");
  said.hidden = true;
  words.append(form, said);

  // What each question says, once: its question and its answer, in lower case, without the
  // "Link to this answer" every answer ends on.
  const says = (q: HTMLDetailsElement) => {
    const copy = q.cloneNode(true) as HTMLElement;
    copy.querySelector(".qa-link")?.remove();
    return (copy.textContent ?? "").toLowerCase().replace(/\s+/g, " ");
  };
  const text = new Map(questions.map((q) => [q, says(q)]));
  /** The questions the field opened, to fold again when it empties. */
  let opened = new Set<HTMLDetailsElement>();

  /** Every block that holds questions, innermost first: a heading's list, a group, a part. */
  const holders = [
    ...document.querySelectorAll<HTMLElement>(".faq-group-body .qa-list"),
    ...document.querySelectorAll<HTMLElement>(".faq-group"),
    ...document.querySelectorAll<HTMLElement>("main > .part"),
  ];

  /** Keeps the questions holding every word of `query`, or all of them for none. */
  function keep(query: string) {
    const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
    let found = 0;
    for (const q of questions) {
      const match = terms.every((term) => text.get(q)!.includes(term));
      q.hidden = !match;
      if (match) found++;
      if (terms.length && match && !q.open) {
        q.open = true;
        opened.add(q);
      }
    }
    if (!terms.length) {
      for (const q of opened) q.open = false;
      opened = new Set();
    }
    for (const holder of holders) holder.hidden = !holder.querySelector("details.qa:not([hidden])");
    // A heading goes with its list.
    for (const list of document.querySelectorAll<HTMLElement>(".faq-group-body .qa-list")) {
      const heading = list.previousElementSibling;
      if (heading?.tagName === "H3") (heading as HTMLElement).hidden = list.hidden;
    }
    said.hidden = !terms.length;
    said.textContent = !terms.length
      ? ""
      : found === 0
        ? `No question has “${query.trim()}”. Try another word, or pick a topic.`
        : `${found === 1 ? "1 question has" : `${found} questions have`} “${query.trim()}”.`;
  }

  // What's left moves every heading under it: the minimap and the topics list follow.
  const find = (query: string) => {
    keep(query);
    dispatchEvent(new Event("scroll"));
  };
  field.addEventListener("input", () => find(field.value));
  field.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && field.value) {
      event.preventDefault();
      field.value = "";
      find("");
    }
  });

  // A printed page shows every answer (site.ts), so the field empties for the print.
  addEventListener("beforeprint", () => {
    if (!field.value) return;
    field.value = "";
    // What it opened stays open for the print, with the rest.
    opened = new Set();
    find("");
  });

  // An address naming something the field hides shows everything, then lands on it.
  addEventListener("hashchange", () => {
    const asked = document.getElementById(decodeURIComponent(location.hash.slice(1)));
    if (!asked || !field.value || asked.closest("[hidden]") === null) return;
    field.value = "";
    find("");
    if (asked instanceof HTMLDetailsElement) asked.open = true;
    asked.scrollIntoView();
  });
}
