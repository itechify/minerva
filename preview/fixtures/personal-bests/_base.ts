import type { BestsView } from "../../../src/personal-bests";

type Character = BestsView["characters"][number];
type Map = Character["maps"][number];

const DAY = 86_400_000;
const NOW = 1_793_000_000_000;

/** A map's row: a Solo Best, perhaps a Party Best, and a Visit for each. */
function map(id: string, streetName: string, mapName: string, solo: number, party: number | null, more: Partial<Map> = {}): Map {
  const ceiling = solo * 2;
  const share = (exp: number) => Math.round((exp / ceiling) * 10_000);
  const best = Math.max(solo, party ?? 0);
  return {
    id,
    streetName,
    mapName,
    region: "Victoria Island",
    mark: null,
    url: `https://example.com/map/${id}`,
    leaderboardUrl: `https://example.com/leaderboards/${id}`,
    bestOfAll: false,
    mobLevels: { lowest: 20, highest: 26 },
    expPerHour: best,
    solo: { expPerHour: solo, setAtUnixMs: NOW - 2 * DAY, ceilingShareHundredths: share(solo) },
    party: party === null ? null : { expPerHour: party, setAtUnixMs: NOW - 5 * DAY, ceilingShareHundredths: share(party) },
    soloExpCeiling: ceiling,
    ceilingShareHundredths: share(best),
    setAtUnixMs: NOW - 2 * DAY,
    here: false,
    visits: [
      { expPerHour: solo, setAtUnixMs: NOW - 2 * DAY, level: 34, job: "Hunter", party: false, ceilingShareHundredths: share(solo), thisVisit: false },
      ...(party === null
        ? []
        : [{ expPerHour: party, setAtUnixMs: NOW - 5 * DAY, level: 32, job: "Hunter", party: true, ceilingShareHundredths: share(party), thisVisit: false }]),
    ],
    ...more,
  };
}

/** One Character's bests on four maps, playing on one of them, shared on the Map
 *  Leaderboards. */
export const owlbert: Character = {
  name: "Owlbert",
  current: true,
  maps: [
    map("100000003", "Victoria Road", "Pig Beach", 61_000, 74_800, { bestOfAll: true, here: true }),
    map("100000004", "Victoria Road", "Ant Tunnel I", 48_200, null, { mobLevels: { lowest: 18, highest: 24 } }),
    map("100000005", "Hidden Street", "Wild Boar Land", 39_900, 41_000),
    map("100000006", "Victoria Road", "Golem's Temple 2", 22_400, null, { mobLevels: { lowest: 15, highest: 19 } }),
  ],
  leaderboard: { on: true, maps: 4, offered: true, status: "upToDate", problem: false, uploadedAtUnixMs: NOW - 3_600_000, rejected: [] },
};

export const view = (characters: Character[], more: Partial<BestsView> = {}) =>
  ({
    shown: true,
    characters,
    mapMarks: {},
    discord: { linkedAs: null, offered: false, linking: false, failed: null, prompt: false },
    ...more,
  }) satisfies BestsView;
