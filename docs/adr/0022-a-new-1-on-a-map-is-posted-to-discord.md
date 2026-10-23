---
status: accepted
---

# A new #1 on a map is posted to Discord

The Map Leaderboards (ADR 0015) are worth looking at, but nothing tells anyone when they change: a player sees their own rank in the Personal Bests window, and no one else ever hears about it. Minerva's Discord is where the small community is, and an empty channel makes a server look dead. This records how a new #1 gets there.

## The decision

- **The Worker posts it, from the upload that caused it.** The upload route already has the write in hand; a top spot taken is worked out in the same request and posted to a **Discord webhook**, the URL a Worker secret (`DISCORD_WEBHOOK_URL`, `docs/map-leaderboards-setup.md`). With no secret set, nothing is posted and uploads are unchanged, so a fork or a local run never posts anywhere.
- **Only a spot *taken* is posted**: the Character holds a map and Level Band's top spot after the write, and **another** Character held it before. That one rule is what keeps the channel worth reading, and it rules out everything that isn't an achievement:
  - the **first entry** in a band nobody had an entry in, which is #1 by being alone;
  - a Character **beating its own best**, which moves no one;
  - a top spot that **fell to someone** because an entry was deleted, opted out or dropped from an upload, which they never took;
  - the **same set uploaded again**, which the app sends as every Visit ends.
- **Reads are kept off the common upload.** Only an entry that is new, or higher than the one it replaces, can take a top spot, so the Worker compares the upload with the Character's entries it already holds and reads the top spots for those places alone, before and after the write. Nearly every upload has none and reads nothing more. The rule above would turn the rest down anyway; this is what makes it cheap.
- **One post per upload, at most three spots named.** A Character mirroring a whole Visit History for the first time can take several at once, so the post names the best three and counts the rest. A post never mentions or pings anyone (`allowed_mentions: { parse: [] }`), and the map's name is a masked link to its Map Leaderboard, so Discord shows no preview under a post that names several.
- **The post is a nicety, never a cost to the upload.** It is handed to `ctx.waitUntil`, so the upload is answered without waiting for Discord, and a webhook that is gone, rate-limited or slow is swallowed and never retried. The Map Leaderboards are the record; a post that didn't go is not.
- **A post says nothing the site doesn't.** The Character's name, the map, the Level Band, the EXP/h and whose spot it was all come from the entry, which is public (ADR 0015). A player who takes their Character off the Map Leaderboards is never posted, and the FAQ answers why a name turned up (`leaderboard-discord`).

A post reads:

```
**Hakuo** took #1 on [Henesys Hunting Ground I](https://owlofminerva.app/leaderboards/map/?map=010001010&band=20&name=Hakuo), levels 20–29, with 153,000 EXP/h, past Egbert's 150,000.
```

The map is named from the map names fixture (ADR 0008) through `site/map-names.ts`, which the Worker and the Map Leaderboards' pages now share as they share `site/level-bands.ts`, so a map is named the same wherever it shows. EXP/h is written by `src/figures.ts`'s `rate`, as the panel and the site write rates. A map's name is escaped for Discord's markdown before it goes in the link, since the fixture holds one with a `*` in it ("G\* Coconut Season"); a Character's name is letters and digits only (ADR 0015), so it never needs it.

**Release posts** need no code: a GitHub repo webhook delivering only the *Releases* event to `<discord-webhook>/github` lets Discord render the release itself. It is set up on the repository, not in it.

## Considered options

- **A bot with a Discord token.** It could answer commands and post richer embeds, but it needs a token, a gateway connection and something to run it. A webhook is one secret and one POST from a Worker that is already there.
- **A scheduled job that diffs the leaderboards.** A Cron Trigger comparing snapshots would catch a #1 that changed for any reason, deletions included. But it needs a snapshot table, it posts late, and "your entry fell off so you're #1 now" is exactly the post nobody wants.
- **Posting every entry, or every top-10 change.** More to read, and on a small server it would drown the channel within a day. #1 taken is rare enough to stay interesting.
- **Posting from the app.** The PC that set the best could post it, but then the webhook URL would ship in the app, where anyone could take it and post anything.
- **Telling the player in the app instead.** Worth doing on its own, but it reaches only the one player, and the point here is to show the Map Leaderboards to people who haven't opened them.

## Consequences

- The Worker's `fetch` now takes its `ExecutionContext`, and `upload` takes it to hand the post to. A direct call without one waits for the post instead, so nothing is left running after the answer.
- An upload that takes a top spot costs two extra D1 reads. One that doesn't costs none beyond the Character's own entries, which the write already needed.
- `MapNameRow`, `MapPlace`, `MapNames`, `mapNames` and `placeOf` moved from `site/leaderboards/leaderboards.ts` to `site/map-names.ts`, and are re-exported from where they were, so the pages' imports are unchanged.
- The map names fixture is now bundled into the Worker (about 31 KB), which is the first data the Worker carries beyond the ceilings table.
- Names a player uploads now also appear in a Discord channel, which is a change a player notices: the FAQ says so (`leaderboard-public`, `leaderboard-discord`).
- The webhook URL is a secret in the same sense as `ADMIN_TOKEN`: whoever holds it can post to that channel. It is never committed, and a lost one is fixed by deleting the webhook in Discord.
