---
status: accepted
---

# Characters are found in the Status Bar

Minerva measures a Session from the EXP bar, which shows no one's name. It told a switch to another character from the readings alone: three in a row that fit another level, after the bar was out of sight (the character select screen). That misses a switch to a character of a similar level until their bar jumps, and Map Leaderboards (#106) need to know who set each Personal Best, at which level and in which job. The game shows all three in its Status Bar, pinned to the bottom of the client area: the level in orange tiles after "LV.", then the job ("Magician") and the name ("Egbert").

So Minerva finds the Status Bar itself, as the Deal Finder finds the Player Shop (ADR 0014): the shell captures a strip across the bottom of the client area every second, and the core finds the bar in it and reads the Character from it. There is no setup step.

## Finding and reading it

- The game draws its UI at a fixed pixel size, and pins the Status Bar to the bottom of the client area, but how far up from the bottom it sits isn't assumed: the samples, cropped by hand, show it 52 to 57 px up. The strip is 96 rows tall, and the bar is found in it.
- The level's tiles are the anchor: orange (255,148,21) components of a tile's size (about 17×20 px), 18 px apart, with the LV box's dark grey all round them. Nothing else on the bar, nor the game above it, lines up so. Live captures show the game's colours a little darker than screenshots do (`prep.rs`), so the orange and the grey are matched at any brightness.
- The job and the name are two lines of light grey on the bar's slate, right of the LV box, at set heights from the tiles. They are placed from the box's right edge, not the tiles: a level of one or three digits may sit elsewhere in the box. A pixel there that isn't the slate, the text or the blend between them (the game's pointer, a window of ours) makes its line unreadable, never misread.
- ocrs reads the level's glyphs (near white on the tiles: their blue sets them apart from the orange), the job and the name at once, black on white (ADR 0002). The reader remembers what each line's ink read as, so an unchanged bar costs a scan of the strip, a few ms, and no recognition.
- A reading counts only if it is one the bar could show. The level is as many digits as there are tiles, a 1 exactly on each tile whose glyph is narrow (the 1's is 5 columns with its flag, the 3's 10), from 1 to 200. The job is one of the game's job names, exactly. The name is 2 to 12 letters and digits. Anything else is unreadable.
- The samples hold only the digits 1 and 3, so the digits aren't matched against templates of the game's font: templates for 0, 2 and 4 to 9 couldn't be tested. Nor are the jobs other than Magician tested: a job the game spells otherwise than the list reads as unreadable, never as another.

## Characters and the Session

- A Character is its name: Classic World is one world, so a name is one character. Minerva learns one the first time it reads the name.
- **The name decides switches.** Another name read after the Status Bar was out of sight (not found, or not captured for 5 s) is another Character: a new Session at once, before any of their EXP readings, with nothing learned of their level. Another name read while the bar stayed in sight is a misread, set aside: no one changes character without the character select screen.
- While the name can be read and stays the same, readings that fit another level after the bar was out of sight are no switch: the level was learned wrong, or the EXP moved while no one saw, and it is learned again from them. **The level misfit stays as the fallback** while no name can be read; so does another level than the next, read with no name after the bar was out of sight.
- **The Status Bar's level wins.** The next level read there makes the next EXP reading a level-up, even if the bar was nowhere near 100% when last read (a quest turned in, or the bar covered as it filled), and even if that reading would fit the level before (2 [0.04%] allows a wide range of totals). Any other level read there is learned afresh from the next reading, and the change since isn't counted. A level-up the EXP bar shows first counts as before, and the Status Bar then agrees with it.
- The shell captures the Status Bar just before the EXP bar, every second, on the same reader thread, so another Character's name reaches the core before their EXP bar does.

## Considered options

- **A region marked at setup**, as the Title Region is (and the EXP Bar Region was, until ADR 0017). Setup would grow a step every player has to take, asked for again whenever the game window changes size, for something Minerva can find by itself at the cost of a scan of a thin strip.
- **A fixed offset from the EXP Bar Region.** The bar's level and name sit at set distances from its EXP figures, but the EXP Bar Region is the player's mark, a box drawn by hand around the figures, as loose as they drew it; and Minerva would know no Character while EXP tracking is off. Finding the EXP figures in the Status Bar too, so that mark goes, came later (#112, ADR 0017).
- **Reading the level from the EXP bar alone.** The Session's level is a range of totals learned from the bar (ADR 0005), with no level number and no EXP table: it can't say which level, only that it changed.
- **Templates of the level's digits.** Exact and cheap at a fixed pixel size, but only 1 and 3 have been seen.

## Consequences

- The bottom 96 rows of the client area are captured every second while the game window shows and no other app is in front (another app's pixels are never read): a BitBlt of about 1 ms, and a scan of a few ms. Our windows keep clear of the level, job and name where they were last found, as they do of the regions the player marked (ADR 0001).
- A reading is an input (ADR 0009): `Event::StatusBar` writes it down, the same as the one before written as such, and a replay takes it in again. Its crop is the level, job and name, 10 px around them, none of the HP, MP or EXP, nor the game above. The name is in the trace and the crop: a Bug Report goes into a public issue, and the player chooses to attach it. A crop becomes a sample in `samples/status-bar` with its name only from the maintainer's own Characters, or a player who agreed to it in the Bug Report flow; otherwise the name is painted out before it is committed.
- The samples are the maintainer's Egbert, a level 13 Magician, in four `--window` screenshots. Not yet covered: the digits 0, 2 and 4 to 9; levels of one and three digits (where their tiles sit in the box); every job but Magician, the second and third jobs' spellings most of all; names with glyphs ocrs confuses (I, l and 1; O and 0; rn and m), digits, and twelve letters; a live capture rather than a screenshot; the pointer or the game's tooltip over the bar; and another client size or UI Size (ADR 0020). A misread name the checks let through would be a phantom Character, kept apart from switching only while the bar stays in sight.
