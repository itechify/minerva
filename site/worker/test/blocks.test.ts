// Blocks (ADR 0031): the maintainer keeps a player off the Map Leaderboards until they lift it.
// A Block reaches the Character's Account, the keys it held, and, if asked, the name itself:
// its uploads are refused, and its Characters are left out of every public read, their entries
// kept for when it's lifted. (D1 starts empty for each test file.)
import { describe, expect, it } from "vitest";
import type { LatestAnswer } from "../../entries";
import { admin, call, entry, key, names, optOut, report, upload } from "./api";
import { asSignedIn, link, signIn, withMadeUpDiscord } from "./accounts";

withMadeUpDiscord();

/** Each test's map of its own, so no other test's entries are in its bands. */
const MAPS = ["100000101", "100000102", "100000103", "100000104", "100000105", "100000106", "100000107", "100000108"];

function block(name: string, body: { name?: boolean; note?: string } = {}): Promise<Response> {
  return admin(`/characters/${name}/block`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
}

async function blocks(): Promise<{ id: number; name: string; blocksName: boolean; account: { username: string } | null; keys: number; note: string }[]> {
  return ((await (await admin("/blocks")).json()) as { blocks: never[] }).blocks;
}

async function unblock(name: string): Promise<Response> {
  const id = (await blocks()).find((b) => b.name === name)!.id;
  return admin(`/blocks/${id}`, { method: "DELETE" });
}

describe("a Block", () => {
  it("refuses its Character's uploads, and the other names its key holds", async () => {
    const mine = key("abe");
    await upload("Abe", mine, [entry(MAPS[0], 31, 300_000)]);
    await upload("Abealt", mine, [entry(MAPS[0], 32, 200_000)]);

    expect((await block("Abe")).status).toBe(201);

    const refused = await upload("Abe", mine, [entry(MAPS[0], 31, 310_000)]);
    expect(refused.status).toBe(403);
    expect(await refused.json()).toEqual({ error: "blocked" });
    expect((await upload("Abealt", mine, [entry(MAPS[0], 32, 210_000)])).status).toBe(403);
    expect((await upload("Abenew", mine, [entry(MAPS[0], 33, 210_000)])).status).toBe(403);
    // A name a refused upload never claimed stays free for anyone else.
    expect((await upload("Abenew", key("someone-else"), [entry(MAPS[0], 33, 210_000)])).status).toBe(200);
  });

  it("still lets the player take their own entries down", async () => {
    const mine = key("bea");
    await upload("Bea", mine, [entry(MAPS[1], 31, 300_000)]);
    await block("Bea");

    expect((await optOut("Bea", mine)).status).toBe(204);
    await unblock("Bea");
    expect(await names(MAPS[1], 30)).not.toContain("Bea");
  });

  it("hides every Character it reaches from the public reads, and Unblock puts them back where they were", async () => {
    const map = MAPS[2];
    const mine = key("cal");
    await upload("Cal", mine, [entry(map, 31, 500_000, 1_760_000_000_000)]);
    await upload("Calalt", mine, [entry(map, 32, 450_000, 1_760_000_000_001)]);
    await upload("Dot", key("dot"), [entry(map, 33, 400_000, 1_760_000_000_002)]);
    const shown = async () => {
      const latest = ((await (await call("/api/latest")).json()) as LatestAnswer).entries.filter((e) => e.map === map).map((e) => e.name);
      const maps = ((await (await call("/api/maps")).json()) as { maps: { map: string; characters: number }[] }).maps.find((m) => m.map === map);
      const training = (await (await call("/api/bands/30")).json()) as { maps: { map: string; best: { name: string } | null }[] };
      return {
        band: await names(map, 30),
        latest,
        characters: maps?.characters ?? 0,
        best: training.maps.find((m) => m.map === map)?.best?.name ?? null,
        page: (await call("/api/characters/Cal")).status,
      };
    };

    await block("Cal");

    expect(await shown()).toEqual({ band: ["Dot"], latest: ["Dot"], characters: 1, best: "Dot", page: 404 });
    expect((await call("/api/characters/Dot")).status).toBe(200);
    expect((await unblock("Cal")).status).toBe(204);
    expect(await shown()).toEqual({ band: ["Cal", "Calalt", "Dot"], latest: ["Dot", "Calalt", "Cal"], characters: 3, best: "Cal", page: 200 });
  });

  it("reaches the Account and its keys, so unlinking or deleting it frees nothing, and a PC linked later is blocked too", async () => {
    const player = await signIn("400000000000001001", "eve_blocked");
    await upload("Eve", key("eve-pc1"), [entry(MAPS[3], 31, 300_000)]);
    await link(key("eve-pc1"), player);
    await link(key("eve-pc2"), player);

    await block("Eve");

    expect(await blocks()).toContainEqual(expect.objectContaining({ name: "Eve", account: expect.objectContaining({ username: "eve_blocked" }), keys: 2 }));
    await link(key("eve-pc3"), player);
    expect((await upload("Eve", key("eve-pc3"), [entry(MAPS[3], 31, 310_000)])).status).toBe(403);
    const pcs = ((await (await asSignedIn(player, "/api/account")).json()) as { pcs: { id: number }[] }).pcs;
    for (const pc of pcs) await asSignedIn(player, `/api/account/pcs/${pc.id}`, { method: "DELETE" });
    await asSignedIn(player, "/api/account", { method: "DELETE" });
    expect((await upload("Eve", key("eve-pc1"), [entry(MAPS[3], 31, 310_000)])).status).toBe(403);
    expect((await upload("Evealt", key("eve-pc2"), [entry(MAPS[3], 32, 310_000)])).status).toBe(403);
    expect((await upload("Evenew", key("eve-pc3"), [entry(MAPS[3], 33, 310_000)])).status).toBe(403);
  });

  it("with the name too, keeps the name off whatever key it's reassigned to", async () => {
    for (const [name, blocksName, status] of [
      ["Fay", true, 403],
      ["Gil", false, 200],
    ] as const) {
      await upload(name, key(`${name}-old`), [entry(MAPS[4], 31, 300_000)]);
      await upload(name, key(`${name}-new`), [entry(MAPS[4], 31, 300_000)]);
      await block(name, { name: blocksName });

      await admin(`/characters/${name}/reassign`, { method: "POST" });

      expect((await upload(name, key(`${name}-new`), [entry(MAPS[4], 31, 300_000)])).status).toBe(status);
    }
  });

  it("dismisses the reports against what it hides, and keeps its note for the maintainer", async () => {
    await upload("Hal", key("hal"), [entry(MAPS[5], 31, 300_000)]);
    await report({ name: "Hal", map: MAPS[5], band: 30, reason: "Too fast" });

    await block("Hal", { note: "  Botting, per three reports  " });

    const reports = (await (await admin("/reports")).json()) as { reports: { name: string }[] };
    expect(reports.reports.map((r) => r.name)).not.toContain("Hal");
    expect(await blocks()).toContainEqual(expect.objectContaining({ name: "Hal", note: "Botting, per three reports", blocksName: false }));
    const looked = (await (await admin("/characters/hal")).json()) as { blocked: { note: string } | null; entries: unknown[] };
    expect(looked.blocked).toMatchObject({ note: "Botting, per three reports" });
    expect(looked.entries).toHaveLength(1);
  });

  it("is refused for a Character one reaches already, or a name that never uploaded", async () => {
    await upload("Ivo", key("ivo"), [entry(MAPS[6], 31, 300_000)]);
    await block("Ivo");

    const again = await block("Ivo");
    const nobody = await block("Neveruploaded");

    expect([again.status, nobody.status]).toEqual([409, 404]);
    expect(await again.json()).toEqual({ error: "blocked" });
    expect((await admin("/blocks/999999", { method: "DELETE" })).status).toBe(404);
  });

  it("is the maintainer's alone", async () => {
    await upload("Jan", key("jan"), [entry(MAPS[7], 31, 300_000)]);
    const player = await signIn("400000000000001002", "jan_player");

    const tries = [
      await asSignedIn(player, "/api/admin/characters/Jan/block", { method: "POST" }),
      await asSignedIn(player, "/api/admin/blocks"),
    ];

    expect(tries.map((r) => r.status)).toEqual([401, 401]);
    expect(await names(MAPS[7], 30)).toContain("Jan");
  });
});
