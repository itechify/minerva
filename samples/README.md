# Samples

Each directory here holds frames of the game that a test feeds to the core and checks the reading of. Every file in one must read as its name (or its readings file) says, or as unreadable, never as something else. A frame the core ever read wrong becomes a sample here: a Bug Report's `crops/` has it already, cut and named as the directory's rule says. Names may only come from the maintainer's own Characters, or from players who agree to it in the Bug Report flow; any other player's name or chat is cut or painted out before a frame is committed.

## `minimap`

Every file in `samples/minimap` must be recognised against the map names fixture, `crates/minerva-core/tests/fixtures/osms-map-names.json`; there, a `--window` sample's client area is fed whole, and the title is found in it wherever it is (ADR 0021). Each is also fed scaled to other UI Sizes, where it must read as its map or as unreadable, never as another (`tests/minimap_samples.rs`). A title ever read wrong becomes a sample there: a Bug Report's `crops/minimap-title-<ms>.png` is one already.

## `exp-bar`

Every file in `samples/exp-bar` must read as its name (or its recording's `readings.json`) says, or as unreadable, never as a wrong number, in two layers (ADR 0017): the files there are the EXP figures alone, fed as a frame of them (`tests/exp_bar_samples.rs`), and the `--window` samples in `samples/minimap` are found in their Status Bar and read too. A number ever read wrong becomes a sample of the figures there: a Bug Report's `crops/exp-bar-<ms>.png` is one already. A Status Bar whose figures are ever not found, or found in the wrong place, becomes a sample of the bottom of the client area, its name's line painted over as for `samples/status-bar` unless that rule allows it: `samples/exp-bar/<exp>__<percent>--<what>-strip.png`, the client area's full width, fed whole as a Status Bar capture, with no other player's name or chat in it. A Bug Report's `crops/status-bar-strip-<ms>.png`, the strip kept whole where no Status Bar was found, is such a sample already: its name's line painted over unless that rule allows it, and any chat in it painted out.

## `item-tooltip`

Every file in `samples/item-tooltip` must read as the item its name says (or as no Item Tooltip, for `no-tooltip`), or as unreadable, never as another item (`tests/item_tooltip_samples.rs`). A frame that is ever read wrong becomes a sample there: a Bug Report's `crops/item-tooltip-<ms>@<x>,<y>.png` is one already, renamed `<item-name>@<x>,<y>.png`, cropped to the tooltip so no other player's name or chat is committed (the pointer's spot moved with it).

## `player-shop`

Every file in `samples/player-shop` must read as its readings say (`<name>.json`: each listing not sold out, with its item's name, quantity and mesos, or `{ "covered": true }` for one the pointer or the game's tooltip covers), or as unreadable ("?"), never as another item or price (`tests/player_shop_samples.rs`). A frame that is ever read wrong becomes a sample there: a Bug Report's `crops/player-shop-<ms>.png` is one already, cropped to the shop's list so no seller's name or chat is committed. Where no shop was found, a Bug Report keeps the whole client area instead, `crops/client-area-<ms>.png`: a frame to cut to the shop's list in the same way, with the margin `crops/player-shop-<ms>.png` keeps, before it is committed. A listing whose name the game cut short so that it could be several items also says `"named"`: the item its Item Tooltip names, "?" until hovered. `<name>@<x>,<y>.png` is `<name>.png`'s shop with the pointer at (x, y) on a listing and the tooltip by it, fed after `<name>.png`: cut as the list is but as far right as the tooltip's name line, the rest of the chat painted out, and the chat showing through the tooltip painted its fill.

## `status-bar`

Every file in `samples/status-bar` must read as its name says (`<level>__<job>__<Name>--<what>.png`; `<level>__<job>--<what>.png` for a crop with the name taken out; `no-status-bar--<what>.png`), or as unreadable, never as another level, job or name (`tests/status_bar_samples.rs`). A frame that is ever read wrong becomes a sample there: a Bug Report's `crops/status-bar-<ms>.png` is one already, cut to the level, job and name. Names may only come from the maintainer's own Characters, or from players who agree to it in the Bug Report flow; any other crop is cut down to the level and job before it is committed, its name's line painted over in the bar's plain slate (58,67,72).
