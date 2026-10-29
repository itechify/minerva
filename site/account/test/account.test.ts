// What the account, link and admin pages say, from the API's answers.
import { describe, expect, it } from "vitest";
import {
  type CharacterEntry,
  type Overview,
  accountLines,
  blockListLine,
  characterLines,
  entryLine,
  foundLine,
  holds,
  linkQuestion,
  listed,
  logLine,
  marketLine,
  overviewLines,
  pcDates,
  portraitsLine,
  removedLine,
  reportLine,
  signInFailed,
  signInHref,
  tokenIn,
} from "../account.ts";

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
    expect(question.warn).toMatch(/^It's linked to another Discord account now\. Linking moves it to this one\./);
  });
});

describe("the admin page", () => {
  it("lists a report with its entry and when it was made", () => {
    const report = { id: 7, name: "Egbert", map: "010001010", band: 20, expPerHour: 153_216, reason: "", reportedAt: OCT_21, party: false };

    expect(reportLine(report, "Henesys Hunting Ground I")).toEqual({
      what: "Egbert on Henesys Hunting Ground I, levels 20–29: 153,216 EXP/h",
      when: "Reported 21 Oct 2026",
    });
    expect(reportLine({ ...report, party: true }, "Henesys Hunting Ground I").what).toBe(
      "Egbert on Henesys Hunting Ground I, levels 20–29: 153,216 EXP/h (Party Best)",
    );
  });

  it("lists an entry taken down with the Visit kept off", () => {
    const removed = { name: "Egbert", map: "010004042", band: 30, expPerHour: 61_536, party: false };

    expect(removedLine(removed, "Over the Wall")).toBe("Egbert on Over the Wall, levels 30–39: 61,536 EXP/h");
    expect(removedLine({ ...removed, party: true }, "Over the Wall")).toBe(
      "Egbert on Over the Wall, levels 30–39: 61,536 EXP/h (Party Best)",
    );
  });

  const entry: CharacterEntry = { map: "010004042", band: 30, party: false, level: 34, job: "Hunter", expPerHour: 61_536, setAt: OCT_3 };
  const looked = { claimedAt: OCT_3, blocked: null, log: [] };

  it("shows a name looked up: its entries, its Account and the last key refused", () => {
    const lines = characterLines({ ...looked, name: "Egbert", refusedAt: OCT_21, account: { id: 1, username: "egberticus" }, entries: [entry] });

    expect(lines).toEqual(["Egbert: 1 entry, claimed 3 Oct 2026.", "Account: egberticus.", "Another key was last refused 21 Oct 2026."]);
  });

  it("shows a name with no Account and no key refused", () => {
    const lines = characterLines({ ...looked, name: "Bob", refusedAt: null, account: null, entries: [entry, entry, entry] });

    expect(lines).toEqual(["Bob: 3 entries, claimed 3 Oct 2026.", "No Account.", "No other key refused."]);
  });

  it("says a name looked up is blocked, with the Block's note", () => {
    const lines = characterLines({ ...looked, name: "Cid", refusedAt: null, account: null, entries: [], blocked: { id: 1, blockedAt: OCT_21, note: "Botting" } });

    expect(lines[lines.length - 1]).toBe("Blocked 21 Oct 2026: Botting. Its entries are hidden.");
  });

  it("lists a name's entry with where it was set and by whom", () => {
    expect(entryLine(entry, "Over the Wall")).toEqual({ what: "Over the Wall, levels 30–39: 61,536 EXP/h", when: "Lv 34 Hunter, set 3 Oct 2026" });
    expect(entryLine({ ...entry, party: true }, "Over the Wall").what).toBe("Over the Wall, levels 30–39: 61,536 EXP/h (Party Best)");
  });

  it("lists a Block with what it reaches", () => {
    const block = { id: 1, name: "Egbert", blocksName: true, account: { id: 1, username: "egberticus" }, keys: 2, note: "", blockedAt: OCT_3 };

    expect(blockListLine(block)).toEqual({ what: "Egbert: Account egberticus, 2 keys, the name too", when: "Blocked 3 Oct 2026" });
    expect(blockListLine({ ...block, blocksName: false, account: null, keys: 1, note: "Botting" })).toEqual({
      what: "Egbert: no Account, 1 key",
      when: "Blocked 3 Oct 2026: Botting",
    });
  });

  it("lists a line of the log: what was done to whom, and by whom", () => {
    const line = { id: 9, at: OCT_21, by: "token", action: "take-down", name: "Egbert", account: null, detail: "010004042 30 solo" };

    expect(logLine(line).what).toBe("Took an entry down · Egbert (010004042 30 solo)");
    expect(logLine(line).when).toMatch(/^21 Oct, .* by token$|^21 Oct 2026, .* by token$/);
    expect(logLine({ ...line, action: "delete-account", name: null, account: { id: 3, username: null }, detail: "bob" }).what).toBe(
      "Deleted an Account · Account 3 (bob)",
    );
  });

  it("shows an Account opened: its PCs, its Characters and its Block", () => {
    const pc = { id: 3, linkedAt: OCT_3, lastUsedAt: OCT_21, characters: ["Egbert"] };

    expect(accountLines({ id: 1, username: "egberticus", createdAt: OCT_3, pcs: [pc], characters: ["Egbert", "Bob"], blocked: null })).toEqual([
      "egberticus: 1 PC, since 3 Oct 2026.",
      "Characters: Egbert and Bob.",
    ]);
  });

  it("gives the Overview's counts, and says a source is off without its key", () => {
    const overview: Overview = {
      characters: 1_204,
      entries: 8_410,
      accounts: 30,
      pcs: 1,
      reports: 0,
      removed: 5,
      blocks: 1,
      newBests: { day: 12, week: 90 },
      topsTaken: { day: 3, week: 20 },
      portraits: { on: false, checkedAt: null },
      market: { on: true, prices: null, listed: null },
    };

    expect(overviewLines(overview)).toEqual([
      "1,204 Characters with 8,410 entries.",
      "30 Accounts, with 1 PC linked.",
      "0 open reports, 5 entries taken down, 1 Block.",
      "New bests: 12 in the last day, 90 in the last week.",
      "Top spots taken: 3 in the last day, 20 in the last week.",
    ]);
    expect(portraitsLine(overview)).toBe("Off: the Worker has no ClassicRank key.");
    expect(marketLine(overview)).toBe("Going rates never asked. Listings never asked.");
  });

  it("lists an Account found with its PCs, or that it has none", () => {
    expect([
      foundLine({ id: 1, username: "egberticus", pcs: 2, lastUsedAt: OCT_21 }),
      foundLine({ id: 2, username: "bob", pcs: 0, lastUsedAt: null }),
    ]).toEqual(["egberticus: 2 PCs, last used 21 Oct 2026", "bob: no PC linked"]);
  });
});
