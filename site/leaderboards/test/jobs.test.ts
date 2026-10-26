// Each job the game names belongs to the class it advances from, whose icon the pages show
// beside it; a job the game doesn't name has none.
import { describe, expect, it } from "vitest";
import { classOf } from "../../jobs.ts";

describe("a job's class", () => {
  it("is the first job each advancement starts from", () => {
    expect(["Beginner", "Hero", "Bishop", "I/L Mage", "Marksman", "Shadower", "Corsair"].map(classOf)).toEqual([
      "beginner",
      "warrior",
      "magician",
      "magician",
      "bowman",
      "thief",
      "pirate",
    ]);
  });

  it("is none for a job the game doesn't name, whatever its case", () => {
    expect(classOf("Jester")).toBeNull();
    expect(classOf("magician")).toBeNull();
  });
});
