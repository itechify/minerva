// The FAQ's topics beside the group you're in: the owl's nine choices (the `.npc-choices`
// list at the top of the page) as one list, held under the current group's heading and
// what it covers, that group's own marked as the one you're in. The heading is held in
// sight as its questions go by, so the way to another topic is too, without scrolling back
// to the owl. The list is one, moved to each group as the scroll reaches it, so the page has
// one set of topic links and one landmark rather than a copy in every group. site.css shows
// it between 961px and 1440px, where the minimap (minimap.ts) doesn't; under that, each
// group ends on a way back to the owl's list. Without the script, the owl's list is the
// index.

import { make } from "../src/dom.ts";

const choices = [...document.querySelectorAll<HTMLAnchorElement>(".npc-choices a")];
const groups = [...document.querySelectorAll<HTMLElement>(".faq-group")];

const nav = make("nav", "faq-topics");
nav.setAttribute("aria-label", "Topics");
const list = make("ul");
const links = choices.map((choice) => {
  const link = make("a", "", choice.textContent ?? "");
  link.href = choice.hash;
  const item = make("li");
  item.append(link);
  list.append(item);
  return link;
});
nav.append(list);

let current: HTMLElement | null = null;

/** Holds the list under `group`'s heading, its own topic marked. */
function show(group: HTMLElement) {
  if (group === current) return;
  current = group;
  group.querySelector(".faq-group-head")!.append(nav);
  for (const link of links) {
    if (link.hash === `#${group.id}`) link.setAttribute("aria-current", "true");
    else link.removeAttribute("aria-current");
  }
}

/** The group you're in: the last whose top has come up past the middle of the screen, so
 *  the list moves to a group's heading as it comes into view, not after the last has gone. */
function here() {
  let found = groups[0];
  for (const group of groups) {
    if (group.getBoundingClientRect().top <= innerHeight / 2) found = group;
    else break;
  }
  if (found) show(found);
}

let queued = false;
addEventListener(
  "scroll",
  () => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      here();
    });
  },
  { passive: true },
);
addEventListener("resize", here);
addEventListener("hashchange", here);
here();
