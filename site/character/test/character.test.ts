// A Character's page shows who they are as their newest entry says, their #1s, podiums, maps
// and best EXP/h, and every entry of theirs a Level Band at a time, highest first, each with
// its rank there and a link to it on its Map Leaderboard (ADR 0027).
import { describe, expect, it } from "vitest";
import type { CharacterEntry, MapMark } from "../../entries.ts";
import { mapNames } from "../../leaderboards/leaderboards.ts";
import { type CharacterAnswer, characterPage, noCharacter, readName } from "../character.ts";

/** Noon UTC, so the day is the same in any time zone the tests run in. */
const OCT_3 = Date.UTC(2026, 9, 3, 12);
const NOW = new Date(Date.UTC(2026, 9, 14, 12));

const names = mapNames([
  { id: 100000000, street_name: "Made-up Road", name: "Pig Field", region: "Victoria Island" },
  { id: 100000001, street_name: "Made-up Road", name: "Ant Hill", region: "Victoria Island" },
  { id: 100000002, street_name: "Made-up Lane", name: "Snail Beach", region: "Maple Island" },
]);

/** A made-up Map Mark, in the API's shape. */
const HARBOR: MapMark = { id: "Rith", name: "Lith Harbor", picture: "https://osms.invalid/images/map_marks/Rith.png" };

function mine(map: string, band: number, rank: number, total: number, expPerHour: number, more: Partial<CharacterEntry> = {}): CharacterEntry {
  return { map, band, rank, total, job: "Magician", level: band + 3, expPerHour, setAt: OCT_3, mobs: [22, 27], mark: HARBOR, ...more };
}

function answer(...entries: CharacterEntry[]): CharacterAnswer {
  return { name: "Egbert", entries };
}

describe("a Character's page", () => {
  it("reads the name its address names, and drops one that can't be a Character's", () => {
    expect(readName("?name=Egbert")).toBe("Egbert");
    expect(readName("?name=not-a-name")).toBeNull();
    expect(readName("")).toBeNull();
  });

  it("says who they are as their newest entry says", () => {
    const page = characterPage(
      answer(
        mine("100000000", 20, 1, 3, 90_000, { job: "Magician", level: 25, setAt: OCT_3 }),
        mine("100000001", 30, 4, 9, 120_000, { job: "Cleric", level: 31, setAt: Date.UTC(2026, 9, 12, 12) }),
      ),
      names,
      NOW,
    );

    expect(page.name).toBe("Egbert");
    expect(page.who).toBe("Cleric Lv 31 · newest best 12 Oct");
  });

  it("counts each map and Level Band's #1 on its own, and the podiums among them", () => {
    const page = characterPage(
      answer(
        mine("100000000", 20, 1, 3, 90_000),
        mine("100000000", 10, 1, 1, 30_000),
        mine("100000001", 20, 3, 8, 70_000),
        mine("100000002", 20, 4, 4, 50_000),
      ),
      names,
      NOW,
    );

    expect(page.stats).toEqual([
      { value: "2", label: "#1s held", detail: null },
      { value: "3", label: "Podiums", detail: "Top 3 in a band" },
      { value: "3", label: "Maps", detail: "in 2 Level Bands" },
      { value: "90,000", label: "Best EXP/h", detail: "Pig Field" },
    ]);
  });

  it("writes one of each in the singular", () => {
    const page = characterPage(answer(mine("100000000", 20, 1, 1, 90_000)), names, NOW);

    expect(page.stats.map((s) => [s.value, s.label, s.detail])).toEqual([
      ["1", "#1 held", null],
      ["1", "Podium", "Top 3 in a band"],
      ["1", "Map", "in 1 Level Band"],
      ["90,000", "Best EXP/h", "Pig Field"],
    ]);
  });

  it("holds none without a #1 or a podium", () => {
    const page = characterPage(answer(mine("100000000", 20, 7, 9, 90_000)), names, NOW);

    expect(page.stats.slice(0, 2).map((s) => [s.value, s.label])).toEqual([
      ["0", "#1s held"],
      ["0", "Podiums"],
    ]);
  });

  it("lists the Level Bands highest first, each band's entries highest EXP/h first", () => {
    const page = characterPage(
      answer(mine("100000000", 10, 2, 5, 30_000), mine("100000001", 20, 5, 6, 40_000), mine("100000002", 20, 1, 2, 80_000)),
      names,
      NOW,
    );

    expect(page.bands.map((b) => [b.levels, b.caption, b.rows.map((r) => r.map)])).toEqual([
      ["Lv 20–29", "2 maps at Lv 20–29", ["Snail Beach", "Ant Hill"]],
      ["Lv 10–19", "1 map at Lv 10–19", ["Pig Field"]],
    ]);
  });

  it("writes each entry with its rank of how many, the map, its mobs, and a link to it marked on its Map Leaderboard", () => {
    const page = characterPage(answer(mine("100000000", 20, 1, 12, 153_216, { level: 23 })), names, NOW);

    expect(page.bands[0].rows).toEqual([
      {
        rank: "#1",
        of: "of 12",
        medal: 1,
        map: "Pig Field",
        street: "Made-up Road",
        mark: { name: "Lith Harbor", picture: "https://osms.invalid/images/map_marks/Rith.png" },
        mobs: "Lv 22–27",
        job: "Magician",
        jobClass: "magician",
        level: "23",
        expPerHour: "153,000",
        date: "3 Oct",
        href: "/leaderboards/map/?map=100000000&band=20&name=Egbert",
      },
    ]);
  });

  it("crowns a place in the band's top 3 gold, silver or bronze, and no place past them", () => {
    const page = characterPage(
      answer(mine("100000000", 20, 2, 12, 90_000), mine("100000001", 20, 3, 12, 80_000), mine("100000002", 20, 4, 12, 70_000)),
      names,
      NOW,
    );

    expect(page.bands[0].rows.map((r) => [r.rank, r.medal])).toEqual([
      ["#2", 2],
      ["#3", 3],
      ["#4", null],
    ]);
  });

  it("names the class of the job their newest entry says they are", () => {
    const page = characterPage(answer(mine("100000000", 20, 1, 3, 90_000, { job: "Hermit" })), names, NOW);

    expect(page.whoClass).toBe("thief");
  });

  it("writes a map the ceilings table and the map list don't hold by its ID, with no mobs or Map Mark", () => {
    const page = characterPage(answer(mine("100000099", 20, 1, 1, 90_000, { mobs: null, mark: null })), names, NOW);

    expect(page.bands[0].rows[0]).toMatchObject({ map: "Map 100000099", street: "Not in the map list", mobs: null, mark: null });
  });

  it("writes a date from another year with its year", () => {
    const page = characterPage(answer(mine("100000000", 20, 1, 1, 90_000, { setAt: Date.UTC(2025, 9, 3, 12) })), names, NOW);

    expect(page.who).toBe("Magician Lv 23 · newest best 3 Oct 2025");
    expect(page.bands[0].rows[0].date).toBe("3 Oct 2025");
  });

  it("says the same for any name with no entries, with where a player's own entry would be", () => {
    expect(noCharacter("Egbert")).toEqual({
      text: "Egbert has no bests on the Map Leaderboards.",
      link: { text: "Where's my character's entry?", href: "/faq/#leaderboard-link" },
    });
  });

  it("sends a link naming no Character to the Map Leaderboards", () => {
    expect(noCharacter(null)).toEqual({ text: "This link doesn't name a Character.", link: { text: "Find a map on the Map Leaderboards.", href: "/leaderboards/" } });
  });
});
