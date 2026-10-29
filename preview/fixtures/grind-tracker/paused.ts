import { solo, view } from "./_base";

/** Paused: no EXP for a while, the figures held and tagged, the Pickup Log covered. */
export default view(
  {
    ...solo,
    state: "paused",
    rolling: { expPerHour: 12_300, percentPerHourHundredths: 340 },
    timeToLevelBadge: "5h 40m",
    pace: null,
    forecast: [{ afterS: 3_600, percentHundredths: 8_544 }],
  },
  { pickupLogCovered: true },
);
