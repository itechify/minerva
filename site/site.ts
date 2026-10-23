/// <reference types="vite/client" />
// The site's behaviours, on every page: on a wide screen the page's minimap shows where you
// are in it (minimap.ts), following the scroll; on the home page the hero films how Minerva
// reads the map you're on as the page scrolls past it (reel.ts); and a question the address
// names opens; and who's signed in shows in the window's bar. Without the script the pages still read whole, the hero is its still shot,
// and every question opens with a click.

import { minimap } from "./minimap.ts";
import { reel } from "./reel.ts";
import { signedIn } from "./signed-in.ts";

// Addresses from when the home page was the whole manual, which release notes and the old
// site's address still link to: a part that has moved goes where it's answered now.
const MOVED: Record<string, string> = {
  install: "/faq/#install",
  setup: "/faq/#setup",
  trouble: "/faq/#trouble",
  "bug-reports": "/faq/#bug-reports",
  data: "/faq/#data",
  "where-you-are": "/#map",
  panel: "/faq/#panel",
  "shared-titles": "/faq/#shared-titles",
  hunting: "/#navigation",
  search: "/faq/#search",
  grinding: "/#grind-tracker",
  "personal-bests": "/faq/#personal-bests",
  items: "/#item-card",
  keys: "/#hotkeys",
  settings: "/faq/#settings",
};
const moved = MOVED[decodeURIComponent(location.hash.slice(1))];
if (moved && location.pathname === "/") location.replace(moved);

const head = document.querySelector<HTMLElement>(".window-head")!;
const strip = document.querySelector<HTMLElement>(".tabs")!;

/** What sticks at the top: the window's head, or on a phone the tabs alone (site.css). */
const stuck = () => (getComputedStyle(head).display === "contents" ? strip : head);

// A jump to a heading lands below what sticks, however tall it wraps.
const heads = new ResizeObserver(() => {
  document.documentElement.style.setProperty("--head", `${stuck().offsetHeight}px`);
});
heads.observe(head);
heads.observe(strip);

// The hero's film, run by the scroll while the hero stays put under what sticks.
const film = reel(() => stuck().offsetHeight);
// A heading is reached 48px below what sticks.
const map = minimap(() => stuck().offsetHeight + 48);

/** Run the hero's film, and put the reader on the minimap. */
function follow() {
  film.draw(scrollY);
  map.draw(map.at(scrollY));
}

/**
 * Fit the film to the screen, and read where every heading lies again, after the page's
 * layout changes: the film's stretch of scroll, or a question opened, moves every heading
 * after it.
 */
function remeasure() {
  film.measure();
  map.measure();
  follow();
}

addEventListener("scroll", follow, { passive: true });
addEventListener("resize", remeasure);
new ResizeObserver(remeasure).observe(document.querySelector(".window")!);
document.fonts.ready.then(remeasure);
remeasure();

/** The question the address names, opened and brought into view. */
function openAsked() {
  const asked = document.getElementById(decodeURIComponent(location.hash.slice(1)));
  if (!(asked instanceof HTMLDetailsElement)) return;
  asked.open = true;
  asked.scrollIntoView();
}
addEventListener("hashchange", openAsked);
openAsked();

// A printed page shows every answer: the questions folded on screen open for the print, and
// fold again after it.
let printing: HTMLDetailsElement[] = [];
addEventListener("beforeprint", () => {
  printing = [...document.querySelectorAll<HTMLDetailsElement>("details.qa:not([open])")];
  for (const question of printing) question.open = true;
});
addEventListener("afterprint", () => {
  for (const question of printing) question.open = false;
  printing = [];
});

// A screenshot that doesn't load leaves a dashed box saying what belongs there. While
// developing, that is a shot not taken yet; the build refuses to publish without it.
for (const img of document.querySelectorAll<HTMLImageElement>(".shot img")) {
  const figure = img.closest("figure")!;
  const missing = () => {
    figure.dataset.missing = import.meta.env.DEV
      ? `Not taken yet: site/${img.getAttribute("src")?.replace(/^(\.\.?\/)+/, "")}. See site/screenshots/README.md.\n\n${img.alt}`
      : img.alt;
    figure.classList.add("missing");
  };
  if (img.complete && img.naturalWidth === 0) missing();
  else img.addEventListener("error", missing);
}

signedIn();
