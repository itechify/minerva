# Minerva

A helper overlay for MapleStory Classic World that recognises which map the player is on and tells them what is there:
the mobs they can hunt and the hidden portals they would otherwise miss. While they hunt, it also measures how fast they
are levelling.

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
The name the game shows for a map (e.g. "Henesys Hunting Ground I"). Even with its Street Name it may name several maps;
the map's ID tells them apart.
_Avoid_: title, map title

**Minimap Title**:
The street name and map name as the game draws them in the minimap window: two lines when the minimap is open, one line
("Street : Map") when it is collapsed or the map has none. The signal Minerva recognises the map from, found wherever
the minimap is (ADR 0021).

**Map Mark**:
The small emblem the in-game minimap draws beside its title, for the area a map belongs to (Henesys's shield, Kerning
City's stone, Sleepywood's skull). Many maps share one; a few have none. Minerva shows it beside a map's name, so a
glance says where in the world the map is.
_Avoid_: map icon, region icon

**Candidate**:
One of several maps that share a Minimap Title, offered to the player when the map they came from doesn't settle which
one they are on.

**Saved Copy**:
Map and mob data cached earlier and shown, with its date, because its source can't be reached right now.

**Refresh**:
The Settings window's Game Data button for fetching OSMS's map set and item files afresh.

**UI Size**:
How large the game draws its windows and text, which its UI Size setting (Default Ratio, sharpened or not, or Ideal
Ratio, which picks a size for the resolution) and its window size decide together. Minerva reads the game at any UI
Size.
_Avoid_: UI scale, zoom, display scale (Windows' own setting, which Minerva's windows follow)

**Portal**:
A spot on a map that moves the player to another map (or another spot on the same map).

**Hidden Portal**:
A portal that is not shown on the minimap or visibly in the world, so the player must already know it is there. Its
destination may be any map, not only a Hidden Street.
_Avoid_: hidden street (when meaning the portal), secret exit

**Hidden Street**:
A map whose street name is literally "Hidden Street". Not the same thing as a hidden portal: a Hidden Street can be
reached through a visible portal, and a hidden portal can lead somewhere that is not a Hidden Street.

**Town**:
A map that is a settlement (shops, NPCs, no mobs) rather than a hunting field, as marked in the map data.
_Avoid_: city, village (except inside map names)

**Door**:
A portal from a town into a building interior, drawn as part of the scenery. The player can see it, so it is not a
hidden portal even though the minimap doesn't show it.
_Avoid_: entrance, building portal

**Quest Portal**:
A hidden portal that only works while the player meets a quest condition.
_Avoid_: script portal

**Map Preview**:
A picture of the whole of the shown map, with every portal marked where it is, its hidden portals set apart, and every
Spawn Point dotted; and, on the map of a Destination picked through an NPC, that NPC marked in green where it stands.
_Avoid_: minimap (the game's own), map image

### Hunting

**Mob**:
A hostile monster that spawns on a map.
_Avoid_: monster, enemy, creature

**Mob Card**:
The card that appears when the player hovers a mob's name in the panel: its image and the figures the row doesn't show.
Clicking the name opens the mob's MeowDB page.
_Avoid_: tooltip, popup, mob info

**Spawn Point**:
A spot on a map where one of its mobs appears, again and again once killed. The Map Preview dots each one in the
minimap's red.
_Avoid_: spawn, spawner

**Spawn Count**:
How many of a given mob a map holds at once: its spawn points there.

**Spawn Share**:
A mob's spawn count as a fraction of all mob spawns on the map.

**Mob Rate**:
A map's multiplier on how fast its mobs respawn (e.g. 0.5x, 1.6x).

**Density**:
How many of a given mob a player can kill on a map: its spawn count times the map's mob rate. A map with more of the
mob, or that refills faster, is denser for that mob.
_Avoid_: spawn rate, mob density (when meaning spawn share)

**Density Tier**:
High, Medium or Low: how a map's density for a mob compares with that mob's densest map (High from two-thirds of it,
Medium from one-third).

**Distance**:
The fewest map changes it takes to get from the player's current map to another map. A map change is going through a
portal (but not a Quest Portal) or a ship ride, and, if the player has turned them on, a taxi ride or a Teleport Scroll.
Minerva knows only which map the player is on, never where on it they stand.
_Avoid_: position, proximity

**Search**:
One name search over mobs, maps, items and NPCs, in a view over the panel. Its matches lead to a Hunting Target, a
Destination or an Item Card.
_Avoid_: mob search, item search

**Hunting Target**:
The mob and map the player picked in Search (the mob, then one of its maps) to go and hunt. Minerva shows the way there
from the player's current map, then that they have arrived, until they leave that map or clear it.
_Avoid_: waypoint, goal

**Destination**:
A map the player picked in Search to go to, either for itself or for an NPC on it, with no mob in mind. It takes the
Hunting Target's place.
_Avoid_: waypoint, goal, target (alone)

**Ship**:
An NPC-run crossing between regions (e.g. Ellinia, on Victoria Island, to Orbis, in Ossyria). The only way to reach
Ossyria, so it always counts as a map change.

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
The window beside the panel that shows the Session's EXP rates, Time to Level, Pace Trace, deaths and Loot, and compares
the Rolling Rate with the Personal Best.
_Avoid_: EXP tracking, EXP tracker

**Session**:
The stretch of play Minerva measures rates over, from the first EXP change, gain or loss, until the player resets it or
switches Character. Idle stretches pause it; a reset can be undone until the next EXP change.
_Avoid_: run, grind (the Grind Tracker measures a Session; "grind" alone names nothing)

**Character**:
One of the player's characters in the game, known by their in-game name on the Status Bar. A player may have many
Characters; an Account may link their names for uploads.
_Avoid_: player (the person, who may have many), account, toon

**Status Bar**:
The game's bar at the bottom of the client area, showing the Character's level, job and name, HP, MP and the EXP bar
with its figures.
_Avoid_: HUD, bottom bar, EXP bar (the gauge in it)

**Rolling Rate**:
A rate measured over the last few minutes of active play in a session, as opposed to the average over the whole session.
_Avoid_: current rate, live rate

**Pace Trace**:
The last 5 minutes of active play drawn kill by kill on the Grind Tracker, raced against the 5 minutes before them: how
far ahead of that earlier pace the player is, or behind it. On a map with a Personal Best, once the player comes within
reach of its pace, it races that too.
_Avoid_: graph, chart, sparkline

**Personal Best**:
A Character's highest Rolling Rate in EXP/h over a full 5 minutes of active play on one map, including the Visit under
way. A map has two kinds, its Solo Best and Party Best.
_Avoid_: record, high score, PB, max

**Solo Best**:
A map's Personal Best over 5 minutes with no party bonus in them.
_Avoid_: personal solo best, solo record

**Party Best**:
A map's Personal Best over 5 minutes with the party bonus in them, kept apart from its Solo Best so a party run never
hides what the Character does alone. The Grind Tracker races the Party Best while the player is partied.
_Avoid_: group best, party record

**Party EXP**:
The EXP the Pickup Log says came from being in a party: a share of a partymate's kill ("You received EXP" in yellow) and
the bonus on every kill while partied ("Bonus EXP for PARTY"). The player's own EXP is "You received EXP" in white.
Only the bonus, told by its words, makes 5 minutes party play: a white line can read as yellow.
_Avoid_: shared EXP, group EXP, bonus EXP (only part of it)

**EXP Split**:
How much of the Session's EXP was the player's own and how much was Party EXP, as shares of what the EXP bar gained,
given on hovering the Grind Tracker's Session EXP/h once the party bonus has come in.
_Avoid_: EXP breakdown, EXP sources

**Level Band**:
Ten levels, 1–9, 10–19, 20–29 and so on, that a Visit's best is counted in by the level that set it. A Visit History
keeps the best Visit in each, and a Map Leaderboard ranks each on its own.
_Avoid_: bracket, tier, level range

**Unassigned**:
Personal Bests with no known Character, kept apart from each Character's bests and never uploaded. The player can assign
them to a Character.
_Avoid_: unknown character, shared bests

**Personal Bests window**:
The window over the game for comparing one Character's Solo Bests and Party Bests across maps, or the Unassigned ones,
with each map's Visit History.
_Avoid_: leaderboard (the site's are Map Leaderboards), records, best list, Personal Bests page

**Visit**:
The stretch from the panel confirming a map until it confirms another. Losing the title for a while doesn't end it;
closing Minerva does.
_Avoid_: stay, trip

**Visit History**:
A Character's best ended Visits on a map, or the Unassigned ones: the 10 best of each kind, Solo and Party, and the best
of each kind in each Level Band. Its highest is the Personal Best.
_Avoid_: runs, attempts, records

**Map Leaderboard**:
A map's page on owlofminerva.app ranking Characters' Personal Bests by EXP/h within each Level Band. It ranks Solo
Bests, Party Bests or each Character's higher entry; ties go to whoever set it first.
_Avoid_: leaderboard (alone), ranking, high scores, hiscores

**Where to train**:
The Map Leaderboards page's table of maps for one Level Band: maps whose middle half of spawns is at the band's levels,
and maps with an entry in it.
_Avoid_: best maps, recommended maps, map ranking

**Pedestal**:
Three Characters standing on steps on the Map Leaderboards page, ranked by Most podiums (places in a map and Level
Band's top 3, each kind apart) or Highest EXP/h on any map at any level.
_Avoid_: hall of fame, top players, podium (that is one place in a band's top 3)

**Character page**:
A Character's page on owlofminerva.app showing their Map Leaderboard entries and ranks by Level Band, their #1s,
podiums, maps and best EXP/h, with their Portrait. It ties no Characters or Accounts together.
_Avoid_: profile, player page, Personal Bests page

**Portrait**:
A Character's photo from ClassicRank, shown with their name on the site and in a Top spot taken post on Discord.
_Avoid_: avatar, picture, profile picture

**Top spot taken**:
An upload that leaves its Character holding a map and Level Band's #1 that another Character held before: posted to
Discord (ADR 0022), and marked "took #1" in Latest while the entry that took it stands (ADR 0024). A first entry in an
empty band, or a Character beating their own best, isn't one.
_Avoid_: record, new record, crown

**Leaderboard Key**:
The secret identity of one Minerva install for Map Leaderboard uploads. A Character's first upload claims their name for
that key; keys linked to the same Account can also upload for it.
_Avoid_: password, token, account

**Account**:
An optional Discord sign-in on owlofminerva.app linking a player's PCs' Leaderboard Keys, so their claimed names follow
them between PCs. Uploading needs no Account.
_Avoid_: profile, login (alone), user

**Block**:
The maintainer's bar on Map Leaderboard uploads through a Character's Account and its linked Leaderboard Keys, the key
that claimed the name, and optionally the name itself. It hides their entries until lifted.
_Avoid_: ban, suspension

**Time to Level**:
How long the player would take to reach the next level at their net EXP rate.
_Avoid_: ETA, time to next level

**Pickup Log**:
The fading lines in the bottom-right corner of the game screen that announce gained EXP, gained mesos and picked-up
items.
_Avoid_: chat, drop log, kill feed

**Meso**:
The in-game currency; the game itself says "mesos".
_Avoid_: miso, gold, money

**Loot**:
The mesos and Items the Pickup Log says the player picked up in a Session, each Item tallied by name. Read from the
Pickup Log, it may be less than was picked up, never more (ADR 0026). The Grind Tracker shows it as a collapsible
section.
_Avoid_: pickups, drops, haul

### Items

**Item**:
Something the player can hold in the inventory: equipment, potions, scrolls, chairs, ores, Cash items. Each has its own
MeowDB page.
_Avoid_: drop, loot (when meaning one item; Loot is the Session's mesos and Items)

**Scroll Tier**:
Which of the four scrolls that improve the same thing one is: Lesser (100%), Intermediate (60%), Greater (10%) or Chaos,
the end of its name ("One-Handed Sword Attack Scroll: Greater"). Every scroll of a tier has the same picture, whatever
it improves, coloured by the tier: blue-grey, red, gold or grey.
_Avoid_: grade, rank, success rate (each tier's rate, not the tier)

**Item Tooltip**:
The box the game draws by the mouse pointer when the player hovers an item: the item's name on the line with the blue
bullet (a crafted item's maker sits above it), then its picture and figures. It is the game's, not ours: the Item Card
and the Mob Card are Minerva's own.

**Item Card**:
Minerva's window of information and prices for an item picked from an Item Tooltip or Search: what a shop pays, Going
rate, Cheapest now and the player's Market Lines.
_Avoid_: item tooltip (the game's), price check, popup

**Market Line**:
One named set of market prices the player keeps across items ("Your price", "Typical", "Lowest ask"). Every Item Card
lists every line, with that item's price or "Not set"; a player starts with none.

**Going rate** and **Cheapest now**:
maplestory.market's two item prices: its established price (the 3-day average sale, or a fallback to the median asking
price), and the cheapest listing for one. They are read-only figures on the Item Card, separate from Market Lines.
_Avoid_: column, field, price type

**Market Import**:
A file of prices from the player's own tool for their Market Lines. It adds lines and newer prices to existing lines,
removing nothing.
_Avoid_: sync, feed (the tool's business, not Minerva's)

**Player Shop**:
The shop window another player opens in the game to sell their items, with a listing per item: its name, and "1 for 999
mesos" (how many for how much). The game's, not ours. A sold-out listing is greyed.
_Avoid_: store, Hired Merchant (a shop the game keeps open without its player, which the Deal Finder doesn't read yet)

**Deal Finder**:
What the hotkey (Ctrl+Shift+D by default) turns on and off: while it is on, Minerva reads the open Player Shop's
listings a few times a second and puts a Deal Badge beside each. Off at every launch.
_Avoid_: scanner, price check

**Deal Line**:
What the Deal Finder compares listings with: the Going rate, Cheapest now or one of the player's Market Lines, chosen in
the Settings window. The Going rate until they choose (with it off, their first line).
_Avoid_: target price, reference price

**Deal Margin**:
How far below the Deal Line, in percent, the price of one item in a listing (of a throwing star's or a bullet's set, the
listing's price) must be for it to be a deal. Set in Settings; 0 by default, so any price below the line is a deal.
_Avoid_: threshold, discount

**Deal Badge**:
The mark beside a Player Shop listing showing how its price compares with the Deal Line and whether it meets the Deal
Margin, or why it can't be compared.
_Avoid_: tag, label, highlight

**Item Lookup**:
The hotkey (Ctrl+Shift+U by default) that reads the Item Tooltip nearest the pointer and opens the item's MeowDB page.
The Owl Bubble says what came of it, and when several items share the name, lists them for the player to pick. The same
line says what kept an Item Card from showing.
_Avoid_: item search (Search is the search view's)

### Over the game

**Owl Bubble**:
The owl's speech bubble at the top centre of the game window, with one line about Item Lookup, an Item Card that
couldn't show, or a newer Minerva version.
_Avoid_: alert, toast, notification, popup

### Settings

**Settings window**:
The window over the game where the player changes Minerva's settings, opened from the panel's gear or setup's Change
hotkeys.
_Avoid_: settings view, settings page, preferences, options

### Reporting bugs

**Bug Report**:
The `.zip` Minerva saves when the player chooses Bug Report in the tray menu, to go with a bug's issue: everything it
read and made of it since launch (up to a day), with crops of the regions it captured, and, where a reader found
nothing, the latest such capture whole. Fed back to the core, it goes through the same states again (ADR 0009).
_Avoid_: log, diagnostics, dump, trace (to the player; "trace" is its record of inputs, in the code)

**Near miss**:
What a reader that found nothing came closest to recognising, kept with its reading for a Bug Report.

### Updating

**Update Check**:
Minerva's ask of its release feed for the latest version: up to date, a newer version, or "Couldn't check". It installs
nothing.
_Avoid_: ping, poll, auto-update (nothing installs by itself)

**Update when I quit**:
The player's choice to download a newer version now and install it when they choose Quit Minerva, without starting
Minerva again. It lasts for this run.

**Notify-only**:
A build that only says a newer version is out, with its release page, and never installs one: the portable `.exe` and
dev builds, which the bundler didn't patch.
