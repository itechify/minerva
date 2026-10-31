---
status: accepted
---

# Entries far above a busy map's #1 are reported automatically

An EXP bar misread, or an upload made without the app, can put an entry on a Map Leaderboard that no one earned. The cap (ADR 0015) turns down only the wildest, over 5× the map's solo EXP/h ceiling, and a ceiling assumes every mob dies as it spawns: The Murky Swamp I's is 575,000 EXP/h, where good runs make around 100,000, so a misread at 150,000 passes it. The Worker now reports such an entry to the maintainer by itself, judged against what the map's other players have done.

## The decision

- **The Worker judges an entry at upload**, when it's new or higher than the Character's entry of the same map, band and kind. The app sends every entry on every upload, so one left as it was is never judged again, and a report the maintainer dismissed doesn't come back. A higher best is judged afresh.
- **It's reported when all three hold:**
  1. its map has at least **8 entries of other Characters**, of every band and both kinds. Below that, a map has too little to say what's normal, and its entries are left to players' reports;
  2. its band and kind has **another Character's entry** to compare it with;
  3. its EXP/h is **more than 2×** the best of those: the band's #1, leaving the Character's own entries out.

  The entries counted are those the public boards show: a blocked Character's, or one taken down, count for nothing.
- **A report, never a rejection.** The entry stays on the Map Leaderboards, and a top spot it takes is posted to Discord as ever (ADR 0022). The maintainer deals with it in `/admin/`'s Reports as with a player's (ADR 0031): Dismiss, Take entry down, Look up, Block. It reads "Flagged by Minerva" where a player's reads "Reported", and its reason names the #1 it passed: "2.2× the #1 it passed: Bob's 43,536 EXP/h." (rounded down, and to two decimals just over the line, "2.00×", so it never reads as more than it is). An automatic report stands only while its entry does, at the EXP/h it was made at: a new best, reported again or not, the entry dropped from an upload or the Character opted out takes it away, so an entry has one at most and the maintainer never acts on a figure that's gone. Players' reports are left alone.
- **Players aren't told.** Nothing changes for them, and saying where the line is would tell a cheater to stay under it.

## Why these numbers

On the boards of 30 October 2026, 98 maps held 281 entries, 57 of them with only one or two Characters; 26 bands held 3 or more, every one of them Party. There, #1 stood up to 3.16× #2, and the three widest gaps (2.11×, 2.91×, 3.16×) were all real play: a Rogue ahead of Magicians, a level 19 ahead of a 14, level 8 and 9 Beginners. Within one band, job and level open 2–3× gaps by themselves, so 2× reports some real runs. That costs the maintainer a few seconds and the player nothing, where a misread that drops or doubles a digit lands far past it. Only three maps had 8 entries of others then: the check starts small and widens as the boards fill.

## The API

As ADR 0031, beside it:

- `GET /api/admin/reports`: each report gains `automatic`, `true` for one the Worker made and `false` for a player's.
- `PUT /api/characters/<name>/entries` answers as before: the player's upload never says it was reported.

## Considered options

- **A lower multiple of the ceiling.** The table is there already (`probe ceilings`), but a ceiling is only as near real play as the map's mobs are easy to kill: real bests run at 20–40% of one, so any multiple that spares honest runs on an easy map misses a misread on a hard one.
- **The band's median.** It would judge entries at any rank, but trips on every band with a wide spread (61k, 58k, 25k, 14k), and below #1 an entry misleads no one about what a map can do.
- **Comparing Solo and Party, or neighbouring bands, together.** More entries to judge by, but a party run and a level ten higher are different play: the comparison would be unfair, and a thin band is left to reports anyway.
- **Reporting every #1 on a map too quiet to judge.** Today that's most of them; the maintainer would be reviewing the boards by hand.
- **Holding a flagged entry, or its Discord post, until it's reviewed.** The player would think the upload failed, or a real #1 on a busy map would be announced late. A bad entry taken down leaves the boards right; the post stays as a history (ADR 0031).
- **Catching misreads in the app.** Only the app sees the run itself: the bar's readings, the Pickup Log's EXP, the map's mobs. But it keeps no Pickup Log EXP per kill, the Session never sees each mob's EXP, and no Bug Report holds the hunting to tune a tolerance by. Left for later (#213). The Worker's check is the one a forged upload can't skip.

## Consequences

- An upload that raises an entry costs one more D1 read per 30 such entries; one that raises none costs nothing more.
- Migration 0008 adds `reports.automatic`. A Worker from before it ignores the column, and lists an automatic report as a player's.
- A map's first 8 entries of others go unjudged, as do bands with no other entry: those rest on players' reports, as before.
