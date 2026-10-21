/**
 * The Map section: the Map Preview, OSMS's picture of the whole map with every portal
 * ringed where it is, then a row per Hidden Portal. Rings are drawn in screen pixels over
 * the fitted picture, whatever its scale: blue to another map (Doors included), green
 * within the map with a dashed arrow to where it lands, grey to a map not in the data;
 * dashed when hidden or a Quest Portal. A portal the Hunting Target's route takes next
 * wears a wide magenta ring around its own, always. Hovering a ring says where it leads
 * and lights its row; hovering a row lights its ring. The wheel zooms the picture inside
 * its box towards the pointer, from the whole map to the picture's own size, the rings
 * keeping theirs; zoomed in, a drag moves it, and a double-click shows the whole map
 * again. A new map shows whole. A click does nothing.
 *
 * The card of a ring or a row opens on a close-up of the spot: OSMS's picture around the
 * portal at half the game's own size, diving to it from the map as the preview shows it.
 *
 * Every spawn point is dotted under the rings, as the minimap dots mobs. Resting on a
 * mob's row lights its dots and dims the rest; resting on a dot lights its mob's dots and
 * row, and shows its card, close up.
 */

import { byId, cardTop, span } from "./dom";
import { mapMark, type MapMarks } from "./map-marks";

/** Mirror of `minerva_core::HiddenPortal`. */
export type HiddenPortal = {
  name: string;
  toMapId: string;
  toMapName: string;
  toStreetName: string;
  /** The destination's Map Mark, by OSMS's name for it. */
  toMark: string | null;
  toHiddenStreet: boolean;
  quest: boolean;
};

/** Mirror of `minerva_core::Leads`. */
type Leads = "anotherMap" | "withinTheMap" | "mapNotInTheData";

/** Mirror of `minerva_core::PortalKind`. */
type PortalKind = "visible" | "hidden" | "door" | "quest";

/** Mirror of `minerva_core::PortalMark`: where it is in the picture's pixels. */
type PortalMark = {
  x: number;
  y: number;
  leads: Leads;
  kind: PortalKind;
  toMapId: string;
  toMapName: string;
  toStreetName: string;
  /** The destination's Map Mark, by OSMS's name for it; within the map, the map's own. */
  toMark: string | null;
  /**
   * A teleport within the map: where it lands, the mark there if it has one, and whether
   * that mark lands back here (a pair).
   */
  landsAt: { x: number; y: number; mark: number | null; bothWays: boolean } | null;
  /** Its row among the Hidden Portals. */
  hiddenPortal: number | null;
  /** The Hunting Target's route goes on from the shown map through this portal. */
  nextOnRoute: boolean;
};

/**
 * How the picture is laid out: the room for it in its box, inside the paper, the most it
 * may be zoomed (to the picture's own size), where its top left is in that room, and its
 * screen pixels to a picture pixel.
 */
type Frame = { width: number; height: number; most: number; left: number; top: number; scale: number };

/** Mirror of `minerva_core::SpawnMark`: where it is in the picture's pixels, and its mob. */
type SpawnMark = { x: number; y: number; mob: string };

/** A mob the map lists, as a spawn point's card names it (part of `minerva_core::Mob`). */
export type SpawnMob = { id: string; name: string; level: number; spawnCount: number };

/** Mirror of `minerva_core::MapPreview`. */
export type MapPreview = {
  picture: string;
  image: { status: "loading" | "unavailable" } | { status: "ready"; url: string };
  marks: PortalMark[];
  spawns: SpawnMark[];
};

/** The tallest the picture is drawn, inside its paper's 5px margin: the box stays within 180px. */
const MAX_PICTURE_HEIGHT = 168;
/** The paper's margin around the picture, so a ring at its edge isn't cut. */
const PAPER = 5;
/** A ring's radius (to the middle of its stroke), in screen pixels. */
const RING = 4;
/** The lit ring's outer ring. */
const LIT_RING = 7.5;
/** The ring around a portal next on the Hunting Target's route, outside the lit ring. */
const ROUTE_RING = 10;
/** The space kept between the card and the ring, and the panel's edges. */
const CARD_GAP = 6;
/** How much a notch of the mouse wheel zooms in, or out. */
const ZOOM_PER_NOTCH = 1.25;
/** The wheel's delta for a notch, in CSS px; a touchpad sends smaller ones. */
const NOTCH = 100;
/**
 * The close-up's screen pixels to a picture pixel. OSMS draws its pictures at the game's
 * own size, so this is half of it: a ledge and what stands on it, and on a screen at
 * twice the scale, the picture's own pixels.
 */
const CLOSE_UP_SCALE = 0.5;
/** How far down the close-up the portal sits: the ground it's on low, what's around it above. */
const CLOSE_UP_SPOT = 0.62;
/** How long the close-up takes to dive from the map to the spot. */
const DIVE_MS = 360;
/** The close-up's rings: larger than the map's, as its picture is. */
const CLOSE_RING = 6;
const CLOSE_LIT_RING = 10.5;
const CLOSE_ROUTE_RING = 15;
/** A spawn point's dot and its lit ring, on the map and in the close-up: the minimap's mob dot. */
const SPAWN_DOT = 2.25;
const SPAWN_LIT_RING = 5;
const CLOSE_SPAWN_DOT = 3.5;
const CLOSE_SPAWN_LIT_RING = 7.5;
/** How faint the other mobs' dots go while one mob's are lit. */
const DIMMED = 0.3;

const SVG = "http://www.w3.org/2000/svg";

const preview = byId("map-preview");
const picture = byId("map-picture");
const marksSvg = byId("map-marks") as unknown as SVGSVGElement;
/** The picture, under its rings; given a source once the shell has saved it. */
const image = document.createElement("img");
image.alt = "";
image.draggable = false;
image.hidden = true;
picture.prepend(image);
const loading = byId("map-loading");
const failed = byId("map-failed");
const rows = byId("portal-rows");
const card = byId("mark-card");
const panelEl = byId("panel");
/** The card's close-up of the spot, drawn from the picture the preview shows. */
const closeUp = document.createElement("canvas");
closeUp.className = "close-up";
closeUp.setAttribute("aria-hidden", "true");
/** What the card's close-up was opened on (the picture and the mark), and its dive. */
let closeUpFor = "";
let dive = 0;

/** The preview last rendered, and the shown map's name for its label. */
let shown: MapPreview | null = null;
let mapName = "";
/** Every Map Mark, as the rows were last drawn with, for the card's destination. */
let cardMarks: MapMarks = {};
/** The overlay couldn't load the picture at this URL. */
let brokenUrl: string | null = null;
/** What the rings were last drawn for: the marks and the picture's drawn size. */
let drawnFor = "";
/** The marks and spawn points as a key, and the preview they were read from. */
let marksKey = "";
let keyedFor: MapPreview | null = null;
/**
 * Where the picture's corner was when its rings were drawn. A drag moves the picture, not
 * its size, so the rings drawn then move with it as one rather than being drawn again.
 */
let drawnAt = { left: 0, top: 0 };
/** The rings, arrows and dots as drawn, moved as one while the picture is dragged. */
let drawnMarks: SVGGElement | null = null;
/** The mark the pointer is on (or on whose row), if any. */
let lit: { mark: number; onRing: boolean } | null = null;
/** The spawn point whose dot the pointer is on, if any (an index into the preview's spawns). */
let litSpawn: number | null = null;
/** The mob whose row in the Mobs table the pointer is on, if any. */
let rowMob: string | null = null;
/** The mobs the map lists, for a spawn point's card. */
let spawnMobs: SpawnMob[] = [];
/** The mob whose spawn points the panel was last told are lit. */
let toldMob: string | null = null;
/**
 * How far the player has zoomed in, over the fit: 1 shows the whole map. Kept while the
 * same map shows, as is the spot in the picture's pixels at the middle of the box (null:
 * the picture's middle), so the section opening again or the panel's width changing
 * shows the same part of it.
 */
let zoom = 1;
let centre: { x: number; y: number } | null = null;
/** How the picture was last laid out, while it shows. */
let frame: Frame | null = null;
/** A drag moving the zoomed picture: where it started, and where the picture was then. */
let panning: { x: number; y: number; left: number; top: number } | null = null;
/** Whether the mark card may show now: the panel expanded, on its normal view. */
let cardAllowed: () => boolean = () => true;
/** The panel's height may have changed: the overlay measures it again. */
let laidOut: () => void = () => {};
/** Another mob's spawn points are lit (or none's): the panel lights its row. */
let mobLit: (mob: string | null) => void = () => {};

/** Called once: what the preview asks of the panel. */
export function initMapPreview(options: {
  cardAllowed: () => boolean;
  laidOut: () => void;
  mobLit: (mob: string | null) => void;
}) {
  cardAllowed = options.cardAllowed;
  laidOut = options.laidOut;
  mobLit = options.mobLit;
}

/** The mob whose spawn points are lit: the one whose dot the pointer is on, else whose row. */
export function litMob(): string | null {
  const spawn = litSpawn === null ? undefined : shown?.spawns[litSpawn];
  return spawn?.mob ?? rowMob;
}

/** Light the spawn points of the mob whose row the pointer is on, and dim the rest; or none. */
export function lightMob(mob: string | null) {
  if (mob === rowMob) return;
  rowMob = mob;
  relight();
}

function svg<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number>) {
  const el = document.createElementNS(SVG, tag);
  for (const [name, value] of Object.entries(attrs)) el.setAttribute(name, String(value));
  return el;
}

/** The classes a mark's ring is drawn with: where it leads, and dashed when hidden. */
function ringClass(mark: PortalMark): string {
  const dashed = mark.kind === "hidden" || mark.kind === "quest" ? " dashed" : "";
  return `ring ${mark.leads}${dashed}`;
}

/** A ring: a white halo, then the ring in its colour over a faint fill. */
function ring(mark: PortalMark, cx: number, cy: number): SVGGElement {
  const g = svg("g", { class: ringClass(mark) });
  g.append(
    svg("circle", { class: "halo", cx, cy, r: RING }),
    svg("circle", { class: "stroke", cx, cy, r: RING }),
    svg("circle", { class: "lit-ring", cx, cy, r: LIT_RING }),
  );
  return g;
}

/** The ring a Hidden Portal's row leads with: its mark's, as on the map. */
function rowRing(mark: PortalMark | undefined): SVGSVGElement {
  const el = svg("svg", { class: "row-ring", viewBox: "0 0 12 12", "aria-hidden": "true" });
  const leads: PortalMark = mark ?? {
    x: 0,
    y: 0,
    leads: "anotherMap",
    kind: "hidden",
    toMapId: "",
    toMapName: "",
    toStreetName: "",
    toMark: null,
    landsAt: null,
    hiddenPortal: null,
    nextOnRoute: false,
  };
  el.append(ring(leads, 6, 6));
  return el;
}

/**
 * A row per Hidden Portal, each led by its mark's ring: "Northern Top of Construction Site
 * [Hidden Street]". Hovering one lights its mark.
 */
export function renderPortalRows(portals: HiddenPortal[], marks: PortalMark[], mapMarks: MapMarks) {
  cardMarks = mapMarks;
  rows.replaceChildren(
    ...portals.map((portal, i) => {
      const at = marks.findIndex((m) => m.hiddenPortal === i);
      const row = document.createElement("li");
      if (at >= 0) row.dataset.mark = String(at);
      const to = document.createElement("div");
      to.className = "dest";
      to.append(rowRing(marks[at]), mapMark(mapMarks, portal.toMark), span("to", portal.toMapName));
      if (portal.toHiddenStreet) to.append(span("tag", "Hidden Street"));
      if (portal.quest) to.append(span("tag quest", "Quest"));
      row.append(to);
      return row;
    }),
  );
  relight();
}

/**
 * The Map Preview, or nothing while there is none (no map, no picture, or not settled
 * which map it is). While the picture loads, "Loading map…" in a box of fixed height; if
 * it can't be had, "Couldn't load the map picture" in its place. Whether either shows.
 */
export function renderMapPreview(next: MapPreview | null, name: string, mobs: SpawnMob[] = []): boolean {
  // Another map, or none, its picture loaded or not: nothing on it is lit until the pointer
  // moves onto it, its card goes, and no mob of the last map stays lit.
  const another = next?.picture !== shown?.picture;
  shown = next;
  mapName = name;
  spawnMobs = mobs;
  if (another) {
    rowMob = null;
    hoverMark(null);
  }
  const url = next?.image.status === "ready" ? next.image.url : null;
  const unavailable = next?.image.status === "unavailable" || (url !== null && url === brokenUrl);
  preview.hidden = !next || unavailable;
  failed.hidden = !next || !unavailable;
  if (!next || unavailable) {
    hoverMark(null);
    return !!next;
  }
  if (url && image.getAttribute("src") !== url) {
    // Another picture: it shows whole.
    drawnFor = "";
    zoom = 1;
    centre = null;
    image.hidden = true;
    image.src = url;
  }
  if (!url) image.removeAttribute("src");
  layOutMapPreview();
  return true;
}

/**
 * The picture fitted and centred, zoomed and moved inside its box as the player has, its
 * rings drawn over it; or the placeholder while it loads. Again whenever the section
 * opens, the panel changes width, or the player zooms or drags.
 */
export function layOutMapPreview() {
  const ready = !!shown && image.hasAttribute("src") && image.complete && image.naturalWidth > 0;
  preview.classList.toggle("loading", !ready);
  loading.hidden = ready;
  picture.hidden = !ready;
  image.hidden = !ready;
  frame = null;
  // Collapsed, the box has no width to fit the picture to.
  if (!ready || !shown || preview.clientWidth === 0) return;
  // Fitted as if the panel's scrollbar showed, whether it does or not. Fitted to the room
  // the scrollbar leaves, the picture would be taller without it: in a panel just short of
  // the picture's height the scrollbar would come and go every frame.
  const room = preview.clientWidth - (scrollbarShows() ? 0 : scrollbarWidth()) - 2 * PAPER;
  const scale = Math.min(room / image.naturalWidth, MAX_PICTURE_HEIGHT / image.naturalHeight);
  const width = Math.round(image.naturalWidth * scale);
  const height = Math.round(image.naturalHeight * scale);
  // The box is the fitted picture's height, the column's width, whatever the zoom.
  picture.style.height = `${height}px`;
  const boxWidth = preview.clientWidth - 2 * PAPER;
  // Zoomed in no further than the picture's own size. It grows over the whole box, centred
  // along a side while it fits there, and moved along it no further than its paper.
  const most = Math.max(1, 1 / scale);
  zoom = Math.min(Math.max(zoom, 1), most);
  const drawnScale = scale * zoom;
  // The whole map fills its fitted box, as ever; zoomed in, the picture keeps its own shape.
  const pictureWidth = zoom > 1 ? image.naturalWidth * drawnScale : width;
  const pictureHeight = zoom > 1 ? image.naturalHeight * drawnScale : height;
  const middle = centre ?? { x: image.naturalWidth / 2, y: image.naturalHeight / 2 };
  const left = place(middle.x * drawnScale, pictureWidth, boxWidth);
  const top = place(middle.y * drawnScale, pictureHeight, height);
  centre = zoom > 1 ? { x: (boxWidth / 2 - left) / drawnScale, y: (height / 2 - top) / drawnScale } : null;
  frame = { width: boxWidth, height, most, left, top, scale: drawnScale };
  image.style.left = `${Math.round(left)}px`;
  image.style.top = `${Math.round(top)}px`;
  image.style.width = `${pictureWidth}px`;
  image.style.height = `${pictureHeight}px`;
  preview.classList.toggle("zoomed", zoom > 1);
  const hidden = shown.marks.filter((m) => m.hiddenPortal !== null).length;
  const next = shown.marks.filter((m) => m.nextOnRoute).length;
  const portals = shown.marks.length;
  const onRoute = next > 0 ? `, ${next} next on your route` : "";
  const points = shown.spawns.length;
  const spawns = points > 0 ? `, and ${points} mob spawn point${points === 1 ? "" : "s"}` : "";
  picture.setAttribute(
    "aria-label",
    `Map of ${mapName}: ${portals} portal${portals === 1 ? "" : "s"} marked, ${hidden} of them hidden${onRoute}${spawns}`,
  );
  if (shown !== keyedFor) {
    keyedFor = shown;
    marksKey = JSON.stringify([shown.marks, shown.spawns]);
  }
  const key = `${marksKey}|${frame.width}|${frame.height}|${frame.scale}`;
  if (key !== drawnFor) {
    drawnFor = key;
    drawnAt = { left: Math.round(left), top: Math.round(top) };
    drawMarks(shown.marks, shown.spawns, frame);
    relight();
  }
  drawnMarks?.setAttribute(
    "transform",
    `translate(${Math.round(left) - drawnAt.left} ${Math.round(top) - drawnAt.top})`,
  );
}

/**
 * Where the picture's edge goes along a side of the box `room` long, the picture `size`
 * long: centred while it fits, else with the point `spot` in from its edge at the middle,
 * but never leaving more than its paper beside it.
 */
const place = (spot: number, size: number, room: number) =>
  size <= room ? (room - size) / 2 : Math.min(Math.max(room / 2 - spot, room - size), 0);

/**
 * Where a ring or a landing is drawn, in screen pixels over the box. The picture is drawn
 * from a whole pixel, so it and its rings snap together; zooming works from where it
 * really is, so a spot held under the pointer doesn't creep away notch by notch.
 */
const at = (x: number, y: number, f: Frame) => ({
  x: Math.round(f.left) + x * f.scale,
  y: Math.round(f.top) + y * f.scale,
});

/** The spawn points' dots, then the arrows of the teleports within the map, then the rings. */
function drawMarks(marks: PortalMark[], spawns: SpawnMark[], f: Frame) {
  marksSvg.setAttribute("width", String(f.width));
  marksSvg.setAttribute("height", String(f.height));
  const defs = svg("defs", {});
  const head = svg("marker", {
    id: "landing-head",
    viewBox: "0 0 6 6",
    refX: 5,
    refY: 3,
    markerWidth: 6,
    markerHeight: 6,
    markerUnits: "userSpaceOnUse",
    orient: "auto-start-reverse",
  });
  head.append(svg("path", { d: "M0 0.5 6 3 0 5.5z", class: "head" }));
  defs.append(head);
  const arrows = svg("g", { class: "arrows" });
  marks.forEach((mark, i) => {
    const landing = mark.landsAt;
    if (mark.leads !== "withinTheMap" || !landing) return;
    // A pair is one line with a head at each end, drawn once.
    if (landing.bothWays && landing.mark !== null && landing.mark < i) return;
    const from = at(mark.x, mark.y, f);
    const to = at(landing.x, landing.y, f);
    const length = Math.hypot(to.x - from.x, to.y - from.y);
    // From ring edge to ring edge, or to the landing point when it has no ring.
    const start = RING + 2;
    const end = landing.mark !== null ? RING + 2 : 0;
    if (length <= start + end + 4) return;
    const ux = (to.x - from.x) / length;
    const uy = (to.y - from.y) / length;
    const line = {
      x1: from.x + ux * start,
      y1: from.y + uy * start,
      x2: to.x - ux * end,
      y2: to.y - uy * end,
    };
    const g = svg("g", { class: "arrow", "data-from": i, "data-to": landing.mark ?? "" });
    const dash = svg("line", { ...line, class: "dash", "marker-end": "url(#landing-head)" });
    if (landing.bothWays) dash.setAttribute("marker-start", "url(#landing-head)");
    g.append(svg("line", { ...line, class: "halo" }), dash);
    arrows.append(g);
  });
  // Next on the Hunting Target's route: a wide ring around the portal's own, on a halo of
  // its own, always shown. Over the arrows and under every ring, so it never hides one.
  const routeRings = svg("g", { class: "route-rings" });
  const rings = svg("g", { class: "rings" });
  marks.forEach((mark, i) => {
    const { x, y } = at(mark.x, mark.y, f);
    if (mark.nextOnRoute) {
      routeRings.append(
        svg("circle", { class: "route-halo", cx: x, cy: y, r: ROUTE_RING }),
        svg("circle", { class: "route-ring", cx: x, cy: y, r: ROUTE_RING }),
      );
    }
    const g = ring(mark, x, y);
    g.dataset.mark = String(i);
    // A target larger than the ring, so a small ring is easy to rest on.
    g.append(svg("circle", { class: "hit", cx: x, cy: y, r: 8 }));
    rings.append(g);
  });
  // Every spawn point, under everything else: the minimap's red mob dot on a white halo,
  // with a ring around it in the same red while its mob is lit.
  const dots = svg("g", { class: "spawns" });
  spawns.forEach((spawn, i) => {
    const { x, y } = at(spawn.x, spawn.y, f);
    const g = svg("g", { class: "spawn" });
    g.dataset.spawn = String(i);
    g.dataset.mob = spawn.mob;
    g.append(
      svg("circle", { class: "lit-ring", cx: x, cy: y, r: SPAWN_LIT_RING }),
      svg("circle", { class: "halo", cx: x, cy: y, r: SPAWN_DOT + 1 }),
      svg("circle", { class: "dot", cx: x, cy: y, r: SPAWN_DOT }),
      svg("circle", { class: "hit", cx: x, cy: y, r: 5 }),
    );
    dots.append(g);
  });
  drawnMarks = svg("g", {});
  drawnMarks.append(dots, arrows, routeRings, rings);
  marksSvg.replaceChildren(defs, drawnMarks);
}

/** Light `mark` (or nothing) and its row, its arrows, and its card. */
function hoverMark(mark: number | null, onRing = false) {
  lit = mark === null ? null : { mark, onRing };
  litSpawn = null;
  relight();
}

/** Light what's at `el` on the map: a ring, else a spawn point's dot, else nothing. */
function pointAt(el: Element | null) {
  const ring = el?.closest<SVGGElement>(".rings > g") ?? null;
  const dot = ring ? null : (el?.closest<SVGGElement>(".spawns > g") ?? null);
  lit = ring ? { mark: Number(ring.dataset.mark), onRing: true } : null;
  litSpawn = dot ? Number(dot.dataset.spawn) : null;
  relight();
}

/** Light the lit mark again after a redraw: its ring, its arrows, its row and its card. */
function relight() {
  const i = lit?.mark ?? -1;
  for (const g of marksSvg.querySelectorAll<SVGGElement>(".rings > g")) {
    const on = Number(g.dataset.mark) === i;
    g.classList.toggle("lit", on);
    // Over its neighbours while lit.
    if (on && g.nextSibling) g.parentNode?.append(g);
  }
  for (const g of marksSvg.querySelectorAll<SVGGElement>(".arrow")) {
    g.classList.toggle("lit", g.dataset.from === String(i) || g.dataset.to === String(i));
  }
  for (const row of rows.querySelectorAll<HTMLElement>("li")) {
    row.classList.toggle("lit", row.dataset.mark === String(i));
  }
  // A mob's spawn points, lit over the others, which dim; and its row in the Mobs table.
  const mob = litMob();
  const dots = marksSvg.querySelector(".spawns");
  dots?.classList.toggle("lighting", mob !== null);
  for (const g of marksSvg.querySelectorAll<SVGGElement>(".spawns > g")) {
    const on = g.dataset.mob === mob;
    g.classList.toggle("lit", on);
    if (on) dots?.append(g);
  }
  if (mob !== toldMob) {
    toldMob = mob;
    mobLit(mob);
  }
  renderCard();
}

/** The tags saying what kind of portal a mark is. A visible portal to a known map has none. */
function kindTags(mark: PortalMark): HTMLElement[] {
  const tags: HTMLElement[] = [];
  if (mark.kind === "hidden") tags.push(span("tag", "Hidden"));
  if (mark.kind === "quest") tags.push(span("tag quest", "Quest"));
  if (mark.kind === "door") tags.push(span("tag", "Door"));
  if (mark.leads === "withinTheMap") tags.push(span("tag", "Within the map"));
  if (mark.leads === "mapNotInTheData") tags.push(span("tag", "Not in the map list"));
  if (mark.nextOnRoute) tags.push(span("tag", "Next on your route"));
  return tags;
}

/** What a portal's card says: where it leads, its street and its kind. */
function portalWords(mark: PortalMark): Node[] {
  const head = document.createElement("div");
  head.className = "mark-card-to";
  head.append(mapMark(cardMarks, mark.toMark), mark.toMapName);
  const parts: Node[] = [head];
  if (mark.toStreetName) parts.push(span("mark-card-street", mark.toStreetName));
  const tags = kindTags(mark);
  if (tags.length) {
    const line = document.createElement("div");
    line.className = "mark-card-kinds";
    line.append(...tags);
    parts.push(line);
  }
  return parts;
}

/** What a spawn point's card says: the mob, after its red dot, then its level and spawn points. */
function spawnWords(mob: SpawnMob): Node[] {
  const head = document.createElement("div");
  head.className = "mark-card-to mark-card-mob";
  head.textContent = mob.name;
  const points = `${mob.spawnCount} spawn point${mob.spawnCount === 1 ? "" : "s"}`;
  return [head, span("mark-card-street", `Lv ${mob.level} · ${points}`)];
}

/**
 * The card of the ring or the spawn point's dot the pointer is on, or of the Hidden
 * Portal's row: the spot close up, then what it is. From a ring or a dot, above it,
 * centred on it, or below it when there's no room above; from a row, below it, or above it
 * when there's no room below. Never outside the panel or over its window bar. The pointer
 * passes through it.
 */
function renderCard() {
  const portal = lit && shown ? shown.marks[lit.mark] : undefined;
  const spawn = !portal && litSpawn !== null && shown ? shown.spawns[litSpawn] : undefined;
  const mob = spawn && spawnMobs.find((m) => m.id === spawn.mob);
  const anchor = portal
    ? lit!.onRing
      ? marksSvg.querySelector<SVGGElement>(`.rings > g[data-mark="${lit!.mark}"] .stroke`)
      : rows.querySelector<HTMLElement>(`li[data-mark="${lit!.mark}"]`)
    : mob
      ? marksSvg.querySelector<SVGGElement>(`.spawns > g[data-spawn="${litSpawn}"] .dot`)
      : null;
  const spot = portal ?? (mob ? spawn : undefined);
  if (!spot || !anchor || !frame || !cardAllowed() || preview.hidden) {
    card.hidden = true;
    closeUpFor = "";
    cancelAnimationFrame(dive);
    return;
  }
  const text = document.createElement("div");
  text.className = "mark-card-text";
  text.append(...(portal ? portalWords(portal) : spawnWords(mob!)));
  card.replaceChildren(closeUp, text);
  card.hidden = false;
  // A new spot dives in from the map as the preview shows it; the same one, lit again after
  // a redraw, holds where it is.
  const key = `${image.src}#${portal ? `portal ${lit!.mark}` : `spawn ${litSpawn}`}`;
  if (key !== closeUpFor) {
    closeUpFor = key;
    diveTo(spot, frame.scale);
  } else if (!dive) {
    paintCloseUp(spot, CLOSE_UP_SCALE);
  }
  const fromRow = !!portal && !lit!.onRing;
  const box = anchor.getBoundingClientRect();
  const { offsetWidth: width, offsetHeight: height } = card;
  const frameLeft = panelEl.getBoundingClientRect().left;
  const left = fromRow ? box.left - frameLeft : box.left + box.width / 2 - frameLeft - width / 2;
  card.style.left = `${Math.max(8, Math.min(left, panelEl.clientWidth - 8 - width))}px`;
  const prefer = fromRow ? "below" : "above";
  card.style.top = `${cardTop(panelEl, panelEl.querySelector(".window-bar"), box, height, CARD_GAP, prefer)}px`;
}

/**
 * Open the close-up on `spot`, diving from `from` (the preview's screen pixels to a
 * picture pixel) to the close-up's own scale: fast, then settling, evenly in zoom rather
 * than in size. The first frame is drawn at once, so the card is never blank. At once with
 * reduced motion, or when the preview is already as close.
 */
function diveTo(spot: { x: number; y: number }, from: number) {
  cancelAnimationFrame(dive);
  dive = 0;
  const still = from >= CLOSE_UP_SCALE || matchMedia("(prefers-reduced-motion: reduce)").matches;
  const start = performance.now();
  const step = (now: number) => {
    // A frame's time can come from just before the pointer arrived: never before the start.
    const t = still ? 1 : Math.min(Math.max((now - start) / DIVE_MS, 0), 1);
    // An exponential ease-out, landing exactly.
    const eased = t >= 1 ? 1 : (1 - 2 ** (-10 * t)) / (1 - 2 ** -10);
    paintCloseUp(spot, from * (CLOSE_UP_SCALE / from) ** eased);
    dive = t < 1 ? requestAnimationFrame(step) : 0;
  };
  step(start);
}

/** The marks' colours, read once from the panel's own. */
let markColours: Record<Leads | "route" | "mob" | "mobEdge", string> | null = null;

/**
 * The close-up at `scale`: the picture around `spot`, `CLOSE_UP_SPOT` of the way down,
 * kept to the picture where it reaches the close-up's sides; then every spawn point in it
 * (the lit mob's lit, the others dimmed while one is), every route ring, and every ring,
 * the lit one last. Drawn at the screen's own pixels.
 */
function paintCloseUp(spot: { x: number; y: number }, scale: number) {
  const ctx = closeUp.getContext("2d");
  if (!shown || !ctx || !image.naturalWidth) return;
  const width = closeUp.clientWidth;
  const height = closeUp.clientHeight;
  const ratio = window.devicePixelRatio || 1;
  const pixels = { width: Math.round(width * ratio), height: Math.round(height * ratio) };
  if (closeUp.width !== pixels.width) closeUp.width = pixels.width;
  if (closeUp.height !== pixels.height) closeUp.height = pixels.height;
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  ctx.clearRect(0, 0, width, height);
  // What it shows, in the picture's pixels: from a whole pixel, so at the close-up's own
  // scale on a screen at twice the scale each of its pixels is one of the picture's.
  const view = { width: width / scale, height: height / scale };
  const left = Math.round(within(spot.x - view.width / 2, view.width, image.naturalWidth));
  const top = Math.round(within(spot.y - view.height * CLOSE_UP_SPOT, view.height, image.naturalHeight));
  // Only the part of the picture inside it, where the close-up runs past the picture.
  const sx = Math.max(left, 0);
  const sy = Math.max(top, 0);
  const sw = Math.min(left + view.width, image.naturalWidth) - sx;
  const sh = Math.min(top + view.height, image.naturalHeight) - sy;
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = "high";
  if (sw > 0 && sh > 0) {
    ctx.drawImage(image, sx, sy, sw, sh, (sx - left) * scale, (sy - top) * scale, sw * scale, sh * scale);
  }
  if (!markColours) {
    const style = getComputedStyle(panelEl);
    const colour = (name: string) => style.getPropertyValue(name).trim();
    markColours = {
      anotherMap: colour("--portal"),
      withinTheMap: colour("--teleport"),
      mapNotInTheData: colour("--unknown"),
      route: colour("--route"),
      mob: colour("--mob"),
      mobEdge: colour("--mob-edge"),
    };
  }
  const colours = markColours;
  const place = (x: number, y: number) => ({ x: (x - left) * scale, y: (y - top) * scale });
  const margin = CLOSE_ROUTE_RING;
  const inView = (s: { x: number; y: number }) =>
    s.x > -margin && s.x < width + margin && s.y > -margin && s.y < height + margin;
  const circle = (x: number, y: number, r: number) => {
    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
  };
  const stroke = (colour: string, lineWidth: number, dash: number[] = []) => {
    ctx.strokeStyle = colour;
    ctx.lineWidth = lineWidth;
    ctx.setLineDash(dash);
    ctx.stroke();
  };
  const halo = "rgba(255, 255, 255, 0.95)";
  const mob = litMob();
  const dots = shown.spawns.map((s) => ({ s, ...place(s.x, s.y) })).filter(inView);
  // The lit mob's dots over the others.
  dots.sort((a, b) => Number(a.s.mob === mob) - Number(b.s.mob === mob));
  for (const d of dots) {
    const isLit = d.s.mob === mob;
    ctx.globalAlpha = mob !== null && !isLit ? DIMMED : 1;
    if (isLit) {
      circle(d.x, d.y, CLOSE_SPAWN_LIT_RING);
      stroke(halo, 3.5);
      stroke(colours.mob, 1.5);
    }
    circle(d.x, d.y, CLOSE_SPAWN_DOT);
    stroke(halo, 3);
    ctx.fillStyle = colours.mob;
    ctx.fill();
    stroke(colours.mobEdge, 1);
  }
  ctx.globalAlpha = 1;
  const spots = shown.marks.map((m) => ({ m, ...place(m.x, m.y) }));
  for (const s of spots) {
    if (!s.m.nextOnRoute || !inView(s)) continue;
    circle(s.x, s.y, CLOSE_ROUTE_RING);
    stroke(halo, 5);
    stroke(colours.route, 2.5);
  }
  const ringAt = (s: (typeof spots)[number], isLit: boolean) => {
    const colour = colours[s.m.leads];
    if (isLit) {
      circle(s.x, s.y, CLOSE_LIT_RING);
      stroke(halo, 4);
      stroke(colour, 1.75);
    }
    circle(s.x, s.y, CLOSE_RING);
    stroke(halo, 4.5);
    ctx.globalAlpha = 0.22;
    ctx.fillStyle = colour;
    ctx.fill();
    ctx.globalAlpha = 1;
    const dashed = s.m.kind === "hidden" || s.m.kind === "quest";
    stroke(colour, 2, dashed ? [3.4, 2.1] : []);
  };
  const litRing = lit?.mark ?? -1;
  spots.forEach((s, i) => i !== litRing && inView(s) && ringAt(s, false));
  if (spots[litRing] && inView(spots[litRing])) ringAt(spots[litRing], true);
}

/**
 * Where a view `size` long starts along a picture `total` long, to show `start` onwards:
 * kept to the picture while it's the longer, centred on it while it's the shorter.
 */
const within = (start: number, size: number, total: number) =>
  size >= total ? (total - size) / 2 : Math.min(Math.max(start, 0), total - size);

// Resting on a ring lights it and its row and shows its card; on a row, lights its ring and
// shows the same card. Resting on a spawn point's dot lights its mob's dots and row, and
// shows its card. Nothing lights while a drag moves the picture.
marksSvg.addEventListener("mouseover", (e) => {
  if (panning) return;
  pointAt(e.target instanceof Element ? e.target : null);
});
marksSvg.addEventListener("mouseleave", () => hoverMark(null));
rows.addEventListener("mouseover", (e) => {
  const row = e.target instanceof Element ? e.target.closest<HTMLElement>("li[data-mark]") : null;
  hoverMark(row ? Number(row.dataset.mark) : null);
});
rows.addEventListener("mouseleave", () => hoverMark(null));

/** Light the ring or dot at a point the pointer rests on, if any: the picture moved under it. */
function hoverAt(x: number, y: number) {
  pointAt(document.elementFromPoint(x, y));
}

/** Show the picture at `next` times the fit, the spot under the pointer staying under it. */
function zoomAt(next: number, e: MouseEvent) {
  if (!frame) return;
  const box = picture.getBoundingClientRect();
  const x = e.clientX - box.left;
  const y = e.clientY - box.top;
  const spot = { x: (x - frame.left) / frame.scale, y: (y - frame.top) / frame.scale };
  const was = zoom;
  zoom = Math.min(Math.max(next, 1), frame.most);
  // Back out to the whole map, never a hair's breadth from it.
  if (zoom < 1.001) zoom = 1;
  const scale = (frame.scale * zoom) / was;
  centre = { x: spot.x + (frame.width / 2 - x) / scale, y: spot.y + (frame.height / 2 - y) / scale };
  layOutMapPreview();
  hoverAt(e.clientX, e.clientY);
}

// Over the shown picture's box, its paper included, the wheel zooms and never scrolls the
// panel: in as far as the picture's own size, out as far as the whole map. While it loads,
// the wheel scrolls the panel as anywhere else.
preview.addEventListener("wheel", (e) => {
  if (!frame) return;
  e.preventDefault();
  zoomAt(zoom * ZOOM_PER_NOTCH ** (-e.deltaY / NOTCH), e);
});

// Zoomed in, a drag moves the picture, no further than its paper.
preview.addEventListener("pointerdown", (e) => {
  if (e.button !== 0 || !frame || zoom === 1) return;
  panning = { x: e.clientX, y: e.clientY, left: frame.left, top: frame.top };
  preview.setPointerCapture(e.pointerId);
});
preview.addEventListener("pointermove", (e) => {
  if (!panning || !frame || (e.clientX === panning.x && e.clientY === panning.y)) return;
  if (!preview.classList.contains("panning")) {
    preview.classList.add("panning");
    hoverMark(null);
  }
  const left = panning.left + (e.clientX - panning.x);
  const top = panning.top + (e.clientY - panning.y);
  centre = { x: (frame.width / 2 - left) / frame.scale, y: (frame.height / 2 - top) / frame.scale };
  layOutMapPreview();
});
function endPan() {
  panning = null;
  preview.classList.remove("panning");
}
// Let go, the ring it rests on lights.
preview.addEventListener("pointerup", (e) => {
  if (!panning) return;
  const moved = preview.classList.contains("panning");
  endPan();
  if (moved) hoverAt(e.clientX, e.clientY);
});
preview.addEventListener("pointercancel", endPan);

// A double-click shows the whole map again.
preview.addEventListener("dblclick", (e) => {
  if (frame && zoom > 1) zoomAt(1, e);
});

image.addEventListener("load", () => {
  layOutMapPreview();
  laidOut();
});
// The overlay couldn't load the picture the source saved: as if it couldn't be had.
image.addEventListener("error", () => {
  if (!image.hasAttribute("src")) return;
  brokenUrl = image.getAttribute("src");
  renderMapPreview(shown, mapName);
  laidOut();
});
/** Whether the panel's scrolling view shows its scrollbar now. */
function scrollbarShows(): boolean {
  const view = preview.closest<HTMLElement>(".view");
  return !!view && view.offsetWidth - view.clientWidth - view.clientLeft * 2 > 0;
}

/** How wide a scrollbar is here, in CSS px: it changes with the display's scale. */
let scrollbar: { ratio: number; width: number } | null = null;
function scrollbarWidth(): number {
  if (scrollbar?.ratio === devicePixelRatio) return scrollbar.width;
  const probe = document.createElement("div");
  probe.style.cssText = "position: absolute; visibility: hidden; width: 50px; height: 50px; overflow-y: scroll";
  document.body.append(probe);
  const width = probe.offsetWidth - probe.clientWidth;
  probe.remove();
  scrollbar = { ratio: devicePixelRatio, width };
  return width;
}

// The box changed width (the player resized the panel, or a scrollbar came or went): the
// picture is fitted again, and the panel's height, which follows it, reported.
let fittedTo = 0;
new ResizeObserver(() => {
  if (preview.clientWidth === fittedTo) return;
  fittedTo = preview.clientWidth;
  if (shown && !preview.hidden) {
    layOutMapPreview();
    laidOut();
  }
}).observe(preview);
