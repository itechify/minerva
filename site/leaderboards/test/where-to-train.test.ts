// Where to train, on the Map Leaderboards page, lists a Level Band's maps as GET
// /api/bands/<band> gives them: each with its mob levels, solo ceiling and the band's #1
// there, sorted by players' bests or by ceiling, on every street or one (ADR 0024).
import { describe, expect, it } from "vitest";
import type { TrainingAnswer, TrainingMap } from "../../entries.ts";
import { mapNames, readTrainingLink, trainingBands, trainingLinkTo, trainingTable, type TrainingLink } from "../leaderboards.ts";
import { NOW, OCT_3 } from "./common.ts";

// Made-up maps, in the fixture's shape: two streets, and an Event map.
const names = mapNames([
  { id: 100000000, street_name: "Made-up Road", name: "Pig Field", region: "Victoria Island" },
  { id: 100000001, street_name: "Made-up Road", name: "Boar Field", region: "Victoria Island" },
  { id: 100000002, street_name: "Hidden Street", name: "Quiet Field", region: "Victoria Island" },
  { id: 100000003, street_name: "Hidden Street", name: "Party Field", region: "Victoria Island" },
  { id: 100000004, street_name: "Event Street", name: "Event Field", region: "Event" },
]);

function map(id: string, ceiling: number | null, best: number | null, entries = best === null ? 0 : 1): TrainingMap {
  return {
    map: id,
    ceiling,
    mobs: ceiling === null ? null : [22, 27],
    entries,
    best: best === null ? null : { name: "Bea", job: "Magician", level: 25, expPerHour: best, setAt: OCT_3 },
  };
}

/** Band 20's maps: Pig Field's #1 under its ceiling, Boar Field with no entries, Quiet Field
 *  with no entries and the lowest ceiling, Party Field's #1 above its ceiling, and a map the
 *  fixture doesn't name with an entry but no ceiling. */
const BAND_20: TrainingAnswer = {
  band: 20,
  levels: [20, 29],
  maps: [
    map("100000000", 300_000, 250_000, 3),
    map("100000001", 500_000, null),
    map("100000002", 100_000, null),
    map("100000003", 200_000, 600_000),
    map("100000009", null, 50_000),
  ],
};

const LINK: TrainingLink = { band: 20, sort: "best", street: null };

function table(link: Partial<TrainingLink> = {}, answer = BAND_20) {
  return trainingTable(answer, names, { ...LINK, ...link }, NOW);
}

describe("where to train's address", () => {
  it("names the band, the sort and the street", () => {
    expect(readTrainingLink("?band=20&sort=ceiling&street=Hidden%20Street")).toEqual({ band: 20, sort: "ceiling", street: "Hidden Street" });
    expect(trainingLinkTo({ band: 20, sort: "ceiling", street: "Hidden Street" })).toBe("/leaderboards/?band=20&sort=ceiling&street=Hidden+Street");
  });

  it("opens with no band, by players' bests, on every street, when it names none", () => {
    expect(readTrainingLink("")).toEqual({ band: null, sort: "best", street: null });
    expect(trainingLinkTo({ band: null, sort: "best", street: null })).toBe("/leaderboards/");
  });

  it("reads a level as the band holding it, and drops what can't be one", () => {
    expect(readTrainingLink("?band=23&sort=fastest&street=")).toEqual({ band: 20, sort: "best", street: null });
    expect(readTrainingLink("?band=999").band).toBeNull();
  });
});

describe("where to train's Level Bands", () => {
  it("say how many maps each lists, the open one marked, each keeping the sort and street", () => {
    const answer = {
      bands: [
        { band: 1, levels: [1, 9] as [number, number], maps: 1, entries: 0 },
        { band: 20, levels: [20, 29] as [number, number], maps: 24, entries: 3 },
      ],
    };

    expect(trainingBands(answer, { band: 20, sort: "ceiling", street: "Hidden Street" })).toEqual([
      { band: 1, levels: "Lv 1–9", maps: "1 map", href: "/leaderboards/?band=1&sort=ceiling&street=Hidden+Street", open: false },
      { band: 20, levels: "Lv 20–29", maps: "24 maps", href: "/leaderboards/?band=20&sort=ceiling&street=Hidden+Street", open: true },
    ]);
  });
});

describe("where to train's table", () => {
  it("writes each map with its mob levels, ceiling, #1, share of the ceiling and entries", () => {
    expect(table().rows[1]).toEqual({
      name: "Pig Field",
      street: "Made-up Road",
      href: "/leaderboards/map/?map=100000000&band=20",
      mobs: "Lv 22–27",
      ceiling: "300,000",
      best: { expPerHour: "250,000", who: "Bea · Magician Lv 25", date: "3 Oct" },
      share: "83%",
      over: false,
      entries: "3 entries",
    });
  });

  it("puts the maps with entries first, by their #1, then the rest by ceiling", () => {
    expect(table().rows.map((r) => r.name)).toEqual(["Party Field", "Pig Field", "Map 100000009", "Boar Field", "Quiet Field"]);
  });

  it("sorts by ceiling, highest first, a map the ceilings table doesn't hold last", () => {
    expect(table({ sort: "ceiling" }).rows.map((r) => r.name)).toEqual(["Boar Field", "Pig Field", "Party Field", "Quiet Field", "Map 100000009"]);
  });

  it("marks a #1 above the map's solo ceiling", () => {
    expect(table().rows[0]).toMatchObject({ name: "Party Field", share: "300%", over: true });
  });

  it("writes a map with no entries, and one the ceilings table doesn't hold, without what it lacks", () => {
    const rows = table().rows;

    expect(rows.find((r) => r.name === "Boar Field")).toMatchObject({ best: null, share: null, over: false, entries: "No entries yet" });
    expect(rows.find((r) => r.name === "Map 100000009")).toMatchObject({ mobs: null, ceiling: null, share: null, entries: "1 entry" });
  });

  it("shows one street's maps, and offers the streets the band's maps are on", () => {
    const shown = table({ street: "Hidden Street" });

    expect(shown.streets).toEqual(["Hidden Street", "Made-up Road", "Not in the map list"]);
    expect(shown.street).toBe("Hidden Street");
    expect(shown.rows.map((r) => r.name)).toEqual(["Party Field", "Quiet Field"]);
    expect(shown.caption).toBe("2 maps for Lv 20–29 on Hidden Street");
  });

  it("shows every street for a street the band has no maps on", () => {
    const shown = table({ street: "Nowhere Road" });

    expect(shown.street).toBeNull();
    expect(shown.rows).toHaveLength(5);
    expect(shown.caption).toBe("5 maps for Lv 20–29");
  });

  it("leaves out an Event map with no entries, as search does, and keeps one with entries", () => {
    const quiet = { ...BAND_20, maps: [map("100000004", 900_000, null)] };
    const busy = { ...BAND_20, maps: [map("100000004", 900_000, 10_000)] };

    expect(table({}, quiet).rows).toEqual([]);
    expect(table({}, busy).rows.map((r) => r.name)).toEqual(["Event Field"]);
  });

  it("says so for a band with no maps", () => {
    expect(table({ band: 150 }, { band: 150, levels: [150, 159], maps: [] })).toEqual({
      rows: [],
      streets: [],
      street: null,
      caption: null,
      empty: {
        text: "No map's mobs are at Lv 150–159, and no one has put a best there yet.",
        link: { text: "How to put yours here from Minerva", href: "/faq/#leaderboard-appear" },
      },
    });
  });
});
