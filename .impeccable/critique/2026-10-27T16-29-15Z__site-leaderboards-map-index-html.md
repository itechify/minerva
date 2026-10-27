---
target: "PR #182 portraits on the map band and Character page"
total_score: 24
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 3
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-739e11b2\\site\\leaderboards\\map\\index.html"
target_fingerprint: "sha256:8d717fd2a503af29799af1ebc0bda5cc2b80997176bbb232509030a9b10b65fa"
target_path: "C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-739e11b2\\site\\leaderboards\\map\\index.html"
timestamp: 2026-10-27T16-29-15Z
slug: site-leaderboards-map-index-html
---
Method: dual-agent (A: design review sub-agent · B: detector + browser sub-agent)

Target: PR #182's new additions — Character portraits on a map's Map Leaderboard band (site/leaderboards/map/index.html, map.ts, leaderboards.ts, site.css) and the Character page header (site/character/index.html, page.ts), plus site/art/portrait-none.png. Checked live at 1280, 800, 390, 375, 360 and 340px.

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|---|---|---|
| 1 | Visibility of System Status | 2 | "Waiting" (not asked yet) draws nothing: an empty 96px hole that reads as a broken image; a failed photo and "no photo" look the same |
| 2 | Match System / Real World | 4 | The game's own sprites, pixel for pixel; a ? figure the game would use; rank/face/name/job as players know it |
| 3 | User Control and Freedom | 2 | A portrait is changed or removed only on ClassicRank, within a day (by design, ADR 0029); nothing on the board says so |
| 4 | Consistency and Standards | 2 | Credit misaligned with the caption; band header not sticky where Where to train's is; screen readers get 7 headers over 8 cells |
| 5 | Error Prevention | 2 | Deploy day: every row "waiting" until 05:17 UTC; phone table breaks names and levels mid-word |
| 6 | Recognition Rather Than Recall | 2 | The ? figure has no title or hint on the page; only the FAQ explains it |
| 7 | Flexibility and Efficiency | 2 | Rows 39px → 105px; the lanes' ruler and the headers scroll away after ~7 rows |
| 8 | Aesthetic and Minimalist Design | 3 | Clean at 1280/800; the near-black ? figure is the heaviest thing on the board |
| 9 | Error Recovery | 2 | A failed ask leaves a row a hole until the next day, with nothing said |
| 10 | Help and Documentation | 3 | FAQ answers are honest and complete; nothing on the board points to them |
| **Total** | | **24/40** | **Acceptable** |

## Design Specificity Verdict

LLM assessment: authored for this product. The sprites at their own pixels, crowns, class icons and the EXP Lime lanes read as an in-game ranking board; the ? figure is MapleStory's own idiom, standing at a character's size with its feet where a photo's are. The Character page header (portrait, name, job line, credit) is the strongest new composition on the site. Generic spots: the "waiting" state (an undesigned blank) and the credit line (a stock footnote).

Deterministic scan: `impeccable detect` on the two pages: 3 warnings, all in map/index.html (broken-image and buried-raster on #map-image, cramped-padding on the Report dialog), identical on main, all false positives (the map picture gets its src by script and fades in; the dialog's content is inset by its children). Browser overlay at 1280 and 390 on three pages: ai-color-palette on the minimap fold button and nested-cards on the window bar, both site-wide and pre-existing; nothing flagged on any portrait element. The detector agrees with the review that the PR introduces no mechanical anti-patterns; the browser measurement confirms the review's phone overflow (table box scrollWidth 357 vs 354 at 390).

## Overall Impression

The idea lands: the sprites make the boards feel like the game, and finding your own starred row with your own sprite is the payoff. What drags it to 24 is states and edges, not the concept: deploy day shows a board of blank squares, the phone table overflows and splits names, the portrait column mislabels every cell for screen readers, and 105px rows send the lanes' ruler off-screen.

## What's Working

1. Pixel honesty: photos never enlarged or smoothed (`object-fit: scale-down`, `image-rendering: pixelated`), a 96px column at the photos' own size.
2. The Character page header: a 96px / 1fr grid with the portrait spanning name, job and credit; works at 390; absent without a key.
3. The phone merge: "Character / Job" with "Cleric Lv 25" under the name keeps column headers on phones, scoped to portrait bands so forks keep the old cards.

## Priority Issues

**[P1] "Waiting" is an empty hole, and on deploy day it fills every board.** Until the first daily run, every Character is `portrait: null`; `bandTable` turns portraits on for null too (leaderboards.ts:415), so every board becomes 105px rows of blank 96px squares under a "Portraits from ClassicRank" credit with no portraits. A failed upload ask leaves a single row like that for a day. Fix: turn the column on only when some entry has an answer (photo or none); give the waiting square a visible look (inset tint frame); run the refresh by hand right after the deploy and note it in the setup doc. → /impeccable harden

**[P1] The phone table overflows and splits words at ordinary widths.** Table wider than its box at 390/375/360/340 (by 3–10px); at 360 "Nobodyxy/z123", "Cleric/Lv 25" and the header "Character / / Job". Cause: `overflow-wrap: anywhere` lets auto layout squeeze the name column; rank + face take 144px. Fix: " Lv N" nowrap, icon kept with the job's first word, header as "Character" over "Job", `break-word` not `anywhere`, trim the rank cell and the face cell's side padding. → /impeccable adapt

**[P1] The portrait column shifts every column header by one for screen readers.** The face th is aria-hidden but each face td is role="cell": 7 headers over 8 cells, so "EXP/h, 25", "Date, 84,600". Fix: aria-hidden the face cell and drop its role (it is decoration). → /impeccable audit

**[P2] Rows 2.7× taller; the lanes' ruler and headers scroll away.** 50 rows ≈ 5,250px; the lanes lose their scale after ~7 rows. Fix: sticky band thead (ruler included) from 641px, as Where to train's. → /impeccable layout

**[P2] The ? figure is the loudest thing on the board and unexplained.** Solid #121010 near-black, heavier than any sprite; alt="" and no title. Fix: quieter rendering (reduced opacity or tinted toward Ink), `title="No photo on ClassicRank"`, a different title for a photo that didn't load; a faint inset-tint square behind every portrait helps waiting, opaque photos and the figure at once. → /impeccable quieter

## Persona Red Flags

**Sam (screen reader):** every figure mislabelled by the shifted headers; the Character page's profile link is only the image, with no visible link cue.

**Casey (360px Android):** "Nobodyxy/z123", "Cleric/Lv 25", "Character / / Job"; the table box slides 3px under a vertical swipe; no lanes or dates, so distance to #1 is numbers only.

**Riley (stress tester):** deploy day boards of blank squares; a 140×60 photo shrinks unevenly and renders jagged under pixelated; a 16-character name on the Character page at 360 breaks into three lines beside the portrait.

## Minor Observations

- Credit starts at x=80, caption at x=92; align or join them on one line.
- The Character page's profile link is only the image; consider pointing the visible credit link at the profile when there is one.
- `loading="lazy"` also applies to the Character page's above-the-fold header portrait.
- "Dragon Knight" wraps on desktop while the name column has room.
- The face cell's 12px side padding separates the portrait from its name.
- Untested with a long band: the colSpan-9 gap row and a linked Character below #50, desktop and phone.
- ADR 0029 quotes "Portraits from ClassicRank" for the Character page, which says "Portrait from ClassicRank".

## Questions to Consider

- If most Minerva players have no ClassicRank photo, does the column become a stack of black silhouettes?
- With #30 about 3,000px below #1, does "see at a glance how far each entry is from #1" still hold?
- Was the reference layout ever tested with 16-character names?
- Is a 96px blank ever better than the compact row it replaces?
