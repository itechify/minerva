// A new #1 on a Map Leaderboard is posted to Discord, and only when a Character really took
// the spot from another (ADR 0022): a first entry in an empty band, a Character beating its
// own best, a re-upload of the same set and an opt-out that hands the spot on all stay quiet.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MOST_TOLD, PORTRAIT_CREDIT, type Post, type PostedMark, type Taken, type Top, postFor, takenTops, tell } from "../announce";
import { entry, key, optOut, upload } from "./api";
import { classicRank, knownOnClassicRank, photoOf } from "./classicrank";

/** The made-up webhook the test config sets (vitest.config.ts). */
const WEBHOOK = "https://discord.invalid/api/webhooks/made-up/webhook";

/** Maps of their own, so no other test's entries are in their bands. Henesys Hunting Ground I
 *  is a real map, to show a post names a map as the site does. Of them only MARKED has a Map
 *  Mark in the made-up ceilings (vitest.config.ts). */
const HUNTING_GROUND = "010001010";
const QUIET = "100000050";
const CROWNED = "100000051";
const MARKED = "100000060";

/** The made-up Map Mark's picture (vitest.config.ts). */
const MADE_UP_TOWN = "https://osms.invalid/images/map_marks/MadeUpTown.png";

/** The made-up Map Mark, as a post shows it. */
const MADE_UP_MARK: PostedMark = { name: "Made-up Town", picture: MADE_UP_TOWN };

/** No map has a Map Mark. */
const NO_PICTURES = (): PostedMark | null => null;

/** A Character's name as a post writes it: a link to their page (ADR 0027). */
function linked(name: string): string {
  return `[${name}](https://owlofminerva.app/character/?name=${name})`;
}

let posts: Post[];
let realFetch: typeof globalThis.fetch;

beforeEach(() => {
  posts = [];
  realFetch = globalThis.fetch;
  // Only the webhook is taken off: D1 and the rest of the runtime keep the real fetch.
  globalThis.fetch = ((input: RequestInfo | URL, init?: RequestInit) => {
    const url = input instanceof Request ? input.url : String(input);
    if (!url.startsWith(WEBHOOK)) return realFetch(input as RequestInfo, init);
    posts.push(JSON.parse(String(init?.body)) as Post);
    return Promise.resolve(new Response(null, { status: 204 }));
  }) as typeof globalThis.fetch;
});

afterEach(() => {
  globalThis.fetch = realFetch;
});

/** The posts made once the Worker has finished with them: a post is sent after the upload is
 *  answered, so a count is waited for rather than read straight away. */
function postsSettle(count: number): Promise<void> {
  return vi.waitFor(() => expect(posts).toHaveLength(count));
}

function top(map: string, band: number, name: string, expPerHour: number): Top {
  return { map, band, nameKey: name.toLowerCase(), name, expPerHour };
}

function taken(map: string, band: number, expPerHour = 90_000): Taken {
  return { map, band, name: "Hakuo", expPerHour, from: "Egbert", fromExpPerHour: 80_000 };
}

describe("taken top spots", () => {
  it("is a spot another Character held", () => {
    const before = [top(QUIET, 20, "Egbert", 80_000)];
    const after = [top(QUIET, 20, "Hakuo", 90_000)];

    expect(takenTops(before, after, "hakuo")).toEqual([
      { map: QUIET, band: 20, name: "Hakuo", expPerHour: 90_000, from: "Egbert", fromExpPerHour: 80_000 },
    ]);
  });

  it("is not a band nobody held", () => {
    expect(takenTops([], [top(QUIET, 20, "Hakuo", 90_000)], "hakuo")).toEqual([]);
  });

  it("is not a Character beating its own best", () => {
    const before = [top(QUIET, 20, "Hakuo", 80_000)];
    const after = [top(QUIET, 20, "Hakuo", 90_000)];

    expect(takenTops(before, after, "hakuo")).toEqual([]);
  });

  it("is not a spot another Character took", () => {
    const before = [top(QUIET, 20, "Egbert", 80_000)];
    const after = [top(QUIET, 20, "Berta", 90_000)];

    expect(takenTops(before, after, "hakuo")).toEqual([]);
  });

  it("names the best first", () => {
    const before = [top(QUIET, 20, "Egbert", 10_000), top(CROWNED, 30, "Egbert", 10_000)];
    const after = [top(QUIET, 20, "Hakuo", 20_000), top(CROWNED, 30, "Hakuo", 500_000)];

    expect(takenTops(before, after, "hakuo").map((t) => t.map)).toEqual([CROWNED, QUIET]);
  });
});

describe("the post", () => {
  it("is nothing with no spot taken", () => {
    expect(postFor([], NO_PICTURES)).toBeNull();
  });

  it("is an embed for the spot, titled with the map as the site names it and linked to its Map Leaderboard", () => {
    const post = postFor([taken(HUNTING_GROUND, 20, 153_216)], NO_PICTURES)!;

    expect(post.content).toBeUndefined();
    expect(post.embeds).toEqual([
      {
        title: "Henesys Hunting Ground I",
        url: `https://owlofminerva.app/leaderboards/map/?map=${HUNTING_GROUND}&band=20&name=Hakuo`,
        description:
          "**[Hakuo](https://owlofminerva.app/character/?name=Hakuo)** took #1, levels 20–29, with 153,000 EXP/h, " +
          "past [Egbert](https://owlofminerva.app/character/?name=Egbert)'s 80,000.",
      },
    ]);
  });

  it("shows the map's Map Mark", () => {
    const markOf = (map: string) => (map === MARKED ? MADE_UP_MARK : null);

    const [marked, unmarked] = postFor([taken(MARKED, 30, 90_000), taken(QUIET, 20, 80_000)], markOf)!.embeds;

    expect(marked.thumbnail).toEqual({ url: MADE_UP_TOWN });
    expect(marked).not.toHaveProperty("footer");
    expect(unmarked).not.toHaveProperty("thumbnail");
  });

  it("shows the Character's portrait, the Map Mark under it beside its area, and credits ClassicRank", () => {
    const markOf = (map: string) => (map === MARKED ? MADE_UP_MARK : null);

    const [marked, unmarked] = postFor([taken(MARKED, 30, 90_000), taken(QUIET, 20, 80_000)], markOf, photoOf("Hakuo"))!.embeds;

    expect(marked.thumbnail).toEqual({ url: photoOf("Hakuo") });
    expect(marked.footer).toEqual({ text: "Made-up Town", icon_url: MADE_UP_TOWN });
    expect(marked.description.endsWith(" · Portrait: [ClassicRank](https://classicrank.com)")).toBe(true);
    expect(unmarked.thumbnail).toEqual({ url: photoOf("Hakuo") });
    expect(unmarked).not.toHaveProperty("footer");
  });

  it("says nothing of ClassicRank with no portrait", () => {
    expect(postFor([taken(QUIET, 20)], NO_PICTURES)!.embeds[0].description).not.toContain(PORTRAIT_CREDIT);
  });

  it("names a map the fixture doesn't by its ID", () => {
    expect(postFor([taken(QUIET, 20)], NO_PICTURES)!.embeds[0].title).toBe(`Map ${QUIET}`);
  });

  it("titles a map whose name holds Discord formatting as it is written", () => {
    // "G* Coconut Season" (090040203) is one in the map names fixture.
    expect(postFor([taken("090040203", 20)], NO_PICTURES)!.embeds[0].title).toBe("G* Coconut Season");
  });

  it("pings nobody", () => {
    expect(postFor([taken(QUIET, 20)], NO_PICTURES)!.allowed_mentions).toEqual({ parse: [] });
  });

  it("counts the rest once an upload takes more than it names", () => {
    const many = Array.from({ length: MOST_TOLD + 2 }, (_, at) => taken(QUIET, (at + 2) * 10));

    const post = postFor(many, NO_PICTURES)!;

    expect(post.embeds).toHaveLength(MOST_TOLD);
    expect(post.content).toBe("…and 2 more top spots in the same upload.");
  });

  it("counts one left over as a spot", () => {
    const many = Array.from({ length: MOST_TOLD + 1 }, (_, at) => taken(QUIET, (at + 2) * 10));

    expect(postFor(many, NO_PICTURES)!.content).toBe("…and 1 more top spot in the same upload.");
  });
});

describe("telling Discord", () => {
  it("posts nothing with no webhook set up", async () => {
    await tell({} as Env, [taken(QUIET, 20)]);

    expect(posts).toHaveLength(0);
  });

  it("lets a webhook that fails pass", async () => {
    globalThis.fetch = (() => Promise.reject(new Error("no webhook"))) as typeof globalThis.fetch;

    await expect(tell({ DISCORD_WEBHOOK_URL: WEBHOOK } as Env, [taken(QUIET, 20)])).resolves.toBeUndefined();
  });
});

describe("an upload", () => {
  it("posts when it takes a top spot", async () => {
    await upload("Egbert", key("egbert"), [entry(CROWNED, 25, 80_000)]);
    await upload("Hakuo", key("hakuo"), [entry(CROWNED, 23, 90_000)]);

    await postsSettle(1);
    expect(posts[0].embeds[0].description).toContain(`**${linked("Hakuo")}** took #1`);
    expect(posts[0].embeds[0].description).toContain(`past ${linked("Egbert")}'s 80,000`);
    expect(posts[0].embeds[0].url).toContain(`map=${CROWNED}&band=20&name=Hakuo`);
    // CROWNED has no Map Mark in the made-up ceilings.
    expect(posts[0].embeds[0]).not.toHaveProperty("thumbnail");
  });

  it("posts nothing for the same set uploaded again", async () => {
    const set = [entry(QUIET, 25, 80_000)];
    await upload("Dora", key("dora"), set);
    await upload("Emil", key("emil"), [entry(QUIET, 22, 90_000)]);
    await postsSettle(1);

    // The app uploads a Character's whole set as each Visit ends, so this is almost every
    // upload: nothing changed, so nothing is posted.
    await upload("Emil", key("emil"), [entry(QUIET, 22, 90_000)]);
    await upload("Dora", key("dora"), set);

    // Only a real take is posted, so the next one proves the two above posted nothing.
    await upload("Fritz", key("fritz"), [entry(QUIET, 27, 95_000)]);
    await postsSettle(2);
    expect(posts[1].embeds[0].description).toContain(`**${linked("Fritz")}** took #1`);
  });

  it("posts nothing for the first entry in a band", async () => {
    await upload("Greta", key("greta"), [entry("100000052", 25, 80_000)]);

    await upload("Horst", key("horst"), [entry("100000053", 25, 10_000)]);
    await upload("Ilse", key("ilse"), [entry("100000053", 25, 20_000)]);
    await postsSettle(1);
    expect(posts[0].embeds[0].description).toContain(`**${linked("Ilse")}** took #1`);
  });

  it("posts nothing when a Character beats its own best", async () => {
    await upload("Jonas", key("jonas"), [entry("100000054", 25, 10_000)]);
    await upload("Klara", key("klara"), [entry("100000054", 25, 20_000)]);
    await postsSettle(1);

    await upload("Klara", key("klara"), [entry("100000054", 25, 60_000)]);

    await upload("Lena", key("lena"), [entry("100000054", 25, 70_000)]);
    await postsSettle(2);
    expect(posts[1].embeds[0].description).toContain(`**${linked("Lena")}** took #1`);
  });

  it("posts nothing when a top spot falls to someone because an entry went", async () => {
    await upload("Mara", key("mara"), [entry("100000055", 25, 10_000)]);
    await upload("Nils", key("nils"), [entry("100000055", 25, 90_000)]);
    await postsSettle(1);

    // Nils opting out leaves Mara on top, which Mara never took.
    await optOut("Nils", key("nils"));

    await upload("Olga", key("olga"), [entry("100000055", 25, 95_000)]);
    await postsSettle(2);
    expect(posts[1].embeds[0].description).toContain(`**${linked("Olga")}** took #1`);
  });

  it("posts nothing for an entry the cap rejects", async () => {
    // 100000000's made-up ceiling is 100,000, so its cap is 500,000.
    await upload("Pia", key("pia"), [entry("100000000", 25, 400_000)]);
    await upload("Rolf", key("rolf"), [entry("100000000", 25, 900_000)]);

    await upload("Sven", key("sven"), [entry("100000000", 25, 450_000)]);
    await postsSettle(1);
    expect(posts[0].embeds[0].description).toContain(`**${linked("Sven")}** took #1`);
  });

  it("posts one message for several spots taken at once", async () => {
    const held = [entry("100000056", 25, 10_000), entry("100000057", 25, 10_000)];
    await upload("Tina", key("tina"), held);

    await upload("Udo", key("udo"), [entry("100000056", 25, 20_000), entry("100000057", 25, 30_000)]);

    await postsSettle(1);
    expect(posts[0].embeds).toHaveLength(2);
  });

  it("shows the Map Mark the ceilings table gives the map", async () => {
    await upload("Vera", key("vera"), [entry(MARKED, 25, 10_000)]);
    await upload("Wim", key("wim"), [entry(MARKED, 25, 20_000)]);

    await postsSettle(1);
    expect(posts[0].embeds[0].thumbnail).toEqual({ url: MADE_UP_TOWN });
  });

  it("shows the portrait ClassicRank gives the Character that took the spot", async () => {
    knownOnClassicRank("Xena");
    try {
      await upload("Yuri", key("yuri"), [entry(MARKED, 35, 10_000)]);
      await upload("Xena", key("xena"), [entry(MARKED, 35, 20_000)]);

      await postsSettle(1);
      expect(posts[0].embeds[0].thumbnail).toEqual({ url: photoOf("Xena") });
      expect(posts[0].embeds[0].footer).toEqual({ text: "Made-up Town", icon_url: MADE_UP_TOWN });
    } finally {
      classicRank.known.clear();
    }
  });
});
