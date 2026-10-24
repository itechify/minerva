// An entry with EXP/h above 5× the map's solo ceiling is rejected; a map with no ceiling
// falls back to one absolute cap, 25,000,000 EXP/h. The rest of the upload is taken.
import { describe, expect, it } from "vitest";
import type { Ceiling } from "../ceilings";
import ceilings from "../osms-ceilings.json";
import { FIELD, NO_CEILING, RICH_FIELD, band, entry, key, names, upload } from "./api";

describe("the cap", () => {
  it("takes an entry at 5× the map's solo ceiling", async () => {
    const response = await upload("Paula", key("paula"), [entry(FIELD, 71, 500_000)]);

    expect(await response.json()).toEqual({ accepted: 1, rejected: [] });
    expect(await names(FIELD, 70)).toContain("Paula");
  });

  it("rejects an entry above 5× the map's solo ceiling, and takes the rest", async () => {
    const response = await upload("Quirin", key("quirin"), [entry(FIELD, 72, 500_001), entry(RICH_FIELD, 72, 3_000_000)]);

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({
      accepted: 1,
      rejected: [{ map: FIELD, band: 70, reason: "over-cap", cap: 500_000 }],
    });
    expect(await names(FIELD, 70)).not.toContain("Quirin");
    expect(await names(RICH_FIELD, 70)).toContain("Quirin");
  });

  it("caps a map with no solo ceiling at 25,000,000 EXP/h", async () => {
    await upload("Rosa", key("rosa"), [entry(NO_CEILING, 81, 25_000_000)]);
    const response = await upload("Sven", key("sven"), [entry(NO_CEILING, 81, 25_000_001)]);

    expect(await response.json()).toEqual({
      accepted: 0,
      rejected: [{ map: NO_CEILING, band: 80, reason: "over-cap", cap: 25_000_000 }],
    });
    expect((await band(NO_CEILING, 80)).entries.map((e) => e.name)).toEqual(["Rosa"]);
  });

  // The table the deployed Worker loads, written by `probe ceilings` from OSMS's live set.
  it("loads a table of 9-digit map IDs, positive solo ceilings below the absolute cap, and mob levels", () => {
    const table = Object.entries(ceilings as unknown as Record<string, Ceiling>);

    expect(table.length).toBeGreaterThan(0);
    for (const [map, { ceiling, mobs }] of table) {
      expect(map).toMatch(/^\d{9}$/);
      expect(Number.isSafeInteger(ceiling) && ceiling > 0 && ceiling * 5 <= 25_000_000).toBe(true);
      expect(mobs).toHaveLength(2);
      expect(mobs.every(Number.isSafeInteger) && 1 <= mobs[0] && mobs[0] <= mobs[1] && mobs[1] <= 200).toBe(true);
    }
  });
});
