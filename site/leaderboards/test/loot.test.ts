import { describe, expect, it } from "vitest";
import type { MapLootAnswer } from "../../entries.ts";
import { lootPanel, mesoTier, mesosCounted } from "../leaderboards.ts";

/** A map's Map Loot: 12 Characters on Both, 9 solo, 2 in a party. */
const answer: MapLootAnswer = {
  map: "100000000",
  builtAt: 1_760_000_000_000,
  both: {
    characters: 12,
    mesosPerHour: 41_234,
    items: [
      { id: 4000008, name: "Pig's Head", perHour: 64.4 },
      { id: 2000000, name: "Red Potion", perHour: 1.2 },
      { id: 4000010, name: "Pig's Ribbon", perHour: 0.4 },
      { id: 1002100, name: "Ribboned Pig Headband", perHour: 0.07 },
      { id: 4031000, name: "Pig's Crown", perHour: 0.004 },
      { id: 4031001, name: "Pig's Sceptre", perHour: 0 },
    ],
  },
  solo: { characters: 9, mesosPerHour: 38_000, items: [] },
  party: { characters: 2 },
};

describe("a map's Map Loot panel", () => {
  it("shows the middle Character's mesos an hour, and each Item an hour, with its icon and market page", () => {
    const panel = lootPanel(answer, null, "Pig Park");

    expect(panel.figure).toBe("41,200");
    expect(panel.perHour).toBe(41_234);
    expect(panel.waiting).toBe(false);
    expect(panel.line).toBeNull();
    expect(panel.items[0]).toEqual({
      name: "Pig's Head",
      rate: "64",
      icon: "https://osmsdataexplorer.com/data/current/images/items/04000008.png",
      href: "https://maplestory.market/items/4000008?ref=76rbdbd7",
    });
  });

  it("writes an Item rarer than one an hour as a decimal, to 2 significant figures", () => {
    const rates = lootPanel(answer, null, "Pig Park").items.map((item) => item.rate);

    expect(rates).toEqual(["64", "1", "0.4", "0.07", "0.004", "0"]);
    const rare = { ...answer, both: { characters: 12, mesosPerHour: 1, items: [{ id: 4000008, name: "Pig's Head", perHour: 0.0567 }] } };
    expect(lootPanel(rare, null, "Pig Park").items[0].rate).toBe("0.057");
  });

  it("shows the mesos as the game drops them, a bigger drop for a richer map", () => {
    expect([0, 9_999, 10_000, 49_999, 50_000, 199_999, 200_000, 5_000_000].map(mesoTier)).toEqual([
      "bronze",
      "bronze",
      "gold",
      "gold",
      "bills",
      "bills",
      "bag",
      "bag",
    ]);
    expect(lootPanel(answer, null, "Pig Park").tier).toBe("gold");
    expect(lootPanel(answer, "party", "Pig Park").tier).toBeNull();
  });

  it("follows the kind the boards show", () => {
    expect(lootPanel(answer, "solo", "Pig Park")).toMatchObject({ figure: "38,000", line: null, items: [] });
  });

  it("says Not yet, and what it waits for, until 3 Characters gave the map Visits of the kind", () => {
    const party = lootPanel(answer, "party", "Pig Park");
    expect(party.figure).toBe("Not yet");
    expect(party.waiting).toBe(true);
    expect(party.perHour).toBeNull();
    expect(party.items).toEqual([]);
    expect(party.line).toBe("Mesos and items show here once 3 characters have played Pig Park in a party for 10 minutes with Minerva.");

    const none: MapLootAnswer = { map: "100000000", builtAt: null, both: { characters: 0 }, solo: { characters: 0 }, party: { characters: 0 } };
    expect(lootPanel(none, null, "Pig Park").line).toBe("Mesos and items show here once 3 characters have played Pig Park for 10 minutes with Minerva.");
    expect(lootPanel(none, "solo", "Pig Park").line).toBe("Mesos and items show here once 3 characters have played Pig Park solo for 10 minutes with Minerva.");
  });

  it("counts the figure up as the Items drop in, on the site's ease-out, landing on the figure the panel shows", () => {
    const frames = Array.from({ length: 31 }, (_, i) => mesosCounted(0, 41_234, i / 30));

    expect(frames[0]).toBe("0");
    expect(frames.at(-1)).toBe(lootPanel(answer, null, "Pig Park").figure);
    const counted = frames.map((frame) => Number(frame.replaceAll(",", "")));
    expect(counted).toEqual([...counted].sort((a, b) => a - b));
    // Most of the way there in the first third: the count slows as it lands.
    expect(counted[10]).toBeGreaterThan(41_234 * 0.85);
  });

  it("counts from the kind shown before to the one switched to, down as well as up, never past either", () => {
    expect(mesosCounted(41_234, 38_000, 0)).toBe("41,200");
    expect(mesosCounted(41_234, 38_000, 1)).toBe("38,000");
    expect(mesosCounted(41_234, 38_000, 2)).toBe("38,000");
    expect(mesosCounted(41_234, 38_000, -1)).toBe("41,200");
  });
});
