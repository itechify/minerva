---
target: the Character page
total_score: 26
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 2
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-4cbf9e83\\site\\character\\index.html"
target_fingerprint: "sha256:d03dd969ff444228af6f0bccb43c4a704a9baf74dd6aab393f07afb2d3fb0c24"
target_path: "C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-4cbf9e83\\site\\character\\index.html"
timestamp: 2026-10-26T03-05-41Z
slug: site-character-index-html
closed: true
---
Method: dual-agent (A: design review sub-agent · B: detector + browser sub-agent)

# Critique: the Character page (site/character/index.html, with its empty state and the site 404)

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Loading and unreachable lines are right; the h1 flashes "Character" over an empty block while loading, and nothing announces the bests arriving |
| 2 | Match System / Real World | 3 | Bands, Map Marks, "#2 of 3" speak the player's language; "3 / #1s held" reads as two numerals in a row |
| 3 | User Control and Freedom | 3 | Back link, per-row deep links and Back all work; a many-band Character is a long scroll with no way to jump to a band |
| 4 | Consistency and Standards | 2 | You Yellow means "the Character you followed here" on a map's page but "#1" (the crown) here; figure boxes and crown aren't in DESIGN.md |
| 5 | Error Prevention | 3 | Case-insensitive lookup; impossible names rejected before any request |
| 6 | Recognition Rather Than Recall | 3 | Sticky headers and "of N" carry context; phone cards put labels back |
| 7 | Flexibility and Efficiency | 2 | No band jump, no gap to #1; sizing up a rival means clicking map by map |
| 8 | Aesthetic and Minimalist Design | 2 | Four equal boxes with no hierarchy; Best EXP/h repeats row one; Job repeats down each band; long map names make 3-line rows at 1280px |
| 9 | Error Recovery | 2 | The empty state explains in one small grey line and offers no next step |
| 10 | Help and Documentation | 3 | FAQ inline and on topic; lacks "Where's my character's entry?", the question an empty state raises |
| **Total** | | **26/40** | **Good, with real gaps** |

## Design Specificity Verdict

LLM assessment: split. The tables are this product's own: Map Mark and name over street, "Lv 30–39" band heads, right-aligned figures written as the Grind Tracker writes them, "Map 999999999 / Not in the map list", sticky heads, the owl, and each row handing off to the map's board, which judges. The header is generic gaming-profile furniture (name, grey subline, four equal bordered stat boxes). It ignores the language the product already built for exactly this content: the Personal Bests window, which reads one Character's bests across maps as EXP-lime lanes on one scale, with one You Yellow star on the best of all.

Deterministic scan: CLI detect found nothing in either site/character/index.html or site/404.html (exit 0). In-page, the same three findings appeared on every view, none in this PR's markup: ai-color-palette on the hidden 0×0 mini map fold button (site/minimap.ts:100, false positive); nested-cards on the shared header.window-bar (pre-existing, every page); dark-glow (#ffba00), source not found in the site's CSS, most likely the detector's own overlay (unresolved).

Visual overlays: injected into a headless Chrome page only; no user-visible [Human] tab.

## Overall Impression

The tables are good, specific work; the header is where the page goes generic, and it's also where the fun was meant to land. For most Characters it opens on "0 #1s held / 0 Podiums", and for a #1 holder the crown is a 14px glyph in a table while the figure box beside it weighs the same as three others. Biggest opportunity: make the top of the page the Personal Bests window opened wide for someone else.

## What's Working

- Every row is a doorway: the map links to its board at that band with the Character starred and their rank said on top. The page summarises, the board judges.
- The domain language is exact: mark and name over street, years only when they differ from now, unknown maps named as the app names them; careful cell labels ("#1 of 2, the band's top spot") and the figures as a definition list.
- Privacy restraint is designed in: no other names, every no-entries case reads the same, and the FAQ tells the subject the page exists.

## Priority Issues

[P1] Phone cards overflow on long map names. At 390px "Tree Dungeon, Forest Up North VII" pushes 300,000 past the card edge; dates go ragged; a 7-figure EXP/h is clipped. Cause: the phone grid's max-content tracks size the map span to its unbroken name. Fix: auto auto minmax(0, 1fr) auto, min-width: 0 on .ch-map, justify-self: end on .ch-exp and .ch-date. Command: /impeccable adapt

[P1] The header has no hierarchy and leads with zeros. Four equal boxes; for most Characters the first two say 0; Best EXP/h repeats row one. Fix: a crown-led standing sentence in the map page's voice, dropping zero clauses, or the Personal Bests window's language (lanes on every row, one star on the best of all). Record the choice in DESIGN.md. Command: /impeccable layout (or shape for the lanes)

[P2] You Yellow means two things: the followed Character's star on a map's page, every #1's crown here. Fix: crown in ink or the EXP gauge's gold, or keep You Yellow for one best-of-all mark. Command: /impeccable colorize

[P2] Trivial #1s and podiums count in full ("#1 of 1" crowned; "#2 of 2", last place, a podium). Fix: count a #1 only on a board of 2+, a podium on a board of 4+, or write "#1 of 1" as "Only entry" in grey; say the rule in the FAQ. Command: /impeccable clarify

[P2] The empty state is a dead end. The owl (as chosen) over one small grey line, no way on; alt "404 Not Found" read on a 200 page headed with a real name. Fix: keep the owl; Body Ink line with a reason and a link to #leaderboard-link; that question in the page's FAQ list; alt="" when the line names the Character. Command: /impeccable onboard

## Persona Red Flags

Jordan (first-timer from a Discord #1 post): generic link card (#175), no lit tab, h1 flashes "Character"; first figure usually "0 #1s held"; "Podium" only explained as "Top 3 in a band"; the crowned row is a 14px glyph.

Alex (power user sizing up a rival): "#2 of 2" with no gap to #1; no band index (6 bands ≈ 3,300px); Job repeats while rank gets the narrowest room; the Best EXP/h box's map isn't a link.

Casey (mobile): the card overflow; the 2×2 figure boxes push the first entry below the fold at 390×844.

Sam (screen reader): hears "404 Not Found" on a Character's empty page; no announcement when bests load; each band's heading and caption say nearly the same thing. Rank labels and row headers are done well.

## Minor Observations

- The h1 doesn't wrap: a 16-character name overflows 390px by about 90px. Add overflow-wrap: anywhere to #name.
- Fixed desktop columns take 576px; Rank (104px) and Job (120px) could give back 30–40px.
- "4 / Maps / in 6 Level Bands" reads oddly; "on 4 maps, across 6 Level Bands".
- The subline comes from the newest entry, which can contradict the band shown first.
- "How they work" is ambiguous on a one-Character page; its list ends on taking your character off.
- Inherited from the map page: the caption repeats the band; no row hover tint.
- Measured: secondary text 7.06:1, subline 6.35:1; no horizontal overflow at 600px; 404 answers 404, the empty state 200; titles update to the name; the only console error is the API's deliberate 404.

## Questions to Consider

- Why isn't this page the Personal Bests window opened wide, with lanes and one star?
- Is "#1 of 1" a #1? If it's crowned, what's a crown on a busy board worth?
- Does ADR 0027's "no other names" also rule out the gap to #1, which needs no name?
- Who comes first: the visitor from Discord, or the Character checking themself?
