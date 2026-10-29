---
version: 1
slug: "site-leaderboards-map-index-html"
primary_target: "site/leaderboards/map/index.html"
related_targets: ["site/leaderboards/map.ts","site/leaderboards/leaderboards.ts","site/site.css"]
---

# A map's Map Leaderboard

Scope: a map's page on the site (site/leaderboards/map/index.html, site/leaderboards/map.ts, the logic in site/leaderboards/leaderboards.ts, the styles in site/site.css): the map, its Level Bands, and one band's top 50 by EXP per hour, with the linked Character's entry marked even outside them. Mode: Operate. Read at a pause from the game, often from the Personal Bests window's link, to see where a Character stands on a map and what others at their level get there.

Job: see at a glance how far each entry is from #1 and from the player's own, and move between Level Bands without losing the thread. Constraints: the site's opaque window at up to 1200px, the table up to 60em in the hotkey table's inset box, cards under 640px without portraits; portrait bands keep a horizontally scrollable table, 96px portraits and no lanes or dates; at most 51 rows; the You Yellow star on the linked Character's entry; read from the site's API by script.

## Lanes and morphing bands (overdrive, 15 Oct 2026)

Chosen by the user. The critique of 15 Oct found the board generic ("could be any game's leaderboard... Nothing draws EXP the way the game or the Personal Bests window does"); this answers it with the Personal Bests window's own drawing, and makes switching bands, the page's one control, show who moved. Nothing new is asked of the API.

- **Lanes.** Each entry gets the Personal Bests window's lane after EXP/h: its EXP/h as the game's lime EXP gauge in its grey frame, on one linear scale from 0 to the band's #1, whose lane runs full. Read down the page, the table is a chart. The header over the lanes is a ruler at three to five round steps past 0, thinned where the lanes are short. The linked Character's row keeps its star and periwinkle, and its lane is on the same scale; outside the top 50 it is drawn under the "more entries" row.
- **Room.** The lane column from 900px; from 641px to 899px the lane goes under its figure, as the Personal Bests window's does under 840px; in phone cards without portraits it runs the card's width under the figures. Portrait bands stay tables on phones and omit lanes and dates.
- **The EXP Lime Rule** now covers these lanes: EXP, and nothing else, so the player's own lane is lime like everyone's.
- **Morphing bands.** A click on a Level Band (and Back and Forward between them) switches the page in place with a same-document View Transition and pushes the address, so the link still shares. Characters in both bands slide to their new rank, each row by a sanitised name of its own; the rest cross-fades; the lanes regrow from the left. 300ms on DESIGN.md's exponential ease-out, `transform` and `opacity` only; the sticky head held over the moving rows.
- **Fallbacks.** No View Transitions, reduced motion, or the browser's own swipe-back animation: the switch is at once. Script off: the page says it needs it, as before. First load and deep links never move. Nothing runs on a timer.
- **Logic is tested.** The scale, the ruler's steps, each lane's fraction and the rows two bands share are plain functions in leaderboards.ts (`bandTable`'s `ruler` and each row's `lane`, `morphNames`), tested in site/leaderboards/test/lanes.test.ts and morph.test.ts.
- **Checked** in the browser at 1280, 900, 700 and 375px, on made-up entries: Henesys Hunting Ground I with 60 in Lv 20–29 (the linked Character #60) and 18 in Lv 30–39, 12 of them in both; a switch held mid-transition, Back and Forward, a deep link outside the top 50, reduced motion and no View Transitions.

## Map picture (15 Oct 2026)

Under the map's name, before the Level Bands, show the whole OSMS map picture fitted on Minimap Paper in a shallow inset. Read the content hash from OSMS's live map manifest and load the picture independently of the rankings. Neither file is bundled. Keep the frame's height through loading, missing and failed pictures, and explain each state in plain Ink Steel.
