---
status: accepted
---

# The Best row shows the best as it stands

The Grind Tracker's Best row first showed a map's Personal Best from earlier Visits, fixed on arriving, "so the target doesn't move while the player chases it" (#44). This Visit's best joined the history only when the Visit ended. In play that read as a bug: a player who beat their best saw the old figure until they left the map and came back, and on a first Visit the row never showed at all. The star on Last 5 min went as soon as the rate dropped, so nothing on the Grind Tracker said what the player had just set; only the Personal Bests window did. This reverses that decision.

## The decision

- **The row is the best as it stands.** It shows the best of the earlier Visits from arriving, as before, and this Visit's from the refresh where it reads higher, as the Grind Tracker writes both (3 significant figures). A tie stays the earlier Visits', as a tie on a Map Leaderboard goes to whoever set it first. On a first Visit the row shows from its first best, 5 minutes in.
- **"this visit" after "Best"** says when the row's figure was set on this Visit, a first Visit included, so the player can tell a best they just set from one they came back to.
- **The star marks Last 5 min while it is that best**: the row is this Visit's and Last 5 min reads the same. It goes at the first refresh that is lower. Last 5 min can match a live row but never beat it, so "while it beats the best" would never show.
- **The race still works.** The Pace Trace races the row, so its gold line moves up with each new best. Once the player slows, it shows how far they are from the pace they set, which is what a fixed target was there for.
- **The × removes what the row shows**, this Visit's best included, and so does the Personal Bests window's × on "This visit" (it still can't be moved until the Visit ends). A misread spike now reaches the row at once, which is when the player wants it gone. After this Visit's best is removed, only 5-minute windows that start after the removal count towards it, so the play still inside Last 5 min that set it isn't counted again at the next refresh.
- **Uploads don't change.** The Map Leaderboards still take a Visit's best when it joins the history, when the Visit ends (ADR 0015). A best removed during the Visit is never uploaded, and the site never shows a peak that may still turn out to be a misread.

Both removals were already Events (`PanelControl::RemoveBest` and `RemoveVisit`), so a Bug Report replays them as it did (ADR 0009).
