// A new #1 on a Map Leaderboard is posted to Discord, and only when a Character really took
// the spot from another (ADR 0022): a first entry in an empty band, a Character beating its
// own best, a re-upload of the same set and an opt-out that hands the spot on all stay quiet.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import noPortrait from "../../art/portrait-none.png";
import { EMBEDS_PER_POST, NO_PORTRAIT, type Picture, type PostedMark, type Taken, type Top, pictureOf, postsFor, takenTops, tell } from "../announce";
import { entry, key, optOut, partyEntry, upload } from "./api";
import { classicRank, knownOnClassicRank, photoOf, pictureBytesOf } from "./classicrank";
import { discordWebhook } from "./discord-webhook";

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
const NO_MARKS = (): PostedMark | null => null;

/** A made-up portrait from ClassicRank, as a post shows it. */
const PORTRAIT: Picture = { bytes: pictureBytesOf("Hakuo").buffer as ArrayBuffer, type: "image/png", extension: "png", portrait: true };

/** A Character's name as a post writes it: a link to their page (ADR 0027). */
function linked(name: string): string {
  return `[${name}](https://owlofminerva.app/character/?name=${name})`;
}

/** The posts the made-up webhook (test/discord-webhook.ts) is sent in this test, and each
 *  one's files. */
const posts = discordWebhook.posts;
const files = discordWebhook.files;
let realFetch: typeof globalThis.fetch;

beforeEach(() => {
  posts.length = 0;
  files.length = 0;
  realFetch = globalThis.fetch;
});

afterEach(() => {
  globalThis.fetch = realFetch;
});

/** The posts made once the Worker has finished with them: a post is sent after the upload is
 *  answered, so a count is waited for rather than read straight away. */
function postsSettle(count: number): Promise<void> {
  return vi.waitFor(() => expect(posts).toHaveLength(count));
}

function top(map: string, band: number, name: string, expPerHour: number, party = false): Top {
  return { map, band, nameKey: name.toLowerCase(), name, expPerHour, party };
}

function taken(map: string, band: number, expPerHour = 90_000, party = false): Taken {
  return { map, band, name: "Hakuo", expPerHour, party, from: "Egbert", fromExpPerHour: 80_000 };
}

describe("taken top spots", () => {
  it("is a spot another Character held", () => {
    const before = [top(QUIET, 20, "Egbert", 80_000)];
    const after = [top(QUIET, 20, "Hakuo", 90_000)];

    expect(takenTops(before, after, "hakuo")).toEqual([
      { map: QUIET, band: 20, name: "Hakuo", expPerHour: 90_000, party: false, from: "Egbert", fromExpPerHour: 80_000 },
    ]);
  });

  it("says when the spot was taken with a Party Best", () => {
    const before = [top(QUIET, 20, "Egbert", 80_000, true)];
    const after = [top(QUIET, 20, "Hakuo", 90_000, true)];

    expect(takenTops(before, after, "hakuo").map((t) => t.party)).toEqual([true]);
  });

  it("is not a band nobody held", () => {
    expect(takenTops([], [top(QUIET, 20, "Hakuo", 90_000)], "hakuo")).toEqual([]);
  });

  it("is not the first Party entry when only the solo board had a holder", () => {
    expect(takenTops([top(QUIET, 20, "Egbert", 80_000)], [top(QUIET, 20, "Hakuo", 90_000, true)], "hakuo")).toEqual([]);
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
    expect(postsFor([], NO_MARKS)).toEqual([]);
  });

  it("is an embed for the spot, titled with the map as the site names it and linked to its Map Leaderboard", () => {
    const post = postsFor([taken(HUNTING_GROUND, 20, 153_216)], NO_MARKS)[0];

    expect(post).not.toHaveProperty("content");
    expect(post.embeds).toEqual([
      {
        title: "Henesys Hunting Ground I",
        url: `https://owlofminerva.app/leaderboards/map/?map=${HUNTING_GROUND}&band=20&name=Hakuo&kind=solo`,
        description:
          "**[Hakuo](https://owlofminerva.app/character/?name=Hakuo)** took #1 for levels 20–29 with 153,000 EXP/h. " +
          "[Egbert](https://owlofminerva.app/character/?name=Egbert) held it with 80,000.",
      },
    ]);
  });

  it("says when the spot was taken in a party", () => {
    const post = postsFor([taken(HUNTING_GROUND, 20, 153_216, true)], NO_MARKS)[0];

    expect(post.embeds[0].description).toBe(
      "**[Hakuo](https://owlofminerva.app/character/?name=Hakuo)** took #1 for levels 20–29 with 153,000 EXP/h in a party. " +
        "[Egbert](https://owlofminerva.app/character/?name=Egbert) held it with 80,000.",
    );
  });

  it("shows the map's Map Mark beside its area in the footer", () => {
    const markOf = (map: string) => (map === MARKED ? MADE_UP_MARK : null);

    const [marked, unmarked] = postsFor([taken(MARKED, 30, 90_000), taken(QUIET, 20, 80_000)], markOf)[0].embeds;

    expect(marked.footer).toEqual({ text: "Made-up Town", icon_url: MADE_UP_TOWN });
    expect(marked).not.toHaveProperty("thumbnail");
    expect(unmarked).not.toHaveProperty("footer");
  });

  it("shows the Character's portrait from a file of its own, with ClassicRank's credit after the Map Mark", () => {
    const markOf = (map: string) => (map === MARKED ? MADE_UP_MARK : null);

    const [post] = postsFor([taken(MARKED, 30, 90_000), taken(QUIET, 20, 80_000)], markOf, PORTRAIT);
    const [marked, unmarked] = post.embeds;

    expect(post.attachments).toEqual([
      { id: 0, filename: "portrait-1.png" },
      { id: 1, filename: "portrait-2.png" },
    ]);
    expect(marked.thumbnail).toEqual({ url: "attachment://portrait-1.png" });
    expect(marked.footer).toEqual({ text: "Made-up Town · Portrait: classicrank.com", icon_url: MADE_UP_TOWN });
    expect(marked.description).not.toContain("ClassicRank");
    expect(unmarked.thumbnail).toEqual({ url: "attachment://portrait-2.png" });
    expect(unmarked.footer).toEqual({ text: "Portrait: classicrank.com" });
  });

  it("shows the question-mark figure for a Character with no portrait, and says nothing of ClassicRank", () => {
    const markOf = (map: string) => (map === MARKED ? MADE_UP_MARK : null);

    const [post] = postsFor([taken(MARKED, 30, 90_000), taken(QUIET, 20, 80_000)], markOf, NO_PORTRAIT);
    const [marked, unmarked] = post.embeds;

    expect(post.attachments).toEqual([
      { id: 0, filename: "portrait-1.png" },
      { id: 1, filename: "portrait-2.png" },
    ]);
    expect(marked.thumbnail).toEqual({ url: "attachment://portrait-1.png" });
    expect(marked.footer).toEqual({ text: "Made-up Town", icon_url: MADE_UP_TOWN });
    expect(unmarked.thumbnail).toEqual({ url: "attachment://portrait-2.png" });
    expect(unmarked).not.toHaveProperty("footer");
    expect(JSON.stringify(post).toLowerCase()).not.toContain("classicrank");
  });

  it("shows no picture and has no files without one", () => {
    const [post] = postsFor([taken(QUIET, 20)], NO_MARKS);

    expect(post.attachments).toEqual([]);
    expect(post.embeds[0]).not.toHaveProperty("thumbnail");
  });

  it("names a map the fixture doesn't by its ID", () => {
    expect(postsFor([taken(QUIET, 20)], NO_MARKS)[0].embeds[0].title).toBe(`Map ${QUIET}`);
  });

  it("titles a map whose name holds Discord formatting as it is written", () => {
    // "G* Coconut Season" (090040203) is one in the map names fixture.
    expect(postsFor([taken("090040203", 20)], NO_MARKS)[0].embeds[0].title).toBe("G* Coconut Season");
  });

  it("pings nobody", () => {
    expect(postsFor([taken(QUIET, 20)], NO_MARKS)[0].allowed_mentions).toEqual({ parse: [] });
  });

  it("tells every spot an upload took as an embed, with no text", () => {
    const several = Array.from({ length: 3 }, (_, at) => taken(QUIET, (at + 2) * 10));

    const told = postsFor(several, NO_MARKS);

    expect(told).toHaveLength(1);
    expect(told[0].embeds).toHaveLength(3);
    expect(told[0]).not.toHaveProperty("content");
  });

  it("goes on in another post past what one Discord message holds, in order", () => {
    const many = Array.from({ length: EMBEDS_PER_POST + 2 }, (_, at) => taken(QUIET, (at + 2) * 10));

    const told = postsFor(many, NO_MARKS, PORTRAIT);

    expect(told.map((post) => post.embeds.length)).toEqual([EMBEDS_PER_POST, 2]);
    expect(told[1].embeds[0].description).toContain(`levels ${(EMBEDS_PER_POST + 2) * 10}–`);
    expect(told[1]).not.toHaveProperty("content");
    expect(told[1].attachments.map((a) => a.filename)).toEqual(["portrait-1.png", "portrait-2.png"]);
    expect(told[1].embeds[1].thumbnail).toEqual({ url: "attachment://portrait-2.png" });
  });
});

describe("the picture", () => {
  afterEach(() => {
    classicRank.known.clear();
    classicRank.photosDown = false;
  });

  it("is the Character's photo, from where ClassicRank's address for it leads", async () => {
    knownOnClassicRank("Hakuo");

    const picture = await pictureOf(photoOf("Hakuo"));

    expect(new Uint8Array(picture.bytes)).toEqual(pictureBytesOf("Hakuo"));
    expect(picture).toMatchObject({ type: "image/png", extension: "png", portrait: true });
  });

  it("is the question-mark figure for a Character with no photo", async () => {
    expect(await pictureOf(null)).toBe(NO_PORTRAIT);
    expect(new Uint8Array(NO_PORTRAIT.bytes)).toEqual(new Uint8Array(noPortrait));
  });

  it("is the question-mark figure when the photo won't load, or isn't a picture", async () => {
    knownOnClassicRank("Hakuo");
    classicRank.photosDown = true;

    expect(await pictureOf(photoOf("Hakuo"))).toBe(NO_PORTRAIT);
    // An address the made-up ClassicRank gives nothing at answers with its 404 page.
    expect(await pictureOf(photoOf("Nobody"))).toBe(NO_PORTRAIT);
  });

  it("is the question-mark figure when ClassicRank can't be reached", async () => {
    globalThis.fetch = (() => Promise.reject(new Error("unreachable"))) as typeof globalThis.fetch;

    expect(await pictureOf(photoOf("Hakuo"))).toBe(NO_PORTRAIT);
  });
});

describe("telling Discord", () => {
  it("posts nothing with no webhook set up", async () => {
    await tell({} as Env, [taken(QUIET, 20)]);

    expect(posts).toHaveLength(0);
  });

  it("posts each of several posts, in order", async () => {
    const many = Array.from({ length: EMBEDS_PER_POST + 1 }, (_, at) => taken(QUIET, (at + 2) * 10));

    await tell({ DISCORD_WEBHOOK_URL: WEBHOOK } as Env, many);

    expect(posts.map((post) => post.embeds.length)).toEqual([EMBEDS_PER_POST, 1]);
  });

  it("sends each embed's picture as a file of the post", async () => {
    await tell({ DISCORD_WEBHOOK_URL: WEBHOOK, CLASSICRANK_KEY: "crk_made-up-key" } as Env, [taken(QUIET, 20), taken(QUIET, 30)]);

    expect(files[0].map((f) => [f.field, f.name, f.type])).toEqual([
      ["files[0]", "portrait-1.png", "image/png"],
      ["files[1]", "portrait-2.png", "image/png"],
    ]);
    expect(files[0][1].bytes).toEqual(new Uint8Array(noPortrait));
  });

  it("shows no picture without a ClassicRank key, as the site shows no portraits", async () => {
    await tell({ DISCORD_WEBHOOK_URL: WEBHOOK } as Env, [taken(QUIET, 20)]);

    expect(posts[0].embeds[0]).not.toHaveProperty("thumbnail");
    expect(files[0]).toEqual([]);
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
    expect(posts[0].embeds[0].description).toContain(`${linked("Egbert")} held it with 80,000`);
    expect(posts[0].embeds[0].url).toContain(`map=${CROWNED}&band=20&name=Hakuo`);
    // CROWNED has no Map Mark in the made-up ceilings, and ClassicRank has no photo of Hakuo.
    expect(posts[0].embeds[0]).not.toHaveProperty("footer");
    expect(posts[0].embeds[0].thumbnail).toEqual({ url: "attachment://portrait-1.png" });
    expect(files[0][0].bytes).toEqual(new Uint8Array(noPortrait));
  });

  it("posts each kind's taken top spot even below the other board's top", async () => {
    const map = "100000058";
    await upload("Kora", key("kora"), [entry(map, 25, 10_000), partyEntry(map, 25, 80_000)]);
    await upload("Lars", key("lars"), [partyEntry(map, 25, 90_000)]);
    await postsSettle(1);
    expect(posts[0].embeds[0].description).toContain("**[Lars](https://owlofminerva.app/character/?name=Lars)** took #1");
    expect(posts[0].embeds[0].description).toContain("in a party");
    expect(posts[0].embeds[0].url).toContain("kind=party");

    // His Solo Best takes the solo board, although his Party Best is higher.
    await upload("Lars", key("lars"), [entry(map, 25, 20_000), partyEntry(map, 25, 90_000)]);
    await postsSettle(2);
    expect(posts[1].embeds[0].description).not.toContain("in a party");
    expect(posts[1].embeds[0].description).toContain("[Kora](https://owlofminerva.app/character/?name=Kora) held it with 10,000");
    expect(posts[1].embeds[0].url).toContain("kind=solo");

    await upload("Kora", key("kora"), [entry(map, 25, 50_000), partyEntry(map, 25, 80_000)]);
    await postsSettle(3);
    expect(posts[2].embeds[0].description).toContain("**[Kora](https://owlofminerva.app/character/?name=Kora)** took #1");
    // Improving her own solo top, even past the party top, takes no new spot.
    await upload("Kora", key("kora"), [entry(map, 25, 95_000), partyEntry(map, 25, 80_000)]);
    await upload("Lars", key("lars"), [entry(map, 25, 96_000), partyEntry(map, 25, 90_000)]);
    await postsSettle(4);
    expect(posts[3].embeds[0].description).toContain("[Kora](https://owlofminerva.app/character/?name=Kora) held it with 95,000");
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
    expect(posts[0].embeds[0].footer).toEqual({ text: "Made-up Town", icon_url: MADE_UP_TOWN });
  });

  it("shows the portrait ClassicRank gives the Character that took the spot", async () => {
    knownOnClassicRank("Xena");
    try {
      await upload("Yuri", key("yuri"), [entry(MARKED, 35, 10_000)]);
      await upload("Xena", key("xena"), [entry(MARKED, 35, 20_000)]);

      await postsSettle(1);
      expect(posts[0].embeds[0].thumbnail).toEqual({ url: "attachment://portrait-1.png" });
      expect(files[0][0].bytes).toEqual(pictureBytesOf("Xena"));
      expect(posts[0].embeds[0].footer).toEqual({ text: "Made-up Town · Portrait: classicrank.com", icon_url: MADE_UP_TOWN });
    } finally {
      classicRank.known.clear();
    }
  });

  it("shows the portrait kept from before when ClassicRank is busy as the Character uploads", async () => {
    knownOnClassicRank("Zora");
    try {
      await upload("Zora", key("zora"), [entry(MARKED, 45, 10_000)]);
      await upload("Uma", key("uma"), [entry(MARKED, 45, 20_000)]);
      await postsSettle(1);
      classicRank.busy = true;
      await upload("Zora", key("zora"), [entry(MARKED, 45, 30_000)]);

      await postsSettle(2);
      expect(posts[1].embeds[0].description).toContain(`**${linked("Zora")}** took #1`);
      expect(files[1][0].bytes).toEqual(pictureBytesOf("Zora"));
      expect(posts[1].embeds[0].footer).toEqual({ text: "Made-up Town · Portrait: classicrank.com", icon_url: MADE_UP_TOWN });
    } finally {
      classicRank.known.clear();
      classicRank.busy = false;
    }
  });
});
