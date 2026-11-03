// Map Loot (ADR 0033): the app sends a Visit's mesos and Items, with its key, for a Character it
// shows on the Map Leaderboards; the site keeps each Character's running sums on each map, and
// the daily run works each map's figures out from them: the middle Character's mesos an hour, and
// each Item's average an hour, of each kind. Made-up Items from vitest.config.ts's TEST_ITEMS.
import { createExecutionContext, createScheduledController, waitOnExecutionContext } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import worker from "../index";
import { admin, call, entry, key, optOut, upload } from "./api";

const HOUR = 3_600_000;

interface Visit {
  map: string;
  party?: boolean;
  activeMs?: number;
  mesos: number;
  items?: { id: number; count: number }[];
}

/** The app sending `name`'s Visit with its key. */
function send(name: string, withKey: string, visit: Visit): Promise<Response> {
  const body = { party: false, activeMs: HOUR, items: [], ...visit };
  return call(`/api/characters/${encodeURIComponent(name)}/map-loot`, {
    method: "POST",
    headers: { authorization: `Bearer ${withKey}`, "content-type": "application/json" },
    body: JSON.stringify(body),
  });
}

/** `name` shown on the Map Leaderboards (their entry up, which claims the name), then sending
 *  `visit`: the answer. */
async function shownAndSent(name: string, visit: Visit): Promise<Response> {
  await upload(name, key(name), [entry(visit.map, 30, 10_000)]);
  return send(name, key(name), visit);
}

/** The daily run, as the cron trigger runs it. */
async function daily(): Promise<void> {
  const ctx = createExecutionContext();
  await worker.scheduled(createScheduledController({ cron: "17 5 * * *", scheduledTime: Date.now() }), env, ctx);
  await waitOnExecutionContext(ctx);
}

interface Figures {
  characters: number;
  mesosPerHour?: number;
  items?: { id: number; name: string; perHour: number }[];
}

/** A map's Map Loot, as its page reads it. */
async function loot(map: string): Promise<{ map: string; builtAt: number | null; solo: Figures; party: Figures; both: Figures }> {
  const response = await call(`/api/maps/${map}/loot`);
  if (response.status !== 200) throw new Error(`loot read: ${response.status} ${await response.text()}`);
  return response.json();
}

describe("Map Loot", () => {
  it("works out the middle Character's mesos an hour, and each Item's average an hour, once a day", async () => {
    const map = "100000201";
    await shownAndSent("Anya", { map, mesos: 30_000, items: [{ id: 4000000, count: 60 }] });
    await shownAndSent("Bram", { map, mesos: 50_000, activeMs: 2 * HOUR, items: [{ id: 4000000, count: 20 }, { id: 4000001, count: 9 }] });
    await shownAndSent("Cleo", { map, mesos: 10_000, items: [{ id: 2000000, count: 4 }] });
    expect((await loot(map)).builtAt).toBeNull();

    await daily();

    const figures = await loot(map);
    expect(figures.builtAt).not.toBeNull();
    // 30,000, 25,000 and 10,000 an hour: the middle one. Made-up Shell is 60, 10 and 0 an hour,
    // averaged; the others only one Character picked up, so they're left out.
    expect(figures.solo).toEqual({ characters: 3, mesosPerHour: 25_000, items: [{ id: 4000000, name: "Made-up Shell", perHour: 23.33 }] });
    expect(figures.both).toEqual(figures.solo);
    expect(figures.party).toEqual({ characters: 0 });
  });

  it("adds a Character's Visits on a map up, and keeps their solo and party ones apart", async () => {
    const map = "100000202";
    await shownAndSent("Dara", { map, mesos: 10_000 });
    await send("Dara", key("Dara"), { map, mesos: 30_000 });
    await shownAndSent("Eamon", { map, mesos: 40_000, party: true });
    await shownAndSent("Fern", { map, mesos: 5_000 });
    await shownAndSent("Gus", { map, mesos: 7_000 });

    await daily();

    const figures = await loot(map);
    // Dara's two Visits are one Character: 40,000 in 2 hours.
    expect(figures.solo).toMatchObject({ characters: 3, mesosPerHour: 7_000 });
    expect(figures.party).toEqual({ characters: 1 });
    // Both: each Character once, their solo and party Visits together.
    expect(figures.both).toMatchObject({ characters: 4, mesosPerHour: 13_500 });
  });

  it("shows nothing but how many Characters gave a map Visits, until three have", async () => {
    const map = "100000203";
    await shownAndSent("Hana", { map, mesos: 10_000, items: [{ id: 4000000, count: 5 }] });
    await shownAndSent("Ivo", { map, mesos: 10_000, items: [{ id: 4000000, count: 5 }] });

    await daily();

    expect((await loot(map)).both).toEqual({ characters: 2 });
  });

  it("takes only Items the Item names table holds, and the rest of the Visit", async () => {
    const response = await shownAndSent("Jun", { map: "100000204", mesos: 10_000, items: [{ id: 4000000, count: 5 }, { id: 4999999, count: 1 }] });

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ rejectedItems: [4999999] });
  });

  it("refuses a Visit shorter than 10 minutes, with no mesos, or not shaped as one", async () => {
    await upload("Kai", key("Kai"), [entry("100000205", 30, 10_000)]);
    const refused = [
      { map: "100000205", mesos: 1_000, activeMs: 9 * 60_000 },
      { map: "100000205", mesos: 0 },
      { map: "Henesys", mesos: 1_000 },
      { map: "100000205", mesos: 1_000, items: [{ id: 4000000, count: 0 }] },
    ];
    for (const visit of refused) {
      expect((await send("Kai", key("Kai"), visit)).status, JSON.stringify(visit)).toBe(400);
    }
  });

  it("is refused for a name another key holds, or one a Block reaches", async () => {
    await upload("Lumi", key("Lumi"), [entry("100000206", 30, 10_000)]);
    expect((await send("Lumi", key("someone-else"), { map: "100000206", mesos: 1_000 })).status).toBe(409);

    await upload("Mott", key("Mott"), [entry("100000206", 30, 10_000)]);
    await admin("/characters/Mott/block", { method: "POST", headers: { "content-type": "application/json" }, body: "{}" });
    const blocked = await send("Mott", key("Mott"), { map: "100000206", mesos: 1_000 });
    expect([blocked.status, await blocked.json()]).toEqual([403, { error: "blocked" }]);
  });

  it("goes with the Character when they opt out, are taken down or are blocked, from the next day's figures", async () => {
    const map = "100000207";
    for (const name of ["Nia", "Oren", "Pell", "Quill", "Rune"]) await shownAndSent(name, { map, mesos: 10_000 });
    await daily();
    expect((await loot(map)).both.characters).toBe(5);

    await optOut("Nia", key("Nia"));
    await admin("/entries/Oren", { method: "DELETE" });
    await admin("/characters/Pell/block", { method: "POST", headers: { "content-type": "application/json" }, body: "{}" });
    await daily();

    expect((await loot(map)).both.characters).toBe(2);
  });

  it("reads a map no Visit was sent for as none", async () => {
    expect(await loot("100000208")).toEqual({
      map: "100000208",
      builtAt: null,
      solo: { characters: 0 },
      party: { characters: 0 },
      both: { characters: 0 },
    });
  });
});
