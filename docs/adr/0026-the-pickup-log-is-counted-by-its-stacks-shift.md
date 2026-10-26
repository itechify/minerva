---
status: accepted
---

# The Pickup Log is counted by its stack's shift

The Pickup Log is the only live source for mesos and Items (ADR 0005), and the only place the game says which EXP was the player's own and which came from a party. Its lines are hard to count. Two recordings at 10 Hz (2026-10-07 solo, 2026-10-25 partied on The Murky Swamp I, #21) show how the log behaves:

- Lines are right-aligned, 22 px apart at the samples' UI Size, ending at client x=1908 of a 1920×1200 client area, the newest on a fixed bottom row (client y=960).
- A new line pushes every older one up exactly one row between two frames. It never slides. It is at full brightness on its first frame. A 7th line takes the oldest away in the same frame, and at most 6 show.
- Each line fades on its own clock, at a speed that varies (1.4 to 2.7 s to vanish while partied and busy, about 5.5 s solo), so a line's brightness doesn't tell its age.
- Lines arrive in bursts: 58% of the frames that bring lines bring 2 or more, an EXP line and its `Bonus EXP for PARTY` line always together, and now and then 6 or more at once, more than the log can show.
- The same line again is routine (`You received EXP (+14)` and `Bonus EXP for PARTY (+1)` three times running, `You have gained mesos (+29)` twice), so a line's text doesn't tell it from one already counted.
- The log is drawn under the game world: Mobs, platforms, drops, damage numbers, skill effects and name tags cover lines, a whole log for 3 or 4 frames.
- `You received EXP` is white for the player's own kill and yellow for a share of a partymate's. While partied, every kill brings a `Bonus EXP for PARTY` line, yellow.
- An Item's name too long for the line is cut short: `Arrows for Cros... x21 earned. (Use)`. An inventory tab that's full shows no line, because the game doesn't pick the Item up.

So the reader counts lines by how far the stack moved, not by what they say, and never counts a line it isn't sure of.

## The decision

- **It finds the log itself**, as every reader does (ADRs 0014, 0016, 0017, 0021), with no marking step: in a band at the bottom-right of the client area, sized by the UI Size the Status Bar measures (ADR 0020), it looks for right-aligned lines 22 px apart at the samples' size, and reads them there. The log is **anchored to the client area's right edge**: its lines end a fixed small distance from it (11 to 12 px at the samples' size, in both recordings and the Bug Report below), so only a stack of closing parentheses within 2 to 16 px of the band's right edge, at the Status Bar's UI Size (passed with the capture, not measured again from the lines) within 15%, is a candidate. A first version searched the whole band, and a Bug Report (0.15.0, 2026-10-26) showed it locking onto rock and cloud textures at three wrong sizes, and keeping them, so no arrival was ever read in five minutes.
- **A located log is confirmed before it is trusted.** It counts nothing, and reports nothing, until one of its rows reads as a known line kind (EXP, mesos, an Item), though it is followed meanwhile so that lines arriving as it is confirmed are counted. A located layout none of whose rows has read for 3 s is a miss: it is dropped and searched for again, so a lookalike is never kept. A confirmed log that goes empty is kept.
- **Counted by the shift.** Each capture's lines are matched to the last capture's by their shapes, not their brightness. The new lines are the bottom ones, as many as the stack moved, and only they are read: they are at full brightness, the easiest to read. Where more than one shift fits, the text and colour decide (a white `(+22)` where a yellow `(+21)` stood), then brightness (a line at full brightness where a faded one stood is new), and otherwise the smallest shift that fits. Six new rows count as six: any more were never on the screen.
- **Never overcount.** A pickup it can't be sure of isn't counted, so a total can be low, never high. The Grind Tracker says the figures come from the Pickup Log and may undercount (ADR 0005).
- **A covered capture is skipped.** One whose rows can't be read is left out, and the next clean capture is matched to the last clean one: a line lasts at least about 1.4 s and the stack holds 6, so a cover loses nothing unless more than 6 lines arrive under it. A cover is what would otherwise fake a burst.
- **10 times a second, on a thread of its own.** At 10 Hz the footage loses lines only to bursts no rate would catch. The `reader` thread reads the Minimap Title, the Status Bar and the Deal Finder's whole client area in turn, and a tick held up there would lose a burst. The capture is the log's band alone, and only new lines are read.
- **Only while it's needed:** while the Grind Tracker is on, a Character is on the Status Bar, and the overlay isn't hidden, as for the EXP figures.
- **Every line is recorded**, of whichever kind: the player's own EXP, a party share, the party bonus, mesos, and Items with their quantity and tab. EXP's amount still comes from the EXP bar (ADR 0005); its lines give the EXP Split and tell party play from solo.
- **Items are named from OSMS's item files** (ADR 0010), with the matcher Item Lookup uses. A name cut short is matched by what it starts with, its tab narrowing the choice. One that starts more than one Item's name is tallied as the game shows it (`Arrows for Cros...`), and one that matches no Item is tallied as read once the same text has been read on two lines, so a single misread never makes an Item of its own.

## Considered options

- **The text of each line as its identity**, counting a line not seen before. The same line again is routine, so it would count three `(+14)` lines as one.
- **A line's brightness as its age.** Lines fade at speeds that vary, by how busy the log is, so two lines that arrived together can be far apart in brightness a second later.
- **A best guess where the shift is unclear**, which would come closer on average. A guess can overcount, and a figure that may be high can't say what it is.
- **A Pickup Log Region the player marks**, as #14 planned. No other reader has a marking step now (ADR 0021), and the log's lines are their own anchor.
- **The `reader` thread at 10 Hz**, with the other readers. Simpler, but a busy tick loses a burst.
- **A slower rate, such as 5 Hz.** Half the captures, but twice as many lines arrive between two of them, and a burst of more than 6 loses lines.

## Consequences

- Before the log has been located since a reset, finding no lines cannot prove it is empty: a Mob may cover it. The first clean sighting establishes a baseline, so its lines may be missed rather than counted twice. A Bug Report keeps no crop for an unlocated empty log, a common case; it keeps one when previously seen lines vanish.
- Mesos come first, on the Grind Tracker's Session: the Session's total and mesos/h, with a ⓘ saying where they come from. The Item tally, under a Pickups header collapsed at first, comes next on the same reader.
- Party play is known from the log: any 5 minutes with Party EXP in them are party. That lets each map have a Solo Best and a Party Best. ADR 0015 ruled out party tags on the Map Leaderboards because Minerva couldn't tell party play from the screen; it now can, and that decision is reopened in an issue of its own.
- Our windows over the log hide it from the reader, as from every reader. The player may put one there; while one has covered it for more than a few seconds, the Grind Tracker says the Pickup Log is covered.
- A Bug Report writes down each capture's reading (the lines found, the shift taken, the lines read, and whether it had to choose between shifts), and keeps a crop of the log only for a capture where it had to choose, lost previously seen lines, or read a name it couldn't match. Those become samples.
- The samples in `samples/pickup-log` are short sequences of 10 Hz crops, each with a list of the lines that arrived, checked by eye: repeated lines, a burst of 6 or more, a covered log, the first lines into an empty one, names cut short, over more than one background. None shows another player's name: a name tag in one is painted out.
- Not yet covered: another UI Size or window size, and quest Items.
