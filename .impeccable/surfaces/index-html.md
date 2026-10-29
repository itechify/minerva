---
version: 1
slug: "index-html"
primary_target: "index.html"
related_targets: ["src/main.ts","src/styles.css"]
---

# Overlay panel

Scope: the in-game overlay panel (index.html, src/main.ts, src/styles.css), every state it can show. Mode: Operate. A glanceable, clickable tool read mid-hunt.

Job: on each map change, tell the player which map this is, where its Hidden Portals are, and whether its mobs are worth grinding, in one glance. Constraints: 380px initial width, resizable down to 320px, content-height up to 600px or the available space, 44px collapsed square. Pointer interaction is always available; window bar and title panel drag the panel, sides and bottom corners resize width. Credit is an info-glyph tooltip in the footer.

Scene: a player at a desk, the bright, saturated 2D game filling the screen, glancing at the right edge between attacks. The game's own windows there (Quest Helper, minimap) are light, translucent periwinkle and lavender, so the panel is light.

## Direction contract

THESIS: The panel is the window the game forgot to ship. It sits under the Quest Helper in the game's own window language, and puts hidden exits first. It refuses the generic dark dev-tools card with a stats table on top. Boundary: the game lends materials (body, chrome, lettering, minimap glyphs) and one signature move. Organisation, hierarchy and density are a working tool's, never a screenshot of a game window.

OWN-WORLD: Quest Helper glass with near-black Panel Ink and its light text edge, under the Mini Map's grey window bar and sky-blue title panel. Street Name above Map Name in light, blue-outlined lettering. Portal blue, mob red and you yellow keep their minimap meanings; game orange marks doubt and problems. Maplestory Light and Bold throughout.

STORY: The player learns the map, sees each hidden exit's spot and where it leads, checks EXP/h, and gets back to hunting. When unsure, the panel says so before anything else.

FIRST VIEWPORT: Grey window bar with toolbar, then sky-blue title panel. Map section: the ringed Map Preview and Hidden Portal rows. Navigation while a target is set. Mobs: two-row summary, then every mob in a table; narrow panels use labelled cards. Credit info glyph at the foot. See DESIGN.md for spacing and states.

FORM: user-pinned, no roll (the user chose "lean into MapleStory" and confirmed this direction). Code-led: no image generation is available.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Portals and the Map Preview (#60)

Design pass for the Portals section, 10 Oct 2026. Reference: OSMS Data Explorer's map detail view (Maps tab, a map's row opened), read from its live page: the whole-map picture with a ring per portal, positioned at `x·scale − size/2`; blue rings (`#33aaff` on an 18% fill, a soft glow) to other maps, green rings (`#2ecc40` on a dark 45% fill, a stronger glow) for teleports within the map, grey (`#999`) to a map it doesn't know; dashed green arrows (`6 3`, 2px) for teleports, one line with a head at each end for a mutual pair, and one-way arrows that run to the landing point even where that point has no ring (a teleport landing on a spawn point). Its hover card names the destination with a thumbnail and its mobs. OSMS doesn't set hidden portals apart; we do.

Fixed by the ticket: the section is Portals; the preview sits on top at full width, at most about 180 px tall, fitted and centred, on every map; rows below, or "No hidden portals"; OSMS's three colours; Hidden and Quest dashed; Doors like visible portals; hover a mark for name, street and kind, and to light its row; hover a row to light its mark; a click does nothing; collapsing hides both, the count stays the Hidden Portal count; "Loading map…" in a fixed-height placeholder; the rows alone with "Couldn't load the map picture" on failure; no preview while shared titles are unsettled; the Quest tag off green.

Assumed in this pass (no one to ask in the session; correct here if wrong):

- **Ground.** OSMS's pictures carry alpha (three checked: Sunset Sky, Northern Top, Kerning City), so the glass would show the game through them. The picture sits on opaque Minimap Paper inside the inset frame (1px Inset Edge, 4px radius): the retired mini-minimap's paper carries over to the map it stood for.
- **Rings in screen pixels, not picture pixels.** Pictures run 1,500 to 5,400 px wide and show at about 358, so a ring is a fixed 9 px across wherever the picture is scaled to, over a white halo so it holds on dark art and on the paper alike. Blue is the panel's own Portal Ring Blue, not OSMS's lighter one: the Minimap Legend Rule keeps one blue for portals. Green is a new Teleport Green, grey a new Unknown Grey from the panel's cool inks.
- **Dashes.** Hidden and Quest rings are dashed over their solid white halo, so the gaps read white. A hidden teleport within the map is dashed too.
- **Arrows.** As OSMS draws them: a dashed Teleport Green line from ring edge to ring edge, or to the landing point when it has no ring, with a small head where it lands; a mutual pair is one line with two heads. A white under-stroke keeps it on any art.
- **Lit.** A lit mark (hovered, or its row hovered) gains a second ring around it in its own colour, a static target; a lit teleport's arrow thickens. A lit row takes Selection Periwinkle with its inset edge, whichever side the pointer is on. No motion: the panel has none.
- **Rows.** Each row leads with its mark's ring, dashed, in the mark's colour (grey when the destination isn't in the data), so the list and the picture read as one legend.
- **Card.** Hovering a mark shows a small card at once (marks are small and sparse; the Mob Card's 250 ms wait would feel broken), floating over the panel above the mark, or below it when there's no room, never leaving the panel, the pointer passing through: opaque Body Solid, Window Edge, 4px radius, the window's inner ring. The destination in 13px bold, its street in 12px Panel Ink Steel, then a tag per kind: "Hidden", "Quest", "Door", "Within the map", "Not in the map list". A visible portal to a known map has no tag.
- **Quest tag.** Not amber: cream and amber tags already mean a state to notice ("paused", "Taken", "Invalid") and the Orange Means Doubt Rule keeps the game's orange for problems. Quest moves to a violet no other meaning holds.
- **Loading.** A 140 px placeholder in the same frame and paper, "Loading map…" centred in 12px Panel Ink Steel; then the box takes the picture's own fitted height. **Failure** is the panel's problem note, "Couldn't load the map picture", in the preview's place.
- **Cursor and focus.** Marks keep the default cursor (a click does nothing) and take no focus; the preview is one image to assistive tech, named with the map and its portal count, and the rows carry the Hidden Portals as text.

## Portal close-up (overdrive, 12 Oct 2026)

Chosen by the user from three directions (close-up, labelled map at zoom, prefetched pictures), with motion allowed in the windows: DESIGN.md's Motion section replaces the old ban, and supersedes "No motion: the panel has none" above.

- **What.** The Mark Card opens on a close-up of the spot: OSMS's picture around the portal, which OSMS draws at the game's own size (mobs in it), at half that, 104px tall across a card at least 272px wide that grows with its words, as the old card did. A player sees the ledge, the fence or the signpost a hidden portal stands by, not a 9px ring on a map shrunk 4 to 15 times. Every ring in view is drawn in it, the card's own lit; the portal sits 62% of the way down, and the view stays on the picture at its edges.
- **Rows too.** Resting on a Hidden Portal's row shows the same card, below the row (above when there's no room), as the Mob Card sits by its row.
- **The one motion.** A new spot dives from the map at the preview's scale to the close-up in 360ms, on an exponential ease-out interpolated in log scale, the first frame drawn at once. The same spot lit again holds; reduced motion, or a preview already as close, opens on the spot.
- **How.** A canvas at the screen's resolution, drawn from the preview's own decoded `<img>`, so no picture is fetched twice and no layer is ever larger than the card. On a screen at twice the scale, the settled close-up is the picture's own pixels.
- **Checked** headless at 1× and 2× on a real OSMS picture (3-Way Road-Split's two hidden portals, and its exits at either end), filmed at a twentieth of the speed, and with reduced motion forced.

## Spawns on the map (overdrive, 12 Oct 2026)

Chosen by the user over a map that moves like a map and a labelled map at zoom. OSMS's `maps.json` gives every spawn point's `x` and `y` in the same picture pixels as the portals, and the core already read the points to count them; it now passes them to the overlay in `MapPreview.spawns`, each with its mob's ID.

- **The map.** Every spawn point is the minimap's red mob dot, under the arrows and rings, on a white halo a pixel wider so it holds on dark art. A busy map wears dozens, as its minimap does: 3-Way Road-Split has 68.
- **Lit together.** Resting on a mob's row (or its name taking focus) rings that mob's dots in the same red and fades the others to 30%. Resting on a dot does the same for its mob and lights its row in Selection Periwinkle, the row's own red dot ringed as the map's. The Mobs table is set out 5px at each side, its text where it was, so the lit tint clears it as a Hidden Portal row's does.
- **A dot's card.** The Mark Card, with its close-up and dive, for a spawn point: OSMS draws the mobs into its pictures, so the close-up shows the mob standing on its spot, and the words name it after the table's red dot, with "Lv 8 · 9 spawn points". Every close-up draws the dots in view, the lit mob's ringed.
- **No new motion.** Lighting is at once, as a ring's is.
- **Checked** headless at 2× on 3-Way Road-Split's real picture with its real 68 spawn points (its seven mobs stood in for by the made-up world's), at rest, with a row lit, a dot's card, a ring's card, and the narrow card layout with a row lit.
