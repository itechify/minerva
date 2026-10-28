---
status: accepted, amended 2026-10-28 (Minerva fetches maplestory.market's prices, ADR 0030; imports in shape 2 still read)
---

# Market Imports, taken in while Minerva runs

ADR 0011 let a tool of the player's own fill a Market Line of its own by writing `market-prices.json` while Minerva is closed. That rules out a tool that runs on a schedule. Minerva reads the file only at launch, so a running Minerva doesn't show what the tool wrote. And it writes the whole file whenever a price or line changes, so whatever the tool wrote since launch is lost then. Two writers of one file lose an update whenever one writes between the other's read and write.

So Minerva stays the only writer of `market-prices.json`, and a tool hands its prices over through a folder beside it, `market-imports`.

## The folder

- A tool leaves a `.json` file there in `market-prices.json`'s shape: version 2, and its lines, each with its ID, its name and its prices by the game's item ID. It writes the file whole under another name (a dot file, or one not ending in `.json`), then renames it into place: Minerva reads neither. A tool that keeps one name (`tool.json`) replaces its own import that wasn't taken in yet.
- Minerva looks in the folder every 2 seconds while it runs, so at launch too. It moves each file out of the tool's way (`<name>.json.taken`), reads it and deletes it, in the files' names' order. One that doesn't read, or is in another shape, is set aside as `<name>.json.bad` and logged, as `market-prices.json` is.
- While `market-prices.json` didn't read at launch and couldn't be set aside, nothing is saved, so imports are left waiting rather than taken in and lost.

## Taking an import in

- Each line's prices go onto the line of the same ID, the newest price winning: a price held stays if it was set after the imported one, as when the player typed it since.
- A line not held is added at the end, named as the import names it, with the first number from 2 after the name if another line has it in any case (the line's ID if the name is only spaces). A line held keeps the name and place the player gave it.
- An import removes nothing: neither a line nor a price. Only the player removes them, on the card, which asks first because a line's prices don't come back (ADR 0011). A tool's line the player removed comes back with the tool's next import that names it.
- The prices are saved at once, as typed ones are, and every card shows them from then on, an open one included. A field left open stays open.

Minerva still fetches no market prices (ADR 0010). The player's tool fetches them, from where the player may, as the player types theirs.

## Considered options

- **Minerva re-reads `market-prices.json` when it changes.** The tool's change shows, but two programs still write one file: a price typed while the tool runs is lost, or the tool's run is.
- **A lock both honour.** Minerva would wait on a tool, and a tool that crashed holding it would stop Minerva saving.
- **Imports that remove lines or prices.** A tool could take a line's prices away while the player looks at a card. Leaving a line out of a tool's imports stops its prices changing; the player removes it on the card.

## Consequences

- An import is an input (ADR 0009): `Event::ImportMarketPrices` writes it down, and a replay takes it in again. A tool that imports only the prices that differ from `market-prices.json` (read, never written) keeps a Bug Report's trace small, and an import that was missed is sent again with its next.
- ADR 0011's tool that writes `market-prices.json` while Minerva is closed becomes one that leaves an import, whether Minerva runs or not.
