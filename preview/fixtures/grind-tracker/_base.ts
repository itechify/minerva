import type { TrackerView } from "../../../src/grind-tracker";

type Tracking = TrackerView["tracking"];

/** The 5 minutes before (EXP gained since the trace's first point), then a kill every 20s
 *  or so, a little ahead of that pace. */
const points = [0, 300, 650, 980, 1330, 1640, 2010, 2340, 2700, 3010, 3390, 3720, 4080, 4400, 4790].map((exp, i, all) => ({
  agoMs: (all.length - 1 - i) * 21_000,
  exp,
}));

/** A Session under way for 34 minutes, alone, on a map with a Personal Best. */
export const solo: Tracking = {
  state: "active",
  timeToLevelS: 4_320,
  timeToLevelBadge: "1h 12m",
  rolling: { expPerHour: 57_400, percentPerHourHundredths: 1_570 },
  sessionAverage: { expPerHour: 52_100, percentPerHourHundredths: 1_420 },
  deaths: { count: 0, expLost: 0, percentLostHundredths: 0 },
  characterSwitched: false,
  activeS: 2_040,
  rollingS: 300,
  resetUndoable: false,
  bar: { exp: 300_120, percentHundredths: 8_204 },
  personalBest: 61_000,
  personalBestThisVisit: false,
  personalBestParty: false,
  newBest: false,
  pace: { spanMs: 300_000, points, earlier: [{ agoMs: 300_000, exp: 0 }, { agoMs: 0, exp: 4_400 }], best: null, arrivedAgoMs: null },
  forecast: [
    { afterS: 300, percentHundredths: 8_335 },
    { afterS: 1_800, percentHundredths: 8_989 },
  ],
  lostFromHundredths: null,
  noStatusBar: false,
  loot: {
    mesos: 18_430,
    mesosPerHour: 32_500,
    items: [
      { name: "Wild Boar Tooth", count: 41 },
      { name: "Orange Potion", count: 6 },
      { name: "Bronze Ore", count: 2 },
    ],
    itemsTotal: 49,
  },
  expSplit: null,
};

export const view = (tracking: Tracking, more: Partial<TrackerView> = {}) =>
  ({
    shown: true,
    tracking,
    character: { name: "Owlbert", job: "Hunter", level: 34 },
    personalBests: true,
    personalBestsOpen: false,
    pickupLogCovered: false,
    lootOpen: false,
    ...more,
  }) satisfies TrackerView;
