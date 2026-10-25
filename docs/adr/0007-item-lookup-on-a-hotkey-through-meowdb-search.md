---
status: accepted, amended by ADR-0008 (its search is Minerva's only MeowDB request, and no longer bars a public release) and ADR-0010 (its hotkey is Ctrl+Shift+U, and the Item Card's name opens the page too)
---

# Item Lookup on a hotkey through MeowDB's search

Item Lookup opens the MeowDB page of the item the player is hovering in the game. The player presses a hotkey (Ctrl+Shift+I by default) while the game shows the item's Item Tooltip. Minerva then reads the pointer position, captures the game's client area once, finds the Item Tooltip box nearest the pointer, and reads the name on the line with the blue bullet. It sends that name to MeowDB's search and opens the page of the item whose name matches.

## A hotkey, not Ctrl+Shift+click

The natural gesture would be Ctrl+Shift+click on the item. ADR 0001 rules it out: keyboard input comes only through `RegisterHotKey`, and a mouse click can't be registered that way. Seeing one would need a low-level mouse hook (`WH_MOUSE_LL`) or polling key state (`GetAsyncKeyState`), and ADR 0001 bans both. The click would also reach the game, and clicking an inventory item picks it up, which closes the tooltip before it can be read. A hotkey reaches only Minerva, so the tooltip stays where it is. Reading the pointer position (`GetCursorPos`) is window-manager state, like the window pick at setup.

## MeowDB's search, not an index of every item

MeowDB's item pages are numbered by MeowDB's own IDs (`/msclassic/item-db/418`), not by the game's item IDs, so a name has to be looked up. We send one request per lookup to the endpoint MeowDB's own search box uses (`/msclassic/api/search?q=<name>&locale=en`). It answers with up to six close items, each with its ID, name and category. We open one only if its name matches what was read, allowing for the letters the recogniser confuses (I, l and 1; O and 0), or if it's clearly closer than the runner-up. Otherwise the alert says no MeowDB item has that name. When several results share the name (193 names are shared by 406 items, mostly Cash Equip colour and gender variants), the alert lists them with their categories and the player picks one. Answers are cached on disk, so a repeat lookup doesn't ask again.

## An alert at the top centre of the game window

What a lookup comes to (looking up, opened, why nothing opened, or the items to pick from) shows in an alert of its own at the top centre of the game window, where the player is looking when they press the hotkey, for a few seconds. The panel is left as it is, collapsed or not. The alert is a window of ours like the Grind Tracker's (ADR 0001): no-activate, on top of everything only while the game or one of our windows is in front, and clear of the captured regions. Clicks pass through it to the game, except while it lists items to pick from. Like our other windows it shows in our captures, so an alert still showing over the next tooltip's name is reported as covering it.

**Amended by ADR 0025.** The alert is now the Owl Bubble: the owl says the same line in a speech bubble, in the same spot, and says a newer version is out there too, Item Lookup's line going first.

## Considered options

- **An index of `/msclassic/item-db/all`**, fetched and cached like the map index, with the closest name picked. It works offline after the first fetch. But that page leaves out 68 items that have pages ("Hat MP Scroll: Greater" among them), so a perfect reading of a missing item would land on a near neighbour with full confidence. It is also 2.6 MB to fetch.
- **Both:** the index first, then search for names it lacks. This is the most code, and it still opens a wrong neighbour whenever the index has a close one.
- **The panel's own line** for the outcome, expanding a collapsed panel for a failure or a list. The player looks at the item, not the panel, when they press the hotkey, so the outcome was easy to miss.
- **A fixed spot below the pointer** instead of finding the box. The game moves the tooltip near the window's edges (up, or beside the pointer), so a fixed spot reads empty space there.

## Consequences

- The search endpoint isn't documented. If its answer changes shape, Item Lookup says it is unavailable, as a map page in a changed format does. It never guesses.
- The browser opens after a network round trip on a cache miss, not at the instant of the key press.
- This is the first reader of text over a translucent background (ADR 0005 turned the Pickup Log down for that reason). The name is bold white on the tooltip's dark fill, which holds up where the Pickup Log's thin fading lines don't. `samples/item-tooltip` keeps every frame the reader must get right, and a frame it ever reads wrong becomes one.
- The rest of ADR 0003 stands: one request per player's press, an honest user agent, the credit to NiaMeowDB, and no public release without MeowDB's written permission (lifted by ADR 0008, below).

## Since ADR 0008

Maps and mobs now come from OSMS Data Explorer's files (ADR 0008), so this search is Minerva's only remaining request to MeowDB. With no crawl left to keep MeowDB's pace of one request every 2 seconds, a lookup now keeps it itself: its search waits until 2 seconds after the last one that went over the network. An answer from the cache still comes at once. The letter of MeowDB's terms (§4, "automated scripts, bots, or scrapers") arguably covers the search, while its Content Reuse Policy, aimed at bulk scraping and republishing, doesn't. We take it as a grey area to raise with MeowDB, not a bar to a public release.

## Since ADR 0010

The Item Card (ADR 0010) took Ctrl+Shift+I: the key the player presses over an item now shows its card, whose name opens the item's MeowDB page through this search. Item Lookup itself moved to Ctrl+Shift+U, and a settings file from before the card gives its Item Lookup key to the card. So MeowDB's search runs only when the player asks for the page. From the card's name, the name searched for is OSMS's, not a reading, and among several MeowDB items with it, the one whose category is the card's kind of item opens, if just one is; otherwise the alert lists them as before. The alert also says what kept an Item Card from showing, naming the card's hotkey.
