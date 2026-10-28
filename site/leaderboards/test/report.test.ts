// Each entry has a report link: what the page says once POST /api/reports answers.
import { describe, expect, it } from "vitest";
import { reportOutcome } from "../leaderboards.ts";

describe("a report", () => {
  it("thanks the player once it's taken", () => {
    expect(reportOutcome(201)).toEqual({ taken: true, said: "Thanks for the report. Minerva's maker will look at it." });
  });

  it("says when the entry has gone already", () => {
    expect(reportOutcome(404)).toEqual({ taken: false, said: "This entry isn't on the Map Leaderboard any more." });
  });

  it("asks the player to wait after too many reports", () => {
    expect(reportOutcome(429)).toEqual({ taken: false, said: "That's a lot of reports from you. Try again in a few minutes." });
  });

  it("says when it didn't go through, offline or otherwise", () => {
    const failed = { taken: false, said: "The report didn't go through. Try again later." };
    expect(reportOutcome(500)).toEqual(failed);
    expect(reportOutcome(400)).toEqual(failed);
    expect(reportOutcome(null)).toEqual(failed);
  });
});
