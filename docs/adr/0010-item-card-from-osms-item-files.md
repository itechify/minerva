---
status: accepted, amended 2026-10-21 (item files another version saved are fetched afresh at launch, and Refresh fetches them with the map set) and 2026-10-27 (the market section opens the item's page on maplestory.market, in place of MesoWatch's home page) and 2026-10-28 (the card shows maplestory.market's Going rate and Cheapest now, ADR 0030)
---

# The Item Card, from OSMS's item files, with the player's own market price

The Item Card shows, at the pointer, what the game's Item Tooltip doesn't say about the item the player hovers: what a shop pays for it, and the market price the player keeps for it. It is for deciding what to do with loot: sell it to a shop, sell it to players, or keep it. The player presses a hotkey (Ctrl+Shift+I by default) while the game shows the Item Tooltip. The name is read as Item Lookup reads it (ADR 0007), and matched against OSMS Data Explorer's items on the player's machine, so nothing is asked at the press. Opening the item's MeowDB page, Item Lookup, moves to Ctrl+Shift+U and is otherwise unchanged; the card's name opens it too.

## Where the card's figures come from

Every source passes the test OSMS passed in ADR 0008: its terms don't forbid automated access, we read its own data files or JSON rather than its pages, we send our honest user agent, we credit it, and we ship none of its data. A source that fails the test needs its owner's written permission first; it is never scraped meanwhile, and its blocks are never got around.

- **OSMS's item files** (`items.json` and `cash_shop.json` from `data/current/`, about 1.2 MB): every item's game ID, name and kind, what a shop pays for it, and the Cash Shop's items. They come from the game's client, as OSMS's other files do.
- **Market prices: the player's own.** BasilMarket.net's price list met the test when the card was designed, and the card showed its median sale price for a day. On 11 October 2026 BasilMarket began refusing automated access: its `robots.txt` disallows `/api/`, its server answers other clients "Automated access to BasilMarket isn't allowed", and it reserves its data against mining. MesoWatch, MeowDB's Free Market page and classicfreemarket.com forbid automated access too. So the player keeps a market price per item themselves, typed into its card, for reference, with a button beside it that opens MesoWatch (whose terms welcome links) to look the item up. MesoWatch has no page per item and no search in its address, so it opens at its home page.
- **Amended 2026-10-27: the market look-up opens maplestory.market.** maplestory.market lists what the Free Market's stores sell now and the prices items sold at, from its own scans of the stores, with a page per item at the game's item ID (`maplestory.market/items/4010000`, without the game's leading zeros). So the card opens the item's own page there in place of MesoWatch's home page. Of items that look alike it opens the first one's, the one the card keeps prices under. It has an API, with keys an admin approves and terms that welcome showing prices for what a player asks about; Minerva reads nothing from it yet (`docs/research/item-card-sources.md`).
- **Drops aren't on the card.** OSMS has no drop tables (the game keeps them on its servers), and the one source that meets the test, MapleClassic Wiki, covered 26 of the game's 119 mobs when we looked (#78). Sleepywood, MSCW Guidebook and Henesys.GG fail the test, and MeowDB forbids scrapers.

## What it shows

- **Shop price:** what a shop pays for one. Otherwise that a shop won't buy it (OSMS's `notSale`, which carries a placeholder price of 1), that it is a Cash item, or that no shop price is known: OSMS gives none for scrolls, starter gear and arrows, and for throwing stars and bullets it gives what recharging them costs, which a sale in the game hasn't yet confirmed is what a shop pays.
- **Market:** the price the player keeps for the item, or "Not set"; the day they set it in its tooltip. A click on it, or on its pencil, opens its field; the MesoWatch button beside it opens MesoWatch either way. (ADR 0011 makes this a Market Line, one of the player's, and moves the MesoWatch button to the market heading. Since 2026-10-27 that button opens the item's page on maplestory.market, as does "Look it up on maplestory.market" at the market section's foot.)
- Not "Untradeable", which the game's tooltip says. Not the quests and recipes that give or need the item, which the card showed at first: MeowDB's page, a click away, has them.

## The market price the player keeps

_Extended by ADR 0011: the player keeps prices on Market Lines they name, one price per item on each. What follows holds for each line's price._

The player types it as players write prices: "450,000", "450k", "1.2m", with or without "mesos". Enter keeps it, dated by the wall clock; leaving the field keeps what is typed too; Esc leaves the price as it was; an empty field forgets it. What isn't a price keeps the field open, saying so. The core reads what was typed, and keeps the prices by the game's item ID: items that look alike keep one, under the first. They are saved in `market-prices.json` beside the settings, as the Personal Bests are, and a file that doesn't read is set aside rather than written over.

While its field is open the card takes the keyboard, as the search view does (ADR 0001): its window becomes focusable and comes to the front with `SetForegroundWindow` on our own window right after the player's click on it, so their typing goes to it; once the field closes, the window is no-activate again and gives the foreground back to the window below, the game. Nothing is sent to the game.

## Matching, by name

The name read is matched against every item's name as Item Lookup matches MeowDB's answer: the name itself but for the letters the recogniser confuses (I, l and 1; O and 0), case, spacing and punctuation, or one clearly closer than any other.

Items that share a name, a kind and a shop price look alike in the game: the card shows one card for them (the game's two Red Potions differ only in which quests give them). Items that share a name but not their kind or shop price (the Blue Moon top and earring) are listed with their pictures and kinds for the player to pick the one their tooltip shows.

## At the pointer, until closed

The card is one of our windows, like the Personal Bests window: the panel's window bar, which the player drags it by, with its ×, and a body that scrolls under the bar when the card is taller than the space. It opens with its corner a few pixels below and right of the pointer's tip, like the game's tooltip, so a click without moving still lands on the item; it may cover the game's tooltip. It goes left of the pointer or above it at the game window's edges, and moves clear of the captured regions. Dragged, it stays where the player leaves it (on a monitor, clear of the captured regions, and with the game window as it moves) until it closes; the next card opens at the pointer again. One card shows at a time, until its × or the hotkey again: over another item that item's card replaces it, elsewhere the press just closes it. It hides while another app is in front. What keeps a card from showing (no tooltip, an unreadable name, no such item, the item files not in) goes in the Item Lookup alert, as Item Lookup's own outcomes do.

Its window (ADR 0001) is no-activate but for the market price's field, on top while the game or one of our windows is in front, and it takes the clicks on itself while a card shows. Minerva doesn't hear of a click elsewhere: the game gets it. Closing the card on one would need Raw Input or a hook watching the mouse system-wide, a see-through window over the game that would stop it seeing the mouse, or the card taking the keyboard from the game. Each costs the game something or changes ADR 0001, so the card closes only by its × or the hotkey. Our windows show in our captures, so the shell moves the card off the screen before capturing for the next press.

## Considered options

- **A different hotkey for the card, keeping Ctrl+Shift+I for MeowDB.** The card is the more frequent use, and its name still opens MeowDB, so the key players pressed over an item goes to the card. A settings file from before it gives its Item Lookup key to the card, and Item Lookup Ctrl+Shift+U.
- **The card in the alert's spot, at the top centre.** The card is something to read beside the item, not a status; the player looks at the pointer.
- **Closing when the pointer leaves the item and the card.** It would need the pointer polled while the card shows; the player preferred a card that stays until closed.
- **The item files in the map set.** One set is one fetch to schedule, but a change in the item files would hold back maps too, and an older install's saved map set wouldn't read after the upgrade.
- **The quests and recipes that give and need the item** (OSMS's `quests.json` and `crafting.json`). The card showed them at first, three of each; the player found MeowDB's page, a click away, as useful.
- **BasilMarket's price list with a browser's user agent.** It would get around a block BasilMarket put up on purpose.
- **Asking MesoWatch for a link to an item, or for its prices.** The player chose not to.
- **maplestory.market beside MesoWatch** (2026-10-27). Two look-up buttons on the market heading; the one that opens at the item does what the other asks the player to do.

## Consequences

- OSMS's item files are fetched on the map set's schedule (a saved set under a day old that this version saved is used as it is, afresh every 24 hours and on Refresh, a minute after OSMS couldn't be reached with none saved), but read, saved and swapped on their own, all or nothing. A card from item files over 7 days old says it is a Saved Copy, with its date. `probe osms` reads them in full with the map set.
- Item Lookup's MeowDB search now runs only when the player asks for the page: by Ctrl+Shift+U or the card's name. Among several MeowDB items with the name, the one of the card's kind opens, if just one is.
- A Bug Report (ADR 0009) writes the card's inputs down (the drags, the market prices typed, and those restored at launch among them), with a crop of the game 400 px each way from the pointer at each item hotkey's press (from the last 10 minutes; for the run when the name couldn't be read or isn't an item's), so a tooltip read wrong can be seen and become a sample; and only the fingerprint of the item files, as of OSMS's set: a replay is given them (`probe replay --items`, or fetched live when the run had them).
- The card credits "Data: OSMS Data Explorer · Pages: NiaMeowDB, maplestory.market" (MesoWatch until 2026-10-27). Tests write made-up item files in OSMS's shape (`tests/common/items.rs`).
- Market prices from a source wait for one that meets the test, or a site's written permission. Until then they are as fresh as the player keeps them.
