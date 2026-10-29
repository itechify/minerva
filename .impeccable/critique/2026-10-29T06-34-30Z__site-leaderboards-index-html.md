---
target: Map Leaderboards page
total_score: 25
max_score: 36
na_heuristics: 5
p0_count: 0
p1_count: 3
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-ff19b196\\site\\leaderboards\\index.html"
target_fingerprint: "sha256:414eabb45a47a4b8631d8c5fdffe8809db9910ae36a1a2f1b7ecb6dedcd14e07"
target_path: "C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-ff19b196\\site\\leaderboards\\index.html"
timestamp: 2026-10-29T06-34-30Z
slug: site-leaderboards-index-html
---
Method: dual-agent (A: abbfa4718f5fe8788 · B: aa865cd6e1cf2fc4b). Mode: Operate.

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|---|---|---|
| 1 | Visibility of System Status | 3 | Loading lines and kept room are good; a Pedestal's empty 3rd step doesn't say why it's empty |
| 2 | Match System / Real World | 4 | Game terms throughout: Level Bands, Map Marks, job icons, the Party mark |
| 3 | User Control and Freedom | 3 | Band, kind and area are links with history; Show all can't be collapsed |
| 4 | Consistency and Standards | 2 | "Most podiums" / "Podiums Pedestal" / "pedestals" name one thing; 34em left-aligned Pedestal over a 60em table |
| 5 | Error Prevention | n/a | A page you read; nothing on it can go wrong |
| 6 | Recognition Rather Than Recall | 2 | "Top 3 in a band" and the question-mark portrait are explained only in hover tooltips |
| 7 | Flexibility and Efficiency | 3 | Shareable links, Enter opens a search result; no "my band" shortcut |
| 8 | Aesthetic and Minimalist Design | 2 | EXP/h Pedestal's 1st repeats the table's first row; title art plus two Pedestals all draw a podium |
| 9 | Error Recovery | 3 | One plain line and Try again when the API can't be reached |
| 10 | Help and Documentation | 3 | "How they work" folds the right questions, far below the controls it explains |
| **Total** | | **25/36** | **Good (69%), on the edge of Acceptable** |

## Design Specificity Verdict

Authored for this product (Map Marks, job icons, crowns, Lv bands, light-steel chips, inset rows). The Pedestals are its least specific part: pale inset slabs with a 14px crown under title art that already draws a stone 2-1-3 podium with the owl on it. Detector: static scan clean (0); live scans found ai-color-palette (the minimap's fold button, documented --game-button), nested-cards (the site's window frame) and text-overflow (the deliberately shrunk "Mob levels" header on phones), all false positives or intentional, none about the Pedestals. The --cdp scan is unsupported by the installed CLI; URL mode was used instead. No overlay (headless only).

## Priority Issues

- [P1] The primary task is below the fold: at 1280x800 the first screen is title art, lede and Most podiums; Where to train is off-screen. Fix: put #podiums in the head's empty right half next to the title art; after Where to train on narrow screens. (/impeccable layout)
- [P1] The EXP/h Pedestal separates the choices from their result: ~270px between the area chips and the table, its 1st usually the table's first row. Fix: beside the table's caption from 900px up, or after the table; compact, without 96px portraits. (/impeccable layout)
- [P1] The podium shape flips on phones: at 390px the 3rd (more wrapped text) stands higher than the 2nd, since .lb-stand-who grows upward and the blocks differ by only 20/14px. Fix: one shared height for the steps' upper parts so only the blocks set the outline, or job/level/map under the block. (/impeccable adapt)
- [P2] Keyboard focus order doesn't match the screen (DOM 1-2-3, CSS order 2-1-3; WCAG 2.4.3). Fix: build the li's in screen order with the hidden "1st:" labels, or ol value attributes. (/impeccable harden)
- [P2] Facts break across lines: "Bandit / Lv 27" on the Pedestal and phones, a dot hanging after "Cleric Lv 26 ·" in the table. Fix: nowrap on each "Job Lv N" and date; date on its own line. (/impeccable polish)

## Persona Red Flags

- Jordan: meets "Most podiums" before Level Bands are explained, and the line under it defines a podium in terms of Level Bands; question-mark figure and "Top 3 in a band" are tooltip-only; "Yours could be here" reads like a caption.
- Casey: scrolls past title art, the Podiums Pedestal, three rows of bands, kinds, two rows of areas and a second Pedestal before the first map card; 3rd stands taller than 2nd.
- Sam: tab order doesn't follow the podium; the podium count's tooltip can't be reached by keyboard or touch; the ranking itself reads correctly.

## Minor Observations

- The 1st step's block is a large empty pale slab with less weight than the title art's stone podium.
- "Portraits from ClassicRank" appears twice in a band view.
- The area picker shows 6 to 9 choices on two rows.
- DESIGN.md names the Pedestal only in the crown rule; its layout isn't described.

## Questions to Consider

- What if the real top three stood on the title art's podium instead of a second, paler one below it?
- Does the EXP/h Pedestal tell the player anything the table's first rows don't?
- Should the page open on a returning player's band?
