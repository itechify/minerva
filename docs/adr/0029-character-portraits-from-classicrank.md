---
status: accepted
---

# Character portraits from ClassicRank

The Map Leaderboards (ADR 0015) and a Character's page (ADR 0027) show a Character as a name, a job and a level, and nothing of how they look. [ClassicRank](https://classicrank.com) ranks Classic World's characters and keeps a photo of each one its players have shown it, with an API for other tools. Classic World is one world, so a name on the Map Leaderboards is the same character there. This records how Minerva shows those photos: a **portrait** beside each name.

## The decision

- **If ClassicRank shows a Character, so does Minerva.** The photo is already public there, and so is the name. A player who hides their character on ClassicRank (its API never returns a hidden one) drops off here too, at the next ask. This changes ADR 0027's "what the Map Leaderboards hold of them, and no more": a Character's page shows one thing their entries don't, their portrait, and links to their profile on ClassicRank. It still ties no name to another, nor to an Account.
- **Only the Worker asks ClassicRank**, with its key (a Partner read key, the `CLASSICRANK_KEY` Worker secret, `docs/map-leaderboards-setup.md`). ClassicRank says a key never goes in a page. Without the key there is no other way in: keyless asks are limited per IP, and the Worker's outgoing IPs are shared, while asking from each visitor's browser would spend the limit all keyless callers share on every page view. A visitor's browser never calls the API.
- **It asks when a Character uploads, and every day.** Each upload asks for its Character's portrait (one name), then any top spot it took is posted to Discord with it (ADR 0022). Once a day a cron trigger (`site/wrangler.jsonc`) asks again for every Character with entries, 300 names at a time, the most one ask may hold with a Partner key. A changed photo shows within a day, and so does a Character ClassicRank stops showing. An ask that fails (a 429, an unknown key, ClassicRank down) changes nothing: the upload is answered either way, and the next day's ask catches up.
- **D1 keeps what ClassicRank answered** (`portraits`, migration 0004): the photo's address and the profile's, each `NULL` when ClassicRank has none, and when it was asked. A name is matched whatever its case. A name ClassicRank cuts short (`name_cut_off`) is taken as not shown, since the full name might be another Character's. An address that isn't on classicrank.com is taken as none, since the pages show it as a picture and a link. The row goes when the Character opts out.
- **Browsers load the photo from classicrank.com.** The address ClassicRank gives (`/api/avatar/<name>?v=...`) is new whenever the photo changes, and redirects to the PNG, cached for a year, from any origin. Nothing is copied. A visitor's browser does fetch it from classicrank.com, which the FAQ says.
- **Where it shows**: in a map's band, at its own 96px in a column of its own before the names, each row as tall as a portrait; and 96px beside the name on a Character's page, a link to their ClassicRank profile when they have one. On a phone a band stays a table under its headers, its portraits still 96px: Rank, the portrait, the name over the job and level, EXP/h and Report, without its EXP lanes, so it fits. Latest shows none, as a feed of what's new, and nor does Where to train: some 200 small pictures would crowd it. A photo is drawn pixel for pixel at its own size, shrunk only to fit a smaller square (ClassicRank's aren't all 96×96).
- **Three looks, and none.** A Character ClassicRank has a photo of shows it; one it has asked about and has no photo of, or doesn't show, shows a question-mark figure (`site/art/portrait-none.png`), as does a photo that won't load; one not asked about yet keeps the portrait's room, empty. While the Worker has no key, the answers carry no portraits and the pages show none at all, so a fork or a local run looks as it did.
- **ClassicRank is credited wherever its portraits show**, as it asks: a visible link to classicrank.com under a band's table and on a Character's page ("Portraits from ClassicRank"), and in the site's credits. A Discord post with a portrait ends " · Portrait: [ClassicRank](https://classicrank.com)".
- **On Discord, the portrait is the thumbnail.** The Character is the news, so a #1's portrait takes the embed's thumbnail, and the map's Map Mark moves to the footer, beside the area it stands for. A #1 without a portrait posts as before, its Map Mark the thumbnail.

## The API

The reads that give a Character give their portrait, as `portrait`, while the Worker has the key, and leave it out without one: each entry of `GET /api/maps/<map>/bands/<band>` (and its `character`), and `GET /api/characters/<name>`. It is `{ "photo": "https://classicrank.com/api/avatar/Egbert?v=...", "profile": "https://classicrank.com/user/Egbert" }`, either `null` for none, or `null` itself until ClassicRank has been asked. Latest's entries and Where to train's `best` have none.

## Considered options

- **Asking from the visitor's browser, without a key.** No secret to keep, but the keyless limit (120 asks a minute shared by every keyless caller anywhere, 20 names an ask) breaks as soon as a few people browse, and each visitor's IP would reach ClassicRank's API.
- **Copying each photo into R2** and serving it from owlofminerva.app. Visitors would never reach classicrank.com, but ClassicRank's terms don't say copies are allowed, and a hidden character's copy would have to be deleted. Kept as the fallback if ClassicRank asks us not to load its photos from its site.
- **A player opting in from their Account or the app** before their portrait shows. ClassicRank's own hiding already gives them a way out, and an opt-in would leave the boards almost bare.
- **Only `claimed` or `verified` characters on ClassicRank.** Most characters there are neither, and those flags are about ClassicRank's own accounts, not whether a photo may be shown.
- **Asking on every page view**, with a cache. Every read would wait on ClassicRank, and the reads are cached a minute anyway.
- **A rename followed.** ClassicRank follows a renamed character from its old name in a single lookup, but not promised in a `names=` ask. Minerva treats a new name as a new Character (ADR 0027: no name is tied to another), so the old name's portrait goes and the new name's comes.
- **More from ClassicRank**: a "verified" badge, or checking an entry's level against ClassicRank's. ClassicRank's level is today's, not the one the entry was set at, so it can't check one. A badge, and sending ClassicRank the levels Minerva reads (its Ingest API), are left for issues of their own.

## Consequences

- A new public dependency: the Map Leaderboards show what ClassicRank shows, and change when it does. With ClassicRank down, portraits keep their last answer, and photos that won't load show the question-mark figure.
- The Worker has its first cron trigger, and its tests a made-up ClassicRank (`site/worker/test/classicrank.ts`) in place of the real one: nothing from ClassicRank is committed.
- A Character's page is no longer only their entries: the GLOSSARY's Character page and the FAQ's answers on what's public say so.
