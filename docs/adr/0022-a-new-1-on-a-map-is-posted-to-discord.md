---
status: accepted, amended by ADR-0024, ADR-0026, ADR-0027, ADR-0029
---

# A new #1 on a map is posted to Discord

The site's Worker posts a Top spot taken to a Discord webhook. It announces an achievement from the upload that caused it, without making Discord a dependency of uploading.

## The decision

- **The Worker posts it, from the upload that caused it.** The upload route already has the write in hand; a top spot taken is worked out in the same request and posted to a **Discord webhook**, the URL a Worker secret (`DISCORD_WEBHOOK_URL`, `docs/map-leaderboards-setup.md`). With no secret set, nothing is posted and uploads are unchanged, so a fork or a local run never posts anywhere.
- **Only a spot *taken* is posted**: the Character holds a map and Level Band's top spot **of that kind** after the write, and **another** Character held it before. Solo and Party boards are checked separately; a Solo top below the Party top still counts. A Party spot says "in a party". The link opens the board of that kind, while the site's default Both board still ranks each Character by their higher entry. That one rule is what keeps the channel worth reading, and it rules out everything that isn't an achievement:
  - the **first entry** in a band nobody had an entry in, which is #1 by being alone;
  - a Character **beating its own best**, which moves no one;
  - a top spot that **fell to someone** because an entry was deleted, opted out or dropped from an upload, which they never took;
  - the **same set uploaded again**, which the app sends as every Visit ends.
- **Reads are kept off the common upload.** Only an entry that is new, or higher than the one it replaces, can take a top spot, so the Worker compares the upload with the Character's entries it already holds and reads the top spots for those places alone, before and after the write. Nearly every upload has none and reads nothing more. The rule above would turn the rest down anyway; this is what makes it cheap.
- **An embed per spot, ten to a post, and no text.** A Character mirroring a whole Visit History for the first time can take several at once, so the post names each, an **embed** apiece, best first. Ten is the most Discord holds in one message, so an upload that takes more goes on in another post, sent after the first so they show in order; that is rare enough not to crowd the channel. A post has no text of its own: the embeds say it all. A post never mentions or pings anyone (`allowed_mentions: { parse: [] }`). An embed's title is the map's name, linking to its Map Leaderboard, and its thumbnail the Character's portrait, or a question-mark figure for none (ADR 0029). The map's **Map Mark** is in the footer. The link is the title's, never in the text, so Discord shows no preview under a post that names several.
- **The post is a nicety, never a cost to the upload.** It is handed to `ctx.waitUntil`, so the upload is answered without waiting for Discord, and a webhook that is gone, rate-limited or slow is swallowed and never retried. The Map Leaderboards are the record; a post that didn't go is not.
- **A post says nothing the site doesn't.** The Character's name, the map, the Level Band, the EXP/h and whose spot it was all come from the entry, which is public (ADR 0015). A player who takes their Character off the Map Leaderboards is never posted, and the FAQ answers why a name turned up (`leaderboard-discord`).

Map names come from `site/map-names.ts`, rates from `src/figures.ts`. Each embed has its own map, band and kind link: Discord folds embeds with the same link into one. ADR 0029 adds the Character's portrait as the thumbnail, sent with the post as a file, and moves the Map Mark to the footer.

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
- Names a player uploads now also appear in a Discord channel, which is a change a player notices: the FAQ says so (`leaderboard-public`, `leaderboard-discord`).
- The webhook URL is a secret in the same sense as `ADMIN_TOKEN`: whoever holds it can post to that channel. It is never committed, and a lost one is fixed by deleting the webhook in Discord.
