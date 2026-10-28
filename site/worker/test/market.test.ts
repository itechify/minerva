// maplestory.market's prices for the app (ADR 0030): the Worker pulls its two tables whole, at
// most once an hour each, and gives back the items asked about. A made-up maplestory.market
// answers its asks here, in its tables' shapes, with made-up items; nothing real from it is
// in the test, and nothing leaves it.
import { env } from "cloudflare:workers";
import { beforeEach, describe, expect, it } from "vitest";
import { call } from "./api";

/** What the made-up maplestory.market answers each table with, and how: `status` other than
 *  200 answers that, with `body` as it is. */
interface Answers {
  status: number;
  body: { prices: unknown; items: unknown };
  asks: { path: string; key: string | null }[];
}

const market: Answers = { status: 200, body: { prices: [], items: [] }, asks: [] };

const realFetch = globalThis.fetch;
globalThis.fetch = ((input: RequestInfo | URL, init?: RequestInit) => {
  const url = new URL(input instanceof Request ? input.url : String(input));
  if (url.origin !== "https://maplestory.market") return realFetch(input as RequestInfo, init);
  const key = new Headers(init?.headers).get("authorization")?.replace(/^Bearer /, "") ?? null;
  market.asks.push({ path: url.pathname, key });
  const table = url.pathname === "/api/prices" ? market.body.prices : url.pathname === "/api/items" ? market.body.items : null;
  if (market.status !== 200 || table === null) return Promise.resolve(new Response("{}", { status: market.status === 200 ? 404 : market.status }));
  return Promise.resolve(Response.json(table));
}) as typeof globalThis.fetch;

/** A made-up item's row in /api/prices: established at `price` from `sales` sales. */
function priced(id: number, price: number, sales = 10) {
  return {
    item_id: id,
    sold: { sales, units: sales, avg_unit_price: price, median_unit_price: price, day_sales: 2, day_units: 2 },
    seen: { listings: 40, median_unit_price: price + 10, p25_unit_price: price, p75_unit_price: price + 50 },
    reference: { unit_price: price, basis: "sold", samples: sales, scope: "item" },
  };
}

/** A made-up item's row in /api/items: listed `listings` times, the cheapest at `cheapest`. */
function listed(id: number, cheapest: number, listings = 5) {
  return { item_id: id, listings, sold_out: 1, total_units: listings, min_unit_price: cheapest };
}

const ORE = 4010000;
const STAR = 2070000;
const SWORD = 1302000;

function setMarket(prices: unknown, items: unknown) {
  market.status = 200;
  market.body = { prices, items };
}

async function read(ids: string, from = "203.0.113.1"): Promise<Response> {
  return call(`/api/market?ids=${ids}`, { headers: { "cf-connecting-ip": from } });
}

async function readOk(ids: string): Promise<{ prices: { pulledAt: number; items: Record<string, unknown> } | null; listed: { pulledAt: number; items: Record<string, unknown> } | null }> {
  const response = await read(ids);
  expect(response.status).toBe(200);
  return response.json();
}

/** Makes the tables kept `ms` older, as if pulled that long before. */
async function age(ms: number) {
  await env.DB.prepare("UPDATE market_pulls SET pulled_at = pulled_at - ?1, tried_at = tried_at - ?1").bind(ms).run();
}

beforeEach(async () => {
  market.asks = [];
  await env.DB.batch([
    env.DB.prepare("DELETE FROM market_pulls"),
    env.DB.prepare("DELETE FROM market_prices"),
    env.DB.prepare("DELETE FROM market_listed"),
    env.DB.prepare("DELETE FROM rate_events WHERE bucket LIKE 'market:%'"),
  ]);
  setMarket([priced(ORE, 99), priced(STAR, 379)], [listed(ORE, 50, 314), listed(STAR, 300)]);
});

describe("market reads", () => {
  it("give the items asked about from both tables, pulled with the key", async () => {
    const answer = await readOk(`${ORE},${SWORD}`);

    expect(answer.prices?.items).toEqual({ [ORE]: { sold: priced(ORE, 99).sold, seen: priced(ORE, 99).seen, reference: priced(ORE, 99).reference } });
    expect(answer.listed?.items).toEqual({ [ORE]: { min_unit_price: 50, listings: 314 } });
    expect(answer.prices?.pulledAt).toBeGreaterThan(0);
    expect(market.asks).toEqual([
      { path: "/api/prices", key: "mm_made-up-key" },
      { path: "/api/items", key: "mm_made-up-key" },
    ]);
  });

  it("leave out each item's figures by attack roll and its deal price", async () => {
    setMarket([{ ...priced(SWORD, 5000), by_attack: [{ attack: 17 }], typical_attack: 17, deal: { unit_price: 4000 } }], [listed(SWORD, 4500)]);

    const answer = await readOk(String(SWORD));

    expect(Object.keys(answer.prices?.items[SWORD] as object).sort()).toEqual(["reference", "seen", "sold"]);
  });

  it("pull each table once an hour, however many reads", async () => {
    await readOk(String(ORE));
    await readOk(String(STAR));
    await age(59 * 60_000);
    await readOk(String(ORE));
    expect(market.asks).toHaveLength(2);

    setMarket([priced(ORE, 120)], [listed(ORE, 60)]);
    await age(2 * 60_000);
    const answer = await readOk(String(ORE));

    expect(market.asks).toHaveLength(4);
    expect((answer.prices?.items[ORE] as { reference: { unit_price: number } }).reference.unit_price).toBe(120);
    expect(answer.listed?.items[STAR]).toBeUndefined();
  });

  it("keep the copy they have while maplestory.market can't be asked, and ask again a minute on", async () => {
    await readOk(String(ORE));
    await age(61 * 60_000);
    market.status = 503;

    const kept = await readOk(String(ORE));
    const soon = await readOk(String(ORE));

    expect(kept.listed?.items[ORE]).toEqual({ min_unit_price: 50, listings: 314 });
    expect(soon.prices?.items[ORE]).toBeDefined();
    expect(market.asks).toHaveLength(4);

    await env.DB.prepare("UPDATE market_pulls SET tried_at = tried_at - 61000").run();
    market.status = 200;
    await readOk(String(ORE));
    expect(market.asks).toHaveLength(6);
  });

  it("keep the copy they have when an answer isn't a table of rows", async () => {
    await readOk(String(ORE));
    for (const [prices, items] of [
      [[], []],
      [{ error: "session_required" }, { error: "session_required" }],
    ]) {
      await age(61 * 60_000);
      setMarket(prices, items);
      const answer = await readOk(String(ORE));
      expect(answer.prices?.items[ORE]).toBeDefined();
      expect(answer.listed?.items[ORE]).toBeDefined();
    }
  });

  it("have nothing while neither table was ever pulled", async () => {
    market.status = 503;

    expect(await readOk(String(ORE))).toEqual({ prices: null, listed: null });
  });

  it("refuse anything but up to 20 item IDs", async () => {
    const twenty = Array.from({ length: 20 }, (_, i) => 4010000 + i).join(",");

    for (const ids of ["", "abc", "4010000,", `${twenty},4010020`, "1234567890"]) {
      expect((await read(ids)).status, ids).toBe(400);
    }
    expect((await read(twenty)).status).toBe(200);
  });

  it("refuse an address past 2,000 items in 10 minutes, and no other", async () => {
    const twenty = Array.from({ length: 20 }, (_, i) => 4010000 + i).join(",");
    for (let i = 0; i < 100; i++) expect((await read(twenty, "198.51.100.7")).status).toBe(200);

    const refused = await read(String(ORE), "198.51.100.7");

    expect(refused.status).toBe(429);
    expect((await read(String(ORE), "198.51.100.8")).status).toBe(200);
  });
});
