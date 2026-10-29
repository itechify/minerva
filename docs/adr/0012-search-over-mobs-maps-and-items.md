---
status: accepted, amended by ADR-0028
---

# Search over mobs, maps and items

Mob search found a mob and the maps it spawns on, for a Hunting Target. The player also wants to go to a map with no mob in mind, and to look up an item they don't have in hand. So the search view becomes **Search**: what the player types matches mobs, maps and items, in one list with a section for each. A mob still leads to its map list and a Hunting Target. A map becomes the **Destination**, which the Navigation section shows the way to. An item shows its Item Card (ADR 0010) beside the panel. Item Lookup and the card's hotkey are unchanged. ADR 0028 adds NPCs between Maps and Items.

## The matches

- One list, its sections in a fixed order: Mobs, Maps, NPCs (ADR 0028), Items. A section with nothing matching isn't shown. Each match keeps its kind's look: a mob's OSMS animation, name and level; a map's Map Mark, Map Name and street; an item's picture, name and kind.
- Each section shows its best five and counts the rest ("+23 more, keep typing"). Without the limit a two-letter query would list hundreds of items, and ask OSMS for every one of their pictures.
- Best first: names that start with the query, then names with a word that starts with it, then names that hold it anywhere, each group by name. Mobs are ranked so too, where mob search listed every match by name.
- A map matches by its Map Name alone. Its street shows beside it, but matching streets too would list every map on Victoria Road. Event and Dev maps are left out, as mob search leaves them out of a mob's maps.
- Every item in OSMS's item files can be matched, Cash items included: their market prices mean something too. Items that look alike (the same name, kind and shop price, as the card has it) are one match. Items that share a name and don't look alike are listed each with its own picture and kind, so a pick goes straight to that item's card, without the card's own list to pick from.
- ↑ and ↓ move through all the matches as one list, from one section on to the next; Enter picks the highlighted match, and the highlight starts on the first. While OSMS's item files aren't in, a line says so in the Items section's place, so an item the query would match isn't taken for one that doesn't exist.
- Esc and the back arrow go back a step: from a mob's map list to the matches as typed, and from there to the normal view. Mob search went straight back to the normal view, losing what was typed.

## The Destination

A Destination is a map picked in Search, to go to. The Navigation section shows it as it shows a Hunting Target, without the mob or the map's figures for it: the map with its Map Mark and street, its Distance, the route there (which the Map Preview rings the next portal of), and once the player is there, that they have arrived, with the trip. It ends as a Hunting Target does: when the player leaves the map having arrived, or with its ×. There is one target or the other: picking one replaces the other. Picked on the map the player is on, it says they are there at once, where a Hunting Target picked there waits for them to leave and come back, since the player may be hunting where they stand. It has no 🔍: there is no map list to reopen, and Search is a click away. The route ends in the Route Magenta ring, not the mob's red dot.

An item picked in Search opens its card beside the panel, following it until dragged. ADR 0010 defines the card's behaviour.

## Considered options

- Separate searches or a kind filter would ask the player to decide what kind of thing they want before searching.
- One item match per name would show one picture for items that look different. Listing each makes the picture right and the pick final.
- Matching streets would list whole streets rather than the map the player asked for.

## Pictures

Item pictures and mob animations come from OSMS when a match first shows, as the Map Preview's picture and the Mob Cards' animations do (ADR 0008), and are kept on disk; while the search view is open, its matches' pictures are asked for before any other. Nothing is bundled with Minerva: ADR 0008 and ADR 0010 ship none of OSMS's data, and five matches a section keep the requests few.
