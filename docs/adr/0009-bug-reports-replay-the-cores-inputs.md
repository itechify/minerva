---
status: accepted, amended by ADR-0010 (crops of the game around the pointer at an Item Card or Item Lookup press)
---

# A Bug Report is a replayable trace of the core's inputs, not a log

When something looks wrong, such as the Grind Tracker's EXP/h far higher than it should be, the player chooses **Bug Report** in the tray menu. Minerva saves a `.zip`, shows it in Explorer, and opens the bug issue form for it. The report holds every input the core took in since launch, in order, with what came of each:

- the source's answers;
- what the Session made of a reading (a misread set aside, a level-up and the EXP it counted, a death, a level learned anew, another character);
- what the Grind Tracker and the panel's title showed after, where that changed.

The core holds no clock, and all the logic is in it, so feeding the same inputs to a new core takes it through the same states. `probe replay` does that, and says where a replay doesn't. A bug can then be reproduced in `cargo test` rather than guessed at, and the smallest test that reproduces it becomes its regression test.

## What goes in

- **The inputs:** each of `Minerva`'s input methods writes its input down as it takes it, so none can be missed. What came of an input is written once the next one comes.
- **Kept small:**
  - A tick that changes nothing is left out, but for the last of a run of them, which the next line says came first. The loop ticks 20 times a second.
  - An input that only sets a value (the game window, a window's size, what is in front) is written only when the value changes.
  - A title reading the same as the one before says so, instead of what it read.
  - The wall clock is written only when it strays from the capture clock.

  That leaves about four lines a second while playing, about 1 MB an hour. The trace reaches back to launch, but no more than a day; the lines that set the run up (the settings and Personal Bests restored) stay even then. It is held compressed.
- **Crops:**
  - The EXP figures' from the last 10 minutes, where they changed: cut from the Status Bar's capture where they were read (ADR 0017).
  - The Minimap Title's where its reading changed: where it was found (ADR 0021), none of the rest of the client area.
  - The game's client area 400 px each way from the pointer, at each Item Card or Item Lookup press, from the last 10 minutes; named with the pointer's spot in it, as `samples/item-tooltip` names its frames.
  - For the whole run, the crops behind a reading the core found unusual or that changed the map shown, and behind an Item Tooltip whose name couldn't be read or isn't an item's.

  Without the pixels, a reading of 84,350 can't be told from a misread 8,435. With them, a frame read wrong becomes a sample, as the repo requires. The crops cover the two regions the player marked, and the game around the pointer when they press an item hotkey, which may take in their inventory and whatever is beside it: the player chooses to attach the report.
- **At most 5 MB**, well under GitHub's 25 MB for an attachment. Crops are left out to keep it so, the oldest ordinary ones first, then the oldest kept for the run. The text never is.
- **No OSMS data.** OSMS's file set isn't in a report (ADR 0008: nothing grants redistribution, and reports go into public issues), only its fingerprint, and the Map Summary of each map shown. A replay is given a set: OSMS's live one, or the `osms-set.json` from the cache of the PC the report came from. Fingerprints aren't compared, but what the core makes of the set is, so a set that has changed since diverges only where the change shows.
- **Errors:** the shell's (which a release build, without a console, loses) and our windows' uncaught ones, the same one at most once a minute.
- Nothing leaves the PC until the player attaches the file. The 10 newest reports are kept.

## Considered options

- **Log lines in a rolling file.** They are easy to read, but they can't be replayed, and they answer how the core got somewhere only where someone thought to log it. The core's notes give the reading part, and the trace stays the record.
- **The last few minutes only.** That would be smaller, but the Session's state comes from the whole run (the level's total learned, the Session's start, the Visits), and a bug's cause can come long before the press.
- **A snapshot of the core at the start of a window.** Every bit of the core's state would have to be saved and restored, and kept so through every change to it.
- **OSMS's set in the report.** Map recognition would replay exactly, but Minerva may not redistribute the set.
- **A hotkey, or a button on the panel or the Grind Tracker.** The tray is always reachable, even when the panel doesn't show (which can itself be the bug), and it adds nothing to the overlay. The trace covers the minutes before the press, so speed doesn't matter.

## Consequences

- A new input to `Minerva` needs an `Event`, a line in `replay`, and serde on its types.
- The shell's errors go through `errors::error!`, which prints them and hands them to the core.
- Whole reports are never committed: they are big, and hold OSMS's Map Summaries and the player's run. A report becomes the smallest test that reproduces its bug, on a made-up world, and a frame read wrong becomes a sample.
