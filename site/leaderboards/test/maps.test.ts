// The Map Leaderboards name their maps from the committed map names fixture, as the panel
// names them, and find a map by its Map Name as the panel's search does.
import { describe, expect, it } from "vitest";
import fixture from "../../../crates/minerva-core/tests/fixtures/osms-map-names.json";
import { entryCounts, mapMarks, mapNames, placeOf, searchMaps, searchSaid } from "../leaderboards.ts";

const names = mapNames(fixture);

/** What a search shows, as "Map Name (ID)". */
function found(query: string, shown?: number) {
  const { maps, more } = searchMaps(names, query, { shown });
  return { maps: maps.map((m) => `${m.name} (${m.id})`), more };
}

describe("a map's name", () => {
  it("is the fixture's, found by the 9-digit ID the API gives", () => {
    expect(placeOf(names, "010001010")).toEqual({
      id: "010001010",
      name: "Henesys Hunting Ground I",
      street: "Victoria Road",
      region: "Victoria Island",
      known: true,
    });
  });

  it("is trimmed as the panel trims it", () => {
    expect(placeOf(names, "010001020").name).toBe("A Hill West of Henesys");
  });

  it("is the map's ID, not in the map list, for a map the fixture doesn't name", () => {
    expect(placeOf(names, "999999999")).toEqual({
      id: "999999999",
      name: "Map 999999999",
      street: "Not in the map list",
      region: "",
      known: false,
    });
    // The fixture has a map with no name or street.
    expect(placeOf(names, "010006121").known).toBe(false);
  });
});

describe("map search", () => {
  it("finds maps whose name starts with the query first, then a word in it, each by name", () => {
    expect(found("hunting ground", 20)).toEqual({
      maps: [
        "Hunting Ground Middle of the Forest I (000001005)",
        "Hunting Ground Middle of the Forest II (000001006)",
        "Hunting Ground in the Deep Forest I (010005011)",
        "Hunting Ground in the Deep Forest II (010005012)",
        "Beach Hunting Ground (010000022)",
        "Drake Hunting Ground (010005075)",
        "Henesys Hunting Ground I (010001010)",
        "Henesys Hunting Ground II (010001011)",
        "Henesys Hunting Ground III (010001012)",
        "Snail Hunting Ground I (000000040)",
        "Snail Hunting Ground II (000000041)",
        "Snail Hunting Ground III (000000042)",
      ],
      more: 0,
    });
  });

  it("puts a name holding the query inside a word last", () => {
    expect(found("unnel", 20).maps.slice(0, 1)).toEqual(["Ant Tunnel I (010005060)"]);
    expect(found("tunnel I", 20).maps).toEqual([
      "Ant Tunnel I (010005060)",
      "Ant Tunnel II (010005061)",
      "Ant Tunnel III (010005062)",
      "Ant Tunnel IV (010005063)",
      "Deep Ant Tunnel I (010005066)",
      "Deep Ant Tunnel II (010005067)",
    ]);
  });

  it("puts the islands' maps before those off them, so Enter opens Ant Tunnel I for “ant”, not a job advancement test", () => {
    expect(found("ant", 6).maps).toEqual([
      "Ant Tunnel I (010005060)",
      "Ant Tunnel II (010005061)",
      "Ant Tunnel III (010005062)",
      "Ant Tunnel IV (010005063)",
      "Ant Tunnel Park (010005070)",
      "Ant Tunnel For Bowman (080001000)",
    ]);
    expect(searchSaid("ant", searchMaps(names, "ant"))).toMatch(/Enter opens Ant Tunnel I\.$/);
  });

  it("shows a few and counts the rest", () => {
    expect(found("hunting ground", 5)).toEqual({
      maps: [
        "Hunting Ground Middle of the Forest I (000001005)",
        "Hunting Ground Middle of the Forest II (000001006)",
        "Hunting Ground in the Deep Forest I (010005011)",
        "Hunting Ground in the Deep Forest II (010005012)",
        "Beach Hunting Ground (010000022)",
      ],
      more: 7,
    });
  });

  it("matches whatever the case, and ignores spaces around the query", () => {
    expect(found("  HENESYS HUNTING GROUND II ").maps).toEqual([
      "Henesys Hunting Ground II (010001011)",
      "Henesys Hunting Ground III (010001012)",
    ]);
  });

  it("leaves out Event and Dev maps, which the player can't normally enter, as the panel does", () => {
    expect(found("Physical Fitness").maps).toEqual([]);
    expect(found("Manager's Room").maps).toEqual([]);
  });

  it("finds nothing for nothing typed", () => {
    expect(found("   ")).toEqual({ maps: [], more: 0 });
  });

  it("says what it found, and the map Enter opens", () => {
    expect(searchSaid("", searchMaps(names, ""))).toBe("Type part of a map's name as the game writes it, like Ant Tunnel.");
    expect(searchSaid("Zakum", searchMaps(names, "Zakum"))).toBe("No map's name has “Zakum” in it. Check the spelling, as the game writes it.");
    expect(searchSaid("henesys hunting ground", searchMaps(names, "henesys hunting ground"))).toBe(
      "3 maps. Enter opens Henesys Hunting Ground I.",
    );
    expect(searchSaid("ant tunnel", searchMaps(names, "ant tunnel", { shown: 2 }))).toBe(
      "2 maps, and 6 more: type more of the name. Enter opens Ant Tunnel I.",
    );
  });

  it("says how many entries each map has, once the API has said, and lists the maps with entries first", () => {
    const counts = entryCounts({
      maps: [
        { map: "010001012", characters: 9, entries: 12 },
        { map: "010001011", characters: 1, entries: 1 },
      ],
    });
    const { maps } = searchMaps(names, "henesys hunting ground", { counts });

    expect(maps.map((m) => [m.name, m.entries])).toEqual([
      ["Henesys Hunting Ground II", "1 entry"],
      ["Henesys Hunting Ground III", "12 entries"],
      ["Henesys Hunting Ground I", "No entries yet"],
    ]);
    expect(searchSaid("henesys hunting ground", { maps, more: 0 })).toBe("3 maps. Enter opens Henesys Hunting Ground II.");
    // Before the API has said, or when it can't be reached, nothing is said of entries.
    expect(searchMaps(names, "henesys hunting ground").maps.map((m) => m.entries)).toEqual([null, null, null]);
  });

  it("puts the map named the query itself first, whatever the case, even with no entries", () => {
    const counts = entryCounts({ maps: [{ map: "010001010", characters: 2, entries: 2 }] });
    const { maps } = searchMaps(names, " HENESYS ", { counts });

    expect(maps.slice(0, 2).map((m) => [m.name, m.entries])).toEqual([
      ["Henesys", "No entries yet"],
      ["Henesys Hunting Ground I", "2 entries"],
    ]);
    expect(searchSaid("henesys", { maps, more: 0 })).toMatch(/Enter opens Henesys\.$/);
  });

  it("gives each map found its Map Mark from the ceilings table, and none to a map it holds without one", () => {
    // A made-up table in the ceilings table's shape.
    const marks = mapMarks({
      maps: { "010001010": { mobs: [3, 8], mark: "Henesys" }, "010001011": { mobs: [5, 9] } },
      marks: { Henesys: { name: "Henesys", picture: "https://osms.invalid/images/map_marks/Henesys.png" } },
    });
    const { maps } = searchMaps(names, "henesys hunting ground", { marks });

    expect(maps.map((m) => [m.name, m.mark?.name ?? null])).toEqual([
      ["Henesys Hunting Ground I", "Henesys"],
      ["Henesys Hunting Ground II", null],
      ["Henesys Hunting Ground III", null],
    ]);
  });

  it("counts a map's entries among all it finds, not only the ones it shows", () => {
    const counts = entryCounts({ maps: [{ map: "010005063", characters: 3, entries: 3 }] });

    expect(found("ant tunnel", 2).maps).toEqual(["Ant Tunnel I (010005060)", "Ant Tunnel II (010005061)"]);
    expect(searchMaps(names, "ant tunnel", { shown: 2, counts }).maps.map((m) => m.name)).toEqual(["Ant Tunnel IV", "Ant Tunnel I"]);
  });
});
