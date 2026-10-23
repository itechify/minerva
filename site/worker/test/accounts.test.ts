// Optional Accounts (ADR 0023): a player signs in on the site with Discord and links their
// Leaderboard Keys, so a name their Account holds may be written from any PC they linked.
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { FIELD, band, call, entry, key, optOut, upload } from "./api";
import {
  account,
  asSignedIn,
  confirm,
  cookiesSet,
  discordUser,
  finishSignIn,
  link,
  linkState,
  linkToken,
  signIn,
  startSignIn,
  withMadeUpDiscord,
} from "./accounts";

withMadeUpDiscord();

describe("signing in", () => {
  it("signs in with Discord and returns to the page asked", async () => {
    const { state, cookie, location } = await startSignIn("/account/link/?t=abc");
    expect(location.origin + location.pathname).toBe("https://discord.com/oauth2/authorize");
    expect(location.searchParams.get("scope")).toBe("identify");

    discordUser("100000000000000001", "ada");
    const back = await finishSignIn("100000000000000001", state, cookie);

    expect(back.status).toBe(302);
    expect(back.headers.get("location")).toBe("/account/link/?t=abc");
    expect([...cookiesSet(back).keys()]).toEqual(expect.arrayContaining(["__Host-session", "signed_in"]));
  });

  it("shows who's signed in on the account page", async () => {
    const session = await signIn("100000000000000002", "bea");

    expect(await account(session)).toEqual({ username: "bea", maintainer: false, pcs: [] });
  });

  it("refuses a sign-in whose state isn't the one it started with", async () => {
    const { cookie } = await startSignIn();
    discordUser("100000000000000003", "cid");

    const back = await finishSignIn("100000000000000003", "not-the-state", cookie);

    expect(back.headers.get("location")).toBe("/account/?signin=failed");
    expect(cookiesSet(back).has("__Host-session")).toBe(false);
  });

  it("returns to a page whose address holds a dot", async () => {
    const { state, cookie } = await startSignIn("/faq/index.html#leaderboard-claimed");
    discordUser("100000000000000008", "hal");

    const back = await finishSignIn("100000000000000008", state, cookie);

    expect(back.headers.get("location")).toBe("/faq/index.html#leaderboard-claimed");
  });

  it("never returns to an address elsewhere", async () => {
    const { state, cookie } = await startSignIn("//evil.example/");
    discordUser("100000000000000004", "dov");

    const back = await finishSignIn("100000000000000004", state, cookie);

    expect(back.headers.get("location")).toBe("/account/");
  });

  it("is one Account however often the player signs in, with their username as it is now", async () => {
    await signIn("100000000000000005", "eli");
    const session = await signIn("100000000000000005", "eli_renamed");

    expect((await account(session)).username).toBe("eli_renamed");
    const accounts = await env.DB.prepare("SELECT COUNT(*) AS n FROM logins WHERE provider_user_id = ?").bind("100000000000000005").first<number>("n");
    expect(accounts).toBe(1);
  });

  it("signs out", async () => {
    const session = await signIn("100000000000000006", "fay");

    const out = await asSignedIn(session, "/api/auth/sign-out", { method: "POST" });

    expect(out.status).toBe(204);
    expect((await asSignedIn(session, "/api/account")).status).toBe(401);
  });

  it("is never cached", async () => {
    const session = await signIn("100000000000000007", "gil");

    const reads = [await asSignedIn(session, "/api/account"), await call("/api/link", { headers: { authorization: `Bearer ${key("gil")}` } })];

    expect(reads.map((r) => r.headers.get("cache-control"))).toEqual(["no-store", "no-store"]);
  });
});

describe("linking a Leaderboard Key", () => {
  it("lets a new PC write for the names its Account holds once it's linked", async () => {
    const session = await signIn("200000000000000001", "hana");
    await upload("Hana", key("hana-old-pc"), [entry(FIELD, 21, 30_000)]);
    await link(key("hana-old-pc"), session);
    const before = await upload("Hana", key("hana-new-pc"), [entry(FIELD, 22, 31_000)]);

    await link(key("hana-new-pc"), session);
    const after = await upload("Hana", key("hana-new-pc"), [entry(FIELD, 22, 31_000)]);

    expect(before.status).toBe(409);
    expect(after.status).toBe(200);
    expect((await band(FIELD, 20, "Hana")).character).toMatchObject({ expPerHour: 31_000 });
    expect((await optOut("Hana", key("hana-old-pc"))).status).toBe(204);
  });

  it("gives a name first claimed by a linked key its Account", async () => {
    const session = await signIn("200000000000000002", "ivo");
    await link(key("ivo-old-pc"), session);
    await upload("Ivo", key("ivo-old-pc"), [entry(FIELD, 23, 32_000)]);

    await link(key("ivo-new-pc"), session);

    expect((await upload("Ivo", key("ivo-new-pc"), [entry(FIELD, 23, 33_000)])).status).toBe(200);
  });

  it("links nothing until the player confirms it, so a token someone else made can't take their names", async () => {
    const victim = await signIn("200000000000000003", "jun");
    await upload("Jun", key("jun"), [entry(FIELD, 24, 34_000)]);
    await link(key("jun"), victim);
    const token = await linkToken(key("impostor-jun"));

    const shown = await asSignedIn(victim, `/api/link/tokens/${token}`);

    expect(await shown.json()).toEqual({ characters: [], linked: null });
    expect((await upload("Jun", key("impostor-jun"), [entry(FIELD, 24, 99_000)])).status).toBe(409);
  });

  it("refuses a confirmation from another site", async () => {
    const session = await signIn("200000000000000004", "kai");
    const token = await linkToken(key("kai"));

    const response = await confirm(session, token, "https://evil.example");

    expect(response.status).toBe(403);
    expect((await linkState(key("kai"))).linked).toBe(false);
  });

  it("shows the confirmation page the Characters the key holds", async () => {
    const session = await signIn("200000000000000005", "lev");
    await upload("Lev", key("lev"), [entry(FIELD, 25, 35_000)]);
    await upload("Lea", key("lev"), [entry(FIELD, 25, 35_000)]);

    const shown = await asSignedIn(session, `/api/link/tokens/${await linkToken(key("lev"))}`);

    expect(await shown.json()).toEqual({ characters: ["Lea", "Lev"], linked: null });
  });

  it("uses a token up, and refuses one past its 5 minutes", async () => {
    const session = await signIn("200000000000000006", "mia");
    const used = await linkToken(key("mia"));
    await confirm(session, used);
    const stale = await linkToken(key("mia-other"));
    await env.DB.prepare("UPDATE link_tokens SET expires_at = ?").bind(Date.now() - 1).run();

    const responses = [await confirm(session, used), await confirm(session, stale)];

    expect(responses.map((r) => r.status)).toEqual([404, 404]);
  });

  it("needs a signed-in player to confirm", async () => {
    const token = await linkToken(key("nils"));

    const response = await call(`/api/link/tokens/${token}`, { method: "POST", headers: { origin: "https://owlofminerva.app" } });

    expect(response.status).toBe(401);
  });

  it("tells the app what it's linked to", async () => {
    const session = await signIn("200000000000000007", "ola");
    const before = await linkState(key("ola"));

    await link(key("ola"), session);

    expect(before).toEqual({ linked: false });
    expect(await linkState(key("ola"))).toEqual({ linked: true, username: "ola" });
  });

  it("moves a key linked again, and the names it claimed, to the new Account", async () => {
    const first = await signIn("200000000000000008", "pim");
    const second = await signIn("200000000000000009", "pim_alt");
    await upload("Pim", key("pim"), [entry(FIELD, 26, 36_000)]);
    await link(key("pim"), first);
    await link(key("pim-first-other"), first);

    await link(key("pim"), second);

    expect((await upload("Pim", key("pim-first-other"), [entry(FIELD, 26, 37_000)])).status).toBe(409);
    expect((await account(first)).pcs.map((pc) => pc.characters)).toEqual([[]]);
    expect((await account(second)).pcs.map((pc) => pc.characters)).toEqual([["Pim"]]);
  });
});

describe("the account page", () => {
  it("lists the linked PCs, the most recently used first, each with the Characters it claimed", async () => {
    const session = await signIn("300000000000000001", "quin");
    await link(key("quin-laptop"), session);
    await link(key("quin-desktop"), session);
    await upload("Quin", key("quin-desktop"), [entry(FIELD, 27, 38_000)]);

    const { pcs } = await account(session);

    expect(pcs.map((pc) => pc.characters)).toEqual([["Quin"], []]);
  });

  it("moves the names of a PC unlinked to the Account's most recently used other one", async () => {
    const session = await signIn("300000000000000002", "ria");
    await upload("Ria", key("ria-sold-pc"), [entry(FIELD, 28, 39_000)]);
    await link(key("ria-sold-pc"), session);
    await link(key("ria-new-pc"), session);
    const sold = (await account(session)).pcs.find((pc) => pc.characters.includes("Ria"))!;

    const response = await asSignedIn(session, `/api/account/pcs/${sold.id}`, { method: "DELETE" });

    expect(response.status).toBe(204);
    expect((await upload("Ria", key("ria-sold-pc"), [entry(FIELD, 28, 99_000)])).status).toBe(409);
    expect((await upload("Ria", key("ria-new-pc"), [entry(FIELD, 28, 40_000)])).status).toBe(200);
    expect((await account(session)).pcs.map((pc) => pc.characters)).toEqual([["Ria"]]);
  });

  it("leaves the names of the only PC unlinked with it, out of the Account", async () => {
    const session = await signIn("300000000000000003", "sol");
    await upload("Sol", key("sol"), [entry(FIELD, 29, 41_000)]);
    await link(key("sol"), session);
    const [only] = (await account(session)).pcs;

    await asSignedIn(session, `/api/account/pcs/${only.id}`, { method: "DELETE" });
    await link(key("sol-later"), session);

    expect((await upload("Sol", key("sol"), [entry(FIELD, 29, 42_000)])).status).toBe(200);
    expect((await upload("Sol", key("sol-later"), [entry(FIELD, 29, 43_000)])).status).toBe(409);
  });

  it("can't unlink another Account's PC", async () => {
    const owner = await signIn("300000000000000004", "tom");
    const other = await signIn("300000000000000005", "uma");
    await link(key("tom"), owner);
    const [pc] = (await account(owner)).pcs;

    const response = await asSignedIn(other, `/api/account/pcs/${pc.id}`, { method: "DELETE" });

    expect(response.status).toBe(404);
    expect((await account(owner)).pcs).toHaveLength(1);
  });

  it("deletes the Account, each name staying with the key that claimed it, its entries on the site", async () => {
    const session = await signIn("300000000000000006", "vic");
    await upload("Vic", key("vic-claimed"), [entry(FIELD, 30, 44_000)]);
    await link(key("vic-claimed"), session);
    await link(key("vic-other"), session);

    const response = await asSignedIn(session, "/api/account", { method: "DELETE" });

    expect(response.status).toBe(204);
    expect((await asSignedIn(session, "/api/account")).status).toBe(401);
    expect((await band(FIELD, 30, "Vic")).character).toMatchObject({ expPerHour: 44_000 });
    expect((await upload("Vic", key("vic-other"), [entry(FIELD, 30, 45_000)])).status).toBe(409);
    expect((await upload("Vic", key("vic-claimed"), [entry(FIELD, 30, 45_000)])).status).toBe(200);
    expect(await linkState(key("vic-other"))).toEqual({ linked: false });
  });

  it("refuses a write signed in from another site", async () => {
    const session = await signIn("300000000000000007", "wyn");

    const response = await asSignedIn(session, "/api/account", { method: "DELETE" }, "https://evil.example");

    expect(response.status).toBe(403);
    expect((await account(session)).username).toBe("wyn");
  });
});
