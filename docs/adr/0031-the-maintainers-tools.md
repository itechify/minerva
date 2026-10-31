---
status: accepted, amended by ADR-0032
---

# The maintainer's tools: an overview, Accounts, Blocks and a log

`/admin/` gives the maintainer an overview, Account and Character management, Blocks and a log of administrative writes. It uses the same authenticated routes as token-based administration (ADR 0023).

## The decision

- **One maintainer role**, as ADR 0023 has it. Every tool is a route under `/api/admin/`, with `ADMIN_TOKEN` or signed in as a maintainer, and the page only calls them.
- **Five views**, opening on the first, each in the page's address (`#/characters/Egbert`, `#/accounts/1`), so a reload, Back or a link keeps the place. **Overview** opens on the open reports, with the way to them, then gives the counts (Characters, entries, Accounts, linked PCs, entries taken down, Blocks), the new bests and the top spots taken in the last day and week, all from what D1 keeps already (no count of uploads is kept for it), and two buttons: **Refresh portraits** (ADR 0029's daily refresh, at once) and **Pull market prices** (both of maplestory.market's tables, whatever their age, ADR 0030), each with when it last ran, and under them every Block, each with **Unblock**. **Reports** stays as ADR 0023 has it, each report with **Look up** for its Character. **Characters** looks a name up as before, and opens it under its own name, in four groups: **Name** (its Account, and **Reassign**), **Entries** (each with **Take entry down**, and **Take all down**), **Block** (**Block…**, or **Unblock**) and its lines in the **Log**. **Accounts** looks an Account up by its Discord username, and shows its PCs, its Characters and its Block (with **Unblock**), with **Unlink** for a PC and **Delete Account**, as the player's own account page does them. **Log** lists what the maintainer did, each line linking to its Character and Account. A button that asks before it acts names what it acts on, in the problem palette, as the app's do.
- **A Block** keeps a player off the Map Leaderboards until the maintainer lifts it: it never ends by itself. It's made from a Character's lookup, never for a name that hasn't uploaded, and reaches the Character's Account, if it has one, and every key linked to it then, and the key that claimed the name. Each key is kept in the Block, so unlinking a PC or deleting the Account frees nothing, and a PC linked to the Account later is blocked as well. **Block the name too** keeps the name itself off, whatever key uploads it. A Block may carry a note, which only the maintainer sees.
- **What a Block does.** An upload from a blocked key, from a key linked to a blocked Account, or for a blocked name is refused, `403 blocked`, before it claims anything. A player who is blocked can still take their own entries down (`DELETE /api/characters/<name>/entries`), since removing their data never waits on the maintainer. Every Character the Block reaches (each name its keys claimed, each name of its Account, and its own name) is hidden from every public read: the boards, the maps and bands with entries, Where to train, Latest, the Pedestals (ADR 0024, amended) and a Character's page, which answers as for a name that never uploaded. The entries stay in D1, so the ranks behind them move up while the Block stands, and Unblock puts them back where they were, with no post on Discord and no "took #1" added afterwards. An upload's top spots are worked out from what the boards show, so one that passes a hidden entry takes the spot. Making a Block dismisses the reports against the Characters it hides, since nothing is left for them to act on.
- **The app**, refused as blocked, shows the Character's upload as blocked in the Personal Bests window, with where to ask if it was a mistake, and sends nothing more for it until Minerva next starts, when it tries once more.
- **The log** keeps every write under `/api/admin/` that went through: who (the maintainer's Discord username, or "token" for `ADMIN_TOKEN`), when, what, the Character or Account it touched, and a detail (the entry's map and band, the note). It's never trimmed, since a line is a few bytes and the maintainer makes few. The Log view shows it newest first, 50 at a time, and a Character's lookup shows its own lines.
- **Discord posts stay.** A top spot taken stays posted (ADR 0022) whatever becomes of its entry: the channel is a history, and its link leads to a board without the entry.

## The API

As ADR 0015 and ADR 0023, every answer JSON and every refusal `{ "error": "<code>" }`. **The maintainer**, beside their routes:

- `GET /api/admin/overview`: `{ "characters": 120, "entries": 840, "accounts": 30, "pcs": 41, "reports": 2, "removed": 5, "blocks": 0, "newBests": { "day": 12, "week": 90 }, "topsTaken": { "day": 3, "week": 20 }, "portraits": { "on": true, "checkedAt": ... }, "market": { "on": true, "prices": { "pulledAt": ..., "triedAt": ... }, "listed": null } }`. `on` says whether the Worker holds the key to ask (`CLASSICRANK_KEY`, `MM_API_KEY`), and a table never asked for is `null`.
- `POST /api/admin/market`: pulls both tables now. `204`, or `409 market-off` without `MM_API_KEY`. `POST /api/admin/portraits` stays as ADR 0029 has it.
- `GET /api/admin/characters/<name>`: ADR 0023's answer, with `entries` now the list, `[{ "map": "010004042", "band": 30, "party": false, "level": 34, "job": "Hunter", "expPerHour": 61536, "setAt": ... }]`, and `blocked`, the Block that reaches it (`{ "id": 1, "blockedAt": ..., "note": "...", "name": "Egbert" }`, `name` the Character it was made on, or `null`), and `log`, its lines in the log, newest first.
- `DELETE /api/admin/entries/<name>`: takes every entry of the Character down, each kept off as ADR 0015's are. `204`, or `404 no-entry`.
- `GET /api/admin/accounts/<id>`: `{ "id": 1, "username": "egberticus", "createdAt": ..., "pcs": [{ "id": 3, "linkedAt": ..., "lastUsedAt": ..., "characters": ["Egbert"] }], "characters": ["Egbert", "Bob"], "blocked": null }`, `blocked` the Block made on the Account, as a Character's lookup gives it, or `404 no-account`.
- `DELETE /api/admin/accounts/<id>/pcs/<pc>`: unlinks the PC, as the player's own unlink does. `204`, or `404 no-pc`.
- `DELETE /api/admin/accounts/<id>`: deletes the Account, as the player's own delete does, without touching the maintainer's cookies. `204`, or `404 no-account`.
- `POST /api/admin/characters/<name>/block` with `{ "name": true, "note": "..." }`, both optional: `201 { "id": 1 }`, `404 no-character`, or `409 blocked` for a Character a Block reaches already.
- `GET /api/admin/blocks`: `{ "blocks": [{ "id": 1, "name": "Egbert", "blocksName": true, "account": { "id": 1, "username": "egberticus" }, "keys": 2, "note": "...", "blockedAt": ... }] }`, the newest first. `account` is `null` for a key with none.
- `DELETE /api/admin/blocks/<id>`: lifts it. `204`, or `404 no-block`.
- `GET /api/admin/log[?before=<id>]`: `{ "lines": [{ "id": 9, "at": ..., "by": "itechify", "action": "take-down", "name": "Egbert", "account": null, "detail": "010004042 30 solo" }], "more": false }`, 50 at a time, newest first, the next 50 `?before=` the last line's `id`.

**The app**: `PUT /api/characters/<name>/entries` answers `403 blocked` as above.

## Considered options

- **A Block on the key alone.** A blocked player would link a new PC to their Account, or reinstall, and carry on. On the Account alone, unlinking the PC would free its key.
- **Taking a blocked player's entries down**, as Take all down does. Unblocking after a mistake would mean restoring each entry, and each one's next upload. Hidden, they come back as they were.
- **A quiet Block**, every upload answered as taken. The player would see their bests never show, and send a Bug Report the maintainer then has to chase. Minerva doesn't do things to a player silently, as its updates aren't (ADR 0019).
- **Taking a Discord post down with its entry.** The webhook would have to be asked to wait for each post's message ID, and D1 to keep it for each spot, and a post holds several spots, so it would be edited rather than deleted. The channel is a history, and its link already shows the board as it is.
- **A count of uploads for the Overview.** One more write on every upload, for a figure new bests already stand in for.

## Consequences

- A Block, an Unblock or a take-down shows on the public reads within a minute, as long as they're cached (ADR 0015).
- Nothing keeps a player who's blocked from uploading again with a new key and a new name. A Block is for the player who keeps their name; a new name starts at the bottom again.
- A Minerva from before this shows a blocked upload as failed ("HTTP 403: blocked"), and tries again as it does after any failure.
