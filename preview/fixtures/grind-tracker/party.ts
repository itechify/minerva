import { solo, view } from "./_base";

/** In a party: the Best row races the Party Best, the last 5 minutes are a new best, and
 *  a death cost some of the level. */
export default view({
  ...solo,
  rolling: { expPerHour: 74_800, percentPerHourHundredths: 2_050 },
  sessionAverage: { expPerHour: 66_300, percentPerHourHundredths: 1_810 },
  timeToLevelBadge: "52m",
  personalBest: 74_800,
  personalBestThisVisit: true,
  personalBestParty: true,
  newBest: true,
  deaths: { count: 1, expLost: 36_500, percentLostHundredths: 1_000 },
  bar: { exp: 263_620, percentHundredths: 7_204 },
  lostFromHundredths: 8_204,
  expSplit: { ownHundredths: 6_200, partyHundredths: 3_800 },
});
