// A Level Band's lanes: each entry's EXP/h drawn as the game's EXP gauge, on one linear
// scale from 0 to the band's #1, under a ruler of round steps, as the Personal Bests
// window draws them.
import { describe, expect, it } from "vitest";
import { bandTable } from "../leaderboards.ts";
import { answer, NOW, ranked } from "./common.ts";

describe("a Level Band's lanes", () => {
  it("run #1's lane full, and every other's as far as its EXP/h is of #1's", () => {
    const table = bandTable(answer([ranked(1, "Ada", 160_000), ranked(2, "Bea", 120_000), ranked(3, "Cy", 40_000)]), null, NOW);

    expect(table.rows.map((r) => r.lane)).toEqual([1, 0.75, 0.25]);
  });

  it("put the linked Character's entry outside the top 50 on the band's scale, not its own", () => {
    const top50 = Array.from({ length: 50 }, (_, i) => ranked(i + 1, `Player${i + 1}`, 200_000 - i * 2_000));
    const egbert = ranked(57, "Egbert", 50_000);

    expect(bandTable(answer(top50, egbert, 63), "Egbert", NOW).yours?.lane).toBe(0.25);
  });

  it("hang under a ruler of round steps from 0 up to #1's EXP/h, each where it falls on the lanes", () => {
    const table = bandTable(answer([ranked(1, "Ada", 160_000), ranked(2, "Bea", 90_000)]), null, NOW);

    expect(table.ruler).toEqual([
      { at: 0, text: "0" },
      { at: 0.3125, text: "50k" },
      { at: 0.625, text: "100k" },
      { at: 0.9375, text: "150k" },
    ]);
  });

  it("rule a band whose #1 is in the millions, or in the hundreds, in figures as short", () => {
    const millions = bandTable(answer([ranked(1, "Ada", 1_300_000)]), null, NOW);
    expect(millions.ruler.map((s) => s.text)).toEqual(["0", "250k", "500k", "750k", "1M", "1.25M"]);
    const hundreds = bandTable(answer([ranked(1, "Ada", 800)]), null, NOW);
    expect(hundreds.ruler.map((s) => s.text)).toEqual(["0", "200", "400", "600", "800"]);
  });

  it("rule three to five steps past 0, whatever #1 gets", () => {
    for (const top of [1_000, 1_250, 1_999, 2_500, 4_999, 5_001, 7_777, 99_999, 100_000, 125_500, 157_000, 333_333]) {
      const past0 = bandTable(answer([ranked(1, "Ada", top)]), null, NOW).ruler.length - 1;
      expect(past0, `#1 at ${top}`).toBeGreaterThanOrEqual(3);
      expect(past0, `#1 at ${top}`).toBeLessThanOrEqual(5);
    }
  });
});
