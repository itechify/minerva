// An upload replaces the whole set of a Character's entries, so the site mirrors the app's
// Personal Bests: removals and merges carry over, and opting out deletes them all.
import { describe, expect, it } from "vitest";
import { FIELD, RICH_FIELD, band, entry, key, names, optOut, upload } from "./api";

describe("mirroring", () => {
  it("shows an uploaded entry in its map's Level Band", async () => {
    const response = await upload("Egbert", key("egbert"), [entry(FIELD, 23, 50_000, 1_760_000_000_000)]);

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ accepted: 1, rejected: [] });
    // Its portrait from ClassicRank comes after the upload (test/portraits.test.ts).
    expect((await band(FIELD, 20)).entries).toMatchObject([
      { rank: 1, name: "Egbert", job: "Magician", level: 23, expPerHour: 50_000, setAt: 1_760_000_000_000, party: false },
    ]);
  });

  it("replaces the whole set at each upload, so a best removed or merged in the app goes", async () => {
    const mine = key("ferdinand");
    await upload("Ferdinand", mine, [entry(FIELD, 31, 60_000), entry(RICH_FIELD, 33, 900_000)]);

    await upload("Ferdinand", mine, [entry(RICH_FIELD, 34, 950_000)]);

    expect(await names(FIELD, 30)).not.toContain("Ferdinand");
    expect((await band(RICH_FIELD, 30)).entries.find((e) => e.name === "Ferdinand")).toMatchObject({
      level: 34,
      expPerHour: 950_000,
    });
  });

  it("deletes all of a Character's entries when it opts out", async () => {
    const mine = key("gertrude");
    await upload("Gertrude", mine, [entry(FIELD, 41, 70_000), entry(RICH_FIELD, 52, 1_000_000)]);

    const response = await optOut("Gertrude", mine);

    expect(response.status).toBe(204);
    expect(await names(FIELD, 40)).not.toContain("Gertrude");
    expect(await names(RICH_FIELD, 50)).not.toContain("Gertrude");
  });

  it("refuses an upload with two entries for one map and Level Band", async () => {
    const response = await upload("Hildegard", key("hildegard"), [entry(FIELD, 61, 70_000), entry(FIELD, 65, 80_000)]);

    expect(response.status).toBe(400);
    expect(await names(FIELD, 60)).not.toContain("Hildegard");
    // Nor did it claim the name.
    expect((await upload("Hildegard", key("someone-else"), [entry(FIELD, 61, 70_000)])).status).toBe(200);
  });

  // The date breaks ties, so one can't be set ahead to win them.
  it("refuses an entry set more than a day from now", async () => {
    const tomorrowAndMore = Date.now() + 25 * 3_600_000;

    const response = await upload("Irmgard", key("irmgard"), [entry(FIELD, 66, 70_000, tomorrowAndMore)]);

    expect(response.status).toBe(400);
  });
});
