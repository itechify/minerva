// A Level Band on a map's page: its top 50, as GET /api/maps/<map>/bands/<band> ranks them,
// and the linked Character's entry with its rank, even outside them.
import { describe, expect, it } from "vitest";
import { bandTable } from "../leaderboards.ts";
import { answer, NOW, OCT_3, ranked } from "./common.ts";

/** A band's top 50: 157,000 EXP/h at #1, 73,400 at #50. */
const TOP_50 = Array.from({ length: 50 }, (_, i) => ranked(i + 1, `Player${i + 1}`, i === 49 ? 73_400 : 157_000 - i * 1_000));

describe("a Level Band", () => {
  it("lists its entries by rank, each EXP/h written as the Grind Tracker writes it", () => {
    const table = bandTable(
      answer([
        { rank: 1, name: "Egbert", job: "Magician", level: 23, expPerHour: 153_216, setAt: OCT_3 },
        { rank: 2, name: "Ada", job: "Bowman", level: 29, expPerHour: 66_550, setAt: Date.UTC(2025, 11, 30, 12) },
      ]),
      null,
      NOW,
    );

    expect(table.rows).toEqual([
      { rank: "1", medal: 1, name: "Egbert", nameHref: "/character/?name=Egbert", job: "Magician", jobClass: "magician", level: "23", expPerHour: "153,000", date: "3 Oct", you: false, lane: expect.any(Number), portrait: null },
      { rank: "2", medal: 2, name: "Ada", nameHref: "/character/?name=Ada", job: "Bowman", jobClass: "bowman", level: "29", expPerHour: "66,600", date: "30 Dec 2025", you: false, lane: expect.any(Number), portrait: null },
    ]);
    expect(table.yours).toBe(null);
    expect(table.gap).toBe(null);
    expect(table.note).toBe(null);
    expect(table.empty).toBe(null);
  });

  it("crowns the band's top 3 gold, silver and bronze, and no one past them", () => {
    const table = bandTable(answer([1, 2, 3, 4].map((rank) => ranked(rank, `Crowd${rank}`))), null, NOW);

    expect(table.rows.map((r) => r.medal)).toEqual([1, 2, 3, null]);
  });

  it("names the class of a job the game names, and none of one it doesn't", () => {
    const table = bandTable(answer([{ ...ranked(1, "Egbert"), job: "Dragon Knight" }, { ...ranked(2, "Ada"), job: "Jester" }]), null, NOW);

    expect(table.rows.map((r) => r.jobClass)).toEqual(["warrior", null]);
  });

  it("says under the table how many entries it shows, of how many", () => {
    expect(bandTable(answer(TOP_50, null, 63), null, NOW).caption).toBe("Top 50 of 63 at Lv 20–29");
    expect(bandTable(answer(TOP_50), null, NOW).caption).toBe("50 entries at Lv 20–29");
    expect(bandTable(answer([ranked(1, "Ada", 60_000)]), null, NOW).caption).toBe("1 entry at Lv 20–29");
  });

  it("marks the linked Character's entry and says its rank above the table", () => {
    const egbert = ranked(2, "Egbert", 50_000);
    const table = bandTable(answer([ranked(1, "Ada", 60_000), egbert, ranked(3, "Bea", 40_000)], egbert), "egbert", NOW);

    expect(table.rows.map((r) => [r.name, r.you])).toEqual([
      ["Ada", false],
      ["Egbert", true],
      ["Bea", false],
    ]);
    expect(table.yours).toBe(null);
    expect(table.note).toBe("Egbert is #2 of 3 at Lv 20–29.");
  });


  it("shows the linked Character's entry under the top 50 when it's outside them, after how many are between", () => {
    const egbert = ranked(57, "Egbert", 1_234);
    const table = bandTable(answer(TOP_50, egbert, 63), "Egbert", NOW);

    expect(table.rows).toHaveLength(50);
    expect(table.rows.some((r) => r.you)).toBe(false);
    expect(table.gap).toBe("6 more entries");
    expect(table.yours).toEqual({
      rank: "57",
      medal: null,
      name: "Egbert",
      nameHref: "/character/?name=Egbert",
      job: "Magician",
      jobClass: "magician",
      level: "23",
      expPerHour: "1,230",
      date: "3 Oct",
      you: true,
      lane: expect.any(Number),
      portrait: null,
    });
    expect(table.note).toBe("Egbert is #57 of 63 at Lv 20–29.");
  });

  it("puts the linked Character's entry right under the top 50 when it's #51", () => {
    const egbert = ranked(51, "Egbert", 1_234);
    const table = bandTable(answer(TOP_50, egbert, 51), "Egbert", NOW);

    expect(table.gap).toBe(null);
    expect(table.yours?.rank).toBe("51");
    expect(bandTable(answer(TOP_50, ranked(52, "Egbert", 1_000), 52), "Egbert", NOW).gap).toBe("1 more entry");
  });

  it("says when the linked Character's entry has gone", () => {
    const table = bandTable(answer([ranked(1, "Ada", 60_000)]), "Egbert", NOW);

    expect(table.rows.map((r) => r.you)).toEqual([false]);
    expect(table.yours).toBe(null);
    expect(table.note).toBe("Egbert has no entry here at Lv 20–29.");
  });

  it("says when no one has a best in the band, and how to put yours there from Minerva", () => {
    expect(bandTable(answer([]), null, NOW)).toEqual({
      rows: [],
      gap: null,
      yours: null,
      note: null,
      caption: null,
      empty: {
        text: "No one at Lv 20–29 has put a best on this map yet.",
        link: { text: "How to put yours here from Minerva", href: "/faq/#leaderboard-appear" },
      },
      ruler: [],
      portraits: false,
    });
    expect(bandTable(answer([]), "Egbert", NOW).note).toBe("Egbert has no entry here at Lv 20–29.");
  });
});
