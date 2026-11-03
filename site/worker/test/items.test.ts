// The Item names table the deployed Worker loads, written by `probe ceilings` from OSMS's live
// item files. The other tests use a made-up one (vitest.config.ts); this checks only its shape.
import { describe, expect, it } from "vitest";
import type { ItemsTable } from "../items";
import table from "../osms-item-names.json";

describe("the Item names table", () => {
  it("names Items by their IDs, without leading zeros", () => {
    const items = Object.entries((table as ItemsTable).items);

    expect(items.length).toBeGreaterThan(0);
    for (const [id, name] of items) {
      expect(id).toMatch(/^[1-9]\d{6}$/);
      expect(name.trim().length).toBeGreaterThan(0);
    }
  });
});
