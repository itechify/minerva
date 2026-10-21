// The hero's opening: how Minerva reads the map you're on, filmed in the game window as
// the page scrolls. It opens on the hero shot, the panel under the minimap, so the first
// screen shows what Minerva does; the camera draws back to the whole game window, then moves
// in to the minimap's title;
// setup's dashed box is marked around its two lines; a read passes over them and turns them
// into Minerva's own lettering, which drops into the panel's title under the minimap;
// and the camera draws back to the hero shot as it is. While it plays, the hero stays put
// and the scroll runs the film, as the minimap's dot moves only with the scroll: never on a
// timer. With reduced motion, without the script, in print or on a screen too short to
// hold it, the hero is the still shot.

/** A rectangle in the game window's own pixels: game-window.png, 1922×1232. */
type Rect = { x: number; y: number; w: number; h: number };

// Where things are in game-window.png (site/screenshots/README.md: retaking it means
// checking these). The hero shot, hero.png, is its top-left corner at full size.
const WHOLE: Rect = { x: 0, y: 0, w: 1922, h: 1232 };
const HERO: Rect = { x: 0, y: 0, w: 600, h: 960 };
/** The box marked around the minimap title's two lines, with a little of its blue panel. */
const MARK: Rect = { x: 70, y: 68, w: 248, h: 58 };
/** The panel's title panel, under the minimap, where the read lands: beside its Map Mark. */
const TITLE: Rect = { x: 49, y: 409, w: 290, h: 56 };
/** Close on the minimap's title, with the panel's under it, in the hero's shape. */
const CLOSE: Rect = { x: 0, y: 28, w: 360, h: (360 * HERO.h) / HERO.w };
/** What the minimap's title says. */
const STREET = "Victoria Road";
const MAP = "Henesys Hunting Ground I";
/**
 * The read's ground: the minimap's title panel where it is read (Mini Map Sky), and the
 * panel's as the shot shows it, a step lighter, where it lands.
 */
const READ_FROM = [0xa4, 0xc8, 0xda];
const READ_TO = [0xab, 0xcf, 0xe2];

// The film's beats, as shares of its scroll: out from the hero shot to the whole window, in
// to the title, the box marked, the read, the drop into the panel, then back to the shot.
const OUT = [0, 0.12];
const PUSH = [0.14, 0.34];
const VEIL_IN = [0.34, 0.397];
const DRAG = [0.359, 0.472];
const READ = [0.491, 0.623];
const DROP = [0.642, 0.811];
const VEIL_OUT = [0.811, 0.887];
const LANDED = [0.83, 0.887];
const BACK = [0.849, 1];
/**
 * Each caption and the stretch it shows for, fading in after its start and out before its
 * end, so one has gone before the next comes. The first and last are the shot's own caption.
 */
const CAPTIONS: [string, number, number][] = [
  ["", -1, 0.1],
  ["MapleStory, windowed, with its minimap open.", 0.1, 0.331],
  ["At setup, you mark the minimap's title: both its lines.", 0.331, 0.481],
  ["Minerva captures just that box, the way a screenshot does, and reads it.", 0.481, 0.632],
  ["The panel names the map it read.", 0.632, 0.849],
  ["", 0.849, 2],
];
const FADE = 0.015;
/** The share of the pinned scroll the film takes; the rest holds its last frame. */
const PLAY = 0.85;
/** The navy veil the title-marking window lays over the game (region.html). */
const VEIL = "18, 28, 56";

const reduced = matchMedia("(prefers-reduced-motion: reduce)");

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

function camera(p: number): Rect {
  if (p < OUT[1]) return between(HERO, WHOLE, ease(seg(p, OUT)));
  if (p < PUSH[0]) return WHOLE;
  if (p < PUSH[1]) return between(WHOLE, CLOSE, ease(seg(p, PUSH)));
  if (p < BACK[0]) return CLOSE;
  return between(CLOSE, HERO, ease(seg(p, BACK)));
}

function make<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text?: string) {
  const node = document.createElement(tag);
  node.className = className;
  if (text) node.textContent = text;
  return node;
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
  // pixels the veil, the marked box and the read's line. Captions under the frame.
  const pin = make("div", "reel-pin");
  const frame = make("div", "reel-frame");
  const world = make("div", "reel-world");
  const base = make("img", "reel-base");
  base.alt = "";
  base.decoding = "async";
  const read = make("div", "reel-read");
  read.setAttribute("aria-hidden", "true");
  read.append(make("p", "reel-street", STREET), make("p", "reel-map", MAP));
  const veil = make("div", "reel-veil");
  const box = make("div", "reel-box");
  const sweep = make("div", "reel-sweep");
  for (const node of [veil, box, sweep]) node.setAttribute("aria-hidden", "true");
  world.append(base, read);
  frame.append(world, veil, box, sweep);
  const captions = make("div", "reel-captions");
  captions.setAttribute("aria-hidden", "true");
  const lines = CAPTIONS.map(([words]) => captions.appendChild(make("span", "", words || caption.textContent!.trim())));
  pin.append(frame, captions);

  let broken = false;
  let on = false;
  let w = 0;
  let frameTop = 0;
  let start = 0;
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

  /** Fits the film to the screen: the frame as large as the shot or the screen allows. */
  function measure() {
    if (innerWidth !== wide || Math.abs(innerHeight - high) > 150) {
      wide = innerWidth;
      high = innerHeight;
    }
    const top = head() + 16;
    const bottom = high - 16;
    // The frame, as wide as its column and the shot allow, and short enough that it and
    // two lines of caption fit under the window's head.
    const fit = Math.floor(Math.min(figure!.clientWidth, HERO.w, ((bottom - top - 8 - 36) * HERO.w) / HERO.h));
    const missing = shot!.complete && shot!.naturalWidth === 0;
    if (broken || missing || printing || reduced.matches || fit < 280) return set(false);
    w = fit;
    set(true);
    const h = (w * HERO.h) / HERO.w;
    figure!.style.setProperty("--reel-w", `${w}px`);
    const tall = h + 8 + captions.offsetHeight;
    // Side by side, the words stay with the film, and leave with it: both stick for as long
    // as the film, when the words fit under the head. Where they don't, they go on by and
    // the film holds alone, as it does on a narrow screen.
    text!.style.removeProperty("min-height");
    const side =
      getComputedStyle(hero!).gridTemplateColumns.trim().split(/\s+/).length > 1 && text!.offsetHeight <= bottom - top;
    hero!.classList.toggle("reel-side", side);
    const held = side ? Math.max(tall, text!.offsetHeight) : tall;
    frameTop = held - tall;
    const run = Math.round(high * 1.25);
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

    const cam = camera(p);
    const s = w / cam.w;
    const height = cam.h * s;
    frame.style.height = `${height}px`;
    world.style.transform = `scale(${s}) translate(${-cam.x}px, ${-cam.y}px)`;
    world.classList.toggle("close", s > 1);
    // Wider than the shot, its edges are in view over the window at half size.
    world.classList.toggle("feather", cam.w > HERO.w + 0.5);
    captions.style.transform = `translateY(${frameTop + height + 8}px)`;
    lines.forEach((line, k) => {
      const [, a, b] = CAPTIONS[k];
      line.style.opacity = String(clamp((p - a) / FADE - 1) * clamp((b - p) / FADE - 1));
    });

    // The box, dragged from its top-left corner as the player marks it, the veil around it.
    const drag = ease(seg(p, DRAG));
    const shade = seg(p, VEIL_IN) * (1 - seg(p, VEIL_OUT));
    const left = (MARK.x - cam.x) * s;
    const top = (MARK.y - cam.y) * s;
    const width = MARK.w * s * drag;
    const tall = MARK.h * s * drag;
    for (const node of [veil, box]) {
      Object.assign(node.style, { left: `${left}px`, top: `${top}px`, width: `${width}px`, height: `${tall}px` });
    }
    veil.style.boxShadow = `0 0 0 ${Math.ceil(height + w)}px rgba(${VEIL}, ${0.3 * shade})`;
    visible(veil, shade > 0);
    box.style.opacity = String(1 - seg(p, VEIL_OUT));
    visible(box, drag > 0 && p < VEIL_OUT[1]);

    // The read: a line across the box, Minerva's lettering where it has passed.
    const done = seg(p, READ);
    visible(sweep, done > 0 && done < 1);
    sweep.style.transform = `translate(${left + MARK.w * s * done - 1}px, ${top}px)`;
    sweep.style.height = `${tall}px`;
    read.style.clipPath = `inset(0 ${(1 - done) * 100}% 0 0)`;
    visible(read, done > 0 && p < LANDED[1]);

    // Then it drops into the panel's title, which says the same, and gives way to it.
    const drop = ease(seg(p, DROP));
    Object.assign(read.style, {
      left: `${MARK.x + (TITLE.x - MARK.x) * drop}px`,
      top: `${MARK.y + (TITLE.y - MARK.y) * drop}px`,
      width: `${MARK.w + (TITLE.w - MARK.w) * drop}px`,
      height: `${MARK.h + (TITLE.h - MARK.h) * drop}px`,
      opacity: String(1 - seg(p, LANDED)),
      backgroundColor: `rgb(${READ_FROM.map((c, k) => Math.round(c + (READ_TO[k] - c) * drop)).join(", ")})`,
    });
  }

  reduced.addEventListener("change", measure);
  return { measure, draw };
}
