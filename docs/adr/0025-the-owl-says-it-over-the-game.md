---
status: accepted
---

# The owl says it over the game

Minerva had one thing to say over the game outside its panels: Item Lookup's alert, a plain line at the top centre of the game window (ADR 0007). A newer version said nothing over the game at all: the tray and Help & Setup carried it (ADR 0019), and players who live in the game window never looked there, so they stayed on versions whose bugs were already fixed. Both are now said by the owl, Minerva's mascot, in a speech bubble at the top centre of the game window: the **Owl Bubble**.

## The decision

- **One Owl Bubble, one line at a time.** It is the alert's window, renamed: one of our windows over the game (ADR 0001), placed by the core at the top centre of the game's client area and below any captured region it would cover, as the alert was. It shows only while the overlay can show. It says either Item Lookup's line or the update's line; Item Lookup's goes first while it lasts, as the player just asked for it, and the update's comes back after.
- **The owl, then the bubble.** The owl is a picture, 48 CSS px, beside a bubble the page draws in the theme's colours, its tail pointing at the owl, so the words stay sharp at any display scale. Each line has a mood the core gives it (news, a problem, or waiting), which picks the bubble's look (a problem wears the panel's problem note) and the owl's pose. Every mood has the same owl for now; a pose for one is a new picture and nothing more.
- **Item Lookup's line is the alert's, unchanged.** The same words, a few seconds and then a fade (half a minute while it lists items to pick from), "Looking up…" staying until MeowDB answers, no ×, and clicks passing through to the game except on a list to pick from. The same line still says what kept an Item Card from showing.
- **The update's line says a newer version is out**: "A newer version of Minerva is available! (X.Y.Z)". It shows while the notice is at its first stage (out, or notify-only) and the version is one the player hasn't been told of: a check the player pressed Check for updates for shows it in Help & Setup, which they are looking at, and not in the bubble. It has no fade, no motion and no sound, and stays until the player acts on it. Its × dismisses it; a click on the bubble opens Help & Setup's About & Credits, where updating is, and dismisses it too; choosing to update anywhere does the same. The bubble's window fits the owl and its bubble, so it takes no clicks beside them. Dismissed is for this launch and this version: the next launch says it again, and so does a newer version found later in this one. It hides while Help & Setup shows, which says it already.
- **No setting of its own.** With "Check for updates" off the owl says nothing of a newer version, one found before it was turned off included: the only notice then is the one the player asked for, in Help & Setup.
- **The core decides** which line shows, its mood, whether the bubble takes clicks and where it goes. The × and the click are Actions, so a Bug Report replays them (ADR 0009); the page only puts the line into words.

This amends ADR 0007's "An alert at the top centre of the game window", and supersedes ADR 0019's "No notice on the panels" and its considered option "A notice on the panel".

## Considered options

- **A notice on the panel**, a dot or chip by the gear. Rejected again, as in ADR 0019: the panel is for the map and the grind, and it can be docked away from where the player looks.
- **A window of its own for the update**, beside the alert's. Two windows in one spot either stack, moving each other, or hide each other; one bubble saying one line at a time is the same rule with nothing to move.
- **A plain banner** in the alert's style. It works, but the owl talking reads as Minerva speaking, which is what both lines are.
- **Hiding the update's line while a Session is running.** The players furthest behind are the ones who grind for hours at a time; a quiet line with a × costs them one click.
- **Fading the update's line out after a while.** A line that fades mid-fight is one nobody read.

## Consequences

- The bubble is taller than the alert was (the owl's 48 px), so it covers a little more of the scene at the top centre; it is clear of the captured regions all the same, and painted out of our captures like our other windows.
- Item Lookup's "covered" problem names the owl when the bubble covers the next tooltip's name.
- The update's line shows once per launch for as long as the player stays on an older version: ADR 0019's "no skip this version" holds.
