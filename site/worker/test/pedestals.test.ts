// The Pedestals on the Map Leaderboards page (ADR 0024, amended): the three Characters with
// the most Podiums, places in a band's top 3 counted as a Character's page counts them; and in
// each Level Band's Where to train, its three highest EXP/h, one per Character, in every area
// and in each by its Map Mark. (D1 starts empty for each test file; each Podiums test opts its
// Characters out after, so the next starts from none.)
import { describe, expect, it } from "vitest";
import type { PodiumsAnswer, TrainingAnswer } from "../../entries";
import { call, entry, FIELD, key, NO_CEILING, optOut, partyEntry, upload } from "./api";
import { knownOnClassicRank, photoOf, profileOf } from "./classicrank";

/** The test config's map in Lv 20–29 with the made-up Map Mark, and its map in Lv 20–29 and
 *  Lv 30–39 without one. */
const MARKED = "100000060";
const STRADDLING = "100000061";

async function read<T>(path: string): Promise<T> {
  const response = await call(path);
  if (response.status !== 200) throw new Error(`${path}: ${response.status} ${await response.text()}`);
  expect(response.headers.get("cache-control")).toBe("public, max-age=60");
  return response.json();
}

async function podiums(): Promise<{ name: string; podiums: number }[]> {
  return (await read<PodiumsAnswer>("/api/podiums")).characters.map(({ name, podiums }) => ({ name, podiums }));
}

/** Upload each Character's entries with a key of their own. */
async function uploadAll(entries: Record<string, Parameters<typeof upload>[2]>) {
  for (const [name, theirs] of Object.entries(entries)) {
    const response = await upload(name, key(name), theirs);
    if (response.status !== 200) throw new Error(`upload ${name}: ${response.status} ${await response.text()}`);
  }
}

async function optOutAll(names: string[]) {
  for (const name of names) await optOut(name, key(name));
}

describe("the Podiums Pedestal", () => {
  it("is empty before anyone has an entry", async () => {
    expect(await read<PodiumsAnswer>("/api/podiums")).toEqual({ characters: [] });
  });

  it("ranks the three Characters with the most places in a band's top 3, a Solo Best's and a Party Best's each counting", async () => {
    knownOnClassicRank("Ann");
    await uploadAll({
      // #1 on FIELD in Lv 70–79 among the Solo Bests and among the Party Bests, and #1 on MARKED.
      Ann: [entry(FIELD, 72, 90_000), partyEntry(FIELD, 72, 300_000), entry(MARKED, 25, 250_000)],
      Ben: [entry(FIELD, 73, 80_000), entry(MARKED, 26, 200_000)],
      Cal: [entry(FIELD, 74, 70_000)],
      Dee: [entry(FIELD, 75, 60_000)],
    });

    const answer = await read<PodiumsAnswer>("/api/podiums");

    expect(answer.characters).toEqual([
      { name: "Ann", podiums: 3, portrait: { photo: photoOf("Ann"), profile: profileOf("Ann") } },
      { name: "Ben", podiums: 2, portrait: { photo: null, profile: null } },
      { name: "Cal", podiums: 1, portrait: { photo: null, profile: null } },
    ]);
    await optOutAll(["Ann", "Ben", "Cal", "Dee"]);
  });

  it("lists fewer than three while fewer have a podium", async () => {
    await uploadAll({ Eli: [entry(NO_CEILING, 15, 10_000)] });

    expect(await podiums()).toEqual([{ name: "Eli", podiums: 1 }]);
    await optOutAll(["Eli"]);
  });

  it("gives a tie to whoever holds more #1s", async () => {
    await uploadAll({
      Hal: [entry(NO_CEILING, 15, 300_000), entry(NO_CEILING, 25, 300_000)],
      Ivy: [entry(NO_CEILING, 16, 200_000), entry(NO_CEILING, 26, 200_000)],
      Jon: [entry(NO_CEILING, 17, 100_000), entry(NO_CEILING, 35, 300_000)],
    });

    // Each has two podiums: Hal two #1s, Jon one, and Ivy none, for all her #2s.
    expect(await podiums()).toEqual([
      { name: "Hal", podiums: 2 },
      { name: "Jon", podiums: 2 },
      { name: "Ivy", podiums: 2 },
    ]);
    await optOutAll(["Hal", "Ivy", "Jon"]);
  });

  it("then to whoever holds more #2s, then to whoever set their first podium entry first", async () => {
    await uploadAll({
      Lee: [45, 55, 65, 75, 85].map((level) => entry(NO_CEILING, level, 300_000, 1_760_000_000_000)),
      Kai: [entry(NO_CEILING, 46, 200_000, 1_760_000_000_000), entry(NO_CEILING, 56, 200_000, 1_760_000_000_000)],
      // A #2 and a #3, set before anyone else's.
      Mo: [entry(NO_CEILING, 66, 200_000, 1_740_000_000_000), entry(NO_CEILING, 47, 100_000, 1_740_000_000_000)],
      Ola: [entry(NO_CEILING, 76, 200_000, 1_750_000_000_000), entry(NO_CEILING, 86, 200_000, 1_750_000_000_000)],
    });

    expect(await podiums()).toEqual([
      { name: "Lee", podiums: 5 },
      { name: "Ola", podiums: 2 },
      { name: "Kai", podiums: 2 },
    ]);
    await optOutAll(["Lee", "Kai", "Mo", "Ola"]);
  });
});

describe("a Level Band's EXP/h Pedestal", () => {
  async function pedestal(band: number, kind?: "solo" | "party"): Promise<TrainingAnswer["pedestal"]> {
    return (await read<TrainingAnswer>(`/api/bands/${band}${kind === undefined ? "" : `?kind=${kind}`}`)).pedestal;
  }

  /** A pedestal's steps as who, where and how much. */
  function steps(on: TrainingAnswer["pedestal"]["all"]) {
    return on.map((s) => [s.name, s.map, s.expPerHour, s.party]);
  }

  it("is empty in a band with no entries", async () => {
    expect(await pedestal(1)).toEqual({ all: [], areas: {} });
  });

  it("ranks the band's three highest EXP/h, one per Character, in every area and in each by its Map Mark", async () => {
    knownOnClassicRank("Pia");
    await uploadAll({
      Pia: [entry(MARKED, 25, 250_000), entry(STRADDLING, 25, 300_000)],
      Quin: [entry(MARKED, 26, 200_000), partyEntry(MARKED, 26, 280_000)],
      Rex: [entry(NO_CEILING, 27, 150_000)],
      Sam: [entry(MARKED, 28, 100_000)],
    });

    const both = await pedestal(20);

    expect(both.all[0]).toEqual({
      name: "Pia",
      job: "Magician",
      level: 25,
      expPerHour: 300_000,
      setAt: 1_760_000_000_000,
      party: false,
      map: STRADDLING,
      portrait: { photo: photoOf("Pia"), profile: profileOf("Pia") },
    });
    expect(steps(both.all)).toEqual([
      ["Pia", STRADDLING, 300_000, false],
      ["Quin", MARKED, 280_000, true],
      ["Rex", NO_CEILING, 150_000, false],
    ]);
    // A map without a Map Mark counts only in every area's.
    expect(Object.keys(both.areas)).toEqual(["MadeUpTown"]);
    expect(steps(both.areas.MadeUpTown)).toEqual([
      ["Quin", MARKED, 280_000, true],
      ["Pia", MARKED, 250_000, false],
      ["Sam", MARKED, 100_000, false],
    ]);
  });

  it("ranks only the kind asked for", async () => {
    const solo = await pedestal(20, "solo");
    expect(steps(solo.all)).toEqual([
      ["Pia", STRADDLING, 300_000, false],
      ["Quin", MARKED, 200_000, false],
      ["Rex", NO_CEILING, 150_000, false],
    ]);
    expect(steps(solo.areas.MadeUpTown)).toEqual([
      ["Pia", MARKED, 250_000, false],
      ["Quin", MARKED, 200_000, false],
      ["Sam", MARKED, 100_000, false],
    ]);

    const party = await pedestal(20, "party");
    expect(steps(party.all)).toEqual([["Quin", MARKED, 280_000, true]]);
    expect(Object.keys(party.areas)).toEqual(["MadeUpTown"]);
  });

  it("gives a tie to whoever set it first", async () => {
    await uploadAll({
      Tia: [entry(STRADDLING, 31, 400_000, 1_760_000_000_005)],
      Uma: [entry(STRADDLING, 32, 400_000, 1_760_000_000_004)],
    });

    expect(steps((await pedestal(30)).all).map(([name]) => name)).toEqual(["Uma", "Tia"]);
  });
});
