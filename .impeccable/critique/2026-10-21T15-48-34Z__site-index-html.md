---
target: site home page
total_score: 29
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 2
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-5e4826b8\\site\\index.html"
target_fingerprint: "sha256:37d41fa9b6899a22412ecd6cc42b6124c42c33e2fe983676844559b4a79864f1"
target_path: "C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-5e4826b8\\site\\index.html"
timestamp: 2026-10-21T15-48-34Z
slug: site-index-html
---
Method: dual-agent (A: design-review sub-agent · B: detector sub-agent; overlay injected by the parent after A finished, B's tab failed to open)

## Design Health Score
| # | Heuristic | Score | Key Issue |
|---|---|---|---|
| 1 | Visibility of System Status | 3 | Minimap dot and sticky head show place; film captions 13px, change every beat |
| 2 | Match System / Real World | 3 | Game's own terms; "Share, under their name in your Personal Bests" opaque pre-install |
| 3 | User Control and Freedom | 3 | "See what it does" skips the film; otherwise the hero holds ~1,050px of scroll |
| 4 | Consistency and Standards | 3 | Two feature rows show mascot art, not the app; "No sign-up" vs DESIGN.md "No account" |
| 5 | Error Prevention | 3 | SmartScreen and windowed-mode warnings before the download; phones still get a full-width .exe |
| 6 | Recognition Rather Than Recall | 3 | Inline key chips and the hotkey table; minimap's 13 portals unlabelled until hovered |
| 7 | Flexibility and Efficiency | 3 | Deep links, minimap jumps, skip link; no route from the hero to the safety section |
| 8 | Aesthetic and Minimalist Design | 2 | 31 questions on a persuade page; 3 fine-print paragraphs and 4 links in the hero; "All 18 questions about items" x3 |
| 9 | Error Recovery | 3 | Install recovery given twice; all fallbacks work without JS |
| 10 | Help and Documentation | 3 | Every row links its FAQ group; repeated group links are noise |
| **Total** | | **29/40** | **Good** |

## Design Specificity Verdict
LLM: strongly product-specific (window bar, raised tabs, lavender body, docked Mini Map, legend marks on headings, owl NPC, the panel's fold button; the hero film demonstrates the screen-reading method). Structural sameness: eight alternating shot/heading/sentence/3 questions/All N rows. Two stages show art instead of the app (index.html:191 tally board, :246 magnifying-glass owl).
Deterministic: file scan 1 advisory (tab gradient collapsed to rgb(209,215,237): false positive). URL scan at 1280: low-contrast (doubt chip in a closed answer, outlined in #b85e00 at 4.53:1: false positive), ai-color-palette (the hidden minimap's Game Button Blue fold button: committed world), nested-cards (window bar in the window: committed world). At 1600: minimap title lettering #eef3f7 on #a4c8da with outline reaches 4.38:1 (real; site.css:1392-1408).
Overlay: injected in the [Human] tab; only the nested-cards label is visible; the other labels sit on hidden elements.

## Priority Issues
- [P1] The first screen hides the product: the film opens on a small whole-window thumbnail with ~400px empty column (site.css:417); the readable hero shot only shows with reduced motion or no JS (reel.ts:208). Fix: open on the hero shot and pull out, or hold the still until the first scroll. /impeccable animate
- [P1] Reassurance comes after the ask: the hero carries only "at your own risk" (index.html:95); "only looks at the screen" is in #safe (index.html:263) after eight features; "See what it does" skips the film. Fix: a line beside the disclaimer linking #safe, or "Only reads the screen" in the trust line; consider moving #safe up. /impeccable clarify
- [P2] Two rows without proof (Map Leaderboards, Deal Finder art) and three item rows doing one job. Fix: real crops; merge or group the item rows. /impeccable distill then /impeccable layout
- [P2] On phones every Download fetches an .exe and Download vanishes for ~13 screens after the hero. Fix: "Send the link to your PC" (navigator.share or copy) on touch screens, .exe secondary. /impeccable adapt
- [P3] Minimap title lettering 4.38:1 at >=1440px (.mm-street, .mm-place). Fix: darken the outline for the site, or document the exception. /impeccable polish

## Persona Red Flags
- Jordan: "Level Band", "Hunting Target", "Share..." used before they mean anything; unlabelled minimap portals read as decoration.
- Riley: no-JS and reduced motion hold up; the film adds ~1.25 screens of held scroll even on phones; film captions hidden from screen readers with no other carrier.
- Casey: "On a phone?" note above a full-width .exe button, no way to send the link to a PC; Download then gone.
- Player afraid of a ban: first-screen safety words are "at your own risk"; the answer is 75% of the way down, unlinked.

## Minor Observations
- Surface brief stale (six rows/two tabs vs eight rows/three tabs).
- map.png crop shows a clipped Share column ("hare 2% 9%") behind the Mark Card.
- Version shown twice near the top.
- Page ends on a 10-row hotkey table and credits, not reassurance or download.

## Questions to Consider
- If the film is the safety proof, why does "How it stays safe" restate it in prose instead of linking back?
- Would a player afraid of a ban rather see the eighth feature second, or the three "only looks at the screen" bullets?
- On a phone, is the job "download" at all, or "remember to download it on my PC"?
