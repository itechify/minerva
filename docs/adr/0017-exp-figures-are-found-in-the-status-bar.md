---
status: accepted
---

# The EXP figures are found in the Status Bar

Find the EXP figures in the automatically located Status Bar (ADR 0016), at a set distance from its LV box. No region is marked or saved, so resizing needs no setup. The Grind Tracker is on unless the player turns it off in Settings.

## Finding them

- The figures sit at a fixed distance from the LV box's right edge, not its tiles: one- and three-digit levels may place their tiles elsewhere (ADR 0016). Distances scale with the UI Size (ADR 0020).
- The figures are read from just left of the label to just short of the Cash Shop button, 213 px right of the label, and from 12 rows above "[" to 20 below it: the game keeps them in that span however many digits the EXP has. The EXP Bar reader is unchanged: it finds the rightmost pair of brackets in what it is given, and its checks make a frame unreadable, never a wrong number, if it is given the wrong place.
- **The place is remembered.** While the LV box can't be found (the pointer or a window of the game over it), the figures are read where they were last found, until the client area changes size. Covering the LV box doesn't cost EXP readings. The place is kept only while Minerva runs: saving it would bring back a saved region.
- **One capture.** The figures are read from the Status Bar's own strip (the bottom 128 rows, full width), so the level and the EXP always come from the same frame, and there is no second BitBlt.

## Samples and Bug Reports

- Two layers. The samples of the figures alone (a recording, misread crops, a live crop) keep testing the EXP Bar reader, fed as a frame of the figures; the samples with the whole Status Bar (`--window`, and strips recorded later) test finding the place too.
- A number read wrong is the EXP Bar reader's fault, and becomes a sample of the figures alone. A place not found, or found wrong (which reads as unreadable, never as a wrong number), becomes a sample of the strip, its name painted over unless the `samples/status-bar` rule allows it.
- `Event::ExpBar` keeps its crop of the figures, and says where they were read and whether the place was found or remembered.

## Considered options

- **Keeping the EXP Bar Region as an override**, for a client size where the bar isn't found. Characters already depend on finding the Status Bar and nothing has shown it failing; an override would be a second way of reading the figures that nothing tests. A client size where it fails becomes a Bug Report, and a sample.
- **A second capture** of the place worked out from the bar. It costs a BitBlt a second, and its frame could differ from the Status Bar's.
- **Keeping EXP tracking opt-in.** Marking the bar was the opt-in; with no mark, the Grind Tracker is on by default and turned off in Settings.

## Consequences

- When the Status Bar is never found, "Can't read the EXP bar" says Minerva can't find the game's Status Bar, and points to a Bug Report rather than a mark.
- Our windows keep clear of the figures where they were found, as they do of the Status Bar's level, job and name (ADR 0001).
- The figures are read only while the Status Bar is captured: with the game window showing and no other app in front, as ADR 0016 has it. EXP tracking is off without a game window, as it was without a mark that fitted it.
