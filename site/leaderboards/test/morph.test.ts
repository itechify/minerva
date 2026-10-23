// Switching a map's page from one Level Band to another: the Characters with an entry in
// both are the rows that slide to their new rank, each by a name of its own.
import { describe, expect, it } from "vitest";
import { bandTable, morphNames, type Ranked } from "../leaderboards.ts";
import { answer, NOW, ranked } from "./common.ts";

function rows(band: number, names: string[], character: Ranked | null = null) {
  const entries = names.map((name, i) => ranked(i + 1, name));
  const total = names.length + (character === null ? 0 : 1);
  const table = bandTable(answer(entries, character, total, band), character?.name ?? null, NOW);
  return table.yours === null ? table.rows : [...table.rows, table.yours];
}

/** A CSS `<custom-ident>` a `view-transition-name` takes: not `none`, nor a dashed ident. */
const IDENT = /^[a-z_][a-z0-9_-]*$/i;

describe("switching Level Bands", () => {
  it("names only the Characters with an entry in both bands", () => {
    const names = morphNames(rows(20, ["Ada", "Bea", "Egbert"]), rows(30, ["Egbert", "Cy", "Ada"]));

    expect([...names.keys()].sort()).toEqual(["Ada", "Egbert"]);
  });

  it("knows a Character in both whatever case each band writes their name in", () => {
    const names = morphNames(rows(20, ["EGBERT"]), rows(30, ["egbert"]));

    expect(names.get("EGBERT")).toBe(names.get("egbert"));
    expect(names.get("egbert")).toBeDefined();
  });

  it("names the linked Character's entry outside the top 50 too", () => {
    const top50 = Array.from({ length: 50 }, (_, i) => `Player${i + 1}`);
    const names = morphNames(rows(20, ["Egbert"], ranked(1, "Egbert")), rows(30, top50, ranked(57, "Egbert")));

    expect(names.has("Egbert")).toBe(true);
  });

  it("gives each a name a view-transition-name takes, never another's", () => {
    const odd = ["Egbert", "2Fast", "a_b", "a-b", "ab", "none", "Zoë"];
    const names = morphNames(rows(20, odd), rows(30, odd));

    for (const name of odd) expect(names.get(name), name).toMatch(IDENT);
    expect(new Set(names.values()).size).toBe(odd.length);
  });
});
