// The page's minimap: the page drawn as one of the game's maps, in the game's Mini Map
// window. Each of its parts (`data-street`) is a platform with its stops (`.stop`) as
// portals on it, roped down to the next, and the window's foot lies on the ground below. A
// yellow dot, the reader, walks the platforms as the page scrolls, as the game's dot moves
// only as the character walks: never on its own, and with reduced motion it steps from
// portal to portal.

import { make, reducedMotion, svgEl } from "../src/dom.ts";
import { STAR } from "./star.ts";

/** How a stop is marked: as its topic's heading is, or else a portal ring. */
type Mark = "portal" | "route" | "mob" | "star";

/** A stop on the walk: the hero, a feature, a FAQ group or the foot, each a portal on the map. */
type Stop = { el: HTMLElement; part: number; name: string; street: string; href: string; mark: Mark; x: number; y: number };

/** A point on the walk, the scroll it's reached at, and where the reader is from there. */
type Step = { x: number; y: number; at: number; part: number; stop: number };

/** Where the reader is: the dot's place, the part being read and the stop last reached. */
export type Place = { x: number; y: number; part: number; stop: number };

// The map's plan, in its own pixels: platforms a row apart, alternating which end their
// rope hangs from, so the walk zigzags down as the game's maps do; the ground below.
const W = 200;
const FIRST = 26;
const ROW = 28;
const GROUND = 14;
/** Marks stand on their platform: their centre this far above its top. */
const STAND = 7;
/** The scroll a drop down a rope takes, at most. */
const DROP = 160;

const MINUS = "M3.5 8.5h9";
const PLUS = "M3.5 8.5h9M8 4v9";

function icon(path: string) {
  const svg = svgEl("svg", { viewBox: "0 0 16 16" });
  svg.append(svgEl("path", { d: path }));
  return svg;
}

/**
 * Draws the minimap for the page's parts, and returns where the reader is at a scroll, once
 * `measure` has read the page. `line` is how far below the viewport's top a heading counts
 * as reached.
 */
export function minimap(line: () => number) {
  const parts = [...document.querySelectorAll<HTMLElement>("[data-street]")];
  const foot = document.querySelector<HTMLElement>(".window-foot")!;
  const n = parts.length;
  const ledge = (i: number) => FIRST + i * ROW;
  const height = ledge(n) + GROUND;

  // Each platform's span, the end its rope hangs from, and where the walk comes onto it.
  const spans = parts.map((_, i) => (i % 2 === 0 ? [12, W - 34] : [34, W - 12]));
  const ropes = spans.map(([a, b], i) => (i % 2 === 0 ? b - 6 : a + 6));
  const entries = spans.map(([a], i) => (i === 0 ? a + 6 : ropes[i - 1]));

  // The stops, spread along their part's platform.
  const stops: Stop[] = [];
  parts.forEach((part, i) => {
    const els = [...part.querySelectorAll<HTMLElement>(".stop")];
    els.forEach((el, k) => {
      const heading = el.querySelector("h1, h2");
      const mark = heading?.querySelector(".mark.route")
        ? "route"
        : heading?.querySelector(".mark.mob")
          ? "mob"
          : heading?.querySelector(".star")
            ? "star"
            : "portal";
      stops.push({
        el,
        part: i,
        name: el.dataset.mini ?? heading?.textContent?.trim() ?? "",
        street: part.dataset.street!,
        href: `#${el.id || part.id}`,
        mark,
        x: entries[i] + ((ropes[i] - entries[i]) * (k + 0.5)) / els.length,
        y: ledge(i) - STAND,
      });
    });
  });
  stops.push({
    el: foot,
    part: n,
    name: foot.dataset.mini ?? "Credits",
    street: "Minerva",
    href: `#${foot.id}`,
    mark: "portal",
    x: W / 2,
    y: ledge(n) - STAND,
  });

  // The window: the game's Mini Map bar, the sky-blue title saying where you are, the map.
  const nav = make("nav", "minimap");
  nav.setAttribute("aria-label", "Map of the page");
  const bar = make("div", "mm-bar");
  const fold = make("button", "icon-button mm-fold");
  fold.type = "button";
  fold.setAttribute("aria-controls", "mm-body");
  bar.append(make("p", "mm-name", "Mini Map"), fold);
  const title = make("div", "mm-title");
  const emblem = make("img", "mm-emblem");
  // The owl's 16px drawing, the favicon's, at its own size.
  emblem.src = document.querySelector<HTMLLinkElement>('link[rel="icon"][sizes="16x16"]')!.href;
  emblem.alt = "";
  const street = make("p", "mm-street");
  const place = make("p", "mm-place");
  title.append(emblem, street, place);
  const body = make("div", "mm-body");
  body.id = "mm-body";
  const map = make("div", "mm-map");
  map.style.height = `${height}px`;
  body.append(map);
  nav.append(bar, title, body);

  const setFold = (folded: boolean) => {
    nav.classList.toggle("folded", folded);
    fold.setAttribute("aria-expanded", String(!folded));
    fold.setAttribute("aria-label", folded ? "Show the map" : "Hide the map");
    fold.replaceChildren(icon(folded ? PLUS : MINUS));
  };
  setFold(false);
  fold.addEventListener("click", () => setFold(!nav.classList.contains("folded")));

  // The platforms, their ropes and the ground.
  spans.forEach(([a, b], i) => {
    const platform = make("span", "mm-ledge");
    Object.assign(platform.style, { left: `${a}px`, width: `${b - a}px`, top: `${ledge(i)}px` });
    const rope = make("span", "mm-rope");
    Object.assign(rope.style, { left: `${ropes[i]}px`, top: `${ledge(i) + 4}px`, height: `${ROW - 4}px` });
    map.append(platform, rope);
  });
  const ground = make("span", "mm-ground");
  ground.style.top = `${ledge(n)}px`;
  map.append(ground);

  // The portals, each a link to its stop, with the panel's card under the pointer.
  const card = make("div", "mm-card");
  card.setAttribute("role", "tooltip");
  card.id = "mm-card";
  card.hidden = true;
  const marks = stops.map((stop) => {
    const a = make("a", `mm-mark ${stop.mark}`);
    a.href = stop.href;
    a.setAttribute("aria-label", `${stop.street}: ${stop.name}`);
    Object.assign(a.style, { left: `${stop.x}px`, top: `${stop.y}px` });
    if (stop.mark === "star") a.append(icon(STAR));
    const show = () => {
      card.replaceChildren(make("b", "", stop.name), make("span", "", stop.street));
      card.hidden = false;
      // Centred on the portal 6px above it, or below it near the map's top, and kept 4px
      // inside the map's sides, as the panel places its card.
      const left = Math.min(Math.max(stop.x - card.offsetWidth / 2, 4), W - card.offsetWidth - 4);
      const above = stop.y - 8 - card.offsetHeight;
      card.style.left = `${left}px`;
      card.style.top = `${above < 2 ? stop.y + 10 : above}px`;
    };
    const hide = () => (card.hidden = true);
    a.addEventListener("pointerenter", show);
    a.addEventListener("focus", show);
    a.addEventListener("pointerleave", hide);
    a.addEventListener("blur", hide);
    return a;
  });
  const you = make("span", "mm-you");
  you.setAttribute("aria-hidden", "true");
  map.append(...marks, you, card);
  document.querySelector(".window")!.after(nav);

  // The walk: where the reader is at each stop and at each platform's ends, and the scroll
  // each is reached at. A drop down a rope takes the last of the scroll before its part.
  let steps: Step[] = [];
  // Whether the minimap shows (on wide screens), read with the walk rather than on every
  // scroll, where the read would follow the reel's writes and force a layout each frame.
  let showing = false;
  const reach = (el: HTMLElement) => el.getBoundingClientRect().top + scrollY - line();
  function measure() {
    showing = nav.offsetWidth > 0;
    const walk: Step[] = [];
    const ofPart = (i: number) => stops.flatMap((stop, k) => (stop.part === i ? [k] : []));
    for (let i = 0; i < n; i++) {
      const own = ofPart(i);
      const y = ledge(i) - STAND;
      if (i > 0) walk.push({ x: entries[i], y, at: reach(parts[i]), part: i, stop: own[0] });
      for (const k of own) walk.push({ x: stops[k].x, y, at: reach(stops[k].el), part: i, stop: k });
      const next = i + 1 < n ? reach(parts[i + 1]) : reach(foot);
      const last = walk[walk.length - 1].at;
      walk.push({ x: ropes[i], y, at: next - Math.min(DROP, (next - last) / 2), part: i, stop: own[own.length - 1] });
    }
    const credits = stops.length - 1;
    const y = ledge(n) - STAND;
    walk.push({ x: ropes[n - 1], y, at: reach(foot), part: n, stop: credits });
    walk.push({ x: W / 2, y, at: reach(foot) + foot.offsetHeight, part: n, stop: credits });
    // In order, from the top of the page.
    walk.forEach((step, k) => (step.at = Math.max(step.at, k > 0 ? walk[k - 1].at : 0, 0)));
    // The page ends before its last headings reach the line: the walk's tail is drawn in so
    // the end of the page is the end of the walk, the last stretch at least 480px long.
    const end = document.documentElement.scrollHeight - innerHeight;
    const last = walk[walk.length - 1].at;
    if (last > end && end > 0) {
      let pivot = 0;
      walk.forEach((step, k) => {
        if (step.at < end - 480) pivot = k;
      });
      const from = walk[pivot].at;
      for (const step of walk.slice(pivot + 1)) step.at = from + ((step.at - from) * (end - from)) / (last - from);
    }
    steps = walk;
  }

  /** Where the reader is at a scroll. */
  function at(s: number): Place {
    let k = 0;
    while (k + 1 < steps.length && steps[k + 1].at <= s) k++;
    const here = steps[k];
    const next = steps[k + 1];
    if (!next || reducedMotion().matches) {
      const stop = stops[here.stop];
      return { x: next ? stop.x : here.x, y: next ? stop.y : here.y, part: here.part, stop: here.stop };
    }
    const t = next.at > here.at ? Math.min(Math.max((s - here.at) / (next.at - here.at), 0), 1) : 1;
    return { x: here.x + (next.x - here.x) * t, y: here.y + (next.y - here.y) * t, part: here.part, stop: here.stop };
  }

  let shown = -1;
  /** Puts the dot where the reader is, and the title and the lit portal on their stop. */
  function draw(where: Place) {
    if (!showing) return;
    you.style.transform = `translate(${where.x}px, ${where.y}px)`;
    if (where.stop === shown) return;
    if (shown >= 0) marks[shown].removeAttribute("aria-current");
    shown = where.stop;
    marks[shown].setAttribute("aria-current", "location");
    street.textContent = stops[shown].street;
    place.textContent = stops[shown].name;
  }

  return { measure, at, draw };
}
