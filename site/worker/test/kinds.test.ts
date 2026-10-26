// A Character's Solo Best and Party Best on a map in a Level Band are entries of their own
// (ADR 0015, amended): a Map Leaderboard ranks each Character once by their higher entry and
// marks the party ones, and can be filtered to the solo or the party entries alone. An entry
// with no kind, as every upload before this, is solo. (D1 starts empty for each test file.)
import { describe, expect, it } from "vitest";
import { admin, band, call, entry, key, names, partyEntry, report, upload } from "./api";

// Maps of their own, so no other test's entries are in their bands.
const MIXED = "100000100";
const TWINS = "100000101";
const CAPPED = "100000000";

describe("solo and party entries", () => {
  it("takes a Solo Best and a Party Best for one map and Level Band", async () => {
    const response = await upload("Ada", key("ada"), [entry(MIXED, 23, 40_000), partyEntry(MIXED, 24, 90_000)]);

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ accepted: 2, rejected: [] });
  });

  it("refuses two entries of one kind for one map and Level Band", async () => {
    const solo = await upload("Bea", key("bea"), [entry(MIXED, 23, 40_000), entry(MIXED, 25, 50_000)]);
    const party = await upload("Bea", key("bea"), [partyEntry(MIXED, 23, 40_000), partyEntry(MIXED, 25, 50_000)]);

    expect([solo.status, party.status]).toEqual([400, 400]);
  });

  it("refuses a kind that isn't true or false", async () => {
    const response = await upload("Bea", key("bea"), [{ ...entry(MIXED, 23, 40_000), party: "yes" } as never]);

    expect(response.status).toBe(400);
  });

  it("ranks each Character once, by their higher entry, and marks a party one", async () => {
    await upload("Bea", key("bea"), [entry(MIXED, 23, 40_000)]);
    await upload("Cleo", key("cleo"), [entry(MIXED, 22, 70_000), partyEntry(MIXED, 22, 60_000)]);

    const read = await band(MIXED, 20);

    expect(read.entries.map((e) => [e.rank, e.name, e.expPerHour, e.party])).toEqual([
      [1, "Ada", 90_000, true],
      [2, "Cleo", 70_000, false],
      [3, "Bea", 40_000, false],
    ]);
    expect(read.total).toBe(3);
  });

  it("gives a tie between a Character's two entries to the Solo Best", async () => {
    await upload("Dora", key("dora"), [entry(TWINS, 33, 50_000), partyEntry(TWINS, 33, 50_000)]);

    expect((await band(TWINS, 30)).entries.map((e) => e.party)).toEqual([false]);
  });

  it("ranks the solo entries alone, or the party ones", async () => {
    expect((await band(MIXED, 20, undefined, "solo")).entries.map((e) => [e.rank, e.name, e.expPerHour])).toEqual([
      [1, "Cleo", 70_000],
      [2, "Ada", 40_000],
      [3, "Bea", 40_000],
    ]);
    const party = await band(MIXED, 20, undefined, "party");
    expect(party.entries.map((e) => [e.rank, e.name, e.expPerHour, e.party])).toEqual([
      [1, "Ada", 90_000, true],
      [2, "Cleo", 60_000, true],
    ]);
    expect(party.total).toBe(2);
  });

  it("finds a Character's entry and rank on the board being read", async () => {
    expect((await band(MIXED, 20, "cleo", "party")).character).toMatchObject({ rank: 2, expPerHour: 60_000, party: true });
    expect((await band(MIXED, 20, "cleo", "solo")).character).toMatchObject({ rank: 1, expPerHour: 70_000, party: false });
    expect((await band(MIXED, 20, "bea", "party")).character).toBeNull();
  });

  it("reads an upload with no kind as solo", async () => {
    await upload("Emil", key("emil"), [entry(TWINS, 44, 10_000)]);

    expect(await names(TWINS, 40, "solo")).toEqual(["Emil"]);
    expect(await names(TWINS, 40, "party")).toEqual([]);
  });

  it("replaces both kinds at each upload", async () => {
    await upload("Fritz", key("fritz"), [entry(TWINS, 55, 10_000), partyEntry(TWINS, 55, 20_000)]);

    await upload("Fritz", key("fritz"), [entry(TWINS, 55, 10_000)]);

    expect(await names(TWINS, 50, "party")).toEqual([]);
    expect(await names(TWINS, 50, "solo")).toEqual(["Fritz"]);
  });

  it("refuses a kind to read that isn't solo or party", async () => {
    expect((await call(`/api/maps/${MIXED}/bands/20?kind=both`)).status).toBe(400);
  });

  it("holds an entry of either kind to the cap", async () => {
    const response = await upload("Greta", key("greta"), [entry(CAPPED, 72, 500_001), partyEntry(CAPPED, 72, 500_001)]);

    expect(await response.json()).toEqual({
      accepted: 0,
      rejected: [
        { map: CAPPED, band: 70, reason: "over-cap", cap: 500_000 },
        { map: CAPPED, band: 70, party: true, reason: "over-cap", cap: 500_000 },
      ],
    });
  });
});

describe("moderation", () => {
  const MODERATED = "100000112";
  const taken = async () =>
    (await (await admin("/removed")).json()) as { removed: { name: string; band: number; expPerHour: number; party: boolean }[] };

  it("reports the entry of the kind named, solo with none", async () => {
    await upload("Mia", key("mia"), [entry(MODERATED, 25, 30_000), partyEntry(MODERATED, 25, 50_000)]);

    expect((await report({ name: "Mia", map: MODERATED, band: 20, party: true })).status).toBe(201);
    expect((await report({ name: "Mia", map: MODERATED, band: 20 })).status).toBe(201);

    const reports = (await (await admin("/reports")).json()) as { reports: { name: string; expPerHour: number; party: boolean }[] };
    expect(reports.reports.map((r) => [r.expPerHour, r.party]).sort()).toEqual([
      [30_000, false],
      [50_000, true],
    ]);
  });

  it("refuses a report of a kind that isn't true or false", async () => {
    expect((await report({ name: "Mia", map: MODERATED, band: 20, party: "party" })).status).toBe(400);
  });

  it("deletes the entry of the kind named, and keeps the other", async () => {
    const response = await admin(`/entries/Mia/${MODERATED}/20?kind=party`, { method: "DELETE" });

    expect(response.status).toBe(204);
    expect(await names(MODERATED, 20, "party")).toEqual([]);
    expect(await names(MODERATED, 20, "solo")).toEqual(["Mia"]);
    expect((await taken()).removed).toContainEqual({ name: "Mia", map: MODERATED, band: 20, expPerHour: 50_000, party: true });
  });

  it("keeps a deleted party entry off, and takes the same figure as a solo entry", async () => {
    const again = await upload("Mia", key("mia"), [entry(MODERATED, 25, 50_000), partyEntry(MODERATED, 25, 50_000)]);

    expect(await again.json()).toEqual({
      accepted: 1,
      rejected: [{ map: MODERATED, band: 20, party: true, reason: "removed" }],
    });
  });

  it("restores a party entry by its kind", async () => {
    expect((await admin(`/removed/Mia/${MODERATED}/20/50000`, { method: "DELETE" })).status).toBe(404);

    expect((await admin(`/removed/Mia/${MODERATED}/20/50000?kind=party`, { method: "DELETE" })).status).toBe(204);

    await upload("Mia", key("mia"), [entry(MODERATED, 25, 30_000), partyEntry(MODERATED, 25, 50_000)]);
    expect(await names(MODERATED, 20, "party")).toEqual(["Mia"]);
  });

  it("deletes both entries of a band with no kind named", async () => {
    const response = await admin(`/entries/Mia/${MODERATED}/20`, { method: "DELETE" });

    expect(response.status).toBe(204);
    expect(await names(MODERATED, 20)).toEqual([]);
    expect(await names(MODERATED, 20, "party")).toEqual([]);
  });

  it("refuses a kind to delete that isn't solo or party", async () => {
    expect((await admin(`/entries/Mia/${MODERATED}/20?kind=both`, { method: "DELETE" })).status).toBe(400);
  });
});

describe("the pages' reads", () => {
  const PAGES = "100000110";
  const START = 1_760_000_000_000;

  it("counts a map's entries once per Character, and a band's of each kind", async () => {
    await upload("Hana", key("hana"), [entry(PAGES, 25, 30_000, START), partyEntry(PAGES, 25, 50_000, START + 1)]);
    await upload("Ivo", key("ivo"), [partyEntry(PAGES, 22, 40_000, START + 2), entry(PAGES, 3, 1_000, START + 3)]);

    const maps = (await (await call("/api/maps")).json()) as { maps: { map: string; characters: number; entries: number }[] };
    const map = await (await call(`/api/maps/${PAGES}`)).json();

    expect(maps.maps.find((m) => m.map === PAGES)).toEqual({ map: PAGES, characters: 2, entries: 3 });
    expect(map).toEqual({
      map: PAGES,
      bands: [
        { band: 1, levels: [1, 9], entries: 1, solo: 1, party: 0 },
        { band: 20, levels: [20, 29], entries: 2, solo: 1, party: 2 },
      ],
    });
  });

  it("lists a band's maps with the #1 and the entries of the kind asked for", async () => {
    // The made-up table puts 100000060 in Lv 20-29.
    const TRAINED = "100000060";
    await upload("Jil", key("jil"), [entry(TRAINED, 25, 30_000, START), partyEntry(TRAINED, 25, 50_000, START + 1)]);
    await upload("Kai", key("kai"), [entry(TRAINED, 26, 40_000, START + 2)]);
    const best = async (kind = "") => {
      const { maps } = (await (await call(`/api/bands/20${kind}`)).json()) as {
        maps: { map: string; entries: number; best: { name: string; expPerHour: number; party: boolean } | null }[];
      };
      return maps.find((m) => m.map === TRAINED);
    };

    expect(await best()).toMatchObject({ entries: 2, best: { name: "Jil", expPerHour: 50_000, party: true } });
    expect(await best("?kind=solo")).toMatchObject({ entries: 2, best: { name: "Kai", expPerHour: 40_000, party: false } });
    expect(await best("?kind=party")).toMatchObject({ entries: 1, best: { name: "Jil", expPerHour: 50_000, party: true } });
    expect((await call("/api/bands/20?kind=both")).status).toBe(400);
  });

  it("lists the newest entries with their kind, or only a kind's", async () => {
    const LISTED = "100000111";
    await upload("Lou", key("lou"), [entry(LISTED, 25, 30_000, START + 10), partyEntry(LISTED, 35, 50_000, START + 11)]);
    const latest = async (kind = "") =>
      ((await (await call(`/api/latest${kind}`)).json()) as { entries: { name: string; level: number; party: boolean }[] }).entries;

    // Each entry is listed, a Character's two on one map and band apart.
    expect((await latest()).slice(0, 2).map((e) => [e.name, e.level, e.party])).toEqual([
      ["Lou", 35, true],
      ["Lou", 25, false],
    ]);
    const solo = await latest("?kind=solo");
    expect(solo.every((e) => !e.party)).toBe(true);
    expect(solo[0]).toMatchObject({ name: "Lou", level: 25 });
    const party = await latest("?kind=party");
    expect(party.every((e) => e.party)).toBe(true);
    expect(party[0]).toMatchObject({ name: "Lou", level: 35 });
    expect((await call("/api/latest?kind=both")).status).toBe(400);
  });
});
