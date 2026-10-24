// The FAQ's questions, each written once, on the FAQ page (site/faq/index.html), and shown
// again on the home page where a feature names it, and on the Map Leaderboards pages. The
// build fills every page from the FAQ page's source (vite.config.ts), and fails on a
// question or group no page has.
//
// A question as written: `<details class="qa" id="…"><summary>The question?</summary>
// …its answer…</details>`, inside a `<section class="faq-group" id="…" data-about="…">`.
// A list of questions on the home page: `<div class="qa-list" data-qa="id id id"
// data-group="…"></div>`.

/** A question: its address on the FAQ page, the question, and its answer as written. */
type Question = { id: string; summary: string; answer: string };

/** A group of questions: its address, what it's about ("the panel and the map"), and its questions. */
type Group = { id: string; about: string; ids: string[] };

export type Faq = { questions: Map<string, Question>; groups: Map<string, Group> };

const QUESTION = /<details class="qa" id="([a-z0-9-]+)">\s*<summary>([\s\S]*?)<\/summary>([\s\S]*?)<\/details>/g;
const GROUP = /<section class="faq-group[^"]*" id="([a-z0-9-]+)" data-about="([^"]+)"[^>]*>([\s\S]*?)<\/section>/g;
const LIST = /<div class="qa-list" data-qa="([a-z0-9 -]+)" data-group="([a-z0-9-]+)"><\/div>/g;

/** The FAQ page's questions and groups, read from its source. */
export function readFaq(html: string): Faq {
  const questions = new Map<string, Question>();
  for (const [, id, summary, answer] of html.matchAll(QUESTION)) {
    if (questions.has(id)) throw new Error(`site: two FAQ questions have the id "${id}"`);
    questions.set(id, { id, summary: summary.trim(), answer: answer.trim() });
  }
  const groups = new Map<string, Group>();
  for (const [, id, about, body] of html.matchAll(GROUP)) {
    groups.set(id, { id, about, ids: [...body.matchAll(QUESTION)].map((m) => m[1]) });
  }
  return { questions, groups };
}

/** The panel's fold button, as a section heading shows it: a dash while open, a plus while closed. */
const FOLD = `<span class="fold" aria-hidden="true"><svg viewBox="0 0 16 16"><path d="M3.5 8.5h9" /><path class="post" d="M8 4v9" /></svg></span>`;

/**
 * A question as a page shows it: closed, its answer under it ending on a link to it. `page`
 * is the FAQ page's address from the page it's on: "" on the FAQ page itself, where it keeps
 * its id, and "/faq/" on any other. `root` is the site's folder from the page it's on ("./"
 * on the home page, "../" a folder down).
 */
function render(question: Question, page: string, root = "../"): string {
  // On another page, an answer's own links lead back to the FAQ, and its pictures, written
  // from the FAQ's folder ("../screenshots/"), are found from the page's.
  const answer = page
    ? question.answer.replace(/href="#/g, `href="${page}#`).replace(/src="\.\.\//g, `src="${root}`)
    : question.answer;
  const id = page ? "" : ` id="${question.id}"`;
  return (
    `<details class="qa"${id}><summary><span>${question.summary}</span>${FOLD}</summary>` +
    `<div class="qa-answer">${answer}<p class="qa-link"><a href="${page}#${question.id}">Link to this answer</a></p></div>` +
    `</details>`
  );
}

/** The FAQ page with each question drawn as it shows. */
export function fillFaqPage(html: string, faq: Faq): string {
  return html.replace(QUESTION, (_, id: string) => render(faq.questions.get(id)!, ""));
}

/**
 * A page (the home page, or the Map Leaderboards') with each list of questions filled from
 * the FAQ, the last list naming a group followed by a link to the rest of it, so features
 * side by side from one group (the Item Card, Item Lookup and the Deal Finder) share one.
 * `root` is the site's folder from the page: "./" for the home page.
 */
export function fillQuestionLists(html: string, faq: Faq, root = "./"): string {
  const last = new Map<string, number>();
  for (const match of html.matchAll(LIST)) last.set(match[2], match.index);
  return html.replace(LIST, (_, ids: string, groupId: string, at: number) => {
    const group = faq.groups.get(groupId);
    if (!group) throw new Error(`site: a page names a FAQ group there isn't: "${groupId}"`);
    const shown = ids.trim().split(/\s+/).map((id) => {
      const question = faq.questions.get(id);
      if (!question) throw new Error(`site: a page names a FAQ question there isn't: "${id}"`);
      return render(question, "/faq/", root);
    });
    const more =
      at === last.get(groupId)
        ? `<p class="qa-more"><a href="/faq/#${group.id}">All ${group.ids.length} questions about ${group.about}</a></p>`
        : "";
    return `<div class="qa-block"><div class="qa-list">${shown.join("")}</div>${more}</div>`;
  });
}
