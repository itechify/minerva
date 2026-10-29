// The hero's opening: how Minerva reads the map you're on, filmed in the game window as
// the page scrolls. It opens on the hero shot, the panel under the minimap, so the first
// screen shows what Minerva does; the camera draws back to the whole game window, then moves
// in to the minimap's title;
// Minerva's dashed box finds its two lines; a read passes over them and turns them
// into Minerva's own lettering, which drops into the panel's title under the minimap;
// and the camera draws back to the hero shot as it is. While it plays, the hero stays put
// and the scroll runs the film, as the minimap's dot moves only with the scroll: never on a
// timer. The frame is as wide as the still shot, so the panel reads as well in the film as
// out of it: where the screen is shorter than the shot, its foot waits below the fold until
// the hero moves on, and each step's words are a band along the foot of what's in view, as
// the game's tooltips are drawn. With reduced motion, without the script, in print or on a
// screen too narrow to hold it, the hero is the still shot.

import { make, reducedMotion } from "../src/dom.ts";

/** A rectangle in the game window's own pixels: game-window.png, 1922×1232. */
type Rect = { x: number; y: number; w: number; h: number };

// Where things are in game-window.png (site/screenshots/README.md: retaking it means
// checking these). The hero shot, hero.png, is its top-left corner at full size.
const WHOLE: Rect = { x: 0, y: 0, w: 1922, h: 1232 };
const HERO: Rect = { x: 0, y: 0, w: 600, h: 960 };
/** Where Minerva finds the minimap title: its two lines, with a little of its blue panel. */
const FOUND: Rect = { x: 70, y: 68, w: 248, h: 58 };
/** The panel's title panel, under the minimap, where the read lands: beside its Map Mark. */
const TITLE: Rect = { x: 49, y: 409, w: 290, h: 56 };
/** Close on the minimap's title, with the panel's under it, in the hero's shape: at most
 *  this close, and drawn back as far as a shorter screen needs to keep the panel's title,
 *  where the read lands, above the words' band (`measure`). */
const CLOSEST = 360;
const closeAt = (w: number): Rect => ({ x: 0, y: 28, w, h: (w * HERO.h) / HERO.w });
/** What the minimap's title says. */
const STREET = "Victoria Road";
const MAP = "Henesys Hunting Ground I";
/**
 * The read's ground: the minimap's title panel where it is read (Mini Map Sky), and the
 * panel's as the shot shows it, a step lighter, where it lands.
 */
const READ_FROM = [0xa4, 0xc8, 0xda];
const READ_TO = [0xab, 0xcf, 0xe2];

// The film's beats, as shares of its scroll: out from the hero shot to the whole window and
// in to the title, quickly, then the title found, the read and the drop into the panel,
// which take most of it, then back to the shot.
const OUT = [0, 0.08];
const PUSH = [0.09, 0.24];
const FIND = [0.26, 0.4];
const READ = [0.42, 0.58];
const DROP = [0.6, 0.78];
const BOX_OUT = [0.78, 0.86];
const LANDED = [0.8, 0.86];
const BACK = [0.82, 1];
/**
 * Each step's words and the stretch they show for, fading in after its start and out before
 * its end, so one has gone before the next comes. The shot itself, at the start and the end,
 * has none on it: its own caption is for screen readers.
 */
const CAPTIONS: [string, number, number][] = [
  ["MapleStory in a window, with its minimap open.", 0.08, 0.25],
  ["Minerva finds the minimap's title for you, wherever the minimap is.", 0.25, 0.41],
  ["It captures just the title, the way a screenshot does, and reads it.", 0.41, 0.59],
  ["The panel names the map it read.", 0.59, 0.82],
];
const FADE = 0.015;
/** The share of the pinned scroll the film takes; the rest holds its last frame. */
const PLAY = 0.88;
/** The pinned scroll, in screen heights. */
const RUN = 0.8;

const clamp = (t: number) => Math.min(Math.max(t, 0), 1);
/** How far through a beat the film is. */
const seg = (p: number, [a, b]: number[]) => clamp((p - a) / (b - a));
/** Eased in and out, so no beat starts or stops with a jolt. */
const ease = (t: number) => t * t * (3 - 2 * t);

/** The camera between two framings: its size in even steps of zoom, its place following. */
function between(a: Rect, b: Rect, t: number): Rect {
  const w = a.w * (b.w / a.w) ** t;
  const h = a.h * (b.h / a.h) ** t;
  const u = a.w === b.w ? t : (w - a.w) / (b.w - a.w);
  return { x: a.x + (b.x - a.x) * u, y: a.y + (b.y - a.y) * u, w, h };
}

function camera(p: number, close: Rect): Rect {
  if (p < OUT[1]) return between(HERO, WHOLE, ease(seg(p, OUT)));
  if (p < PUSH[0]) return WHOLE;
  if (p < PUSH[1]) return between(WHOLE, close, ease(seg(p, PUSH)));
  if (p < BACK[0]) return close;
  return between(close, HERO, ease(seg(p, BACK)));
}

/**
 * The game window at half size, the WebP copy the build makes for the shot under Placement.
 * Without it there is no film: the PNG is far too heavy to load first.
 */
function wholeWindow(): string | undefined {
  const source = document.querySelector<HTMLSourceElement>("source[type='image/webp'][srcset*='game-window']");
  const candidates = (source?.srcset ?? "")
    .split(",")
    .map((c) => c.trim().split(/\s+/))
    .filter(([url]) => url?.includes("game-window"))
    .map(([url, w]) => ({ url, w: parseInt(w) || Infinity }))
    .sort((a, b) => a.w - b.w);
  return candidates[0]?.url;
}

/**
 * Films the hero, once `measure` has fitted it to the screen. `head` is how tall the
 * window's head is while it sticks. `draw` puts the film at a scroll.
 */
export function reel(head: () => number) {
  const hero = document.querySelector<HTMLElement>(".hero");
  const text = hero?.querySelector<HTMLElement>(".hero-text");
  const figure = hero?.querySelector<HTMLElement>(".hero-shot");
  const shot = figure?.querySelector<HTMLImageElement>("img");
  const picture = shot?.closest("picture") ?? shot;
  const caption = figure?.querySelector("figcaption");
  const url = wholeWindow();
  if (!hero || !text || !figure || !shot || !picture || !caption || !url) return { measure() {}, draw() {} };

  // The film: a frame onto the game window, the shot's top-left corner in it at full size
  // over the whole window at half, the read's lettering, and over them in the frame's own
  // pixels the box around the title found and the read's line. Each step's words on a band
  // along the foot of what's in view.
  const pin = make("div", "reel-pin");
  const frame = make("div", "reel-frame");
  const world = make("div", "reel-world");
  const base = make("img", "reel-base");
  base.alt = "";
  base.decoding = "async";
  const read = make("div", "reel-read");
  read.setAttribute("aria-hidden", "true");
  read.append(make("p", "reel-street", STREET), make("p", "reel-map", MAP));
  const box = make("div", "reel-box");
  const sweep = make("div", "reel-sweep");
  for (const node of [box, sweep]) node.setAttribute("aria-hidden", "true");
  world.append(base, read);
  frame.append(world, box, sweep);
  const captions = make("div", "reel-captions");
  captions.setAttribute("aria-hidden", "true");
  const lines = CAPTIONS.map(([words]) => captions.appendChild(make("span", "", words)));
  pin.append(frame, captions);

  let broken = false;
  let on = false;
  let w = 0;
  let frameTop = 0;
  let start = 0;
  /** Where the film holds under the head, and the foot of the screen. */
  let holdAt = 0;
  let screenFoot = 0;
  let run = 0;
  let close = closeAt(CLOSEST);
  /** The scroll the film takes, from where the hero holds still. */
  let span = 1;
  let last = -1;

  const visible = (node: HTMLElement, show: boolean) => (node.style.visibility = show ? "" : "hidden");

  /** Plays the film, or shows the still shot. */
  function set(playing: boolean) {
    if (playing === on) return;
    on = playing;
    last = -1;
    hero!.classList.toggle("reeling", playing);
    if (!playing) hero!.classList.remove("reel-side");
    if (playing) {
      if (!base.src) base.src = url!;
      picture!.classList.add("reel-shot");
      world.prepend(picture!);
      figure!.prepend(pin);
    } else {
      pin.remove();
      picture!.classList.remove("reel-shot");
      figure!.prepend(picture!);
      for (const node of [figure!, text!]) node.style.removeProperty("height");
      text!.style.removeProperty("min-height");
    }
  }
  // A picture that doesn't load leaves the still shot, or its dashed box (site.ts).
  for (const img of [base, shot]) {
    img.addEventListener("error", () => {
      broken = true;
      set(false);
    });
  }

  // The screen's height the film is fitted to. A phone's browser bar showing or hiding
  // changes it a little as the page scrolls; the film keeps its length through that rather
  // than jump, and is fitted again only when the screen really changes.
  let wide = 0;
  let high = 0;

  let printing = false;
  addEventListener("beforeprint", () => {
    printing = true;
    measure();
  });
  addEventListener("afterprint", () => {
    printing = false;
    measure();
  });

  /** Fits the film to the screen: the frame as wide as the shot and its column allow. */
  function measure() {
    if (innerWidth !== wide || Math.abs(innerHeight - high) > 150) {
      wide = innerWidth;
      high = innerHeight;
    }
    const top = head() + 16;
    const bottom = high - 16;
    holdAt = top;
    screenFoot = bottom;
    // The frame, as wide as its column and the shot allow: the still shot's size.
    const fit = Math.floor(Math.min(figure!.clientWidth, HERO.w));
    const missing = shot!.complete && shot!.naturalWidth === 0;
    if (broken || missing || printing || reducedMotion().matches || fit < 280) return set(false);
    w = fit;
    set(true);
    const h = (w * HERO.h) / HERO.w;
    figure!.style.setProperty("--reel-w", `${w}px`);
    // Close in only as far as keeps the panel's title, where the read lands, clear of the
    // words' band at the foot of the screen.
    const room = bottom - top - captions.offsetHeight - 16;
    const reach = TITLE.y + TITLE.h - closeAt(CLOSEST).y;
    close = closeAt(Math.min(HERO.w, Math.max(CLOSEST, room > 0 ? (reach * w) / room : HERO.w)));
    const tall = h;
    // Side by side, the words stay with the film, and leave with it: both stick for as long
    // as the film. Where the words fit under the head, the film's foot lines up with theirs;
    // where they don't, as on a laptop's shorter screen, they go on up until their foot is
    // in view and hold there, their top under the head, so neither column is ever left
    // empty. On a narrow screen the words go on by, above the film.
    text!.style.removeProperty("min-height");
    const side = getComputedStyle(hero!).gridTemplateColumns.trim().split(/\s+/).length > 1;
    const fits = text!.offsetHeight <= bottom - top;
    hero!.classList.toggle("reel-side", side);
    hero!.style.setProperty("--reel-text-top", `${fits ? top : bottom - text!.offsetHeight}px`);
    const held = side && fits ? Math.max(tall, text!.offsetHeight) : tall;
    frameTop = held - tall;
    run = Math.round(high * RUN);
    hero!.style.setProperty("--reel-top", `${top}px`);
    pin.style.height = `${held}px`;
    figure!.style.height = `${held + run}px`;
    if (side) text!.style.minHeight = `${held}px`;
    frame.style.top = `${frameTop}px`;
    start = figure!.getBoundingClientRect().top + scrollY - top;
    span = run * PLAY;
    last = -1;
  }

  /** Puts the film at a scroll. */
  function draw(scroll: number) {
    if (!on) return;
    const p = clamp((scroll - start) / span);
    if (p === last) return;
    last = p;

    const cam = camera(p, close);
    const s = w / cam.w;
    const height = cam.h * s;
    frame.style.height = `${height}px`;
    world.style.transform = `scale(${s}) translate(${-cam.x}px, ${-cam.y}px)`;
    world.classList.toggle("close", s > 1);
    // Wider than the shot, its edges are in view over the window at half size.
    world.classList.toggle("feather", cam.w > HERO.w + 0.5);
    // The words' band at the foot of what's in view of the frame: the screen's foot while the
    // frame runs past it, the frame's own once it doesn't.
    const pinTop = holdAt + Math.max(0, start - scroll) - Math.max(0, scroll - start - run);
    const foot = Math.min(frameTop + height, screenFoot - pinTop);
    captions.style.transform = `translateY(${foot - captions.offsetHeight}px)`;
    lines.forEach((line, k) => {
      const [, a, b] = CAPTIONS[k];
      line.style.opacity = String(clamp((p - a) / FADE - 1) * clamp((b - p) / FADE - 1));
    });

    // The box around the title, as Minerva finds it: nothing for the player to mark.
    const found = seg(p, FIND) * (1 - seg(p, BOX_OUT));
    const left = (FOUND.x - cam.x) * s;
    const top = (FOUND.y - cam.y) * s;
    const tall = FOUND.h * s;
    Object.assign(box.style, { left: `${left}px`, top: `${top}px`, width: `${FOUND.w * s}px`, height: `${tall}px` });
    box.style.opacity = String(found);
    visible(box, found > 0);

    // The read: a line across the box, Minerva's lettering where it has passed.
    const done = seg(p, READ);
    visible(sweep, done > 0 && done < 1);
    sweep.style.transform = `translate(${left + FOUND.w * s * done - 1}px, ${top}px)`;
    sweep.style.height = `${tall}px`;
    read.style.clipPath = `inset(0 ${(1 - done) * 100}% 0 0)`;
    visible(read, done > 0 && p < LANDED[1]);

    // Then it drops into the panel's title, which says the same, and gives way to it.
    const drop = ease(seg(p, DROP));
    Object.assign(read.style, {
      left: `${FOUND.x + (TITLE.x - FOUND.x) * drop}px`,
      top: `${FOUND.y + (TITLE.y - FOUND.y) * drop}px`,
      width: `${FOUND.w + (TITLE.w - FOUND.w) * drop}px`,
      height: `${FOUND.h + (TITLE.h - FOUND.h) * drop}px`,
      opacity: String(1 - seg(p, LANDED)),
      backgroundColor: `rgb(${READ_FROM.map((c, k) => Math.round(c + (READ_TO[k] - c) * drop)).join(", ")})`,
    });
  }

  reducedMotion().addEventListener("change", measure);
  return { measure, draw };
}
