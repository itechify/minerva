// The maintainer's tools beside Blocks (ADR 0031): the Overview, Take all down, the Accounts
// view and the log of every write they make. (D1 starts empty for each test file.)
import { describe, expect, it } from "vitest";
import { admin, entry, key, names, report, upload } from "./api";
import { MAINTAINER_ID, account, asSignedIn, cookiesSet, link, signIn, withMadeUpDiscord } from "./accounts";

withMadeUpDiscord();

const MAP = "100000201";
const OTHER_MAP = "100000202";

interface Line {
  id: number;
  by: string;
  action: string;
  name: string | null;
  account: { id: number; username: string | null } | null;
  detail: string;
}

async function log(before?: number): Promise<{ lines: Line[]; more: boolean }> {
  return (await admin(`/log${before === undefined ? "" : `?before=${before}`}`)).json();
}

async function accountId(username: string): Promise<number> {
  const found = (await (await admin(`/accounts?username=${username}`)).json()) as { accounts: { id: number }[] };
  return found.accounts[0].id;
}

describe("the Overview", () => {
  it("counts what D1 keeps, and the new bests and top spots taken in the last day and week", async () => {
    const now = Date.now();
    await upload("Ada", key("ada"), [entry(MAP, 41, 300_000, now - 3_600_000), entry(OTHER_MAP, 41, 300_000, now - 3 * 86_400_000)]);
    await upload("Bo", key("bo"), [entry(MAP, 42, 400_000, now - 60_000)]);
    await report({ name: "Ada", map: MAP, band: 40 });

    const overview = await (await admin("/overview")).json();

    expect(overview).toMatchObject({
      characters: 2,
      entries: 3,
      reports: 1,
      blocks: 0,
      newBests: { day: 2, week: 3 },
      topsTaken: { day: 1, week: 1 },
      portraits: { on: true },
      market: { on: true },
    });
  });
});

describe("Take all down", () => {
  it("takes every entry of a Character down, each kept off until restored", async () => {
    const mine = key("cy");
    await upload("Cy", mine, [entry(MAP, 51, 300_000), entry(OTHER_MAP, 52, 310_000)]);

    expect((await admin("/entries/Cy", { method: "DELETE" })).status).toBe(204);

    expect(await names(MAP, 50)).not.toContain("Cy");
    const again = await upload("Cy", mine, [entry(MAP, 51, 300_000), entry(OTHER_MAP, 52, 310_000)]);
    expect(((await again.json()) as { accepted: number }).accepted).toBe(0);
    expect((await admin("/entries/Cy", { method: "DELETE" })).status).toBe(404);
  });
});

describe("the Accounts view", () => {
  it("shows an Account's PCs and Characters, and unlinks a PC as its player would", async () => {
    const player = await signIn("400000000000002001", "dee_accounts");
    await upload("Dee", key("dee-pc1"), [entry(MAP, 61, 300_000)]);
    await link(key("dee-pc1"), player);
    await link(key("dee-pc2"), player);
    const id = await accountId("dee_accounts");

    const read = (await (await admin(`/accounts/${id}`)).json()) as { username: string; characters: string[]; pcs: { id: number }[]; blocked: null };

    expect(read).toMatchObject({ username: "dee_accounts", characters: ["Dee"], blocked: null });
    expect(read.pcs).toHaveLength(2);
    expect((await admin(`/accounts/${id}/pcs/${read.pcs[1].id}`, { method: "DELETE" })).status).toBe(204);
    expect((await account(player)).pcs).toHaveLength(1);
    expect((await admin("/accounts/999999")).status).toBe(404);
  });

  it("deletes an Account without signing the maintainer out", async () => {
    const maintainer = await signIn(MAINTAINER_ID, "the_maintainer");
    const player = await signIn("400000000000002002", "eli_accounts");
    await upload("Eli", key("eli"), [entry(MAP, 62, 300_000)]);
    await link(key("eli"), player);
    const id = await accountId("eli_accounts");

    const response = await asSignedIn(maintainer, `/api/admin/accounts/${id}`, { method: "DELETE" });

    expect(response.status).toBe(204);
    expect(cookiesSet(response).size).toBe(0);
    expect((await asSignedIn(maintainer, "/api/account")).status).toBe(200);
    expect((await asSignedIn(player, "/api/account")).status).toBe(401);
    expect(await names(MAP, 60)).toContain("Eli");
  });
});

describe("the log", () => {
  it("keeps each write that went through, by who made it, and a Character's lookup shows its own", async () => {
    const maintainer = await signIn(MAINTAINER_ID, "the_maintainer");
    await upload("Flo", key("flo"), [entry(MAP, 71, 300_000), entry(OTHER_MAP, 71, 300_000)]);

    await asSignedIn(maintainer, `/api/admin/entries/Flo/${MAP}/70?kind=solo`, { method: "DELETE" });
    await admin(`/entries/Flo/${MAP}/70`, { method: "DELETE" });
    await admin(`/removed/Flo/${MAP}/70/300000?kind=solo`, { method: "DELETE" });
    await admin("/characters/Flo/block", { method: "POST", body: JSON.stringify({ note: "Testing" }) });

    const lines = (await log()).lines.filter((l) => l.name === "Flo");
    expect(lines.map((l) => [l.by, l.action, l.detail])).toEqual([
      ["token", "block", "Testing"],
      ["token", "restore", `${MAP} 70 solo`],
      ["the_maintainer", "take-down", `${MAP} 70 solo`],
    ]);
    const looked = (await (await admin("/characters/Flo")).json()) as { log: Line[] };
    expect(looked.log.map((l) => l.action)).toEqual(["block", "restore", "take-down"]);
  });

  it("gives 50 lines at a time, the next 50 before the last", async () => {
    await upload("Gia", key("gia"), [entry(MAP, 81, 300_000)]);
    // Each from an address of its own, as reports are limited by the reporter's.
    for (let i = 0; i < 55; i++) await report({ name: "Gia", map: MAP, band: 80 }, `198.51.100.${i}`);
    const { reports } = (await (await admin("/reports")).json()) as { reports: { id: number; name: string }[] };
    for (const r of reports.filter((r) => r.name === "Gia")) await admin(`/reports/${r.id}`, { method: "DELETE" });

    const first = await log();
    const next = await log(first.lines.at(-1)!.id);

    expect(first.lines).toHaveLength(50);
    expect(first.more).toBe(true);
    expect(first.lines[0]).toMatchObject({ action: "dismiss", name: "Gia", detail: `${MAP} 80 solo` });
    expect(next.lines.every((l) => l.id < first.lines.at(-1)!.id)).toBe(true);
  });
});
