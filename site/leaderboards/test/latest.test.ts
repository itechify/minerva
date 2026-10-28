// Latest, on the Map Leaderboards page, lists the newest entries as GET /api/latest gives
// them, each linking to its Map Leaderboard with the entry marked, and says when one took
// a top spot (ADR 0024).
import { describe, expect, it } from "vitest";
import type { LatestEntry, MapMark } from "../../entries.ts";
import { latestKinds, latestLines, mapNames } from "../leaderboards.ts";
import { NOW, OCT_3 } from "./common.ts";

const names = mapNames([{ id: 100000000, street_name: "Made-up Road", name: "Pig Field", region: "Victoria Island" }]);

/** A made-up Map Mark, in the API's shape. */
const HARBOR: MapMark = { id: "Rith", name: "Lith Harbor", picture: "https://osms.invalid/images/map_marks/Rith.png" };

function latest(took: LatestEntry["took"] = null, setAt = OCT_3, mark: MapMark | null = HARBOR, party = false): LatestEntry {
  return { name: "Hakuo", job: "Magician", level: 23, expPerHour: 153_216, setAt, map: "100000000", mark, band: 20, party, took };
}

describe("latest", () => {
  it("writes each entry with who set it, its EXP/h, where (with the map's Map Mark), and when", () => {
    expect(latestLines({ entries: [latest()] }, names, NOW)).toEqual({
      empty: null,
      lines: [
        {
          name: "Hakuo",
          nameHref: "/character/?name=Hakuo",
          who: "Magician Lv 23",
          jobClass: "magician",
          expPerHour: "153,000",
          map: "Pig Field",
          mark: { name: "Lith Harbor", picture: "https://osms.invalid/images/map_marks/Rith.png" },
          levels: "Lv 20–29",
          date: "3 Oct",
          // The board of the entry's own kind, where it stands as Latest says.
          href: "/leaderboards/map/?map=100000000&band=20&name=Hakuo&kind=solo",
          party: false,
          took: null,
        },
      ],
    });
  });

  it("marks an entry that is a Party Best, and links to the Map Leaderboard of party entries", () => {
    const { lines } = latestLines({ entries: [latest(null, OCT_3, HARBOR, true)] }, names, NOW);

    expect(lines[0]).toMatchObject({ party: true, href: "/leaderboards/map/?map=100000000&band=20&name=Hakuo&kind=party" });
  });

  it("offers both, solo or party, the one shown marked", () => {
    expect(latestKinds("solo")).toEqual([
      { kind: null, label: "Both", open: false },
      { kind: "solo", label: "Solo", open: true },
      { kind: "party", label: "Party", open: false },
    ]);
  });

  it("says when an entry took a top spot, and from whom, linking to their page", () => {
    const { lines } = latestLines({ entries: [latest({ from: "Egbert", fromExpPerHour: 150_000 })] }, names, NOW);

    expect(lines[0].took).toEqual(["Beat ", { text: "Egbert", href: "/character/?name=Egbert" }, "'s 150,000 to take #1"]);
  });

  it("leaves out the name of a Character who has since left the Map Leaderboards", () => {
    const { lines } = latestLines({ entries: [latest({ from: null, fromExpPerHour: 150_000 })] }, names, NOW);

    expect(lines[0].took).toEqual(["Beat a best of 150,000 to take #1"]);
  });

  it("writes a map without a Map Mark without one", () => {
    const { lines } = latestLines({ entries: [latest(null, OCT_3, null)] }, names, NOW);

    expect(lines[0].mark).toBeNull();
  });

  it("writes a date from another year with its year", () => {
    const { lines } = latestLines({ entries: [latest(null, Date.UTC(2025, 9, 3, 12))] }, names, NOW);

    expect(lines[0].date).toBe("3 Oct 2025");
  });

  it("says so when no one has uploaded a best yet, and how to put yours there from Minerva", () => {
    expect(latestLines({ entries: [] }, names, NOW)).toEqual({
      lines: [],
      empty: {
        text: "No one has a best on the Map Leaderboards yet.",
        link: { text: "How to add yours from Minerva", href: "/faq/#leaderboard-appear" },
      },
    });
    expect(latestLines({ entries: [] }, names, NOW, "party").empty?.text).toBe("No one has a party best on the Map Leaderboards yet.");
  });
});
