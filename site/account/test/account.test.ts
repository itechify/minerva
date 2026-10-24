// What the account, link and admin pages say, from the API's answers.
import { describe, expect, it } from "vitest";
import { characterLines, foundLine, holds, linkQuestion, listed, pcDates, removedLine, reportLine, signInFailed, signInHref, tokenIn } from "../account.ts";

const OCT_3 = new Date(2026, 9, 3, 12).getTime();
const OCT_21 = new Date(2026, 9, 21, 12).getTime();

describe("names in a sentence", () => {
  it("lists one, two or more names as a sentence would", () => {
    expect([listed([]), listed(["Egbert"]), listed(["Egbert", "Bob"]), listed(["Egbert", "Bob", "Cid"])]).toEqual([
      "",
      "Egbert",
      "Egbert and Bob",
      "Egbert, Bob and Cid",
    ]);
  });

  it("says what a PC holds, or that it holds none yet", () => {
    expect([holds([]), holds(["Egbert", "Bob"])]).toEqual(["Holds no Characters yet.", "Holds Egbert and Bob."]);
  });
});

describe("the account page", () => {
  it("says when a PC was linked and last used", () => {
    expect(pcDates({ id: 1, linkedAt: OCT_3, lastUsedAt: OCT_21, characters: [] })).toBe("Linked 3 Oct 2026 · last used 21 Oct 2026");
  });

  it("signs in and comes back to the page asked", () => {
    expect(signInHref("/account/link/?t=abc")).toBe("/api/auth/discord?return=%2Faccount%2Flink%2F%3Ft%3Dabc");
  });

  it("tells a sign-in that didn't go through", () => {
    expect([signInFailed("?signin=failed"), signInFailed(""), signInFailed("?t=abc")]).toEqual([true, false, false]);
  });
});

describe("the link page", () => {
  it("takes the token its address carries, and nothing that can't be one", () => {
    const token = "a".repeat(43);

    expect([tokenIn(`?t=${token}`), tokenIn(""), tokenIn("?t=short"), tokenIn("?t=not%20a%20token%20at%20all!!")]).toEqual([token, null, null, null]);
  });

  it("asks before linking, naming the Discord account and the Characters the PC holds", () => {
    expect(linkQuestion("egberticus", { characters: ["Egbert", "Bob"], linked: null })).toEqual({
      ask: "Link this Minerva to your Discord account egberticus?",
      holds: "It holds Egbert and Bob.",
      warn: "Only link it if you just clicked Link Discord in Minerva.",
    });
  });

  it("says a PC with no Characters holds none yet, and that one linked elsewhere moves", () => {
    const question = linkQuestion("egberticus", { characters: [], linked: "other" });

    expect(question.holds).toBe("It holds no Characters on the Map Leaderboards yet.");
    expect(question.warn).toMatch(/^It's linked to another Discord account now, and moves to this one\./);
  });
});

describe("the admin page", () => {
  it("lists a report with its entry and when it was made", () => {
    const report = { id: 7, name: "Egbert", map: "010001010", band: 20, expPerHour: 153_216, reason: "", reportedAt: OCT_21 };

    expect(reportLine(report, "Henesys Hunting Ground I")).toEqual({
      what: "Egbert on Henesys Hunting Ground I, levels 20–29: 153,216 EXP/h",
      when: "Reported 21 Oct 2026",
    });
  });

  it("lists an entry taken down with the Visit kept off", () => {
    const removed = { name: "Egbert", map: "010004042", band: 30, expPerHour: 61_536 };

    expect(removedLine(removed, "Over the Wall")).toBe("Egbert on Over the Wall, levels 30–39: 61,536 EXP/h");
  });

  it("shows a name looked up: its entries, its Account and the last key refused", () => {
    const lines = characterLines({ name: "Egbert", claimedAt: OCT_3, refusedAt: OCT_21, account: { id: 1, username: "egberticus" }, entries: 1 });

    expect(lines).toEqual(["Egbert: 1 entry, claimed 3 Oct 2026.", "Account: egberticus.", "Another key was last refused 21 Oct 2026."]);
  });

  it("shows a name with no Account and no key refused", () => {
    const lines = characterLines({ name: "Bob", claimedAt: OCT_3, refusedAt: null, account: null, entries: 3 });

    expect(lines).toEqual(["Bob: 3 entries, claimed 3 Oct 2026.", "No Account.", "No other key refused."]);
  });

  it("lists an Account found with its PCs, or that it has none", () => {
    expect([
      foundLine({ id: 1, username: "egberticus", pcs: 2, lastUsedAt: OCT_21 }),
      foundLine({ id: 2, username: "bob", pcs: 0, lastUsedAt: null }),
    ]).toEqual(["egberticus: 2 PCs, last used 21 Oct 2026", "bob: no PC linked"]);
  });
});
