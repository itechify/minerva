import "./errors";
import { byId, span } from "./dom";
import type { Character, ExpRate, ExpTracking, Loot, PacePoint, PaceTrace } from "./exp-tracking";
import { percent, rate, whole } from "./figures";
import { dragByHandles, followView, heightReporter, pressControls } from "./shell";

/** What the shell sends: the figures, the Character playing (none until the Status Bar names
 *  one), whether the window shows at all, whether some map has a Personal Best (the Personal
 *  Bests button shows), and whether the Personal Bests window is open (the button shows as on). */
type TrackerView = {
  shown: boolean;
  tracking: ExpTracking;
  character: Character | null;
  personalBests: boolean;
  personalBestsOpen: boolean;
  /** One of our windows has been over the Pickup Log for a while. */
  pickupLogCovered: boolean;
  /** The Loot section is open, as the player left it. */
  lootOpen: boolean;
};

/** How many Items the open Loot section lists before "+N more". */
const TOP_ITEMS = 5;

const panelEl = byId("panel");
const openBests = byId("open-bests");
const tracking = byId("tracking");
const unreadable = byId("tracking-unreadable");
const switched = byId("tracking-switched");
const resetNote = byId("tracking-reset");
const waiting = byId("tracking-waiting");
const figures = byId("tracking-figures");
const headline = byId("tracking-headline");
const timeToLevel = byId("time-to-level");
const tag = byId("tracking-tag");
const rollingRow = byId("rolling-row");
const rollingExp = byId("rolling-exp");
const rollingPercent = byId("rolling-percent");
const rollingStar = byId("rolling-star");
const sessionRow = byId("session-row");
const sessionLength = byId("session-length");
const sessionExp = byId("session-exp");
const sessionPercent = byId("session-percent");
const bestRow = byId("best-row");
const bestExp = byId("best-exp");
const bestThisVisit = byId("best-this-visit");
const mesoTotal = byId("meso-total");
const mesoRate = byId("meso-rate");
const pickupCovered = byId("tracking-pickup-covered");
const bestParty = byId("best-party");
const expSplit = byId("exp-split");
const splitOwn = byId("split-own");
const splitParty = byId("split-party");
const loot = byId("loot");
const lootHead = byId("loot-head");
const lootBody = byId("loot-body");
const lootCount = byId("loot-count");
const lootItems = byId("loot-items");
const lootMore = byId("loot-more");
/** "+N more" opened the rest of the Items: for this window only, not kept. */
let allItems = false;
lootMore.addEventListener("click", () => {
  allItems = !allItems;
  if (shown) render(shown);
});
const deaths = byId("tracking-deaths");
const exp = byId("tracking-exp");
const expFigures = byId("exp-figures");
const expCharacter = byId("exp-character");
const expGauge = byId("exp-gauge");
const expGaugeLost = byId("exp-gauge-lost");
const forecast = byId("forecast");
const pace = byId("pace");
const paceName = byId("pace-name");
const paceAheadPath = byId("pace-ahead-path");
const paceArea = byId("pace-area");
const paceLead = byId("pace-lead");
const paceArrived = byId("pace-arrived");
const paceEarlier = byId("pace-earlier");
const paceBest = byId("pace-best");
const paceTrace = byId("pace-trace");
const paceBestMark = byId("pace-best-mark");
const paceEarlierName = byId("pace-earlier-name");
const paceSplit = byId("pace-split");
const paceYou = byId("pace-you");

/** A Session's length: "7m", "1h 12m". */
function length(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  const [h, m] = [Math.floor(minutes / 60), minutes % 60];
  return h === 0 ? `${m}m` : m === 0 ? `${h}h` : `${h}h ${m}m`;
}

/** The view last rendered, to draw again at another width. */
let shown: TrackerView | null = null;

/** "16.2%" from hundredths of a percent, as the bar would show it to one decimal. */
const barPercent = (hundredths: number) => `${(hundredths / 100).toFixed(1)}%`;

/** A forecast's reach: "5m", "30m", "1h", "24h". */
const reach = (seconds: number) => (seconds < 3600 ? `${seconds / 60}m` : `${seconds / 3600}h`);

/** Why there is no Time to Level, in words: never a bare dash beside the label. */
function noTimeToLevel(rolling: ExpRate | null) {
  if (rolling && rolling.expPerHour < 0) return "losing EXP";
  if (rolling && rolling.expPerHour === 0) return "no gain yet";
  return "not yet";
}

/** The Loot section, once the Session has started: collapsed, its totals on the header ("1,593
 *  mesos 28 items", a part left out while it is none); open, the Mesos row, then the 5 Items
 *  picked up most with their counts, and "+N more" for the rest. */
function renderLoot(l: Loot, open: boolean, started: boolean) {
  loot.hidden = !started;
  lootHead.setAttribute("aria-expanded", String(open));
  loot.classList.toggle("open", open);
  lootBody.hidden = !open;
  const totals = [total(l.mesos, "meso"), total(l.itemsTotal, "item")].filter((part) => part !== null);
  lootCount.replaceChildren(...(open ? [] : totals.flatMap((part, i) => (i === 0 ? [part] : [" ", part]))));
  mesoTotal.textContent = whole(l.mesos);
  mesoRate.textContent = l.mesosPerHour === null ? "–" : rate(l.mesosPerHour);
  lootItems.hidden = l.items.length === 0;
  const shownItems = allItems ? l.items : l.items.slice(0, TOP_ITEMS);
  lootItems.replaceChildren(
    ...shownItems.map((item) => {
      const row = document.createElement("li");
      row.append(span("name", item.name), span("tally", whole(item.count)));
      return row;
    }),
  );
  const rest = l.items.length - TOP_ITEMS;
  lootMore.hidden = rest <= 0;
  lootMore.textContent = allItems ? "Show fewer" : `+${rest} more`;
}

/** One of the Loot header's totals: its figure in Panel Ink, then its word as the label's; none
 *  while it is zero. */
function total(n: number, word: string) {
  if (n === 0) return null;
  const part = document.createElement("span");
  part.append(span("figure", whole(n)), ` ${word}${n === 1 ? "" : "s"}`);
  return part;
}

/** The Grind Tracker: a line until the Session starts, then Time to Level over the rates
 *  and what else the Session carries; the map's Best row from arriving, before the
 *  Session starts too; the EXP bar's last reading at the foot, from the first reading on.
 *  Above them, why the figures went or can't move. */
function render(view: TrackerView) {
  shown = view;
  const { tracking: t, personalBests, personalBestsOpen } = view;
  // Not showing, the window draws nothing (the shell lets clicks through it).
  document.body.classList.toggle("hidden", !view.shown);
  // Hidden, not greyed, while no map has a best to show; pressed while their window is open.
  openBests.hidden = !personalBests;
  openBests.setAttribute("aria-pressed", String(personalBestsOpen));
  const started = t.rolling !== null || t.state === "warmingUp" || t.state === "active" || t.state === "paused";
  tracking.dataset.state = t.state;
  // Rates still settling, held while paused, or the bar out of sight: the figures step back.
  tracking.classList.toggle("held", t.state !== "active");

  unreadable.hidden = t.state !== "unreadable";
  const why = t.noStatusBar
    ? "Can't find the game's bar at the bottom of the window. If it shows, send a Bug Report from the owl's menu by the clock."
    : "Can't read the EXP bar. Move whatever covers it.";
  if (unreadable.textContent !== why) unreadable.textContent = why;
  switched.hidden = !t.characterSwitched;
  resetNote.hidden = !t.resetUndoable;
  waiting.hidden = started || t.state === "unreadable";

  // The Best row shows from arriving on the map: until the Session starts it is all the
  // figures hold, under the waiting note. Time to Level and the rates wait for a gain.
  figures.hidden = !started && t.personalBest === null;
  headline.hidden = !started;
  rollingRow.hidden = !started;
  timeToLevel.textContent = t.timeToLevelBadge ?? noTimeToLevel(t.rolling);
  timeToLevel.classList.toggle("words", !t.timeToLevelBadge);
  // The headline's one tag, "paused". Warming up has none: its figures in Light say it.
  tag.hidden = t.state !== "paused";

  const expPerHour = (r: ExpRate | null) => (r ? rate(r.expPerHour) : "–");
  const percentPerHour = (r: ExpRate | null) =>
    r?.percentPerHourHundredths != null ? percent(r.percentPerHourHundredths) : "–";
  rollingExp.textContent = expPerHour(t.rolling);
  rollingPercent.textContent = percentPerHour(t.rolling);
  // The core says when Last 5 min is a new best on the map. The star keeps its room in
  // the row while it's off, so the figures don't shift as it comes and goes.
  rollingStar.classList.toggle("on", t.newBest);
  // Only once the Session is longer than the Rolling Rate's window does its average say
  // something else.
  sessionRow.hidden = !started || t.activeS <= t.rollingS;
  sessionLength.textContent = length(t.activeS);
  sessionExp.textContent = expPerHour(t.sessionAverage);
  sessionPercent.textContent = percentPerHour(t.sessionAverage);
  // The map's best as it stands, written as the rates are, with its ×, and whether this
  // Visit set it.
  bestRow.hidden = t.personalBest === null;
  bestExp.textContent = t.personalBest === null ? "" : rate(t.personalBest);
  bestThisVisit.hidden = !t.personalBestThisVisit;
  // Racing the Party Best, while Party EXP is in the last 5 minutes: a small party mark.
  bestParty.toggleAttribute("hidden", !t.personalBestParty);
  // The EXP Split, once Party EXP has come in this Session: two colours in proportion, and
  // the shares on hover.
  expSplit.hidden = sessionRow.hidden || t.expSplit === null;
  if (t.expSplit) {
    const { ownHundredths: own, partyHundredths: party } = t.expSplit;
    splitOwn.style.flexGrow = String(own);
    splitParty.style.flexGrow = String(party);
    const shares = `Your own EXP ${percent(own, 0)} of the Session's, Party EXP ${percent(party, 0)}`;
    expSplit.title = shares;
    expSplit.setAttribute("aria-label", shares);
  }

  // The Session's Loot from the Pickup Log, once it has started, even at none; the mesos' rate
  // waits for active time.
  renderLoot(t.loot, view.lootOpen, started);
  // Said quietly, only while the Session shows figures that a cover would leave low.
  pickupCovered.hidden = !started || !view.pickupLogCovered;

  const died = t.deaths.count;
  deaths.hidden = died === 0;
  // No-break spaces: where the line wraps, the loss reads as one.
  const lost = percent(-t.deaths.percentLostHundredths);
  deaths.textContent = `${died} ${died === 1 ? "death" : "deaths"} (${lost} of the level)`;

  exp.hidden = !t.bar;
  if (t.bar) {
    // As the bar writes it: "8,435 [41.72%]".
    const reading = `${whole(t.bar.exp)} [${(t.bar.percentHundredths / 100).toFixed(2)}%]`;
    expFigures.textContent = reading;
    // Who it is: the name and level the game's Status Bar shows beside its EXP bar. The job
    // would rarely fit beside the reading, so it's left to the Personal Bests window.
    const c = view.character;
    expCharacter.hidden = !c;
    if (c) {
      expCharacter.textContent = c.level === null ? c.name : `${c.name} · Lv. ${c.level}`;
    }
    expGauge.setAttribute("aria-valuenow", String(t.bar.percentHundredths / 100));
    const lost = t.lostFromHundredths;
    const before = lost === null ? "" : `, down from ${barPercent(lost)} before a death`;
    expGauge.setAttribute("aria-valuetext", `EXP ${reading}${before}`);
    // The fill and the ground lost both measure from the gauge's own --filled.
    expGauge.style.setProperty("--filled", String(Math.min(t.bar.percentHundredths, 10_000) / 10_000));
    expGaugeLost.hidden = lost === null;
    if (lost !== null) expGaugeLost.style.setProperty("--lost", String(Math.min(lost, 10_000) / 10_000));
  }
  drawForecast(t);
  drawPace(t.pace);
  reportHeight();
}

/** What the forecast was last laid out for. */
let forecastFor = "";
/** The forecast under the gauge: Time to Level at its end, where the bar is full, then a
 *  notch where the bar will be in so long, clear of the fill, of each other and of the
 *  end, three at most. Where two crowd, the further reach takes the nearer's place, so a
 *  slow pace reads in hours rather than skipping one. */
function drawForecast(t: ExpTracking) {
  // Laid out notch by notch, each measured as it goes in: only when what it says, or the
  // room it has, has changed.
  const key = JSON.stringify([t.bar?.percentHundredths ?? null, t.timeToLevelBadge, t.forecast, forecast.clientWidth]);
  if (key === forecastFor) return;
  forecastFor = key;
  forecast.replaceChildren();
  forecast.hidden = !t.bar || (t.timeToLevelBadge === null && t.forecast.length === 0);
  if (!t.bar || forecast.hidden) return;
  const box = forecast.getBoundingClientRect();
  const filled = (Math.min(t.bar.percentHundredths, 10_000) / 10_000) * box.width;
  const notch = (at: number, text: string, className: string) => {
    const el = span(className, text);
    el.style.setProperty("--at", String(at));
    forecast.append(el);
    const r = el.getBoundingClientRect();
    return { el, left: r.left - box.left, right: r.right - box.left };
  };
  const end = t.timeToLevelBadge === null ? null : notch(1, t.timeToLevelBadge, "notch end");
  const room = end ? end.left - 6 : box.width;
  const kept: { el: HTMLElement; left: number; right: number; said: string }[] = [];
  for (const f of t.forecast) {
    const at = f.percentHundredths / 10_000;
    // A notch on the fill's edge says nothing the fill doesn't.
    if (at * box.width - filled < 12) continue;
    const n = { ...notch(at, reach(f.afterS), "notch"), said: `${barPercent(f.percentHundredths)} in ${reach(f.afterS)}` };
    const crowds = () => kept.length > 0 && n.left < kept[kept.length - 1].right + 7;
    if (n.left < 0 || n.right > room || (!crowds() && kept.length === 3)) {
      n.el.remove();
      // Each further reach is further right: none of them fits either.
      if (n.right > room || kept.length === 3) break;
      continue;
    }
    while (crowds()) kept.pop()!.el.remove();
    kept.push(n);
  }
  const said = kept.map((n) => n.said);
  const level = t.timeToLevelBadge === null ? "" : `level up in ${t.timeToLevelBadge}`;
  forecast.setAttribute("aria-label", `At this pace: ${[...said, level].filter(Boolean).join(", ")}`);
}

/** The Pace Trace's height in CSS px; its width is the figures box's. */
const PACE_HEIGHT = 40;
/** Room at the top and the right end for the player's dot. */
const DOT_ROOM = 5;

function set(el: Element, attributes: Record<string, string | number>) {
  for (const [name, value] of Object.entries(attributes)) el.setAttribute(name, String(value));
}

/** To the half pixel, so the 1px lines sit on the pixel grid at 1x. */
const half = (n: number) => Math.round(n * 2) / 2;

/** "+103" ahead, "−240" behind, as the trace's split says it. */
const split = (exp: number) => (exp > 0 ? `+${whole(exp)}` : whole(exp));

/** The Pace Trace, a race drawn as split times against a ghost. The 5 minutes before are
 *  the flat line, and the trace is how far ahead of their pace the player is, falling back
 *  as that pace runs on while the EXP holds and stepping up at each gain; the gap shaded,
 *  the split written by the player's dot. Once the Best's pace is within reach it joins
 *  as a line rising (or falling) against the earlier one, ending in a best's star, the
 *  lead over it lit, and the split is the race with it, in its gold. The scale is the
 *  trace's own, so a lead of a few kills fills it as a race should. */
function drawPace(trace: PaceTrace | null) {
  // An <svg> has no `hidden` property: the attribute, which the page's [hidden] rule hides.
  const none = !trace || trace.points.length === 0;
  pace.toggleAttribute("hidden", none);
  if (!trace || none) return;
  const width = Math.round(pace.getBoundingClientRect().width);
  const height = PACE_HEIGHT;
  set(pace, { viewBox: `0 0 ${width} ${height}`, height });
  const { points, earlier, best, spanMs } = trace;

  // Every figure as how far ahead of the earlier pace it is, at its time.
  const [first, last] = [points[0], points[points.length - 1]];
  const perMs = (earlier[1].exp - earlier[0].exp) / Math.max(1, earlier[0].agoMs - earlier[1].agoMs);
  const ahead = (p: PacePoint) => p.exp - earlier[0].exp - perMs * (earlier[0].agoMs - p.agoMs);
  // The corners, as [ms before the last change, EXP ahead]: along the earlier pace while
  // the EXP holds, then up (down, after a death) to it.
  const corners: [number, number][] = [[first.agoMs, ahead(first)]];
  for (const p of points.slice(1)) {
    const [agoMs, lead] = corners[corners.length - 1];
    corners.push([p.agoMs, lead - perMs * (agoMs - p.agoMs)], [p.agoMs, ahead(p)]);
  }
  const bestLine = best && best.map((p) => [p.agoMs, ahead(p)] as const);

  const values = corners.map(([, v]) => v).concat(bestLine ? bestLine.map(([, v]) => v) : []);
  let [lo, hi] = [Math.min(0, ...values), Math.max(0, ...values)];
  // Never finer than a minute of the earlier pace from top to foot: a lead of a few EXP
  // is a few EXP, not the whole height.
  const least = Math.max(1, Math.abs(perMs) * 60_000);
  if (hi - lo < least) [lo, hi] = [lo - (least - (hi - lo)) / 2, hi + (least - (hi - lo)) / 2];
  const [left, right, top, bottom] = [0.5, width - DOT_ROOM, DOT_ROOM, height - DOT_ROOM];
  const x = (agoMs: number) => half(right - (Math.min(agoMs, spanMs) / spanMs) * (right - left));
  const y = (value: number) => half(bottom - ((value - lo) / (hi - lo)) * (bottom - top));

  const line = corners.map(([agoMs, v], i) => `${i ? "L" : "M"}${x(agoMs)},${y(v)}`).join("");
  const start = x(first.agoMs);
  const zero = y(0);
  const [, now] = corners[corners.length - 1];
  const dot = { x: x(0), y: y(now) };
  paceTrace.setAttribute("d", line);
  // The gap from the earlier pace, either side.
  paceArea.setAttribute("d", `${line}V${zero}H${start}Z`);
  set(paceEarlier, { x1: start, y1: zero, x2: dot.x, y2: zero });
  set(paceYou, { cx: dot.x, cy: dot.y });

  paceArrived.toggleAttribute("hidden", trace.arrivedAgoMs === null);
  const arrived = x(trace.arrivedAgoMs ?? 0);
  set(paceArrived, { x1: arrived, x2: arrived, y1: 0, y2: height });

  // "Earlier" at the start of its line, on the side the trace leaves clear there.
  const early = corners.filter(([agoMs]) => x(agoMs) <= start + 40).map(([, v]) => v);
  const aboveEarly = early.reduce((sum, v) => sum + v, 0) > 0;
  const below = (aboveEarly && zero + 11 <= height) || zero < 11;
  set(paceEarlierName, { x: start + 3, y: below ? zero + 10 : zero - 3 });
  // The split, by the dot: with the 5 minutes before, or once it's within reach, with
  // the best.
  const gap = Math.round(best ? last.exp - best[1].exp : now);
  paceSplit.textContent = split(gap);
  paceSplit.classList.toggle("behind", gap < 0);
  paceSplit.classList.toggle("best", !!best);
  // Under the dot when it is high, over it when low: clear of the trace coming in to it.
  const splitY = dot.y < height / 2 ? dot.y + 14 : dot.y - 6;
  set(paceSplit, { x: dot.x - 4, y: Math.max(9, Math.min(height - 1, splitY)) });
  const standing = (gap: number) =>
    gap > 0 ? `${whole(gap)} EXP ahead` : gap < 0 ? `${whole(-gap)} EXP behind` : "level";
  let name = `Your last 5 minutes of play against the 5 minutes before: ${standing(Math.round(now))}.`;

  for (const el of [paceLead, paceBest, paceBestMark]) el.toggleAttribute("hidden", !bestLine);
  if (best && bestLine) {
    const [from, to] = bestLine.map(([agoMs, v]) => ({ x: x(agoMs), y: y(v) }));
    set(paceBest, { x1: from.x, y1: from.y, x2: to.x, y2: to.y });
    // The best's star, 10px, where its pace has got to.
    paceBestMark.setAttribute("transform", `translate(${to.x - 5},${to.y - 5}) scale(0.625)`);
    // The lead: under the trace and above the Best's pace line.
    paceLead.setAttribute("d", `${line}V${height}H${start}Z`);
    paceAheadPath.setAttribute("d", `M${from.x},${from.y}L${to.x},${to.y}V0H${from.x}Z`);
    name += ` Your best's pace on this map is within reach: ${standing(gap)} of it.`;
  }
  paceName.textContent = name;
}

// The shell sizes the window to the content, as the overlay does the panel.
const reportHeight = heightReporter(panelEl, "page_height");
// The trace and the forecast are drawn to the window's width.
window.addEventListener("resize", () => (shown ? render(shown) : reportHeight()));
// The height depends on the font: measure again once it has loaded.
document.fonts.ready.then(reportHeight);

// Dragged by its header, never by its buttons. A click leaves the keyboard with the game:
// the window can't be activated.
dragByHandles();

// Close, reset and its Undo, removing the best and opening or closing the Personal Bests
// window, through the core like the panel's icons.
pressControls(panelEl);

followView<TrackerView>("grind-tracker-state", "page_view", render);
reportHeight();
