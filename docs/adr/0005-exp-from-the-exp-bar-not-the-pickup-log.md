---
status: accepted
---

# EXP comes from the EXP bar, not the Pickup Log

The game reports EXP in two places: the EXP bar at the bottom of the screen ("EXP 266 [6.33%]") and the Pickup Log in the bottom-right corner ("You received EXP (+7)"). We measure EXP from the bar only, from the figures Minerva finds on the Status Bar (ADR 0017; until then, from the EXP Bar Region the player marked). The bar is the ground truth: it counts every source of EXP (kills, quests, party share) and every loss (death), it sits on a flat, fixed UI background, and a missed reading costs nothing because the next one carries the same total. The Pickup Log is semi-transparent text over the moving game world. Its lines fade and scroll, several can arrive between two captures, and two identical "+7" lines can't be told apart from one line seen twice. So a log-based rate undercounts by an amount that depends on how busy the screen is, and it never sees deaths.

## Consequences

- The session logic works on absolute bar values: rates come from differences between readings, level-ups are added up across the level boundary, and deaths show up as losses. Switching to the log later would mean rewriting it around per-event deltas.
- No EXP table is bundled. The bar's EXP and percentage together give the level's total, which is what Time to Level needs.
- The Pickup Log is still the only live source for mesos and items, so those figures are labelled as coming from it and may undercount. Its reader is designed separately.
