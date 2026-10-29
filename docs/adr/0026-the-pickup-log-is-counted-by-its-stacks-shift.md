---
status: accepted
---

# The Pickup Log is counted by its stack's shift

Count Pickup Log arrivals by the shift of its six-row stack, not by text or brightness. Identical lines repeat, each fades at its own rate, and the game world can cover them. A new line pushes older lines up a whole row; bursts beyond six cannot be recovered. Prefer missed arrivals to invented ones. The EXP bar remains the source of total EXP (ADR 0005); log lines supply mesos, Items and the EXP Split.

## Locate and confirm

Capture the bottom-right band at 10 Hz on a thread separate from the other readers, only while the Grind Tracker needs it, a Character is known and the overlay can show. Use the Status Bar's measured UI Size (ADR 0020), not a size inferred from fading parentheses. Search for right-aligned rows at that size, with closing parentheses near the client area's right edge. At another UI Size, locate on a capture scaled to the samples' size and bring the layout back. This avoids mistaking world textures for the log and losing small, corner-connected strokes.

Follow a candidate immediately, but report and count nothing until a row parses as EXP, mesos or an Item. Drop an unconfirmed layout after 3 seconds without a readable row; keep a confirmed log when empty. The first clean sighting after a reset is a baseline: finding no lines before that proves nothing about arrivals.

## Count and read

- Match successive clean stacks by line shape, independent of brightness. Ink must have the text's shadow below and right; bright world pixels alone are not text. Read only the bottom rows the shift identifies as new.
- If several shifts fit, compare parsed text and colour, then full brightness against a faded predecessor, then take the smallest shift that fits. Six new rows count as six, never an inferred seventh. Skip a covered capture and compare the next clean one to the last clean one. After 3 seconds without a match, resynchronise to a new baseline and miss the interval rather than invent a burst.
- Away from the samples' UI Size, resampling weakens shape agreement: the tail threshold is 65% rather than 80%, only a full-brightness row is new, and a fading row remains the same while its shape matches. Read each new line with three ink preparations and accept it only if all parse to the same kind and figures. Otherwise it is `Unreadable`. This loses recall but prevents a blurred digit becoming a different amount.
- Follow each new line for at most five reads or 500 ms, stopping once three agree, or when it fades, leaves the stack or is covered. Vote by kind and figures (amount, quantity, tab and whose EXP), not Item name. The largest group of parsed reads wins; a tie is `Unreadable`, and a sole parsed read stands. Within the winning group, use the most frequent Item name, earliest on a tie. Repeated reads can recover text hidden by a moving background without counting another arrival.
- Deliver settled lines in arrival order with their original capture times. The Session excludes lines from before its start, reset, undo or Character switch even when their vote ends later. Drop pending lines when the stream ends, the log is lost, the Session resets or the Character changes. Preserve the arrival frame for an unreadable line's Bug Report crop, rather than the vote's final frame.

The parser tolerates the recogniser's slips in the game's fixed words, but never changes a line's kind. The quantity comes from the final word's digits, so digits in an Item's name cannot become its quantity. White `You received EXP` is the player's own; yellow is a party share, and `Bonus EXP for PARTY` is party too.

## Name Items

Match Item names against OSMS's item files (ADR 0010). An exact name may stand despite a misread inventory tab; an approximate match must belong to the tab. A cut-short name is matched by its prefix and tab. If several names still fit, tally the shown shortened name. If none fits exactly, a prefix of at least eight normalised glyphs may match by the same slip-aware comparison as a whole name. Two trailing punctuation marks can be a misread ellipsis.

The tab-local comparison in `pickup_slips.rs` charges 0.4 edits for known recurring OCR slips or an unknown glyph. A name must be at least 75% similar, a whole edit nearer than the runner-up, and the runner-up at least 1.5 times as far away. Plain edits may settle a read only when they choose that same closest Item, at 75% similarity and a two-edit lead. Equal neighbours stay as read. The whole-set matcher uses its stricter 85% threshold before the tab-local fallback.

A name no Item matches is tallied as read only after that same text arrives on two lines; the waiting quantity is then included. This admits new Items without making a separate Item from every single misread. Name correction can occasionally put one Item's quantity under another, so an individual Item count may be high; the total is still limited by counted arrivals. `probe loot` compares this naming with plain edits and reports changed names from a Bug Report. Timestamped replay tallies are diagnostic evidence, not part of the rule.

## Consequences and alternatives

- Text identity would merge repeated pickups. Brightness is not age, and guessing a shift could overcount. A slower capture rate loses more bursts; sharing the other readers' thread lets their work delay a capture. There is no region to mark.
- Any five-minute window with Party EXP is party play. Personal Bests and uploads keep Solo Bests and Party Bests apart (ADR 0015). Older bests and entries of unknown kind are Party Bests; players cannot relabel a Visit's kind.
- Our windows can cover the log. The Grind Tracker says so after a sustained cover and says its Pickup Log figures may undercount.
- Bug Reports record layout, shift, ambiguity and settled lines, so replay needs no pixels. Keep crops for ambiguous shifts, lost visible lines and unreadable arrivals, at most one per cause per 10 seconds. After 30 seconds without finding the log, keep the whole band, no more than once per five minutes. This leaves evidence without letting frequent log crops crowd other readers out (ADR 0009).
- Sequence samples test repeated lines, bursts, covers, shortened names and different backgrounds. Samples at the game's smaller UI Size test the resampling path too. Tests require fewer readings rather than different amounts; a new failure becomes a sample under `samples/README.md`.
