// The Pedestals on the Map Leaderboards page (ADR 0024, amended): the three Characters with
// the most podiums, places in a band's top 3 counted as a Character's page counts them; and the
// three with the highest EXP/h on any map at any level, one per Character. (D1 starts empty
// for each test file; each Podiums test opts its Characters out after, so the next starts from
// none.)
import { describe, expect, it } from "vitest";
import type { PedestalsAnswer } from "../../entries";
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
  return (await read<PedestalsAnswer>("/api/pedestals")).podiums.map(({ name, podiums }) => ({ name, podiums }));
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
    expect((await read<PedestalsAnswer>("/api/pedestals")).podiums).toEqual([]);
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

    const answer = await read<PedestalsAnswer>("/api/pedestals");

    expect(answer.podiums).toEqual([
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

describe("the EXP/h Pedestal", () => {
  async function fastest() {
    return (await read<PedestalsAnswer>("/api/pedestals")).expPerHour;
  }

  /** A pedestal's steps as who, where and how much. */
  function steps(on: PedestalsAnswer["expPerHour"]) {
    return on.map((s) => [s.name, s.map, s.band, s.expPerHour, s.party]);
  }

  it("is empty before anyone has an entry", async () => {
    expect(await fastest()).toEqual([]);
  });

  it("stands the three highest EXP/h on any map at any level, each Character by their best entry", async () => {
    knownOnClassicRank("Pia");
    await uploadAll({
      // Pia's best is her Lv 70s Solo Best, not her other entries.
      Pia: [entry(MARKED, 25, 250_000), entry(FIELD, 72, 400_000), partyEntry(MARKED, 26, 300_000)],
      Quin: [entry(STRADDLING, 31, 200_000), partyEntry(MARKED, 26, 380_000)],
      Rex: [entry(NO_CEILING, 15, 150_000)],
      Sam: [entry(MARKED, 28, 100_000)],
    });

    const on = await fastest();

    expect(steps(on)).toEqual([
      ["Pia", FIELD, 70, 400_000, false],
      ["Quin", MARKED, 20, 380_000, true],
      ["Rex", NO_CEILING, 10, 150_000, false],
    ]);
    expect(on[1]).toEqual({
      name: "Quin",
      job: "Magician",
      level: 26,
      expPerHour: 380_000,
      setAt: 1_760_000_000_000,
      party: true,
      map: MARKED,
      mark: { id: "MadeUpTown", name: "Made-up Town", picture: "https://osms.invalid/images/map_marks/MadeUpTown.png" },
      band: 20,
      portrait: { photo: null, profile: null },
    });
    expect(on[0].portrait).toEqual({ photo: photoOf("Pia"), profile: profileOf("Pia") });
    expect(on[0].mark).toBeNull();
  });

  it("gives a tie to whoever set it first", async () => {
    await uploadAll({
      Tia: [entry(STRADDLING, 31, 500_000, 1_760_000_000_005)],
      Uma: [entry(STRADDLING, 32, 500_000, 1_760_000_000_004)],
    });

    expect(steps(await fastest()).map(([name]) => name)).toEqual(["Uma", "Tia", "Pia"]);
  });
});
