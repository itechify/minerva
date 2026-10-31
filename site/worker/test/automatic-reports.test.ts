// An entry far above a busy map's #1 is reported by the Worker itself (ADR 0032): one new or
// higher than its Character's, on a map with at least 8 entries of other Characters, more than
// 2× the best of another Character's of its band and kind. Each test has a made-up map of its
// own, with no ceiling, so no cap is near.
import { describe, expect, it } from "vitest";
import { admin, entry, key, names, optOut, partyEntry, report, upload } from "./api";

interface AdminReport {
  id: number;
  name: string;
  map: string;
  band: number;
  expPerHour: number;
  reason: string;
  party: boolean;
  automatic: boolean;
}

async function reportsOn(map: string): Promise<AdminReport[]> {
  const read = (await (await admin("/reports")).json()) as { reports: AdminReport[] };
  return read.reports.filter((r) => r.map === map);
}

/** `count` Solo Best entries on `map` at level 35, each a Character of its own, at `expPerHour`
 *  and below: the first at it, each next 100 less. */
async function crowd(map: string, prefix: string, count: number, expPerHour: number): Promise<void> {
  for (let i = 0; i < count; i++) {
    const name = `${prefix}${String.fromCharCode(97 + i)}`;
    const response = await upload(name, key(name), [entry(map, 35, expPerHour - 100 * i)]);
    expect(response.status).toBe(200);
  }
}

describe("automatic reports", () => {
  it("reports an entry more than 2× the #1 of a map with 8 entries of other Characters", async () => {
    const map = "100000300";
    await crowd(map, "Arno", 8, 40_000);

    await upload("Bianca", key("bianca"), [entry(map, 36, 88_000)]);

    expect(await reportsOn(map)).toMatchObject([
      {
        name: "Bianca",
        band: 30,
        expPerHour: 88_000,
        party: false,
        automatic: true,
        reason: "2.2× the #1 it passed: Arnoa's 40,000 EXP/h.",
      },
    ]);
  });

  it("leaves a map with 7 entries of other Characters alone", async () => {
    const map = "100000301";
    await crowd(map, "Carlo", 7, 40_000);

    await upload("Dora", key("dora"), [entry(map, 36, 200_000)]);

    expect(await reportsOn(map)).toEqual([]);
  });

  it("leaves an entry at 2× the #1 alone, and reports one EXP/h over it", async () => {
    const map = "100000302";
    await crowd(map, "Emil", 8, 40_000);

    await upload("Frida", key("frida"), [entry(map, 36, 80_000)]);
    expect(await reportsOn(map)).toEqual([]);

    await upload("Frida", key("frida"), [entry(map, 36, 80_001)]);
    expect(await reportsOn(map)).toMatchObject([{ name: "Frida", reason: "2.00× the #1 it passed: Emila's 40,000 EXP/h." }]);
  });

  it("judges an entry only by other Characters' of its band and kind, however busy the map", async () => {
    const map = "100000303";
    await crowd(map, "Hanno", 8, 40_000);

    // The others are Solo Bests at levels 30–39: nothing to judge a Party Best there by, or
    // any entry at levels 40–49.
    await upload("Ines", key("ines"), [partyEntry(map, 36, 200_000), entry(map, 45, 200_000)]);

    expect(await reportsOn(map)).toEqual([]);
  });

  it("counts none of the Character's own entries, towards the 8 or as the #1", async () => {
    const map = "100000304";
    await crowd(map, "Jonas", 7, 40_000);
    // With Kira's own, the map has 10 entries, but only 7 of others.
    await upload("Kira", key("kira"), [entry(map, 36, 90_000), entry(map, 45, 1_000), partyEntry(map, 36, 1_000)]);
    expect(await reportsOn(map)).toEqual([]);

    await crowd(map, "Lotte", 1, 10_000);
    // Now 8 of others. Her new best is less than 2× her own 90,000, but over 4× Jonasa's 40,000.
    await upload("Kira", key("kira"), [entry(map, 36, 170_000), entry(map, 45, 1_000), partyEntry(map, 36, 1_000)]);
    expect(await reportsOn(map)).toMatchObject([{ name: "Kira", reason: "4.2× the #1 it passed: Jonasa's 40,000 EXP/h." }]);
  });

  it("counts a blocked Character's entries for nothing", async () => {
    const map = "100000305";
    await crowd(map, "Moritz", 8, 40_000);
    const blocked = await admin("/characters/Moritza/block", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({}),
    });
    expect(blocked.status).toBe(201);

    // 7 shown, Moritzb's 39,900 the #1.
    await upload("Nele", key("nele"), [entry(map, 36, 200_000)]);

    expect(await reportsOn(map)).toEqual([]);
  });

  it("reports an entry once, however often it's uploaded, and a dismissed report stays gone", async () => {
    const map = "100000306";
    await crowd(map, "Oskar", 8, 40_000);

    await upload("Petra", key("petra"), [entry(map, 36, 100_000)]);
    await upload("Petra", key("petra"), [entry(map, 36, 100_000)]);
    const reports = await reportsOn(map);
    expect(reports).toHaveLength(1);

    expect((await admin(`/reports/${reports[0].id}`, { method: "DELETE" })).status).toBe(204);
    await upload("Petra", key("petra"), [entry(map, 36, 100_000)]);

    expect(await reportsOn(map)).toEqual([]);
  });

  it("replaces an entry's open automatic report when a higher best is reported, and leaves a player's", async () => {
    const map = "100000307";
    await crowd(map, "Quentin", 8, 40_000);
    await upload("Rhea", key("rhea"), [entry(map, 36, 100_000)]);
    expect((await report({ name: "Rhea", map, band: 30, party: false, reason: "Too fast" })).status).toBe(201);

    await upload("Rhea", key("rhea"), [entry(map, 36, 120_000)]);

    const reports = await reportsOn(map);
    expect(reports).toHaveLength(2);
    expect(reports).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ expPerHour: 120_000, automatic: true, reason: "3.0× the #1 it passed: Quentina's 40,000 EXP/h." }),
        expect.objectContaining({ expPerHour: 100_000, automatic: false, reason: "Too fast" }),
      ]),
    );
  });

  it("keeps an automatic report only while its entry stands at the EXP/h it was made at", async () => {
    const map = "100000309";
    await crowd(map, "Ulf", 8, 40_000);
    await upload("Vera", key("vera"), [entry(map, 36, 100_000)]);
    expect(await reportsOn(map)).toHaveLength(1);

    // The others catch up: Vera's next best is no longer 2× theirs, and the report of her old one goes.
    await crowd(map, "Wanda", 1, 90_000);
    await upload("Vera", key("vera"), [entry(map, 36, 150_000)]);
    expect(await reportsOn(map)).toEqual([]);

    // Reported again, then opted out: it goes with her entries.
    await upload("Vera", key("vera"), [entry(map, 36, 200_000)]);
    expect(await reportsOn(map)).toHaveLength(1);
    expect((await optOut("Vera", key("vera"))).status).toBe(204);
    expect(await reportsOn(map)).toEqual([]);
  });

  it("counts an entry taken down for nothing", async () => {
    const map = "100000310";
    await crowd(map, "Xaver", 8, 40_000);
    const takenDown = await admin(`/entries/Xavera/${map}/30?kind=solo`, { method: "DELETE" });
    expect(takenDown.status).toBe(204);

    // 7 shown, Xaverb's 39,900 the #1.
    await upload("Yara", key("yara"), [entry(map, 36, 200_000)]);

    expect(await reportsOn(map)).toEqual([]);
  });

  it("judges a Solo Best and a Party Best of one upload apart, and leaves a reported one on the board", async () => {
    const map = "100000311";
    await crowd(map, "Zeno", 8, 40_000);
    await upload("Anke", key("anke"), [partyEntry(map, 35, 30_000)]);

    // 1.75× the Solo #1, but over 2× the Party #1.
    await upload("Bodo", key("bodo"), [entry(map, 36, 70_000), partyEntry(map, 36, 70_000)]);

    expect(await reportsOn(map)).toMatchObject([
      { name: "Bodo", party: true, reason: "2.3× the #1 it passed: Anke's 30,000 EXP/h." },
    ]);
    expect(await names(map, 30, "party")).toEqual(["Bodo", "Anke"]);
  });

  it("judges every entry of a first upload, however many", async () => {
    const map = "100000308";
    await crowd(map, "Sabine", 8, 40_000);
    // 40 maps with nothing on them first, so the busy map's entry is past the first read.
    const quiet = Array.from({ length: 40 }, (_, i) => entry(String(100_000_400 + i), 36, 1_000));

    await upload("Till", key("till"), [...quiet, entry(map, 36, 100_000)]);

    expect(await reportsOn(map)).toMatchObject([{ name: "Till", automatic: true }]);
  });
});
