# Minerva

A helper overlay for MapleStory Classic World that recognises which map the player is on and tells them what is there: the mobs they can hunt and the hidden portals they would otherwise miss. While they hunt, it also measures how fast they are levelling.

## Language

### Maps

**Map**:
A single in-game area the player occupies at one time; the unit Minerva identifies.
_Avoid_: zone, area, level, field

**Street Name**:
The broader location label a map belongs to (e.g. "Victoria Road"). Many maps share one street name.
_Avoid_: region, area

**Region**:
The island-level grouping a street belongs to (e.g. "Maple Island", "Victoria Island").
_Avoid_: continent, world

**Map Name**:
The specific name of a map (e.g. "Henesys Hunting Ground I"). Not guaranteed unique on its own; a map is identified by its street name and map name together.
_Avoid_: title, map title

**Minimap Title**:
The street name and map name as the game draws them in the minimap window: two lines when the minimap is open, one line ("Street : Map") when it is collapsed or the map has none. The signal Minerva recognises the map from, found wherever the minimap is (ADR 0021).

**Map Mark**:
The small emblem the in-game minimap draws beside its title, for the area a map belongs to (Henesys's shield, Kerning City's stone, Sleepywood's skull). Many maps share one; a few have none. Minerva shows it beside a map's name, so a glance says where in the world the map is.
_Avoid_: map icon, region icon

**Candidate**:
One of several maps that share a Minimap Title, offered to the player when the map they came from doesn't settle which one they are on.

**Saved Copy**:
Map and mob data cached earlier and shown, with its date, because its source can't be reached right now.

**Refresh**:
The Settings window's button under Game Data that has OSMS's map set and item files fetched afresh now, rather than at the next daily ask, for what a patch brought. It says what it came to: up to date, OSMS unreachable, or Minerva needing an update.

**UI Size**:
How large the game draws its windows and text, which its UI Size setting (Default Ratio, sharpened or not, or Ideal Ratio, which picks a size for the resolution) and its window size decide together. Minerva reads the game at any UI Size.
_Avoid_: UI scale, zoom, display scale (Windows' own setting, which Minerva's windows follow)

**Portal**:
A spot on a map that moves the player to another map (or another spot on the same map).

**Hidden Portal**:
A portal that is not shown on the minimap or visibly in the world, so the player must already know it is there. Its destination may be any map, not only a Hidden Street.
_Avoid_: hidden street (when meaning the portal), secret exit

**Hidden Street**:
A map whose street name is literally "Hidden Street". Not the same thing as a hidden portal: a Hidden Street can be reached through a visible portal, and a hidden portal can lead somewhere that is not a Hidden Street.

**Town**:
A map that is a settlement (shops, NPCs, no mobs) rather than a hunting field, as marked in the map data.
_Avoid_: city, village (except inside map names)

**Door**:
A portal from a town into a building interior, drawn as part of the scenery. The player can see it, so it is not a hidden portal even though the minimap doesn't show it.
_Avoid_: entrance, building portal

**Quest Portal**:
A hidden portal that only works while the player meets a quest condition.
_Avoid_: script portal

**Map Preview**:
A picture of the whole of the shown map, with every portal marked where it is, its hidden portals set apart, and every Spawn Point dotted.
_Avoid_: minimap (the game's own), map image

### Hunting

**Mob**:
A hostile monster that spawns on a map.
_Avoid_: monster, enemy, creature

**Mob Card**:
The card that appears when the player hovers a mob's name in the panel: its image and the figures the row doesn't show. Clicking the name opens the mob's MeowDB page.
_Avoid_: tooltip, popup, mob info

**Spawn Point**:
A spot on a map where one of its mobs appears, again and again once killed. The Map Preview dots each one in the minimap's red.
_Avoid_: spawn, spawner

**Spawn Count**:
How many of a given mob a map holds at once: its spawn points there.

**Spawn Share**:
A mob's spawn count as a fraction of all mob spawns on the map.

**Mob Rate**:
A map's multiplier on how fast its mobs respawn (e.g. 0.5x, 1.6x).

**Density**:
How many of a given mob a player can kill on a map: its spawn count times the map's mob rate. A map with more of the mob, or that refills faster, is denser for that mob.
_Avoid_: spawn rate, mob density (when meaning spawn share)

**Density Tier**:
High, Medium or Low: how a map's density for a mob compares with that mob's densest map (High from two-thirds of it, Medium from one-third).

**Distance**:
The fewest map changes it takes to get from the player's current map to another map. A map change is going through a portal (but not a Quest Portal) or a ship ride, and, if the player has turned them on, a taxi ride or a Teleport Scroll. Minerva knows only which map the player is on, never where on it they stand.
_Avoid_: position, proximity

**Search**:
The search view the 🔍 or the search hotkey (Ctrl+Shift+F by default) opens over the panel: the mobs, maps and items whose name holds what the player types, a section for each, with each kind's best five. A mob leads to the maps it spawns on, to pick a Hunting Target; a map becomes the Destination; an item shows its Item Card.
_Avoid_: mob search, item search

**Hunting Target**:
The mob and map the player picked in Search (the mob, then one of its maps) to go and hunt. Minerva shows the way there from the player's current map, then that they have arrived, until they leave that map or clear it.
_Avoid_: waypoint, goal

**Destination**:
A map the player picked in Search to go to, with no mob in mind. Minerva shows the way there as for a Hunting Target, in its place: there is one or the other, whichever the player picked last. Picked on the map the player is on, it says they are there at once.
_Avoid_: waypoint, goal, target (alone)

**Ship**:
An NPC-run crossing between regions (e.g. Ellinia, on Victoria Island, to Orbis, in Ossyria). The only way to reach Ossyria, so it always counts as a map change.

**Taxi**:
An NPC who carries the player between towns for mesos.
_Avoid_: cab

**Teleport Scroll**:
An item that moves the player from any map to a town: either the map's own nearest town, or a named town.
_Avoid_: return scroll, portal scroll

**Map Summary**:
The hunting figures for a map as a whole, such as the EXP-per-hour ceiling and mobs killed per hour by party size.

### Tracking

**Grind Tracker**:
The window beside the panel, on unless the player turns it off in Settings, that measures a Session from the EXP bar: Time to Level, the Rolling Rate and Session rates, how the Rolling Rate compares with the Personal Best, the Pace Trace, deaths, and where the EXP bar will be at the Rolling Rate. Its window bar opens the Personal Bests window. The player closes it with its ×, and opens it again from the panel; closing it doesn't stop the Session, but turning it off stops reading the EXP figures, and the Session with it.
_Avoid_: EXP tracking, EXP tracker

**Session**:
The stretch of play Minerva measures rates over, from the first EXP gain until the player resets it or switches Character. A reset can be undone until the next EXP gain. Idle stretches pause it rather than count against it, and so does a stretch when the EXP bar can't be read; what the EXP did over one too long to be a gap in play isn't counted either. A misread that leads Minerva to learn the level's total wrong doesn't end it: the total is learned again. The switch is by name: another name on the Status Bar, after it was out of sight, is another Character, and a new Session at once. While no name can be read, readings in a row that fit another level after the EXP bar was out of sight are the switch instead. The level the Status Bar shows is the one the Session learns the total of, and a level-up it shows is counted at once.
_Avoid_: run, grind (the Grind Tracker measures a Session; "grind" alone names nothing)

**Character**:
Who is playing: one of the player's characters in the game, known by the name the Status Bar shows (Classic World is one world, so a name is one character). Minerva learns a Character the first time it reads the name; their level and job are as the Status Bar last showed them, and change, while the name doesn't.
_Avoid_: player (the person, who may have many), account, toon

**Status Bar**:
The game's bar pinned to the bottom of the client area: the Character's level in orange tiles after "LV.", their job and name, then HP, MP and the EXP bar with its figures ("EXP 266 [6.33%]"). The game's, not ours. Minerva finds it by itself, with no setup step, and reads the Character and the EXP figures from it every second.
_Avoid_: HUD, bottom bar, EXP bar (the gauge in it), EXP Bar Region (the mark there once was around the figures)

**Rolling Rate**:
A rate measured over the last few minutes of active play in a session, as opposed to the average over the whole session.
_Avoid_: current rate, live rate

**Pace Trace**:
The last 5 minutes of active play drawn kill by kill on the Grind Tracker, raced against the 5 minutes before them: how far ahead of that earlier pace the player is, or behind it. On a map with a Personal Best, once the player comes within reach of its pace, it races that too.
_Avoid_: graph, chart, sparkline

**Personal Best**:
The highest Rolling Rate, in EXP/h, a Character has had over a full 5 minutes of active play on one map: the best in their Visit History of the map, or the Visit under way's once it beats that. Each is stamped with the Character, level and job the Status Bar showed at the refresh that set it. The Grind Tracker's Best row shows the Character playing's (the Unassigned one while no name is read) as it stands, from arriving on the map and from the refresh that sets a new one, saying "this visit" while the Visit under way set it; a star marks the Rolling Rate while it is that best (ADR 0018).
_Avoid_: record, high score, PB, max

**Level Band**:
Ten levels, 1–9, 10–19, 20–29 and so on, that a Visit's best is counted in by the level that set it. A Visit History keeps the best Visit in each, and a Map Leaderboard ranks each on its own.
_Avoid_: bracket, tier, level range

**Unassigned**:
The Personal Bests no Character is known for: those saved before Minerva read Characters, and any set while the Status Bar's name couldn't be read. They stand apart in the Personal Bests window, the Grind Tracker races them while no name is read, and the player can move each to a Character, or assign them all to one at once. They are never uploaded.
_Avoid_: unknown character, shared bests

**Personal Bests window**:
Every map's Personal Best side by side for one Character at a time, in a window over the game that the Grind Tracker opens and closes, to compare maps: each with its mob levels and how close the best came to the map's solo EXP/h ceiling. Its picker shows the Character playing until the player picks another, or Unassigned. Each map opens to its Visit History, the Visit under way's best among them, each Visit with the level that set it. It shows the bests as they stand, the Visit under way's included, and keeps up with them while it is open. In it the player moves any Visit to another Character, assigns every Unassigned Visit to one, and, under a Character's Manage, removes them with their bests or merges them into another (a name changed in the game shows as a new Character), each after confirming. Under a Character's name, their switch "Share" uploads their bests to the Map Leaderboards and takes them off again, at a click, its tooltip saying the site is public; it starts on for a Character Minerva meets for the first time with a best that can go up, and stays off once turned off. Upload problems show under Manage whether it is open or not, each with Send again. An opened map's Visit History links to its Map Leaderboard. Beside the Share, Link Discord links this PC to the player's Account, or says whose Discord it is linked to; it asks once, quietly, a player sharing a Character who hasn't linked, until they link or say not now.
_Avoid_: leaderboard (the site's are Map Leaderboards), records, best list, Personal Bests page

**Visit**:
The stretch from the panel confirming a map until it confirms another. Losing the title for a while doesn't end it; closing Minerva does.
_Avoid_: stay, trip

**Visit History**:
A Character's best Visits on a map that have ended (or the Unassigned ones), highest first: the first is its Personal Best. It keeps the 10 best, and the best in each Level Band even if it isn't among them, so a Character that levels up keeps their lower bands' bests. The Visit under way's best joins it when the Visit ends, or when another Character is named. The player can remove any Visit in it that came from a bug, and the Visit under way's best before it joins.
_Avoid_: runs, attempts, records

**Map Leaderboard**:
A map's page on owlofminerva.app where players' Personal Bests stand side by side, each Level Band ranked on its own by EXP/h, ties going to whoever set it first. An entry is one Character's best Visit on the map in the band: its name, job, exact level, EXP/h and date. The site mirrors what the player uploads from Minerva, and keeps off an entry above 5× the map's solo ceiling (ADR 0015). The Map Leaderboards page leads to them through Where to train, a search by name, and Latest, the 20 newest entries on every map (ADR 0024).
_Avoid_: leaderboard (alone), ranking, high scores, hiscores

**Where to train**:
The Map Leaderboards page's table of the maps for one Level Band: each map whose middle half of spawns is at the band's levels, and each map with an entry in it, with its Map Mark, mob levels and the band's #1 there, the maps with entries first, in every area or one picked by its Map Mark (ADR 0024).
_Avoid_: best maps, recommended maps, map ranking

**Top spot taken**:
An upload that leaves its Character holding a map and Level Band's #1 that another Character held before: posted to Discord (ADR 0022), and marked "took #1" in Latest while the entry that took it stands (ADR 0024). A first entry in an empty band, or a Character beating their own best, isn't one.
_Avoid_: record, new record, crown

**Leaderboard Key**:
The secret each install of Minerva makes on its first upload to the Map Leaderboards and keeps in a file of its own beside the settings (`leaderboard-key`), sent with every upload. The shell keeps it, never the core, so no Bug Report holds it. A Character's first upload ties its name to the key; later uploads for the name must carry it, and another key's are refused as claimed. Never shown to the player; the site keeps only its hash. A lost one is replaced by linking the new one to the name's Account, if the old one was linked, or else by the maintainer reassigning the name.
_Avoid_: password, token, account

**Account**:
An optional sign-in on owlofminerva.app, with Discord, that the player links their PCs' Leaderboard Keys to (ADR 0023), each from that PC's Personal Bests window (Link Discord). A name its linked keys claimed may be written by any key linked to it, so the name follows the player to a new PC. Never needed to upload. The site keeps the Discord user ID and username, never shown. The maintainer signs in to one to moderate at `/admin/`.
_Avoid_: profile, login (alone), user

**Time to Level**:
How long the player would take to reach the next level at their net EXP rate.
_Avoid_: ETA, time to next level

**Pickup Log**:
The fading lines in the bottom-right corner of the game screen that announce gained EXP, gained mesos and picked-up items.
_Avoid_: chat, drop log, kill feed

**Meso**:
The in-game currency; the game itself says "mesos".
_Avoid_: miso, gold, money

### Items

**Item**:
Something the player can hold in the inventory: equipment, potions, scrolls, chairs, ores, Cash items. Each has its own MeowDB page.
_Avoid_: drop, loot (when meaning the item itself)

**Item Tooltip**:
The box the game draws by the mouse pointer when the player hovers an item: the item's name on the line with the blue bullet (a crafted item's maker sits above it), then its picture and figures. It is the game's, not ours: the Item Card and the Mob Card are Minerva's own.

**Item Card**:
The card the hotkey (Ctrl+Shift+I by default) shows at the pointer for the item whose Item Tooltip the game shows there, or Search shows beside the panel for an item picked there: what a shop pays for it, and the market prices the player keeps for it on their Market Lines (typed into the card, with a button that opens MesoWatch to look it up). It stays until the player closes it or presses the hotkey again, where the player drags it by its bar, and its name opens the item's MeowDB page. When items that don't look alike share the name, it first lists them, with their pictures, for the player to pick.
_Avoid_: item tooltip (the game's), price check, popup

**Market Line**:
One of the market prices the player keeps for every item, under a name they give it ("Your price", "Typical", "Lowest ask"). Every Item Card lists every line, in the player's order, each with the item's price on it or "Not set"; the player adds, renames and removes lines on the card. A player starts with one, "Your price".
_Avoid_: column, field, price type

**Market Import**:
A file of prices a tool of the player's own leaves in the `market-imports` folder beside `market-prices.json`, for their Market Lines. Minerva takes each in within a couple of seconds, at launch if it was closed, onto the lines of the same IDs, the newest price winning, and deletes it. It adds a line it names, but removes nothing. Minerva fetches no market prices itself.
_Avoid_: sync, feed (the tool's business, not Minerva's)

**Player Shop**:
The shop window another player opens in the game to sell their items, with a listing per item: its name, and "1 for 999 mesos" (how many for how much). The game's, not ours. A sold-out listing is greyed.
_Avoid_: store, Hired Merchant (a shop the game keeps open without its player, which the Deal Finder doesn't read yet)

**Deal Finder**:
What the hotkey (Ctrl+Shift+D by default) turns on and off: while it is on, Minerva reads the open Player Shop's listings a few times a second and puts a Deal Badge beside each. Off at every launch.
_Avoid_: scanner, price check

**Deal Line**:
The one Market Line the Deal Finder compares listings with, marked on every Item Card's market section, where the player chooses it. The first line until they choose one.
_Avoid_: target price, reference price

**Deal Margin**:
How far below the Deal Line, in percent, the price of one item in a listing (of a throwing star's or a bullet's set, the listing's price) must be for it to be a deal. Set in Settings; 0 by default, so any price below the line is a deal.
_Avoid_: threshold, discount

**Deal Badge**:
The mark beside a Player Shop listing while the Deal Finder is on: how far below the Deal Line its price is and the line's price ("−32% · 1,470") for a deal, coloured as the game grades items (blue for a good deal, purple for a great one, 20% or more below the line, gold for a steal, 40% or more), how far off it is ("+12%") for none, "no price" when the Deal Line has none for the item, and "?" when the listing can't be read for sure. A listing whose name the shop cut short, so it could be several items, is named by its Item Tooltip while the player hovers it, until the list changes. While the pointer or the game's tooltip covers its listing, it keeps what was read there before, marked with an orange dot. A click on it opens the item's Item Card.
_Avoid_: tag, label, highlight

**Item Lookup**:
The hotkey (Ctrl+Shift+U by default) that reads the Item Tooltip nearest the pointer and opens the item's MeowDB page. An alert at the top centre of the game window says what came of it, and when several items share the name, lists them for the player to pick. The same alert says what kept an Item Card from showing.
_Avoid_: item search (Search is the search view's)

### Settings

**Settings window**:
The window over the game where the player changes Minerva: its hotkeys (rebound by pressing the new keys, or turned off), the Deal Margin, the Grind Tracker's switch, Game Data's Refresh and "Check for updates". The panel's gear opens it beside the panel and closes it again, as its × does, and setup's Change hotkeys opens it; the player drags it anywhere, and it keeps that spot. It has the keyboard as it opens, and again when the player clicks a Change or the Deal Margin field; clicking back into the game gives the keyboard back, and the window stays open. Every change is saved at once. Each launch starts with it closed.
_Avoid_: settings view (the panel's, before the window), settings page, preferences, options

### Reporting bugs

**Bug Report**:
The `.zip` Minerva saves when the player chooses Bug Report in the tray menu, to go with a bug's issue: everything it read and made of it since launch (up to a day), with crops of the regions it captured, and, where a reader found nothing, the latest such capture whole. Fed back to the core, it goes through the same states again (ADR 0009).
_Avoid_: log, diagnostics, dump, trace (to the player; "trace" is its record of inputs, in the code)

**Near miss**:
What a reader that found nothing came closest with, written down with its reading for a Bug Report (#133): for the Player Shop, the stack of band rows of any width with the most rows; for the Status Bar, the row of level tiles with the most in it, and the LV box round them if it was measured. "No near miss" is an answer too. A Bug Report keeps the latest Deal Finder capture with one over the latest without.

### Updating

**Update Check**:
Minerva asking its feed (`latest.json` on GitHub's newest release) which version is the latest: at launch, every 6 hours while "Check for updates" is on, and whenever the player presses Check for updates. It sends only the version. It comes to up to date, a newer version (shown in the tray and Help & Setup, never on the panels), or "Couldn't check", which is also what one unanswered for 2 minutes comes to (ADR 0019).
_Avoid_: ping, poll, auto-update (nothing installs by itself)

**Update when I quit**:
The player's choice, when Help & Setup asks before updating during a Session, to have the newer version download now and install when they choose Quit Minerva, without starting it again. It lasts for this run. Quit Minerva chosen while it is still downloading waits for it; chosen again, it quits without it. The other answers are Update now (install and start it again) and Later (nothing more for now).

**Notify-only**:
A build that only says a newer version is out, with its release page, and never installs one: the portable `.exe` and dev builds, which the bundler didn't patch.
