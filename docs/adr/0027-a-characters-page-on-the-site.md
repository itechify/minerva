---
status: accepted, amended by ADR-0015, ADR-0029
---

# A Character's page on the site

The Map Leaderboards (ADR 0015) show a Character one map at a time. A name on a board, in Where to train or in Latest (ADR 0024) says nothing of where else they hunt, and a player curious about whoever took a #1 off them (ADR 0022) had nowhere to look. The Personal Bests window shows the player their own bests side by side; nothing showed anyone else's. This records the page that does: a **Character page** for each name on the Map Leaderboards.

## The decision

- **Every name links to it.** A Character's page is `/character/?name=Egbert`, a static page that reads the API from the browser, as a map's page does. Every name the Map Leaderboards show is a link to it: a band's rows, Where to train's #1, Latest's names (the one who set it, and the one a top spot was taken from), and the names in a new #1's post on Discord. A name reads as plain words at rest and is a link under the pointer, so the boards look as they did. It sits at `/character/` and lights no tab.
- **It shows their entries, with a portrait under ADR 0029.** Each entry of theirs, a Level Band at a time, highest first, each band's entries highest EXP/h first. Each row has the map's Map Mark, name and street, its mob levels (the ceilings table's, as Where to train's), the job and level that set it, the EXP/h, the date, and its rank of how many the band has there ("#2 of 12"), a crown on a place in the top 3, gold, silver or bronze (as a map's page crowns them too). The crowns aren't You Yellow: You Yellow marks the player's own best, and a crowned row's link leads to a map's page where the same Character may be starred in it. Each row's map links to the entry, marked, on its Map Leaderboard. There is no Visit History (the site holds one entry per map, band and kind), no Report (it stays on the map's page, where the entry is judged beside the others) and no solo ceiling (ADR 0024 kept the ceiling to the cap alone, and a best above it, from a party or an EXP event, would look like cheating).
- **Over the entries, who they are and what they hold.** The page heads with the name as last uploaded, then the job and level of their newest entry (by when its Visit set it), the nearest the site comes to "where they are now". Under it, four figures in panels: the **#1s** they hold now, one for each map, Level Band and kind they top, since each band is its own board and its own #1, however few entries it has; their **podiums**, the places in a band's top 3; how many **maps** they're on, in how many Level Bands; and their **best EXP/h**, with its map. All are worked out from the entries as they stand: no history is kept, so "#1s ever taken" isn't one.
- **No name is tied to another.** The page names no other Character, and never the Account a name's key is linked to (ADR 0023 keeps the Discord account unshown). A player's characters stand alone.
- **A name with no entries reads the same whatever the reason**: never uploaded, opted out, or every entry taken down by the maintainer. The page shows the owl lost and "Egbert has no bests on the Map Leaderboards.", with a link to "Where's my character's entry?" (the FAQ's answer on why an entry may not be there), and says nothing of whether the name is claimed or was moderated.
- **A name followed glides into the page.** A name clicked on the Map Leaderboards grows from where it was into the Character page's heading, the pages cross-fading under it: a cross-document View Transition (`site/seams.html`), so the boards and the page read as one window. Only that click does it; anything else, and any browser without it, is a plain navigation. A map clicked on the Character page would glide into the map's page too, but that page scrolls to the marked entry as it opens, which the API draws after the first paint, so it stays a plain navigation.
- **The owl lost is the site's 404 too.** Cloudflare serves `site/404.html`, a page of the site's own with the owl lost in place of a heading and no tab lit, for any address that is neither a file nor the API's (`not_found_handling: "404-page"`), with its 404 status. A map's page that names no map, or one there isn't, shows the same owl.

## The API

Beside ADR 0015's and ADR 0024's, a Character's page reads one route. It is public and cached for a minute, as the other reads are.

- `GET /api/characters/<name>`: `{ "name": "Egbert", "entries": [{ "map": "010001010", "band": 20, "rank": 2, "total": 12, "job": "Magician", "level": 23, "expPerHour": 153216, "setAt": 1760000000000, "mobs": [22, 27], "mark": { "id": "Henesys", "name": "Henesys", "picture": "https://osmsdataexplorer.com/data/current/images/map_marks/Henesys.png" } }] }`. `name` is as last uploaded, whatever the case asked. `rank` and `total` are the entry's in its map and band, ranked among its own kind (Solo or Party), with the link opening that kind's board (ADR 0015). `mobs` and `mark` are as `GET /api/bands/<band>` gives a map's, `null` for a map the ceilings table doesn't hold or one without a Map Mark. A name with no entries, whatever the reason, is `404 not-found`; a name that can't be a Character's is a 400. Apart from the portrait (ADR 0029), the answer holds only public entry data: no claim date, key or Account.

Each job shows its class's icon where one is known. Each entry is ranked among its own kind, a Party Best carries a party mark, and its map link opens that kind's board (ADR 0015). The page orders, counts and names the entries itself (`site/character/character.ts`), from the map names fixture, as the Map Leaderboards' pages do.

## Considered options

- **Not counting a #1 on a board of one entry**, or a top-3 place on a board of three or fewer. Kept counting them: every Level Band is its own board, and the rank's "of N" says how many it had.

- **The solo ceiling beside each entry**, as the Personal Bests window shows how close a best came to it. Taken out of Where to train in review for the reason above, and the same holds here.
- **A Report button on each entry.** One way to report keeps moderation simple, and an entry is better judged beside the others on its board.
- **"#1s ever taken" and "best rank ever".** Fun, but they need history the site doesn't keep: a taken top spot is kept only while the entry that took it stands (ADR 0024). "Holds now" is worked out from the entries alone.
- **The player's other Characters**, from the Account their keys are linked to. It would show which names are one player's, which ADR 0023 keeps unshown.
- **A per-name link preview**, the Worker rewriting the page's `<title>` and `og:` tags for each name, so a link pasted in Discord shows the Character. It means running the Worker for `/character/` and reading D1 for every preview; deferred (#175). The page sets its `<title>` from the browser, for tabs and history.
- **A "Find a character" box** on the Map Leaderboards page. An exact name is all it could take without listing every name uploaded; not yet, since every name the boards show is a way in.
- **A link from the Personal Bests window** to the Character's own page. Site only for now.
- **The page under `/leaderboards/`.** It is a Character's, not a board's; `/character/` leaves room for more than entries.

## Consequences

- The Map Leaderboards' pages and the Discord post write every name as a link, so a change to how a name is written is a change in each (`site/entries.ts`'s `characterLinkTo` is the one address).
- Every entry of a Character is on one public page, where before it was spread over the maps' pages. Nothing new is shown, but it is easier to see all at once; the FAQ says so ("Can others see all my character's bests in one place?"), and taking the Character off the Map Leaderboards takes the page with it.
- `site:test` runs the page's own tests (`site/character`), and the Worker's test the route.
