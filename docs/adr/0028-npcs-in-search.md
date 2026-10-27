---
status: accepted
---

# NPCs in Search

Search matches mobs, maps and items (ADR 0012). Players also go looking for an NPC: a job instructor, a taxi, the Maple Administrator. To find one, they had to know which map it stands on and search for that. OSMS's `maps.json` already says where every NPC stands: `npc_lookup` names each one (with its picture), and each map lists its `npcs` and their `npc_positions` in its picture. So Search matches NPCs too, and an NPC leads to its map, as a map does.

## The matches

- **An NPC matches by its name alone**, ranked as everything else in Search is. Matching the name of its map too would only repeat the Maps section ("henesys" would list everyone in Henesys), as matching a map's street would (ADR 0012).
- **The NPCs section comes between Maps and Items**, its best five shown and the rest counted. An NPC is a kind of place to go, so it sits by the maps; items stay last, the largest and the slowest to load.
- **Only NPCs on a map Search can send the player to.** An NPC `npc_lookup` names that stands on no map in the map list (98 of 301 in October 2026) has nowhere to go, and an NPC standing only on Event and Dev maps is left out, as those maps are. An NPC `npc_lookup` doesn't name can't be searched for, so the reader leaves it out. Objects that OSMS counts as NPCs (Exit, Mailbox, the crafting stations) are matched like any other: there is no hand-kept list to leave them out.
- **One match per NPC ID.** NPCs sharing a name are different NPCs (a Regular Cab in each town, two Sams), each listed with its picture and its map, which tells them apart.
- **A row shows the NPC's picture, its name, and its map** with its Map Mark, or "on 3 maps" for an NPC on more than one.

## Picking one

- **An NPC on one map makes that map the Destination**, as a map picked from Search does: the search view closes, and picked on the map the player is on, they are there at once.
- **An NPC on several maps lists them**, as a mob does: nearest first, then by name (by name alone with no map shown), each with its Map Mark, street and Distance. The highlight starts on the nearest; Enter or a click makes that map the Destination; Esc goes back to the matches as typed.
- **The Destination keeps the NPC.** The Navigation section leads with the NPC's picture and name, then the map, as a Hunting Target leads with its mob. Everything else is the Destination's: its route, how it ends, its ×. Once the player is on the map, the Map Preview marks every spot the NPC stands, and the Navigation section says so.
- **The mark is the minimap's green NPC dot in the Destination's Route Magenta ring**, over every other mark, so it can't be taken for a Spawn Point or a portal. Only the NPC picked for is marked, and only on its map, the one the player stands on.

## Considered options

- **Matching an NPC by its map's name as well.** It repeats the Maps section; see above.
- **One match per name, its maps put together.** Two Sams would be one, with one picture for both, and "Regular Cab" would need a list to pick a town from where now the row says which town.
- **Merging NPCs whose pictures match, as items that look alike are.** It needs the pictures compared, and the row's map already tells NPCs sharing a name apart.
- **Going straight to the nearest of an NPC's maps.** With no map shown there is no nearest, and the list costs one Enter.
- **Every NPC on every Map Preview**, the picked one set apart. Every town would be covered in green dots for a question the player didn't ask.
- **The NPC's picture drawn on the Map Preview.** Pictures of many sizes over a small preview hide what's under them; a dot in a ring reads at any size.
- **A plain Destination, forgetting the NPC.** On a busy town map, "you're here" leaves the question the player asked open: where is the NPC?

## Consequences

- The search state (`SearchState`) holds the matched NPCs (`npcs`), the NPC whose map list shows (`pickedNpc`) with its maps (`npcMaps`, highlighted by `highlightedResult` as the mob's are), and a Destination's NPC (`destination.npc`). `SearchView::NpcMaps` is the NPC's map list. The Map Preview has the NPC's spots (`npcSpots`).
- A click on an NPC writes itself down for Bug Reports (ADR 0009) as `SearchPickNpc`. A click on a map in the NPC's list is `SearchPickDestination`, which picks from whatever the view lists, so earlier reports replay as they did.
- The site's Search answers name NPCs, and the Map Preview's answer the NPC's mark.
