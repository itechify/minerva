// A Character's portrait from ClassicRank (ADR 0029): asked for when they upload and every
// day after, kept with their entries and shown wherever they are, until ClassicRank stops
// showing them or they opt out.
import { createExecutionContext, createScheduledController, waitOnExecutionContext } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CharacterAnswer, LatestAnswer, PortraitAnswer } from "../../entries";
import worker from "../index";
import { admin, band, call, entry, FIELD, key, optOut, upload } from "./api";
import { asked, classicRank, knownOnClassicRank, photoOf, profileOf } from "./classicrank";

beforeEach(() => {
  classicRank.known.clear();
  classicRank.busy = false;
  classicRank.asks.length = 0;
});

async function page(name: string): Promise<CharacterAnswer> {
  const response = await call(`/api/characters/${name}`);
  if (response.status !== 200) throw new Error(`page read: ${response.status}`);
  return response.json();
}

async function latestOf(name: string) {
  const { entries } = (await (await call("/api/latest")).json()) as LatestAnswer;
  return entries.find((e) => e.name === name);
}

/** `name`'s portrait as their page gives it. */
async function portraitOf(name: string): Promise<PortraitAnswer | undefined> {
  return (await page(name)).portrait;
}

/** Wait until ClassicRank has been asked about `name`, `times` times in all. */
function askedAbout(name: string, times = 1): Promise<void> {
  return vi.waitFor(() => expect(asked().flat().filter((n) => n === name.toLowerCase())).toHaveLength(times));
}

/** The daily refresh, as the cron trigger runs it. */
async function refresh(): Promise<void> {
  const ctx = createExecutionContext();
  await worker.scheduled(createScheduledController({ cron: "0 5 * * *", scheduledTime: Date.now() }), env, ctx);
  await waitOnExecutionContext(ctx);
}

const SHOWN = (name: string) => ({ photo: photoOf(name), profile: profileOf(name) });

describe("a portrait", () => {
  it("is asked for when a Character uploads, and shows on their page and in their band, but not in Latest", async () => {
    knownOnClassicRank("Aria");

    await upload("Aria", key("aria"), [entry(FIELD, 21, 10_000)]);

    await vi.waitFor(async () => expect(await portraitOf("Aria")).toEqual(SHOWN("Aria")));
    expect((await band(FIELD, 20)).entries.find((e) => e.name === "Aria")?.portrait).toEqual(SHOWN("Aria"));
    expect((await band(FIELD, 20, "Aria")).character?.portrait).toEqual(SHOWN("Aria"));
    expect(await latestOf("Aria")).not.toHaveProperty("portrait");
  });

  it("is asked for with the ClassicRank key, by the name as uploaded, for the fields it shows", async () => {
    await upload("Bodo", key("bodo"), [entry(FIELD, 31, 10_000)]);

    await askedAbout("Bodo");
    expect(classicRank.asks[0]).toMatchObject({ names: ["Bodo"], key: "crk_made-up-key" });
    expect(classicRank.asks[0].fields?.split(",")).toEqual(expect.arrayContaining(["name", "photo", "profile_url", "name_cut_off"]));
  });

  it("is null until ClassicRank has answered, and the upload goes through without it", async () => {
    knownOnClassicRank("Cleo");
    classicRank.busy = true;

    const response = await upload("Cleo", key("cleo"), [entry(FIELD, 41, 10_000)]);

    expect(await response.json()).toEqual({ accepted: 1, rejected: [] });
    await askedAbout("Cleo");
    expect(await portraitOf("Cleo")).toBeNull();
  });

  it("has no photo for a Character ClassicRank has none for, and keeps their profile", async () => {
    knownOnClassicRank("Dara", null);

    await upload("Dara", key("dara"), [entry(FIELD, 51, 10_000)]);

    await vi.waitFor(async () => expect(await portraitOf("Dara")).toEqual({ photo: null, profile: profileOf("Dara") }));
  });

  it("is nothing for a Character ClassicRank doesn't show", async () => {
    await upload("Emil", key("emil"), [entry(FIELD, 61, 10_000)]);

    await vi.waitFor(async () => expect(await portraitOf("Emil")).toEqual({ photo: null, profile: null }));
  });

  it("is nothing for a name ClassicRank cuts short, which might be another Character's", async () => {
    knownOnClassicRank("Fennimore", photoOf("Fennimore"), { name_cut_off: true });

    await upload("Fennimore", key("fennimore"), [entry(FIELD, 71, 10_000)]);

    await vi.waitFor(async () => expect(await portraitOf("Fennimore")).toEqual({ photo: null, profile: null }));
  });

  it("matches the name whatever its case", async () => {
    knownOnClassicRank("gUNNAR");

    await upload("Gunnar", key("gunnar"), [entry(FIELD, 81, 10_000)]);

    await vi.waitFor(async () => expect(await portraitOf("Gunnar")).toEqual(SHOWN("gUNNAR")));
  });

  it("takes only a photo and a profile on classicrank.com", async () => {
    knownOnClassicRank("Hedda", "javascript:alert(1)", { profile_url: "https://elsewhere.invalid/Hedda" });

    await upload("Hedda", key("hedda"), [entry(FIELD, 91, 10_000)]);

    await vi.waitFor(async () => expect(await portraitOf("Hedda")).toEqual({ photo: null, profile: null }));
  });

  it("goes when the Character opts out", async () => {
    knownOnClassicRank("Ilse");
    await upload("Ilse", key("ilse"), [entry(FIELD, 101, 10_000)]);
    await vi.waitFor(async () => expect(await portraitOf("Ilse")).toEqual(SHOWN("Ilse")));

    await optOut("Ilse", key("ilse"));
    classicRank.busy = true;
    await upload("Ilse", key("ilse"), [entry(FIELD, 101, 10_000)]);

    await askedAbout("Ilse", 2);
    expect(await portraitOf("Ilse")).toBeNull();
  });

  it("is left out of every answer while the Worker has no ClassicRank key", async () => {
    knownOnClassicRank("Jola");
    await upload("Jola", key("jola"), [entry(FIELD, 111, 10_000)]);
    await askedAbout("Jola");
    const saved = env.CLASSICRANK_KEY;
    env.CLASSICRANK_KEY = undefined;
    try {
      await upload("Jola", key("jola"), [entry(FIELD, 111, 20_000)]);

      expect(await page("Jola")).not.toHaveProperty("portrait");
      expect((await band(FIELD, 110, "Jola")).character).not.toHaveProperty("portrait");
      expect(asked().flat()).toEqual(["jola"]);
    } finally {
      env.CLASSICRANK_KEY = saved;
    }
  });
});

describe("the daily refresh", () => {
  it("runs at once when the maintainer asks, as after the deploy that sets the key", async () => {
    await upload("Odin", key("odin"), [entry(FIELD, 151, 10_000)]);
    await askedAbout("Odin");
    knownOnClassicRank("Odin");

    const response = await admin("/portraits", { method: "POST" });

    expect(response.status).toBe(204);
    expect(await portraitOf("Odin")).toEqual(SHOWN("Odin"));
  });

  it("is the maintainer's alone to run at once", async () => {
    expect((await admin("/portraits", { method: "POST" }, "not-the-token")).status).toBe(401);
  });

  it("asks about every Character with entries, 300 names at a time", { timeout: 60_000 }, async () => {
    classicRank.busy = true;
    const names = Array.from({ length: 301 }, (_, i) => `Many${i}`);
    for (const name of names) await upload(name, key(name), [entry("100000070", 25, 10_000 + names.indexOf(name))]);
    await vi.waitFor(() => expect(classicRank.asks).toHaveLength(names.length));
    for (const name of names) knownOnClassicRank(name);
    classicRank.busy = false;
    classicRank.asks.length = 0;

    await refresh();

    // The Characters this file's other tests made are asked about too, after the first 300.
    expect(classicRank.asks.map((a) => a.names.length)[0]).toBe(300);
    expect(classicRank.asks).toHaveLength(2);
    expect(classicRank.asks.map((a) => a.limit)).toEqual(classicRank.asks.map((a) => String(a.names.length)));
    expect(asked().flat()).toEqual(expect.arrayContaining(names.map((n) => n.toLowerCase())));
    expect((await band("100000070", 20)).entries.every((e) => e.portrait?.photo === photoOf(e.name))).toBe(true);
  });

  it("takes a changed photo, and clears a Character ClassicRank no longer shows", async () => {
    knownOnClassicRank("Kai");
    knownOnClassicRank("Lio");
    await upload("Kai", key("kai"), [entry(FIELD, 121, 10_000)]);
    await upload("Lio", key("lio"), [entry(FIELD, 121, 20_000)]);
    await vi.waitFor(async () => expect(await portraitOf("Lio")).toEqual(SHOWN("Lio")));

    knownOnClassicRank("Kai", photoOf("Kai", "2026-10-27 05:44:54"));
    classicRank.known.delete("lio");
    await refresh();

    expect((await portraitOf("Kai"))?.photo).toBe(photoOf("Kai", "2026-10-27 05:44:54"));
    expect(await portraitOf("Lio")).toEqual({ photo: null, profile: null });
  });

  it("leaves every portrait as it was while ClassicRank is busy", async () => {
    knownOnClassicRank("Mona");
    await upload("Mona", key("mona"), [entry(FIELD, 131, 10_000)]);
    await vi.waitFor(async () => expect(await portraitOf("Mona")).toEqual(SHOWN("Mona")));

    classicRank.known.clear();
    classicRank.busy = true;
    await refresh();

    expect(await portraitOf("Mona")).toEqual(SHOWN("Mona"));
  });

  it("asks nothing while the Worker has no ClassicRank key", async () => {
    await upload("Nele", key("nele"), [entry(FIELD, 141, 10_000)]);
    await askedAbout("Nele");
    classicRank.asks.length = 0;
    const saved = env.CLASSICRANK_KEY;
    env.CLASSICRANK_KEY = undefined;
    try {
      await refresh();
    } finally {
      env.CLASSICRANK_KEY = saved;
    }

    expect(classicRank.asks).toEqual([]);
  });
});
