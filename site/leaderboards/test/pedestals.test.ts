// The Pedestals on the Map Leaderboards page (ADR 0024, amended): the Podiums Pedestal, from
// GET /api/podiums, and a Level Band's EXP/h Pedestal, from GET /api/bands/<band>, each three
// steps, 1st to 3rd, a step no one stands asking for yours.
import { describe, expect, it } from "vitest";
import type { MapMark, PedestalEntry, PodiumsAnswer, TrainingAnswer } from "../../entries.ts";
import { expPedestal, mapNames, podiumsPedestal, type TrainingLink } from "../leaderboards.ts";
import { OCT_3 } from "./common.ts";

const names = mapNames([
  { id: 100000000, street_name: "Made-up Road", name: "Pig Field", region: "Victoria Island" },
  { id: 100000001, street_name: "Hidden Street", name: "Quiet Field", region: "Victoria Island" },
]);

const HARBOR: MapMark = { id: "Rith", name: "Lith Harbor", picture: "https://osms.invalid/images/map_marks/Rith.png" };
const PHOTO = "https://classicrank.com/api/avatar/Ann?v=1";

const YOURS = { text: "Yours could be here", href: "/faq/#leaderboard-appear" };

describe("the Podiums Pedestal", () => {
  it("stands the three with the most podiums, 1st to 3rd, each a link to their page", () => {
    const answer: PodiumsAnswer = {
      characters: [
        { name: "Ann", podiums: 7, portrait: { photo: PHOTO, profile: null } },
        { name: "Ben", podiums: 2, portrait: { photo: null, profile: null } },
        { name: "Cal", podiums: 1, portrait: null },
      ],
    };

    const pedestal = podiumsPedestal(answer);

    expect(pedestal.steps.map((s) => s.place)).toEqual([1, 2, 3]);
    expect(pedestal.steps[0].stood).toEqual({
      name: "Ann",
      nameHref: "/character/?name=Ann",
      portrait: { photo: PHOTO },
      value: "7",
      label: "podiums",
      detail: "Top 3 in a band",
      who: null,
      map: null,
      party: false,
    });
    expect(pedestal.steps.map((s) => [s.stood?.name, s.stood?.portrait, s.stood?.value, s.stood?.label])).toEqual([
      ["Ann", { photo: PHOTO }, "7", "podiums"],
      ["Ben", "none", "2", "podiums"],
      ["Cal", "waiting", "1", "podium"],
    ]);
    expect(pedestal.portraits).toBe(true);
    expect(pedestal.empty).toEqual(YOURS);
  });

  it("leaves a step no one stands empty, and shows no portraits while the API gives none", () => {
    const pedestal = podiumsPedestal({ characters: [{ name: "Ann", podiums: 1 }] });

    expect(pedestal.steps.map((s) => s.stood?.name ?? null)).toEqual(["Ann", null, null]);
    expect(pedestal.steps[0].stood?.portrait).toBeNull();
    expect(pedestal.portraits).toBe(false);
    expect(podiumsPedestal({ characters: [] }).steps.every((s) => s.stood === null)).toBe(true);
  });
});

describe("a Level Band's EXP/h Pedestal", () => {
  function step(name: string, map: string, expPerHour: number, party = false): PedestalEntry {
    return { name, job: "Magician", level: 25, expPerHour, setAt: OCT_3, party, map, portrait: { photo: null, profile: null } };
  }

  const BAND_20: TrainingAnswer = {
    band: 20,
    levels: [20, 29],
    maps: [
      { map: "100000000", mobs: [22, 27], mark: HARBOR, entries: 2, best: null },
      { map: "100000001", mobs: [20, 24], mark: null, entries: 1, best: null },
    ],
    pedestal: {
      all: [step("Ann", "100000001", 300_000), step("Ben", "100000000", 280_000, true), step("Cal", "100000000", 150_000)],
      areas: { Rith: [step("Ben", "100000000", 280_000, true), step("Cal", "100000000", 150_000)] },
    },
  };

  const LINK: TrainingLink = { band: 20, area: null, kind: null };

  it("stands the band's three highest EXP/h in every area, each with their job, level and map", () => {
    const pedestal = expPedestal(BAND_20, names, LINK, null)!;

    expect(pedestal.title).toBe("Highest EXP/h at Lv 20–29");
    expect(pedestal.steps.map((s) => s.stood?.name)).toEqual(["Ann", "Ben", "Cal"]);
    expect(pedestal.steps[1].stood).toEqual({
      name: "Ben",
      nameHref: "/character/?name=Ben",
      portrait: "none",
      value: "280,000",
      label: "EXP/h",
      detail: null,
      who: { text: "Magician Lv 25", jobClass: "magician" },
      map: { name: "Pig Field", href: "/leaderboards/map/?map=100000000&band=20&name=Ben", mark: { name: "Lith Harbor", picture: HARBOR.picture } },
      party: true,
    });
    expect(pedestal.steps[0].stood?.map).toEqual({ name: "Quiet Field", href: "/leaderboards/map/?map=100000001&band=20&name=Ann", mark: null });
  });

  it("stands only the area's in an area, saying so, and links each map at the kind shown", () => {
    const pedestal = expPedestal(BAND_20, names, { ...LINK, area: "Rith", kind: "party" }, "Rith")!;

    expect(pedestal.title).toBe("Highest EXP/h in Lith Harbor at Lv 20–29");
    expect(pedestal.steps.map((s) => s.stood?.name ?? null)).toEqual(["Ben", "Cal", null]);
    expect(pedestal.steps[0].stood?.map?.href).toBe("/leaderboards/map/?map=100000000&band=20&name=Ben&kind=party");
    expect(pedestal.empty).toEqual(YOURS);
  });

  it("is none where no one has an entry", () => {
    expect(expPedestal({ ...BAND_20, pedestal: { all: [], areas: {} } }, names, LINK, null)).toBeNull();
    expect(expPedestal(BAND_20, names, LINK, "KerningCity")).toBeNull();
  });
});
