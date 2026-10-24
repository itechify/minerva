---
target: website
total_score: 27
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 2
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-435dd48b\\site\\index.html"
target_fingerprint: "sha256:e85785904bd146a58c4fff19bf424d6504c344d26b7a92b38e825023b726d3a4"
target_path: "C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-435dd48b\\site\\index.html"
timestamp: 2026-10-24T15-33-05Z
slug: site-index-html
---
Method: dual-agent (A: design review sub-agent · B: detector sub-agent)

Target: the Minerva site, `site/index.html` (home, Persuade), `site/faq/index.html` (FAQ, Read), `site/leaderboards/` and a map's page (Operate). Built and served locally with its API and an empty leaderboard database, checked at 1920, 1440, 1366, 1280, 1000 and 600px in headless Chrome.

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Open tab, minimap "you are here", live lines in search are strong; the hero holds the page still for ~1,130px of scroll with no sign the scroll is driving it |
| 2 | Match System / Real World | 3 | Headings use players' words; the home page's body adds ~14 bold coined terms (Map Summary, Hunting Target, Level Band, Share…) unlinked to their answers |
| 3 | User Control and Freedom | 3 | Questions fold in place, deep links, Cancel on Report; the film's hold can't be skipped |
| 4 | Consistency and Standards | 3 | One coherent system; breaks: topics list only 961–1439px, the WIP art's beige card, Map Mark slot not kept in leaderboard search |
| 5 | Error Prevention | 3 | SmartScreen and fullscreen headed off before download; `?map=999999999` renders a full board inviting a best |
| 6 | Recognition Rather Than Recall | 2 | At ≥1440 the FAQ's only index is ten unlabelled minimap portals, across 108 questions with no search |
| 7 | Flexibility and Efficiency | 2 | Deep links and Enter-to-open are good; no FAQ filter, no expand-all |
| 8 | Aesthetic and Minimalist Design | 2 | Home is 9,211px at 1440 (13,093 at 600): 8 identical rows, 33 folded questions, 11 "All N" links |
| 9 | Error Recovery | 3 | Plain, specific sentences ("This link doesn't name a map…"); unknown map ID not called unknown |
| 10 | Help and Documentation | 3 | The FAQ is a real manual, written once, deep-linkable; no search; one symptom question filed under Safety |
| **Total** | | **27/40** | **Acceptable (top of band)** |

All ten scored: the home page alone could take 7 and 10 as n/a, but the FAQ and Leaderboards are Read and Operate surfaces where both apply.

## Design Specificity Verdict

**LLM assessment:** Authored, not interchangeable. The site is one of the game's windows opened wide: the Mini Map's grey bar with the pixel owl and steel Download, raised tabs, the NPC owl, answers folding behind the panel's fold button, legend marks on headings, the docked minimap whose dot walks the page, the game's glove cursor. The hero film proves the safety claim by showing Minerva find and read the minimap title. Where it slips: 4 of 8 feature rows show no Minerva UI (two WIP signs, two illustrations); all eight rows share one composition, so Hidden Portals (the signature) weighs the same as Deal Finder; the H1 "Your guide for MapleStory Classic World" could head a wiki.

**Deterministic scan:** source HTML 6 findings (design-system-color 2, flat-type-hierarchy, broken-image, buried-raster, cramped-padding); built `site/dist` 38 (31 of them `qa-list` cramped-padding that the project's ignore only suppresses on the source path). Browser (URL mode, 1440): home 6, FAQ 14, leaderboards 4, map page 4. True positives the design review missed: the "Not sure" chip's white 12px text on yellow at 1.4–2.05:1, held up only by a 1px outline; the minimap title outline in `site/site.css:1572` is `#4c7096`, drifted from DESIGN.md/`src/theme.css`'s `#4f7499`; under 640px the FAQ's type steps flatten (15/17/20) and the unselected tabs use a `color-mix` off the palette. False positives: qa-list/dialog padding, the map image's runtime src/opacity, the cyan "AI palette" (the game's documented button), the "nested card" (the window's title bar), the cropped reel, FAQ line length (source indentation). Neither the detector nor the console caught any of the review's structural issues; no console errors or failed requests on any page.

**Visual overlays:** `detect --cdp` isn't in the installed engine (0.1.16), so B used URL mode plus `detect.js` injected over CDP into headless Chrome. It reported the same counts with selectors, but there is no user-visible [Human] tab.

## Overall Impression

The most specific fan-tool site I've seen: it looks like it shipped with the game, and the hero film is a genuinely good idea. What lets it down is the Persuade page's middle: half its feature rows can't show the product, all eight look alike, and it ends on a hotkey table. The biggest opportunity is cutting the home page to the features it can show, at weights that match their importance.

## What's Working

1. **A world of its own.** Window bar, tabs, NPC owl, fold buttons, legend marks, docked minimap, game cursor: a full visual language taken from the app and the game, not a theme on a template.
2. **The film proves the claim.** "It only looks at the screen" is the sentence that matters, and the film shows it happening; with reduced motion it's a still and the page scrolls normally.
3. **The FAQ system.** Answers written once, copied in context, deep-linkable (an address opens its question under the head); a visible 2px focus ring on every control tried; 44px+ touch targets on phones; plain, specific edge-state sentences on the leaderboards.

## Priority Issues

**[P1] Half the home page's features don't show the product.** Grind Tracker and Item Card show `wip.png` ("Screenshot coming soon", `site/index.html:162,203`); Map Leaderboards and Deal Finder show illustrations. The WIP art's beige rounded card and anti-aliased sans clash with the lavender stage. *Why:* two of the lede's five promises (EXP per hour, pricing items) are backed by a construction sign on the page asking for trust and a SmartScreen bypass. *Fix:* retake `grind-tracker.png` and `item-card.png` (and `setup.png` on the FAQ); until then, cut the rows to a line under a neighbour or ship the stale shot with a caption saying what changed, never the sign. *Command:* `/impeccable distill`, then `/impeccable polish` after the retake.

**[P1] The FAQ loses its topic list at 1440px and wider.** The held list shows only at 961–1439px (`site/site.css:1166`); at ≥1440 the docked minimap names the current group but the other topics are unlabelled portals named on hover. 108 questions, no search. *Why:* the page's one job is finding one answer, and the widest screens get the least labelled navigation. *Fix:* keep the list at every width ≥961 (the minimap sits in the gutter, so they don't compete), and add a filter field over the first group that hides non-matches and opens matches. *Command:* `/impeccable layout`, then `/impeccable harden`.

**[P2] The home page is long and every row is the same row.** 9,211px at 1440, 8 rows of shot | words | 3–4 questions | "All N questions", alternating; Get started at ~7,250px; ends on the hotkey table and credits. *Why:* decide → download is buried under 33 objection rows; peak-end leaves a reference table. *Fix:* one question per feature; group into three acts (where you are, where to go, what it's worth) with act 1 at hero weight; end on Get started's owl and Download. *Command:* `/impeccable distill`, then `/impeccable layout`.

**[P2] The hero film holds for a screen and shrinks its own proof.** ~1,130px of frozen words at 1440×900; the 600px hero shot drawn at 456px (1440×900) and 393px (1280×800) against 544/594px as a still, so the panel's figures go unreadable on laptops; the beats are told in 13px captions. *Fix:* hold ≤0.6 screen, cut or speed the draw-back to the whole game window, set the captions at lede size, rest on the shot at full size. *Command:* `/impeccable animate`.

**[P2] Map Leaderboards' empty and unknown states.** An empty band prints "—" and "No entries yet" on every one of 15 rows; `?map=999999999` gets a full board titled "Map 999999999" with a "put yours here" CTA; in search, maps without a Map Mark sit 25px left of those with one. (Real entries weren't available locally.) *Fix:* say "no bests yet" once above an empty table and blank its cells; title an unknown ID as unknown and drop the CTA; reserve the mark's slot as the panel does. *Command:* `/impeccable harden`.

## Persona Red Flags

**Jordan (first-timer):** the H1 doesn't say it's an overlay; the hero doesn't move for a screen ("is it frozen?"); bold coined terms arrive unlinked; two features say WIP; at 1440 the FAQ has no visible topic list once the owl scrolls away.

**Riley (stress tester):** `?map=999999999` → "Map 999999999" plus a put-yours-here CTA; "No one at Lv 20–29 has put a best…" and "Egbert has no entry here…" stacked with no space; "Search doesn't find a new map…" filed under Safety.

**Casey (mobile, 600px):** 13,093px home page, first feature at 2,885px; the full-width primary button downloads a Windows installer on a phone; a deep link opens its question with the group heading off-screen; no FAQ filter where Ctrl+F isn't a habit; the owl's topic links are 30px tall.

**Mia (ordinary MapleStory Classic player, 1280–1366px laptop, worried about her account):** the ban question is answered on line two, which is right. But the panel in the hero is drawn at 393px and its figures are unreadable; the two features she'd use daily are WIP signs; she'd search "exp tracker" or "price check", not "Grind Tracker" or "Item Card", and the FAQ has no search to bridge them.

## Minor Observations

- `screenshots/map.png` leaks table fragments and a mob dot down the Mark Card's right edge; recrop.
- Minimap title outline `#4c7096` (`site/site.css:1572`) vs the system's `#4f7499`.
- "Not sure" chip text contrast rests on its 1px outline (matches the app's chip; worth a deliberate decision).
- Link underlines are Selection Edge at 1.83:1 on lavender, the only cue on Ink-coloured links.
- Under 640px the FAQ's type steps flatten (15/17/20), and the unselected tab tint is an off-palette `color-mix`.
- The ~500px credits close every page, the empty leaderboard included.
- Drawn titles take ~200px of the FAQ's and Leaderboards' first screen at 1440×900.
- Hero fine print: three stacked 13px paragraphs; the unsigned-files line could sit by the Download it qualifies.
- Leaderboard search "ant" opens a Hidden Street ahead of Ant Tunnel I.
- The hotkey table stops at 497px of a 564px column at 600.

## Questions to Consider

- If a feature can't be shown this release, should it be on the Persuade page at all?
- What if the home page were three acts, with Hidden Portals as big as the hero?
- Does the film need the zoom-out to the whole window, or is the story just find → read → name?
- Why does the FAQ get less labelled navigation at 1920 than at 1280?
- Should a phone visitor's primary button be one that works on a phone?
