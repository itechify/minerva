---
status: accepted
---

# The Minimap Title is found in the client area

Every reading but the map's found what it reads in the client area by itself: the Player Shop (ADR 0014), the Status Bar (ADR 0016) and its EXP figures (ADR 0017), at any UI Size (ADR 0020). The Minimap Title was read from a Title Region, a box the player marked at setup. Setup reopened whenever the game window changed size. The reader only knew the open minimap's two lines on its light-blue panel, and read at the one UI Size the samples were taken at. And a map without a minimap (Perion Department Store, Subway Ticketing Booth) shows its title as one line on a grey bar, "Victoria Road : Perion Department Store", which no Title Region read; so does a minimap the player collapsed (#143).

So Minerva finds the Minimap Title wherever the player put the minimap, and there is no Title Region.

## Finding and reading it

- **The anchor is the WORLD button.** The minimap window has an orange WORLD button at its top right, open, collapsed, or on a map without a minimap: 52 or 53 × 15 px of (240,167,21) at the samples' UI Size, its letters cream. It is where rows of that orange (or its letters' cream) over the same columns stack as tall as a button that wide is; or less tall at the capture's top, where a capture of where the title was cuts it off (the `--title-region` samples). The Status Bar's level tiles are another orange, (255,148,21), and smaller orange buttons, such as the Quest Helper's AUTO, aren't its shape.
- **Its width is the UI Size** (ADR 0020). Around the button, the capture is scaled to the samples' size and read there, with the tolerances the two-line reader was tuned on.
- **Two layouts.** The open minimap's title is two lines, street then map, on its panel below the button: read as it was (ADR 0002). Where there is no panel below, the title is one line on the bar left of the button and its two small buttons: dark grey text on light grey, its ink how dark a neutral pixel is, the bar's left edge where its light grey ends on clear rows above and below the text. The line is split at its colon, found in the ink (no wider than a letter's stroke, two dots one above the other), and its street and map are read as the open minimap's two lines are: ocrs drops the colon as often as not ("Victoria RoadPerion"). The text ends at the first gap wider than one between words: a collapsed minimap's bar is wider than a short title, and the edge of its first small button lies past it. The ink is taken next to the bar's own grey, which the game draws darker while a map fades in. The open minimap's header has the same buttons on the same kind of bar, with "MINI MAP" on it: it has no colon, and is unreadable.
- **Lock-on.** The whole client area is captured and searched about once a second until the title is found. From then on only where it was found is captured, three times a second: the minimap window's top, from its left edge, as wide (720 px at the samples' size) and as tall (110) as either layout is for the longest names, so a map change to a longer name, or between the layouts, stays in it. Once a capture there finds no title (the player moved or closed the minimap), or reads it as no map for 3 seconds (a moved minimap whose title the box cuts), the whole client area is searched again: a move costs a read or two. A title found but unreadable for less (the pointer over it) keeps its spot. A moved game window has it looked for again.
- **What the title read as is remembered** by its ink: standing in one map, the title costs a scan and no recognition.
- Nothing is captured while another app is in front, as for the Status Bar: another app's pixels are never read.
- **Our windows are never read.** The capture is of the screen, so it holds our windows; and the panel, its map's name on light blue beside an orange NOT SURE, passes for an open minimap. Wherever the core shows one of them (the panel state's `our_windows`), the capture is painted out before the title is looked for in it. Found in the panel, the title had the panel move clear of itself, and back once the next capture missed it.

## No override

There is no way to mark the title by hand, as there is none for the Player Shop, the Status Bar or its EXP figures (ADRs 0014, 0016, 0017). A frame read wrong, or a title not found, comes in as a Bug Report and becomes a sample. If players run into what can't be found, an override could come back for every reader at once.

## Considered options

- **The Title Region, kept as an override** that takes precedence when set. One more path to keep working and test, a setup step, a box that goes stale on a resize, and one the one-line bar may fall outside of; and the other readers chose against one.
- **The minimap's light-blue panel as the anchor**, as the two-line reader found the title within its box. It is no help with the one-line bar, and in a whole client area a sky of the panel's colour could pass for it.
- **Searching only the top-left corner**, telling players to keep the minimap there. Cheaper, but the game lets them drag it anywhere, and one dragged away would be lost without a word.
- **Capturing the whole client area on every read.** Simplest, but the title is read three times a second where the Status Bar is read once; a box around where it was found is a fraction of the pixels.

## Consequences

- Setup has no marking step and no test read: "Finding your map" shows the map found, and never holds setup up. A settings file with a Title Region in it has finished setup; the region is dropped (settings version 8). A new client size asks for nothing.
- The panel says "Can't find the minimap's title: is the minimap open?" once none has been found for 10 seconds, the map shown kept, with a link to the FAQ's answer. A title found but unreadable isn't that, and time behind another app, when nothing is captured, doesn't count.
- A reading writes down where the title was found (`Event::Title`'s `found`), and a replay takes it in again. A Bug Report's crop is where the title was found (`crops/minimap-title-<ms>.png`), none of the rest of the client area, and becomes a sample in `samples/minimap`; it may hold the game world right of a narrow minimap, as the `--title-region` samples do. A title never found leaves no crop (ADR 0020): such frames come from `probe record`.
- Our windows keep clear of the title where it was last found, while it is looked for again too: a panel dragged over the minimap would hide it from the search.
- The samples are the maintainer's: four `--window` screenshots of the open minimap, two of maps without one (Perion Department Store, Subway Ticketing Booth), a crop of the minimap, three of the old Title Region, and two Bug Reports' crops of a collapsed minimap (Perion, Free Market Entrance, and Free Market <1> to <4>, <1> twice, once as the map fades in, the Free Market's cut to the bar), all at one UI Size; the tests also feed each scaled up and down. Not yet covered: either layout at another UI Size the game draws; the minimap moved from the top-left corner; a Roman numeral in a one-line title; the pointer over one.
