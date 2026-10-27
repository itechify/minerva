# Samples

Each directory here holds frames of the game that a test feeds to the core and checks the reading of. Every file in one must read as its name (or its readings file) says, or as unreadable, never as something else. A frame the core ever read wrong becomes a sample here: a Bug Report's `crops/` has it already, cut and named as the directory's rule says. Names may only come from the maintainer's own Characters, or from players who agree to it in the Bug Report flow; any other player's name or chat is cut or painted out before a frame is committed.

## `minimap`

Every file in `samples/minimap` must be recognised against the map names fixture, `crates/minerva-core/tests/fixtures/osms-map-names.json`; there, a `--window` sample's client area is fed whole, and the title is found in it wherever it is (ADR 0021). Each is also fed scaled to other UI Sizes, where it must read as its map or as unreadable, never as another (`tests/minimap_samples.rs`). A title ever read wrong becomes a sample there: a Bug Report's `crops/minimap-title-<ms>.png` is one already.

## `exp-bar`

Every file in `samples/exp-bar` must read as its name (or its recording's `readings.json`) says, or as unreadable, never as a wrong number, in two layers (ADR 0017): the files there are the EXP figures alone, fed as a frame of them (`tests/exp_bar_samples.rs`), and the `--window` samples in `samples/minimap` are found in their Status Bar and read too. A number ever read wrong becomes a sample of the figures there: a Bug Report's `crops/exp-bar-<ms>.png` is one already. A Status Bar whose figures are ever not found, or found in the wrong place, becomes a sample of the bottom of the client area, its name's line painted over as for `samples/status-bar` unless that rule allows it: `samples/exp-bar/<exp>__<percent>--<what>-strip.png`, the client area's full width, fed whole as a Status Bar capture, with no other player's name or chat in it. A Bug Report's `crops/status-bar-strip-<ms>.png`, the strip kept whole where no Status Bar was found, is such a sample already, renamed to that shape: its name's line painted over unless that rule allows it, and any chat in it painted out.

## `item-tooltip`

Every file in `samples/item-tooltip` must read as the item its name says (or as no Item Tooltip, for `no-tooltip`), or as unreadable, never as another item (`tests/item_tooltip_samples.rs`). A frame that is ever read wrong becomes a sample there: a Bug Report's `crops/item-tooltip-<ms>@<x>,<y>.png` is one already, renamed `<item-name>@<x>,<y>.png`, cropped to the tooltip so no other player's name or chat is committed (the pointer's spot moved with it).

## `player-shop`

Every file in `samples/player-shop` must read as its readings say (`<name>.json`: each listing not sold out, with its item's name, quantity and mesos, or `{ "covered": true }` for one the pointer or the game's tooltip covers), or as unreadable ("?"), never as another item or price (`tests/player_shop_samples.rs`). A frame that is ever read wrong becomes a sample there: a Bug Report's `crops/player-shop-<ms>.png` is one already, cropped to the shop's list so no seller's name or chat is committed. Where no shop was found, a Bug Report keeps the whole client area instead, `crops/client-area-<ms>.png`: a frame to cut to the shop's list in the same way, with the margin `crops/player-shop-<ms>.png` keeps, before it is committed. A scroll's listing whose name the game cut short also says its `"tier"` (`"Lesser"`, `"Intermediate"`, `"Greater"` or `"Chaos"`), as its picture shows it, and only such a listing does: the test's made-up world gives it four scrolls of that start, one of each tier, so only the picture names it. A listing whose name the game cut short so that it could be several items also says `"named"`: the item its Item Tooltip names, "?" until hovered unless it is a scroll its picture names. `<name>@<x>,<y>.png` is `<name>.png`'s shop with the pointer at (x, y) on a listing and the tooltip by it, fed after `<name>.png`: cut as the list is but as far right as the tooltip's name line, the rest of the chat painted out, and the chat showing through the tooltip painted its fill.

## `status-bar`

Every file in `samples/status-bar` must read as its name says (`<level>__<job>__<Name>--<what>.png`; `<level>__<job>--<what>.png` for a crop with the name taken out; `no-status-bar--<what>.png`), or as unreadable, never as another level, job or name (`tests/status_bar_samples.rs`). A frame that is ever read wrong becomes a sample there: a Bug Report's `crops/status-bar-<ms>.png` is one already, cut to the level, job and name. Names may only come from the maintainer's own Characters, or from players who agree to it in the Bug Report flow; any other crop is cut down to the level and job before it is committed, its name's line painted over in the bar's plain slate (58,67,72).

## `pickup-log`

Each directory is a short sequence of 10 Hz log-band crops, named by capture milliseconds,
with `truth.json` listing the arrivals checked by eye, oldest first. Its first frame is a
baseline: lines already there are not arrivals. `source` identifies the maintainer's
2026-10-25 party (`1025`) or 2026-10-07 solo (`1007`) recording. Frames are cut to a 325×139
band at client (1590,830) in both recordings, checked against the source frames. The
2026-10-25 recording was captured at client (1400,730), 520×260: the committed clips
start at (190,100) within it. The 2026-10-07 recording was captured at client
(1560,780), 355×220: the clips start at (30,50) within it. The closing parentheses and six
22 px slots are kept, and the lines end 6 px inside the clip's right edge (11 to 12 px in the game's); the reader is given the UI Size only, never a log position.
Each clip must count only its arrivals, never more. An unlocated empty baseline cannot
prove the log is empty: the first located stack establishes the baseline, so those first
lines may be missed. At other UI Sizes it may read fewer, never another line (`tests/pickup_log_samples.rs`). The `covered` clip has another player's
entire name plate painted black in frames 6600–7200, including its border. All other clips
contain no player names. A Bug Report's `crops/pickup-log-<ms>.png` belongs in a short
sequence showing the frame before and after the problem, with the same privacy rule.

## `pickup-log-band`

Whole 400×480 captures of the Pickup Log's band, as `PanelState::pickup_log_capture` asks for it
at the samples' UI Size on a 1920×1200 client area, from the Bug Report of 0.15.0 at 2026-10-26
07:02 (Perion, Deep Valley and Perion Dungeon Entrance: rock, brick and cloud backgrounds, with
the game's quickslots and Cash Shop button in the band's bottom-left) and the one at 09:03
(Ellinia: bright greens and a white sky, over which the lines were read wrong). Named
`<kind>--<capture ms>.png`: `log--` shows lines of the Pickup Log (fading, covered by Mobs, or
fresh), `no-log--` shows none, and `lines--` are the six frames, in order, of the 09:03 report
whose lines are plainly legible: mesos (+13) and `Squishy Liquid x1 earned. (Etc)` arrive in
183631, another `Squishy Liquid x1` in 187992 and `Slime Bubble x1 earned. (Etc)` in 188167
(an EXP (+2) line arrived unseen between 184483 and 187992, in a crop the report left out).
A `lines--` clip reads as those lines, whole, in order (`tests/pickup_log_samples.rs`).
Lines end at band x≈390 and the newest is at band y≈240. A `log--` frame is never found anywhere
else, and a `no-log--` one finds none (`tests/pickup_log_samples.rs`). The only name in them is the
maintainer's own Character's, `Egbert`; a fragment of another name above it in `log--238612` is
painted over.

## `pickup-log-720p`

Short sequences of 10 Hz crops of the Pickup Log at a UI Size of about 0.64 of the samples' (a
Status Bar 35 px high against 55: a 1280×720 client area), from the maintainer's own 2026-10-26
recording on The Murky Swamp II, where the Bug Report of 0.15.0 at 14:41 never found the log. Lines
are 14 px apart and the text 10 px tall, drawn by the game at that size, not scaled from the other
samples. Each directory is a clip, its frames named by capture milliseconds in the recording, with a
`truth.json` as in `pickup-log`: `baseline` is the first frame, whose lines are not arrivals, and
`arrivals` are the lines that came in each later frame, oldest first, checked by eye. `source` is
`720p`. A recorded crop was 320×250 at client (960,380); these are cut to 208×96 at its (112,100),
which is client (1072,480): the six line slots and their closing parentheses, the lines ending 7 px
inside the right edge, which is the client area's (as at the samples' size, 11 to 12 px). The clips
are fed as the real band is (`tests/pickup_log_720p.rs`): the band `capture_band` makes at a UI Size of
35/55 from a 1280×720 client area, 255×306 at client (1025,414), black, with the clip pasted in at
(47,66) in it, and a cold reader read at 0.636 for each clip. Each clip must count only the lines
that arrived, no earlier than the frame that brought them and no later than half a second after it (a
line is read again at the captures after it arrives, and counted when the reads vote), and tally no more mesos or Items than
arrived. At this size it may read fewer, and `Unreadable` stands for a line seen but not read:
`exp-and-mesos` (EXP, mesos, an Etc Item), `etc-and-use-items` (mesos, Etc and Use Items, a name cut
short, and a Mob's sprite over the top row for the clip's last second), `skill-effect` (a skill's
beam over the log's right end for 8 seconds, and four EXP lines arriving at once under it) and
`burst` (four EXP lines at once, two more one at a time, then a Use Item and mesos together, an Etc
Item and mesos a frame apart, over a thick, moving grass). No
clip shows a player's name: its only text besides the log is the world's. Mobs may show their own
names (the `Ligator` at the left of `etc-and-use-items`).
