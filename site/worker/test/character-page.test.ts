// A Character's page reads every entry of theirs on the Map Leaderboards, each with its rank
// in its band, and the map's mob levels and Map Mark (ADR 0027). A name with no entries reads
// the same whatever the reason. (D1 starts empty for each test file, so only this file's
// uploads are here.)
import { describe, expect, it } from "vitest";
import type { CharacterAnswer } from "../../entries";
import { admin, call, entry, FIELD, key, NO_CEILING, optOut, partyEntry, upload } from "./api";

/** A map the test config's ceilings table gives a Map Mark and mob levels. */
const MARKED = "100000060";

async function page(name: string): Promise<CharacterAnswer> {
  const response = await call(`/api/characters/${encodeURIComponent(name)}`);
  if (response.status !== 200) throw new Error(`character read: ${response.status} ${await response.text()}`);
  expect(response.headers.get("cache-control")).toBe("public, max-age=60");
  return response.json();
}

describe("a Character's page", () => {
  it("lists every entry of theirs with its rank and how many the band has, highest band first", async () => {
    await upload("Rhea", key("rhea"), [entry(MARKED, 25, 90_000), entry(MARKED, 15, 40_000), entry(FIELD, 72, 50_000)]);
    await upload("Sven", key("sven"), [entry(MARKED, 24, 120_000), entry(FIELD, 75, 20_000)]);
    await upload("Tove", key("tove"), [entry(MARKED, 28, 60_000)]);

    const read = await page("Rhea");

    expect(read.name).toBe("Rhea");
    expect(read.entries.map((e) => [e.map, e.band, e.rank, e.total])).toEqual([
      [FIELD, 70, 1, 2],
      [MARKED, 20, 2, 3],
      [MARKED, 10, 1, 1],
    ]);
    expect(read.entries[1]).toEqual({
      map: MARKED,
      band: 20,
      rank: 2,
      total: 3,
      job: "Magician",
      level: 25,
      expPerHour: 90_000,
      setAt: 1_760_000_000_000,
      party: false,
      mobs: [22, 27],
      mark: { id: "MadeUpTown", name: "Made-up Town", picture: "https://osms.invalid/images/map_marks/MadeUpTown.png" },
    });
  });

  it("lists a Solo Best and a Party Best on the same map as entries of their own, each ranked among its kind", async () => {
    await upload("Yara", key("yara"), [entry(MARKED, 55, 50_000), partyEntry(MARKED, 55, 130_000)]);
    await upload("Zed", key("zed"), [entry(MARKED, 56, 70_000), partyEntry(MARKED, 56, 140_000)]);
    await upload("Zoe", key("zoe"), [entry(MARKED, 57, 60_000)]);

    const read = await page("Yara");

    expect(read.entries.map((e) => [e.band, e.party, e.rank, e.total, e.expPerHour])).toEqual([
      [50, true, 2, 2, 130_000],
      [50, false, 3, 3, 50_000],
    ]);
  });

  it("reads a name whatever its case, and gives it as last uploaded", async () => {
    await upload("Ulla", key("ulla"), [entry(FIELD, 71, 10_000)]);

    expect((await page("ULLA")).name).toBe("Ulla");
  });

  it("gives a map the ceilings table doesn't hold no mob levels or Map Mark", async () => {
    await upload("Vera", key("vera"), [entry(NO_CEILING, 40, 10_000)]);

    expect((await page("Vera")).entries[0]).toMatchObject({ map: NO_CEILING, mobs: null, mark: null });
  });

  it("reads a name never uploaded, one opted out, and one whose entries were taken down alike", async () => {
    await upload("Wim", key("wim"), [entry(FIELD, 73, 10_000)]);
    await optOut("Wim", key("wim"));
    await upload("Xia", key("xia"), [entry(FIELD, 74, 10_000)]);
    expect((await admin(`/entries/Xia/${FIELD}/70`, { method: "DELETE" })).status).toBe(204);

    const answers = await Promise.all(["Nobody", "Wim", "Xia"].map((name) => call(`/api/characters/${name}`)));

    for (const answer of answers) {
      expect(answer.status).toBe(404);
      expect(answer.headers.get("cache-control")).toBe("no-store");
      expect(await answer.json()).toEqual({ error: "not-found" });
    }
  });

  it("refuses a name that can't be a Character's", async () => {
    expect((await call("/api/characters/not-a-name")).status).toBe(400);
  });
});
