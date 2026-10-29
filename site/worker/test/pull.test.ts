// Pulling a Character's entries (#206): before an upload would replace what the site holds
// with fewer (the bests file lost, or a new PC), the app gets what the site holds, with the key
// that holds the name, and merges it in.
import { describe, expect, it } from "vitest";
import { FIELD, RICH_FIELD, entry, key, partyEntry, pull, upload } from "./api";
import { link, signIn, withMadeUpDiscord } from "./accounts";

withMadeUpDiscord();

describe("pulling a Character's entries", () => {
  it("gives the key that holds the name every entry, as uploaded", async () => {
    const entries = [entry(FIELD, 14, 20_000, 1_760_000_000_000), partyEntry(RICH_FIELD, 23, 400_000, 1_760_100_000_000)];
    await upload("Quill", key("quill"), entries);

    const response = await pull("Quill", key("quill"));

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ entries });
  });

  it("gives none for a name no key holds, and leaves it unclaimed", async () => {
    const response = await pull("Rook", key("rook-first"));

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ entries: [] });
    expect((await upload("Rook", key("rook-second"), [entry(FIELD, 12, 10_000)])).status).toBe(200);
  });

  it("refuses another key, as an upload would be", async () => {
    await upload("Sable", key("sable"), [entry(FIELD, 15, 21_000)]);

    const response = await pull("Sable", key("impostor"));

    expect(response.status).toBe(409);
    expect(await response.json()).toEqual({ error: "claimed" });
  });

  it("gives a key linked to the name's Account the entries another PC uploaded", async () => {
    const session = await signIn("200000000000000206", "tansy");
    await link(key("tansy-old-pc"), session);
    await upload("Tansy", key("tansy-old-pc"), [entry(FIELD, 16, 22_000)]);
    await link(key("tansy-new-pc"), session);

    const response = await pull("Tansy", key("tansy-new-pc"));

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ entries: [entry(FIELD, 16, 22_000)] });
  });

  it("refuses a pull with no Leaderboard Key", async () => {
    await upload("Umber", key("umber"), [entry(FIELD, 17, 23_000)]);

    const response = await pull("Umber");

    expect(response.status).toBe(401);
    expect(await response.json()).toEqual({ error: "no-key" });
  });

  it("is never cached, since it reads a key", async () => {
    await upload("Vale", key("vale"), [entry(FIELD, 18, 24_000)]);

    const response = await pull("Vale", key("vale"));

    expect(response.headers.get("cache-control")).toBe("no-store");
  });
});
