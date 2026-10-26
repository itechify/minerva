// The maintainer, signed in on the site with a Discord account the Worker lists
// (MAINTAINER_DISCORD_IDS, ADR 0023), for /admin/: reports dismissed, a name looked up, and
// a name reassigned to an Account. ADMIN_TOKEN still works beside it (moderation.test.ts).
import { describe, expect, it } from "vitest";
import { FIELD, admin, entry, key, names, report, upload } from "./api";
import { MAINTAINER_ID, asSignedIn, cookiesSet, discordUser, finishSignIn, link, signIn, startSignIn, withMadeUpDiscord } from "./accounts";

withMadeUpDiscord();

describe("the maintainer, signed in", () => {
  it("reads the reports, where a player signed in can't", async () => {
    const maintainer = await signIn(MAINTAINER_ID, "egberticus");
    const player = await signIn("400000000000000001", "ana");

    const reads = [await asSignedIn(maintainer, "/api/admin/reports"), await asSignedIn(player, "/api/admin/reports")];

    expect(reads.map((r) => r.status)).toEqual([200, 401]);
  });

  it("is told they're the maintainer on the account page", async () => {
    const maintainer = await signIn(MAINTAINER_ID, "egberticus");

    const read = (await (await asSignedIn(maintainer, "/api/account")).json()) as { maintainer: boolean };

    expect(read.maintainer).toBe(true);
  });

  it("stays signed in a day, where a player stays 30", async () => {
    const lasts = [];
    for (const [id, name] of [
      [MAINTAINER_ID, "egberticus"],
      ["400000000000000002", "bo"],
    ]) {
      const { state, cookie } = await startSignIn();
      discordUser(id, name);
      lasts.push(/Max-Age=(\d+)/.exec(cookiesSet(await finishSignIn(id, state, cookie)).get("__Host-session")!)![1]);
    }

    expect(lasts).toEqual([String(24 * 3600), String(30 * 24 * 3600)]);
  });

  it("refuses a write signed in from another site", async () => {
    const maintainer = await signIn(MAINTAINER_ID, "egberticus");
    await upload("Cato", key("cato"), [entry(FIELD, 101, 300_000)]);

    const response = await asSignedIn(maintainer, `/api/admin/entries/Cato/${FIELD}/100`, { method: "DELETE" }, "https://evil.example");

    expect(response.status).toBe(403);
    expect(await names(FIELD, 100)).toContain("Cato");
  });

  it("dismisses a report, and the entry stays", async () => {
    const maintainer = await signIn(MAINTAINER_ID, "egberticus");
    await upload("Dita", key("dita"), [entry(FIELD, 102, 300_000)]);
    await report({ name: "Dita", map: FIELD, band: 100, reason: "Looks off" });
    const { reports } = (await (await admin("/reports")).json()) as { reports: { id: number; name: string }[] };
    const dita = reports.find((r) => r.name === "Dita")!;

    const response = await asSignedIn(maintainer, `/api/admin/reports/${dita.id}`, { method: "DELETE" });

    expect(response.status).toBe(204);
    const after = (await (await admin("/reports")).json()) as { reports: { name: string }[] };
    expect(after.reports.map((r) => r.name)).not.toContain("Dita");
    expect(await names(FIELD, 100)).toContain("Dita");
  });

  it("looks a name up: when it was claimed, refused, and its Account", async () => {
    const player = await signIn("400000000000000003", "edda");
    await upload("Edda", key("edda"), [entry(FIELD, 103, 300_000)]);
    await link(key("edda"), player);
    await upload("Edda", key("edda-stranger"), [entry(FIELD, 103, 300_000)]);

    const read = await (await admin("/characters/edda")).json();

    expect(read).toMatchObject({ name: "Edda", account: { username: "edda" }, entries: 1 });
    expect((read as { refusedAt: number | null }).refusedAt).not.toBeNull();
  });

  it("finds an Account by its Discord username and reassigns a name to it", async () => {
    const player = await signIn("400000000000000004", "finn_classic");
    await upload("Finn", key("finn-lost-pc"), [entry(FIELD, 104, 300_000)]);
    await link(key("finn-new-pc"), player);
    const before = await upload("Finn", key("finn-new-pc"), [entry(FIELD, 104, 310_000)]);
    const found = (await (await admin("/accounts?username=FINN")).json()) as { accounts: { id: number; username: string; pcs: number }[] };

    const response = await admin("/characters/Finn/reassign", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ account: found.accounts[0].id }),
    });

    expect(before.status).toBe(409);
    expect(found.accounts).toMatchObject([{ username: "finn_classic", pcs: 1 }]);
    expect(response.status).toBe(200);
    expect((await upload("Finn", key("finn-new-pc"), [entry(FIELD, 104, 310_000)])).status).toBe(200);
    expect((await upload("Finn", key("finn-lost-pc"), [entry(FIELD, 104, 310_000)])).status).toBe(409);
  });

  it("can't reassign a name to an Account with no PC linked", async () => {
    await signIn("400000000000000005", "gus_nopc");
    await upload("Gus", key("gus"), [entry(FIELD, 105, 300_000)]);
    const found = (await (await admin("/accounts?username=gus_nopc")).json()) as { accounts: { id: number }[] };

    const response = await admin("/characters/Gus/reassign", { method: "POST", body: JSON.stringify({ account: found.accounts[0].id }) });

    expect(response.status).toBe(409);
    expect(await response.json()).toEqual({ error: "no-linked-key" });
  });

  it("takes a name reassigned to a refused key out of the old claimant's Account", async () => {
    const claimant = await signIn("400000000000000006", "hob");
    await upload("Hedy", key("hedy-claimant"), [entry(FIELD, 106, 300_000)]);
    await link(key("hedy-claimant"), claimant);
    await link(key("hedy-claimant-other"), claimant);
    await upload("Hedy", key("hedy-real"), [entry(FIELD, 106, 300_000)]);

    await admin("/characters/Hedy/reassign", { method: "POST" });

    expect((await upload("Hedy", key("hedy-real"), [entry(FIELD, 106, 320_000)])).status).toBe(200);
    expect((await upload("Hedy", key("hedy-claimant-other"), [entry(FIELD, 106, 320_000)])).status).toBe(409);
  });
});

describe("entries taken down", () => {
  it("are listed for the maintainer, and restored: the same Visit then goes up again", async () => {
    const mine = key("ines");
    await upload("Ines", mine, [entry(FIELD, 107, 300_000)]);
    await admin(`/entries/Ines/${FIELD}/100`, { method: "DELETE" });
    const listed = (await (await admin("/removed")).json()) as { removed: { name: string; map: string; band: number; expPerHour: number }[] };
    expect(listed.removed).toContainEqual({ name: "Ines", map: FIELD, band: 100, expPerHour: 300_000, party: false });

    const response = await admin(`/removed/Ines/${FIELD}/100/300000`, { method: "DELETE" });
    const again = await upload("Ines", mine, [entry(FIELD, 107, 300_000)]);

    expect(response.status).toBe(204);
    expect(await again.json()).toEqual({ accepted: 1, rejected: [] });
    expect(await names(FIELD, 100)).toContain("Ines");
    const after = (await (await admin("/removed")).json()) as { removed: { name: string }[] };
    expect(after.removed.map((r) => r.name)).not.toContain("Ines");
  });

  it("can't restore what wasn't taken down", async () => {
    const response = await admin(`/removed/Nobody/${FIELD}/100/1`, { method: "DELETE" });

    expect(response.status).toBe(404);
    expect(await response.json()).toEqual({ error: "no-removed" });
  });

  it("are the maintainer's alone to list or restore", async () => {
    const player = await signIn("400000000000000007", "jo");

    const tries = [await asSignedIn(player, "/api/admin/removed"), await admin(`/removed/Ines/${FIELD}/100/1`, { method: "DELETE" }, "not-the-token")];

    expect(tries.map((r) => r.status)).toEqual([401, 401]);
  });
});
