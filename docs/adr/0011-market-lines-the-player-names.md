---
status: accepted
---

# Market Lines the player names, on every Item Card

ADR 0010 gave the Item Card one market price per item, the player's own. One figure says too little to decide what to do with loot: what an item typically sells for, the cheapest one listed now, and the price the player means to sell theirs at are different numbers, and a player who tracks them keeps all three. So the player keeps prices on **Market Lines** they name ("Your price", "Typical", "Lowest ask"), any number of them, and every card lists every line, in the player's order, with the item's price on each or "Not set".

Minerva still reads no market prices of its own: ADR 0010's test for a source stands. The lines hold what the player keeps.

## On the card

- The market section is a heading, "Market", with two buttons: a + that adds a line, and MesoWatch's (ADR 0010), which used to sit beside the one price. (ADR 0010's amendment makes it maplestory.market's, and adds the look-up in words at the section's foot.) Under it, a row per line.
- A click on a line's price, or its "Not set", opens its price field, which works as ADR 0010's did: typed as players write prices, Enter keeps it dated by the wall clock, an empty field forgets it, Esc leaves it.
- A click on a line's name opens its name field, to rename the line, or to remove it with the bin button beside the field. The + opens the same field as a new row at the end. A name needs more than spaces, and no two lines share one, whatever their case: the field stays open saying so. A name field left empty closes as it was.
- Removing a line asks first, in the line's place, saying how many items' prices go with it ("Remove “Typical” and its prices on 312 items?"), with Remove and Keep. A line can hold hundreds of prices, and they don't come back.
- A player starts with one line, "Your price". A player may remove every line: the heading then says the + adds one.
- New lines go at the end. Lines aren't reordered.

## In `market-prices.json`

Shape 2 holds the lines, in order, each with its ID, its name and its prices by the game's item ID:

```json
{
  "version": 2,
  "lines": [
    { "id": "1", "name": "Your price", "prices": { "04020000": { "mesos": 1000, "setAtUnixMs": 1791676800000 } } },
    { "id": "2", "name": "Lowest ask", "prices": {} }
  ]
}
```

- A file in shape 1 (0.8.0's, one price per item) is read as one line, "Your price", holding its prices, and saved in shape 2 from then on. An older Minerva sets a shape 2 file aside, as it does any later shape.
- A line's ID tells it from the others whatever it is named, so renaming keeps its prices. The core numbers the lines the player adds one past the highest number among the lines' IDs, and keeps any other ID as it was written. So a tool of the player's own may fill a line of its own (an ID such as `"tool-typical"`) while Minerva is closed: Minerva reads the file at launch, and writes it whenever a price or line changes. (ADR 0013 has the tool leave its prices in a folder Minerva takes them in from, whether it runs or not.)

## Considered options

- **Lines of each item's own.** More freedom, but the player would set the lines up item by item, and cards wouldn't line up for comparing; a line with no price for an item already says "Not set".
- **Lines kept in the settings view.** The settings view is about hotkeys, and the card is where the prices are typed: the lines are managed where they show.
- **Removing a line at once, as the Grind Tracker's Best row does.** That × takes one best; a line takes a price on every item it has.
- **A saved counter for new lines' IDs, so none is ever used twice.** A removed line's prices go with it, so a number used again carries nothing over.

## Consequences

- The card's view lists the lines (`ItemCard.market`), and says what its market section has open (`MarketEdit`): a line's price field, a line's name field (or the new line's), or the question before a line goes. A price or name field takes the keyboard as the price field did (ADR 0001); the question is answered with a click, so it doesn't.
- New inputs write themselves down for Bug Reports (ADR 0009): the clicks on a line's price and name, the names entered, and the +, Remove and its confirmation as actions. `Action::EditCardMarket` stays, opening the first line's price field, so 0.8.0's reports still read.
- The site's Item Card topic and its screenshot show the lines.
