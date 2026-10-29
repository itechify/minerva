// The Pedestals on the Map Leaderboards page (ADR 0024, amended), from GET /api/pedestals: the
// most podiums and the highest EXP/h, the page showing one at a time, each three steps, 1st to
// 3rd, a step no one stands asking for yours.
import { describe, expect, it } from "vitest";
import type { ExpStep, MapMark, PedestalsAnswer } from "../../entries.ts";
import { mapNames, pedestalChoices } from "../leaderboards.ts";
import { OCT_3 } from "./common.ts";

const names = mapNames([
  { id: 100000000, street_name: "Made-up Road", name: "Pig Field", region: "Victoria Island" },
  { id: 100000001, street_name: "Hidden Street", name: "Quiet Field", region: "Victoria Island" },
]);

const HARBOR: MapMark = { id: "Rith", name: "Lith Harbor", picture: "https://osms.invalid/images/map_marks/Rith.png" };
const PHOTO = "https://classicrank.com/api/avatar/Ann?v=1";

const YOURS = { text: "Yours could be here", href: "/faq/#leaderboard-appear" };

function fast(name: string, map: string, band: number, expPerHour: number, mark: MapMark | null = null, party = false): ExpStep {
  return { name, job: "Magician", level: band + 5, expPerHour, setAt: OCT_3, party, map, mark, band, portrait: { photo: null, profile: null } };
}

const ANSWER: PedestalsAnswer = {
  podiums: [
    { name: "Ann", podiums: 7, portrait: { photo: PHOTO, profile: null } },
    { name: "Ben", podiums: 2, portrait: { photo: null, profile: null } },
    { name: "Cal", podiums: 1, portrait: null },
  ],
  expPerHour: [fast("Ben", "100000000", 70, 900_000, HARBOR, true), fast("Ann", "100000001", 40, 500_000)],
};

function choices(answer = ANSWER) {
  return pedestalChoices(answer, names);
}

describe("the Pedestals", () => {
  it("offers Most podiums first, then Highest EXP/h, each saying what it ranks", () => {
    expect(choices().map((c) => [c.kind, c.label])).toEqual([
      ["podiums", "Most podiums"],
      ["exp", "Highest EXP/h"],
    ]);
    expect(choices()[0].about).toMatch(/top 3/);
    expect(choices()[1].about).toMatch(/any map, at any level/);
  });

  it("stands the three with the most podiums, 1st to 3rd, each a link to their page", () => {
    const { pedestal } = choices()[0];

    expect(pedestal.steps.map((s) => s.place)).toEqual([1, 2, 3]);
    expect(pedestal.steps[0].stood).toEqual({
      name: "Ann",
      nameHref: "/character/?name=Ann",
      portrait: { photo: PHOTO },
      value: "7",
      label: "podiums",
      who: null,
      map: null,
      party: false,
    });
    expect(pedestal.steps.map((s) => [s.stood?.name, s.stood?.portrait, s.stood?.label])).toEqual([
      ["Ann", { photo: PHOTO }, "podiums"],
      ["Ben", "none", "podiums"],
      ["Cal", "waiting", "podium"],
    ]);
    expect(pedestal.portraits).toBe(true);
  });

  it("stands the highest EXP/h, each with the map at the entry's band, its job and level", () => {
    const { pedestal } = choices()[1];

    expect(pedestal.steps[0].stood).toEqual({
      name: "Ben",
      nameHref: "/character/?name=Ben",
      portrait: "none",
      value: "900,000",
      label: "EXP/h",
      who: { job: "Magician", jobClass: "magician", level: "Lv 75" },
      map: { name: "Pig Field", href: "/leaderboards/map/?map=100000000&band=70&name=Ben", mark: { name: "Lith Harbor", picture: HARBOR.picture } },
      party: true,
    });
    expect(pedestal.steps[1].stood?.map).toEqual({ name: "Quiet Field", href: "/leaderboards/map/?map=100000001&band=40&name=Ann", mark: null });
  });

  it("leaves a step no one stands empty, asking for yours, and shows no portraits while the API gives none", () => {
    const [podiums, fastest] = choices({ podiums: [{ name: "Ann", podiums: 1 }], expPerHour: [] });

    expect(podiums.pedestal.steps.map((s) => s.stood?.name ?? null)).toEqual(["Ann", null, null]);
    expect(podiums.pedestal.steps[0].stood?.portrait).toBeNull();
    expect(podiums.pedestal.portraits).toBe(false);
    expect(podiums.pedestal.empty).toEqual(YOURS);
    expect(fastest.pedestal.steps.every((s) => s.stood === null)).toBe(true);
  });
});
