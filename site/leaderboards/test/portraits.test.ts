// A Character's portrait from ClassicRank beside their name in a Level Band and in Latest
// (ADR 0029): their photo, the question-mark figure once ClassicRank has none, or the
// portrait's room, empty, until it's been asked; and none at all while the API gives none,
// when nothing credits ClassicRank either.
import { describe, expect, it } from "vitest";
import type { LatestEntry } from "../../entries.ts";
import { bandTable, latestLines, mapNames } from "../leaderboards.ts";
import { answer, NOW, OCT_3, ranked } from "./common.ts";

const PHOTO = "https://classicrank.com/api/avatar/Egbert?v=2026-10-26%2007%3A41%3A13";
const PROFILE = "https://classicrank.com/user/Egbert";

const names = mapNames([{ id: 100000000, street_name: "Made-up Road", name: "Pig Field", region: "Victoria Island" }]);

function latest(portrait?: LatestEntry["portrait"]): LatestEntry {
  const entry: LatestEntry = { name: "Egbert", job: "Magician", level: 23, expPerHour: 153_216, setAt: OCT_3, map: "100000000", mark: null, band: 20, took: null };
  return portrait === undefined ? entry : { ...entry, portrait };
}

describe("a portrait in a Level Band", () => {
  it("is the Character's photo from ClassicRank", () => {
    const table = bandTable(answer([{ ...ranked(1, "Egbert"), portrait: { photo: PHOTO, profile: PROFILE } }]), null, NOW);

    expect(table.rows[0].portrait).toEqual({ photo: PHOTO });
    expect(table.portraits).toBe(true);
  });

  it("is the question-mark figure for a Character ClassicRank has no photo of, or doesn't show", () => {
    const table = bandTable(
      answer([
        { ...ranked(1, "Egbert"), portrait: { photo: null, profile: PROFILE } },
        { ...ranked(2, "Ada"), portrait: { photo: null, profile: null } },
      ]),
      null,
      NOW,
    );

    expect(table.rows.map((r) => r.portrait)).toEqual(["none", "none"]);
  });

  it("keeps its room, empty, until ClassicRank has been asked", () => {
    expect(bandTable(answer([{ ...ranked(1, "Egbert"), portrait: null }]), null, NOW).rows[0].portrait).toBe("waiting");
  });

  it("shows for the linked Character's row under the top 50 too", () => {
    const egbert = { ...ranked(60, "Egbert"), portrait: { photo: PHOTO, profile: PROFILE } };

    expect(bandTable(answer([ranked(1, "Ada")], egbert, 60), "Egbert", NOW).yours?.portrait).toEqual({ photo: PHOTO });
  });

  it("isn't there at all while the API gives none, and nothing credits ClassicRank", () => {
    const table = bandTable(answer([ranked(1, "Egbert")]), null, NOW);

    expect(table.rows[0].portrait).toBeNull();
    expect(table.portraits).toBe(false);
  });
});

describe("a portrait in Latest", () => {
  it("is the Character's photo, the question-mark figure, or its room until asked", () => {
    const shown = latestLines(
      { entries: [latest({ photo: PHOTO, profile: PROFILE }), latest({ photo: null, profile: null }), latest(null)] },
      names,
      NOW,
    );

    expect(shown.lines.map((l) => l.portrait)).toEqual([{ photo: PHOTO }, "none", "waiting"]);
    expect(shown.portraits).toBe(true);
  });

  it("isn't there at all while the API gives none, and nothing credits ClassicRank", () => {
    const shown = latestLines({ entries: [latest()] }, names, NOW);

    expect(shown.lines[0].portrait).toBeNull();
    expect(shown.portraits).toBe(false);
  });
});
