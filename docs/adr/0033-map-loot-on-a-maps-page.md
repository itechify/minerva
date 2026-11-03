---
status: accepted
---

# Map Loot on a map's page

The Grind Tracker counts a Session's Loot from the Pickup Log (ADR 0026), but nothing showed what a map is worth to farm. A map's page on the site now shows its **Map Loot**: the mesos an hour and Items an hour players picked up there. It is information about the map, not a race: a Visit's Loot is a lower bound that depends on how well the log was read, and mesos have no ceiling to catch a made-up figure, so a ranking would reward the best-read log rather than the best play.

## The decision

- **A Visit's Loot is sent when it ends**, for a Character shown on the Map Leaderboards, with the same Leaderboard Key: no switch of its own. Only a Visit worth counting goes: on a map with a Map Summary, with at least 10 minutes of active play, some mesos, and the Pickup Log readable (found and not covered) for nine tenths of its active time. A Visit with no mesos is one whose drops went unpicked, and a log covered or unread for long undercounts by more than it always does; either would pull a map's figures down. Solo and party Visits both count, kept apart: a Visit with the party bonus in it is a party one.
- **Items by ID, real ones only.** The app sends each Item's count by its OSMS ID; a name that matches no Item is no longer tallied at all (amending ADR 0026), and a name cut short that starts several Items' names is tallied by the Grind Tracker but never sent. The Worker takes only IDs in the Item names table `probe ceilings` writes beside the ceilings (`site/worker/osms-items.json`), and leaves the others out of a Visit it takes.
- **Kept as running sums**: one row per Character, map and kind, of their active time, mesos and each Item's count. Each map's figures are worked out from them once a day, in the cron run that refreshes portraits (ADR 0029), for the maps sent Visits since, into one row the page reads: the page says they're updated daily.
- **Every Character counts once.** A map's mesos an hour is the median over Characters of each one's own total mesos ÷ total hours there, so one long grinder or a made-up Visit can't move it. An Item's rate is the average of the Characters' own rates (one who picked none up adds a 0). A kind's figures show once 3 Characters gave the map Visits of it, an Item once 2 of them picked it up; Both is each Character's solo and party Visits together. Level Bands aren't split: the samples would be too thin for a long time.
- **Each Item links to its maplestory.market page**, by its game ID with the Item Card's referral code (ADR 0030), beside its OSMS icon.
- **No names.** The figures are anonymous totals. An opt-out, a takedown of all a Character's entries, or a Block removes their rows at once, and the map's figures at the next daily run; none of it comes back if the entry is restored or the Block lifted.
- **What waits is kept.** Closing Minerva ends a Visit (GLOSSARY), the most common way a grind ends, so the app saves what is waiting to be sent, and the Visit under way as if it had ended, beside the settings (`map-loot.json`), and sends it at the next launch. The Visit under way is saved afresh once a minute of its active time, not with every line.

## The API

Beside ADR 0015's:

- `POST /api/characters/<name>/map-loot` with the key, `{ "map": "010001010", "party": false, "activeMs": 600000, "mesos": 3000, "items": [{ "id": 4000000, "count": 120 }] }`: claimed and blocked as an upload of entries is, counted against the key's writes. A Visit under 10 minutes or over a day, with no mesos, over 50,000,000 mesos an hour, or an Item over 100,000 an hour is a 400. Taken, `200 { "rejectedItems": [4999999] }`, the IDs the Item names table doesn't hold.
- `GET /api/maps/<map>/loot`, public and cached for a minute: `{ "map": "010001010", "builtAt": 1760000000000, "solo": {...}, "party": {...}, "both": {...} }`, each kind `{ "characters": 12, "mesosPerHour": 41200, "items": [{ "id": 4000008, "name": "Pig's Head", "perHour": 64 }] }`, or only `{ "characters": 2 }` until 3 have. `builtAt` is `null` for a map the daily run hasn't worked out.

## Considered options

- **Ranked meso boards.** Turned down for the reasons above.
- **The median over Visits.** One Character's many Visits would set it; over Characters, each counts once.
- **Only the Visits that upload as bests.** An average of bests is optimistic, and follows EXP, not mesos.
- **Rebuilding a map's figures as each Visit comes in.** Fresh, but each upload would read every Character's row of the map; D1's reads were just cut down (#221).
- **Items valued in mesos** (an NPC's price or the Going rate). A figure the Grind Tracker doesn't show; the site shows what players see in their own app.
- **Sending names, trusted.** IDs are exact, and the icons need them anyway.
- **Linking each Item to MeowDB**, as the app's Item Card names it. MeowDB's pages go by its own IDs, not the game's, which the app finds through MeowDB's search (ADR 0007), and its Item database takes no search in its address; having the Worker search MeowDB for each Item would be a new outside request.

## Consequences

- The FAQ's `#data` answer says what is sent, `#kept` names `map-loot.json`, and `#map-loot` how a map's figures are worked out.
- An Item new to the game counts once OSMS lists it and `probe ceilings` writes the table again; until then the Grind Tracker doesn't count it, and the Worker would leave it out.
- Where to train and the Character page don't show Map Loot yet.
