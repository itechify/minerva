// Where to train, on the Map Leaderboards page, lists a Level Band's maps as GET
// /api/bands/<band> gives them: each with its Map Mark, mob levels and the band's #1 there,
// the maps with entries first, in every area or one, by its Map Mark (ADR 0024).
import { describe, expect, it } from "vitest";
import type { MapMark, TrainingAnswer, TrainingMap } from "../../entries.ts";
import { mapNames, readTrainingLink, trainingBands, trainingLinkTo, trainingTable, type TrainingLink } from "../leaderboards.ts";
import { NOW, OCT_3 } from "./common.ts";

// Made-up maps, in the fixture's shape, and an Event map.
const names = mapNames([
  { id: 100000000, street_name: "Made-up Road", name: "Pig Field", region: "Victoria Island" },
  { id: 100000001, street_name: "Made-up Road", name: "Boar Field", region: "Victoria Island" },
  { id: 100000002, street_name: "Hidden Street", name: "Quiet Field", region: "Victoria Island" },
  { id: 100000003, street_name: "Hidden Street", name: "Party Field", region: "Victoria Island" },
  { id: 100000004, street_name: "Event Street", name: "Event Field", region: "Event" },
]);

/** Made-up Map Marks, in the API's shape. */
const HARBOR: MapMark = { id: "Rith", name: "Lith Harbor", picture: "https://osms.invalid/images/map_marks/Rith.png" };
const CITY: MapMark = { id: "KerningCity", name: "Kerning City", picture: "https://osms.invalid/images/map_marks/KerningCity.png" };

function map(id: string, mobs: [number, number] | null, mark: MapMark | null, best: number | null, entries = best === null ? 0 : 1): TrainingMap {
  return {
    map: id,
    mobs,
    mark,
    entries,
    best: best === null ? null : { name: "Bea", job: "Magician", level: 25, expPerHour: best, setAt: OCT_3 },
  };
}

/** Band 20's maps: Pig Field and Party Field with entries, Boar Field and Quiet Field
 *  without (Quiet Field's mobs the lower), and a map the fixture doesn't name with an entry
 *  but no mob levels or mark. Two in Lith Harbor, two in Kerning City. */
const BAND_20: TrainingAnswer = {
  band: 20,
  levels: [20, 29],
  maps: [
    map("100000000", [22, 27], HARBOR, 250_000, 3),
    map("100000001", [25, 29], HARBOR, null),
    map("100000002", [20, 24], CITY, null),
    map("100000003", [24, 28], CITY, 600_000),
    map("100000009", null, null, 50_000),
  ],
};

const LINK: TrainingLink = { band: 20, area: null };

function table(link: Partial<TrainingLink> = {}, answer = BAND_20, all = false) {
  return trainingTable(answer, names, { ...LINK, ...link }, NOW, { all });
}

/** Band 20 with `count` made-up maps, each with an entry, the first the best. */
function many(count: number): TrainingAnswer {
  const maps = Array.from({ length: count }, (_, i) => map(String(100_000_100 + i), [22, 27], CITY, 900_000 - i * 1_000));
  return { band: 20, levels: [20, 29], maps };
}

describe("where to train's address", () => {
  it("names the band and the area, by its Map Mark", () => {
    expect(readTrainingLink("?band=20&area=KerningCity")).toEqual({ band: 20, area: "KerningCity" });
    expect(trainingLinkTo({ band: 20, area: "KerningCity" })).toBe("/leaderboards/?band=20&area=KerningCity");
  });

  it("opens with no band, in every area, when it names neither", () => {
    expect(readTrainingLink("")).toEqual({ band: null, area: null });
    expect(trainingLinkTo({ band: null, area: null })).toBe("/leaderboards/");
  });

  it("reads a level as the band holding it, and drops what can't be one", () => {
    expect(readTrainingLink("?band=23&area=")).toEqual({ band: 20, area: null });
    expect(readTrainingLink("?band=999&area=Kerning%20City")).toEqual({ band: null, area: null });
  });
});

describe("where to train's Level Bands", () => {
  it("say how many maps each lists, the open one marked, each keeping the area", () => {
    const answer = {
      bands: [
        { band: 1, levels: [1, 9] as [number, number], maps: 1, entries: 0 },
        { band: 20, levels: [20, 29] as [number, number], maps: 24, entries: 3 },
      ],
    };

    expect(trainingBands(answer, { band: 20, area: "Rith" })).toEqual([
      { band: 1, levels: "Lv 1–9", maps: "1 map", href: "/leaderboards/?band=1&area=Rith", open: false },
      { band: 20, levels: "Lv 20–29", maps: "24 maps", href: "/leaderboards/?band=20&area=Rith", open: true },
    ]);
  });
});

describe("where to train's table", () => {
  it("writes each map with its Map Mark, mob levels, #1 and entries", () => {
    expect(table().rows[1]).toEqual({
      name: "Pig Field",
      street: "Made-up Road",
      mark: { name: "Lith Harbor", picture: "https://osms.invalid/images/map_marks/Rith.png" },
      href: "/leaderboards/map/?map=100000000&band=20",
      mobs: "Lv 22–27",
      best: { expPerHour: "250,000", name: "Bea", nameHref: "/character/?name=Bea", who: "Magician Lv 25", jobClass: "magician", date: "3 Oct" },
      entries: "3 entries",
    });
  });

  it("puts the maps with entries first, by their #1, then the rest by their mob levels, lowest first", () => {
    expect(table().rows.map((r) => r.name)).toEqual(["Party Field", "Pig Field", "Map 100000009", "Quiet Field", "Boar Field"]);
  });

  it("writes a map with no entries, and one the ceilings table doesn't hold, without what it lacks", () => {
    const rows = table().rows;

    expect(rows.find((r) => r.name === "Boar Field")).toMatchObject({ best: null, entries: "No entries yet" });
    expect(rows.find((r) => r.name === "Map 100000009")).toMatchObject({ mark: null, mobs: null, entries: "1 entry" });
    // A band some of whose maps have entries says nothing over them.
    expect(table().none).toBeNull();
  });

  it("says once, over them, that no one has put a best on a band's maps when none has, their rows saying nothing of it", () => {
    const quiet = { ...BAND_20, maps: [map("100000001", [25, 29], HARBOR, null), map("100000002", [20, 24], CITY, null)] };
    const shown = table({}, quiet);

    expect(shown.none).toEqual({
      text: "No one at Lv 20–29 has put a best on these maps yet. They're the maps whose mobs are at your level.",
      link: { text: "How to put yours here from Minerva", href: "/faq/#leaderboard-appear" },
    });
    expect(shown.rows.map((r) => [r.name, r.mobs, r.best, r.entries])).toEqual([
      ["Quiet Field", "Lv 20–24", null, null],
      ["Boar Field", "Lv 25–29", null, null],
    ]);
    expect(shown.caption).toBe("2 maps for Lv 20–29");
  });

  it("says so over one area's maps when none of them has an entry, though another area's have", () => {
    const answer = { ...BAND_20, maps: [...BAND_20.maps, map("100000005", [21, 26], { id: "Ellinia", name: "Ellinia", picture: "https://osms.invalid/images/map_marks/Ellinia.png" }, null)] };

    expect(table({ area: "Ellinia" }, answer).none?.text).toMatch(/^No one at Lv 20–29 has put a best on this map yet\./);
    expect(table({}, answer).none).toBeNull();
  });

  it("says it of one map as one", () => {
    const one = { ...BAND_20, maps: [map("100000001", [25, 29], HARBOR, null)] };

    expect(table({}, one).none?.text).toBe("No one at Lv 20–29 has put a best on this map yet. Its mobs are at your level.");
  });

  it("offers the areas the band's maps are in, by name, each linking to it", () => {
    expect(table().areas).toEqual([
      { ...CITY, href: "/leaderboards/?band=20&area=KerningCity", open: false },
      { ...HARBOR, href: "/leaderboards/?band=20&area=Rith", open: false },
    ]);
  });

  it("shows one area's maps, the area marked, and a map without a Map Mark only among every area's", () => {
    const shown = table({ area: "KerningCity" });

    expect(shown.area).toBe("KerningCity");
    expect(shown.areas.map((a) => [a.id, a.open])).toEqual([
      ["KerningCity", true],
      ["Rith", false],
    ]);
    expect(shown.rows.map((r) => r.name)).toEqual(["Party Field", "Quiet Field"]);
    expect(shown.caption).toBe("2 Kerning City maps for Lv 20–29");
  });

  it("shows every area for an area the band has no maps in", () => {
    const shown = table({ area: "Ellinia" });

    expect(shown.area).toBeNull();
    expect(shown.rows).toHaveLength(5);
    expect(shown.caption).toBe("5 maps for Lv 20–29");
  });

  it("leaves out an Event map with no entries, as search does, and keeps one with entries", () => {
    const quiet = { ...BAND_20, maps: [map("100000004", [22, 27], null, null)] };
    const busy = { ...BAND_20, maps: [map("100000004", [22, 27], null, 10_000)] };

    expect(table({}, quiet).rows).toEqual([]);
    expect(table({}, busy).rows.map((r) => r.name)).toEqual(["Event Field"]);
  });

  it("says so for a band with no maps", () => {
    expect(table({ band: 150 }, { band: 150, levels: [150, 159], maps: [] })).toEqual({
      rows: [],
      areas: [],
      area: null,
      caption: null,
      more: null,
      none: null,
      empty: {
        text: "No map's mobs are at Lv 150–159, and no one has put a best there yet.",
        link: { text: "How to put yours here from Minerva", href: "/faq/#leaderboard-appear" },
      },
    });
  });

  it("shows a band of more than 20 maps by its first 15, and a button for all of them", () => {
    const shown = table({}, many(22));

    expect(shown.rows.map((r) => r.name)).toEqual(many(22).maps.slice(0, 15).map((m) => `Map ${m.map}`));
    expect(shown.more).toBe("Show all 22 maps");
    expect(shown.caption).toBe("15 of 22 maps for Lv 20–29");
  });

  it("shows every map once asked for all, and every map of a band of 20", () => {
    expect(table({}, many(22), true)).toMatchObject({ more: null, caption: "22 maps for Lv 20–29" });
    expect(table({}, many(22), true).rows).toHaveLength(22);
    expect(table({}, many(20))).toMatchObject({ more: null, caption: "20 maps for Lv 20–29" });
    expect(table({}, many(20)).rows).toHaveLength(20);
  });

  it("counts an area's maps when it cuts one area's table short", () => {
    expect(table({ area: "KerningCity" }, many(21))).toMatchObject({ more: "Show all 21 maps", caption: "15 of 21 Kerning City maps for Lv 20–29" });
  });
});
