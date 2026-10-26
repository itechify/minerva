// The Map Leaderboards page's search says how many entries each map has, and a map's page
// lists its Level Bands.
// (D1 starts empty for each test file, so only this file's uploads are here.)
import { describe, expect, it } from "vitest";
import { call, entry, key, upload } from "./api";

const BUSY = "100000020";
const QUIET = "100000021";

describe("maps", () => {
  it("lists the maps with entries, the most first", async () => {
    await upload("Ada", key("ada"), [entry(BUSY, 12, 10_000), entry(BUSY, 25, 20_000), entry(QUIET, 12, 9_000)]);
    await upload("Bea", key("bea"), [entry(BUSY, 14, 11_000)]);

    const response = await call("/api/maps");

    expect(response.status).toBe(200);
    expect(response.headers.get("cache-control")).toBe("public, max-age=60");
    expect(await response.json()).toEqual({
      maps: [
        { map: BUSY, characters: 2, entries: 3 },
        { map: QUIET, characters: 1, entries: 1 },
      ],
    });
  });

  it("lists a map's Level Bands with how many entries each has", async () => {
    await upload("Cleo", key("cleo"), [entry(QUIET, 7, 5_000), entry(QUIET, 31, 30_000)]);

    const response = await call(`/api/maps/${QUIET}`);

    expect(await response.json()).toEqual({
      map: QUIET,
      bands: [
        { band: 1, levels: [1, 9], entries: 1, solo: 1, party: 0 },
        { band: 10, levels: [10, 19], entries: 1, solo: 1, party: 0 },
        { band: 30, levels: [30, 39], entries: 1, solo: 1, party: 0 },
      ],
    });
  });

  it("lists every map with entries, not only 50, so the page's search can say how many each has", async () => {
    // 60 maps with one entry each, and one with four entries from a single Character, which
    // goes first: the maps are ranked by the entries the page shows.
    const many = Array.from({ length: 60 }, (_, i) => entry(String(100_000_100 + i), 15, 1_000));
    await upload("Dora", key("dora"), many);
    await upload("Finn", key("finn"), [entry("100000099", 5, 1_000), entry("100000099", 15, 1_000), entry("100000099", 25, 1_000), entry("100000099", 35, 1_000)]);

    const { maps } = (await (await call("/api/maps")).json()) as { maps: { map: string; characters: number; entries: number }[] };

    expect(maps).toHaveLength(63);
    expect(maps.slice(0, 3)).toEqual([
      { map: "100000099", characters: 1, entries: 4 },
      { map: BUSY, characters: 2, entries: 3 },
      { map: QUIET, characters: 2, entries: 3 },
    ]);
  });

  it("answers an unknown API path with a 404", async () => {
    const response = await call("/api/nothing-here");

    expect(response.status).toBe(404);
    expect(await response.json()).toEqual({ error: "not-found" });
    expect(response.headers.get("cache-control")).toBe("no-store");
  });
});
