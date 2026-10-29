---
target: the /admin/ maintainer page
total_score: 19
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 3
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-26076b79\\site\\admin\\index.html"
target_fingerprint: "sha256:4db51ba23bd6604c26e11b247e48d46061432686c86fe6b1799a2908fcef0231"
target_path: "C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3-26076b79\\site\\admin\\index.html"
timestamp: 2026-10-29T06-37-04Z
slug: site-admin-index-html
---
Method: dual-agent (A: a5ce00c47d852d2ca · B: a40ea334d599094cc)

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 2 | No per-view loading state; Character action results land in #character-said far above the button; disabled buttons look enabled |
| 2 | Match System / Real World | 3 | Glossary terms used well; "Reassign to refused key" / "last refused" assume the data model; log details are raw codes |
| 3 | User Control and Freedom | 2 | No state in the URL; report Open loses the place; Take all down undone only one Visit at a time from Reports |
| 4 | Consistency and Standards | 2 | Two tab systems on one page; tablist without arrow keys; armed labels inconsistent; skips DESIGN.md's Remove-a-Visit confirm pattern |
| 5 | Error Prevention | 2 | Confirms exist but look like every button and never name subject or reach |
| 6 | Recognition Rather Than Recall | 2 | Names carried by hand between views; Overview counts and Log rows aren't links |
| 7 | Flexibility and Efficiency | 1 | No deep links, partial search, log filter or bulk actions; Report to Block is ~7 steps |
| 8 | Aesthetic and Minimalist Design | 2 | Calm and on-brand, but player chrome (Download, Credits) outweighs the tool; site-wide Blocked list under one Character |
| 9 | Error Recovery | 1 | Every failure reads "Can't reach owlofminerva.app"; three handlers erase their own error on refresh |
| 10 | Help and Documentation | 2 | Good inline help for Block and Taken down; the rest is hover-only title text |
| **Total** | | **19/40** | **Poor** |

## Design Specificity Verdict

LLM: shell and copy are authored for Minerva (Visits, Party Best, Blocks, "Its entries stay, for Unblock."); the interior is the account page's row stack reused five times, capped at 44em, with destructive actions styled like Look up. Reads as the account page with more lists, not a maintainer's console.

Deterministic scan: CLI 0 findings on site/admin/index.html. Rendered overlay (5 states): ai-color-palette on the hidden Mini Map fold button, nested-cards on header.window-bar, dark-glow on body. All three are site-wide chrome or the overlay's own highlight; none land on admin markup. --cdp unavailable in bundled engine 0.1.11. Contrast checked only the window bar (7.13:1, pass).

## Priority Issues

- [P1] Destructive confirms are invisible, generic and not reliably announced (armed() swaps text only; Block/Delete Account/Take all down look like Look up; asks don't name subject or reach). Fix: DESIGN.md:762 pattern (problem palette when armed, polite announcement, 400ms guard), asks that name the subject and reach. /impeccable harden
- [P1] Disabled buttons look enabled (site.css:135 only sets the cursor; --disabled-* tokens unused on the site). Fix: .button:disabled with the disabled tokens, plus the reason in words. /impeccable polish
- [P1] Every error reads UNREACHABLE, and Take entry down (Characters), Unlink and Unblock (Blocked list) erase their own error when they refresh. Taken down never loads when Reports fails. Fix: map 401/404/409 to words, write the message after the refresh, load Taken down on its own. /impeccable harden
- [P2] The Characters view is a case file with no case: name is a bullet, Block status the 4th bullet, ~15 controls at once, site-wide Blocked list underneath, panel hides and jumps on every action. Fix: heading with status tag, three groups (Name, Entries, Block) each with its own result line, Block form behind a disclosure, keep the panel up while refetching, Blocked list moved to Overview/its own spot. /impeccable layout
- [P2] Nothing links and nothing is in the URL: no hash routing, Overview counts and Log rows aren't links, Report rows lack Look up, Accounts view shows a Block with no Unblock and no "made on", log details unmapped. /impeccable clarify

## Persona Red Flags

Alex: Report to Block ~7 steps, exact-name search only, no #hash state, no log filter.
Sam: tablist ignores arrow keys; armed confirms change label silently (second Enter runs the action); lookups announce nothing and don't move focus; consequences only in title.
Sole maintainer back after weeks: "my uploads are blocked" lands on Accounts, which shows the Block but offers no Unblock and doesn't name the Character it was made on; Delete Account is the strongest-looking action there. "Name claimed": "refused key" isn't said to be the complainant's.

## Minor Observations

DESIGN.md:774 stale (two tabs); run-on market line; h1 + repeated h2 + duplicate "Signed in"; Credits and Download dominate at 390px; selected tab low contrast; found-list stays after opening; Open/Look up/Open verbs; Taken down rows lack date/by; single-line note input; no per-view loading; empty Accounts view; Dismiss beside Take entry down on mobile.

## Questions to Consider

Should a Character be a page (#characters/Egbert) with its status as the heading? Why does a player's Remove a Visit get a stronger confirm than the maintainer's Block or Delete Account? Does /admin/ need the player site's window at all?
