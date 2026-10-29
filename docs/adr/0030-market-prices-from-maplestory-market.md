---
status: accepted
---

# Market prices from maplestory.market

The Item Card adds fetched prices alongside the player's Market Lines (ADRs 0010, 0011). BasilMarket and MesoWatch forbid automated access. [maplestory.market](https://maplestory.market) scans every Free Market room about once an hour, works out each item's established price from what sold and what is asked, and offers it through an API whose terms allow showing prices for what a tool's users ask about. Its admins approved a key for Minerva. This records how Minerva uses it. What the API serves and its terms are in `docs/research/item-card-sources.md`.

## The decision

- **Two figures on every Item Card**, above the player's own Market Lines and read-only:
  - **Going rate:** maplestory.market's established price (`reference` in `/api/prices`). That is the 3-day average sale price, or with too few sales (or sales far from what is asked) the 3-day median asking price.
  - **Cheapest now:** the cheapest listing for one (`min_unit_price` in `/api/items`). A throwing star's or a bullet's listing is one set, so its whole price is used, as the Deal Finder judges one (ADR 0014).

  Their tooltips say what each is based on: sales and listings, the middle half of the asking prices, and when it was seen. For equipment they cover every copy whatever its stats, which Minerva can't read. Scrolled copies aren't told apart yet.
- **Only the Worker asks maplestory.market**, with its key, the `MM_API_KEY` Worker secret (`docs/map-leaderboards-setup.md`). The terms keep a key off anything a user can copy, so it never ships in the app. The Worker pulls `/api/prices` and `/api/items` whole, each at most once an hour and only when a read finds its copy that old, and keeps them in D1 (`market_pulls`, `market_prices`, `market_listed`, migration 0006). That is 2 requests an hour, however many players there are. `/api/items` gives the cheapest price per item without downloading every listing. If maplestory.market can't be reached, or answers with no table, the copy kept stays. Without the key, reads have nothing.
- **The app asks for a few items at a time**: `GET /api/market?ids=…`, at most 20 IDs, one item's for a card and one shop's for the Deal Finder. There is no read of the whole table, and each IP may ask about 2,000 items in 10 minutes. Together these keep anyone from copying the table through Minerva, which the terms forbid.
- **The core asks; the shell makes the ask.** As the leaderboards' uploads do, the core lists its asks in `PanelState::market_asks`, the shell makes each once and tells the answer to `Minerva::on_market_answer`, and the answer goes into Bug Reports (`Event::MarketAnswer`). An item is asked for when its card shows, or its shop is read while one of these figures is the Deal Line. It is asked again once the table it came from is an hour old, and never within a minute of the last ask. Answers are kept in memory only: a market price saved by Minerva would look like one the player kept.
- **Rows that have no figure say why:** "…" while asked, "Not available" when the site couldn't be reached or has no table, "None listed" when the item wasn't listed (or for the Going rate, sold) lately, and "No price" when maplestory.market has no established price for it.
- **The Deal Line is chosen in the Settings window**, from the Going rate, Cheapest now and the player's own lines. The rings on the card's lines go. With no choice made it is the Going rate, so the Deal Finder works for a new player. Cheapest now can be chosen too, though against the cheapest listing few are deals.
- **A player starts with no lines of their own.** Market prices saved in shape 2 become shape 3: the "Your price" line every player started with goes if it holds no prices, and with no Deal Line chosen, the first line left is chosen, as it was then, so no player's badges change. Market Imports in shape 2 are still taken in.
- **The player may turn it off** in the Settings window ("Prices from maplestory.market", on by default). Then nothing is asked, the rows hide, and a Deal Line among them falls back to the player's first line until it's on again. The FAQ says what is sent: the items' numbers, to owlofminerva.app.
- **Credit, as the terms ask:** while the rows show, the market section's foot reads "Prices from maplestory.market", linking to the item's page. The panel's Deal Finder line names the figure as maplestory.market's ("Comparing with maplestory.market's Going rate"). Every link to the site carries Minerva's link code (`?ref=76rbdbd7`), which isn't secret.

## Considered options

- **A key per player, pasted into Minerva.** No server, but every player would need an account and an admin's approval, and would get 6 requests a minute.
- **The key in the app.** The terms forbid it.
- **The whole table sent to the app.** One fetch, and the Deal Finder would never wait. But it republishes the table in bulk, which the terms forbid.
- **`/api/listings` for Cheapest now.** The same figure, but it downloads every listing.
- **A cron trigger for the pulls.** It would ask whether anyone plays or not.
- **maplestory.market's rows as Market Lines** (fed like a Market Import). Lines are the player's to name, type on and remove; a fetched figure isn't, and a price the player typed would be replaced every hour.
- **Their `deal` figure for the Deal Finder**, a lower one on a few items. A badge would disagree with the card.

## Consequences

- The player's own lines and Market Imports remain separate from fetched prices (ADRs 0011, 0013).
- ADR 0011's lines are still the player's, but a player starts with none, and the Deal Line is chosen in the Settings window, not on the card (ADR 0014).
