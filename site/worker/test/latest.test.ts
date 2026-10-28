// Latest lists the newest entries on every map, by when their Visit set them, and marks the
// ones that took a top spot from another Character, for as long as that entry stands
// (ADR 0024). (D1 starts empty for each test file, so only this file's uploads are here.)
import { describe, expect, it } from "vitest";
import type { LatestAnswer, LatestEntry } from "../../entries";
import { admin, call, entry, key, optOut, upload } from "./api";

/** Each test's maps, of their own, so no other test's entries are in their bands. */
const LISTED = "100000070";
const TAKEN = "100000071";
const DELETED = "100000072";
const LEFT = "100000073";
const BEATEN = "100000074";
/** A map the test config's ceilings table gives a Map Mark. */
const MARKED = "100000060";

/** A day in ms, to date each test's Visits after the last's: Latest lists the newest first. */
const DAY = 86_400_000;
const START = 1_760_000_000_000;

async function latest(): Promise<LatestEntry[]> {
  const response = await call("/api/latest");
  expect(response.headers.get("cache-control")).toBe("public, max-age=60");
  return ((await response.json()) as LatestAnswer).entries;
}

/** The newest entry `name` has on `map`, as Latest lists it. */
async function latestOf(name: string, map: string): Promise<LatestEntry | undefined> {
  return (await latest()).find((e) => e.name === name && e.map === map);
}

describe("latest", () => {
  it("lists the newest entries on every map, newest first, by when their Visit set them", async () => {
    await upload("Ada", key("ada"), [entry(LISTED, 5, 1_000, START + 2), entry(LISTED, 25, 3_000, START)]);
    await upload("Bea", key("bea"), [entry(LISTED, 15, 2_000, START + 1)]);

    const entries = await latest();

    expect(entries.map((e) => [e.name, e.level])).toEqual([
      ["Ada", 5],
      ["Bea", 15],
      ["Ada", 25],
    ]);
    expect(entries[1]).toEqual({
      name: "Bea",
      job: "Magician",
      level: 15,
      expPerHour: 2_000,
      setAt: START + 1,
      map: LISTED,
      mark: null,
      band: 10,
      party: false,
      took: null,
    });
  });

  it("lists no more than 10", async () => {
    const maps = Array.from({ length: 25 }, (_, i) => entry(String(100_000_200 + i), 50, 5_000, START + DAY + i));
    await upload("Cleo", key("cleo"), maps);

    const entries = await latest();

    expect(entries).toHaveLength(10);
    expect(entries[0].setAt).toBe(START + DAY + 24);
    expect(entries[9].setAt).toBe(START + DAY + 15);
  });

  it("marks an entry that took a top spot from another Character, with whose it was", async () => {
    await upload("Egbert", key("egbert"), [entry(TAKEN, 23, 80_000, START + 2 * DAY)]);
    await upload("Hakuo", key("hakuo"), [entry(TAKEN, 24, 90_000, START + 2 * DAY + 1)]);

    expect((await latestOf("Hakuo", TAKEN))?.took).toEqual({ from: "Egbert", fromExpPerHour: 80_000 });
    expect((await latestOf("Egbert", TAKEN))?.took).toBeNull();
  });

  it("keeps the mark when another Character takes the spot in turn", async () => {
    await upload("Ines", key("ines"), [entry(TAKEN, 25, 100_000, START + 2 * DAY + 2)]);

    expect((await latestOf("Ines", TAKEN))?.took).toEqual({ from: "Hakuo", fromExpPerHour: 90_000 });
    expect((await latestOf("Hakuo", TAKEN))?.took).toEqual({ from: "Egbert", fromExpPerHour: 80_000 });
  });

  it("drops the name of a Character who opts out from the spots taken off them", async () => {
    await optOut("Egbert", key("egbert"));

    expect((await latestOf("Hakuo", TAKEN))?.took).toEqual({ from: null, fromExpPerHour: 80_000 });
  });

  it("drops the mark when the Character beats their own best there", async () => {
    await upload("Jona", key("jona"), [entry(BEATEN, 33, 80_000, START + 3 * DAY)]);
    await upload("Kira", key("kira"), [entry(BEATEN, 34, 90_000, START + 3 * DAY + 1)]);

    await upload("Kira", key("kira"), [entry(BEATEN, 35, 95_000, START + 3 * DAY + 2)]);

    expect(await latestOf("Kira", BEATEN)).toMatchObject({ expPerHour: 95_000, took: null });
  });

  // The same Visit coming back, after the mark's entry went, without taking the spot again
  // (another Character holds it by then), carries no mark.
  it("drops the mark when the maintainer deletes the entry", async () => {
    await upload("Lena", key("lena"), [entry(DELETED, 43, 80_000, START + 4 * DAY)]);
    await upload("Mika", key("mika"), [entry(DELETED, 44, 90_000, START + 4 * DAY + 1)]);
    await admin(`/entries/Mika/${DELETED}/40`, { method: "DELETE" });
    await upload("Nils", key("nils"), [entry(DELETED, 45, 100_000, START + 4 * DAY + 2)]);
    await admin(`/removed/Mika/${DELETED}/40/90000`, { method: "DELETE" });

    await upload("Mika", key("mika"), [entry(DELETED, 44, 90_000, START + 4 * DAY + 1)]);

    expect(await latestOf("Mika", DELETED)).toMatchObject({ expPerHour: 90_000, took: null });
  });

  it("drops the mark when the Character opts out", async () => {
    await upload("Olga", key("olga"), [entry(LEFT, 53, 80_000, START + 5 * DAY)]);
    await upload("Pia", key("pia"), [entry(LEFT, 54, 90_000, START + 5 * DAY + 1)]);
    await optOut("Pia", key("pia"));
    await upload("Rhea", key("rhea"), [entry(LEFT, 55, 100_000, START + 5 * DAY + 2)]);

    await upload("Pia", key("pia"), [entry(LEFT, 54, 90_000, START + 5 * DAY + 1)]);

    expect(await latestOf("Pia", LEFT)).toMatchObject({ expPerHour: 90_000, took: null });
  });

  it("gives each entry its map's Map Mark, as Where to train does", async () => {
    await upload("Sana", key("sana"), [entry(MARKED, 23, 10_000, START + 6 * DAY)]);

    expect((await latestOf("Sana", MARKED))?.mark).toEqual({
      id: "MadeUpTown",
      name: "Made-up Town",
      picture: "https://osms.invalid/images/map_marks/MadeUpTown.png",
    });
  });
});
