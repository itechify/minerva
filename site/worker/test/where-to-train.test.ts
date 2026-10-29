// Where to train lists, for each Level Band, the maps whose middle half of spawns is at its
// levels (the ceilings table) and any map with an entry in it, each with its mob levels and
// the band's #1 there, and its Map Mark (ADR 0024). The test config's table puts 100000062 in Lv 1–9, 100000060 in
// Lv 20–29, 100000061 in both Lv 20–29 and Lv 30–39, and api.ts's FIELD and RICH_FIELD in
// Lv 70–79 and Lv 100–109. (D1 starts empty for each test file.)
import { describe, expect, it } from "vitest";
import { call, entry, key, upload } from "./api";

const TWENTIES = "100000060";
const STRADDLING = "100000061";
/** The test config's made-up Map Mark, which TWENTIES and 100000062 have. */
const MADE_UP_TOWN = { id: "MadeUpTown", name: "Made-up Town", picture: "https://osms.invalid/images/map_marks/MadeUpTown.png" };
/** A map the table doesn't hold. */
const UNLISTED = "100000063";

async function read<T>(path: string): Promise<T> {
  const response = await call(path);
  if (response.status !== 200) throw new Error(`${path}: ${response.status} ${await response.text()}`);
  return response.json();
}

describe("where to train", () => {
  it("lists every Level Band with maps, with how many maps and entries each has", async () => {
    await upload("Ada", key("ada"), [entry(TWENTIES, 23, 200_000, 1_760_000_000_000), entry(UNLISTED, 45, 10_000)]);
    await upload("Bea", key("bea"), [entry(TWENTIES, 25, 250_000, 1_760_000_000_001)]);

    const response = await call("/api/bands");

    expect(response.headers.get("cache-control")).toBe("public, max-age=60");
    expect(await response.json()).toEqual({
      bands: [
        { band: 1, levels: [1, 9], maps: 1, entries: 0 },
        { band: 20, levels: [20, 29], maps: 2, entries: 2 },
        { band: 30, levels: [30, 39], maps: 1, entries: 0 },
        { band: 40, levels: [40, 49], maps: 1, entries: 1 },
        { band: 70, levels: [70, 79], maps: 1, entries: 0 },
        { band: 100, levels: [100, 109], maps: 1, entries: 0 },
      ],
    });
  });

  it("lists a band's maps with their mob levels, Map Marks, entries and #1", async () => {
    const response = await call("/api/bands/20");

    expect(response.headers.get("cache-control")).toBe("public, max-age=60");
    expect(await response.json()).toEqual({
      band: 20,
      levels: [20, 29],
      maps: [
        {
          map: TWENTIES,
          mobs: [22, 27],
          mark: MADE_UP_TOWN,
          entries: 2,
          best: { name: "Bea", job: "Magician", level: 25, expPerHour: 250_000, setAt: 1_760_000_000_001, party: false },
        },
        { map: STRADDLING, mobs: [25, 34], mark: null, entries: 0, best: null },
      ],
    });
  });

  it("lists a map with an entry in the band that the table doesn't hold, without mob levels or a mark", async () => {
    const { maps } = await read<{ maps: unknown[] }>("/api/bands/40");

    expect(maps).toEqual([
      {
        map: UNLISTED,
        mobs: null,
        mark: null,
        entries: 1,
        best: { name: "Ada", job: "Magician", level: 45, expPerHour: 10_000, setAt: 1_760_000_000_000, party: false },
      },
    ]);
  });

  it("gives a tie for #1 to whoever set it first", async () => {
    await upload("Cleo", key("cleo"), [entry(STRADDLING, 31, 400_000, 1_760_000_000_005)]);
    await upload("Dana", key("dana"), [entry(STRADDLING, 32, 400_000, 1_760_000_000_004)]);

    const { maps } = await read<{ maps: { best: { name: string } | null; entries: number }[] }>("/api/bands/30");

    expect(maps).toMatchObject([{ entries: 2, best: { name: "Dana" } }]);
  });

  it("lists a band with no maps as empty, and refuses one that isn't a band's first level", async () => {
    expect(await read<{ maps: unknown[] }>("/api/bands/150")).toEqual({ band: 150, levels: [150, 159], maps: [] });
    expect((await call("/api/bands/25")).status).toBe(400);
    expect((await call("/api/bands/twenty")).status).toBe(400);
  });
});
