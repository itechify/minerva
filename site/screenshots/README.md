# The site's screenshots

The pictures on the Minerva site, taken by hand in the game. The site's build fails while one the page shows is missing, so the page is never published without it.

## How to take them

- The game at **1920×1200**, windowed or borderless, with the minimap open.
- Minerva from the release the site describes, with the panel inside the game window.
- Capture as PNG, for example with Snipping Tool: its window mode for `game-window.png`, a rectangle for the others.
- **`game-window.png`** is the whole game window as Windows draws it, title bar included. **The others** are crops at full size, never scaled, so their text stays readable, each framing what the table says at about the size it gives. Cut the hero from `game-window.png`: it is that window's top-left corner at full size, and the hero's film (`site/reel.ts`) ends on that corner of the window, so the two must match pixel for pixel.
- Save each under its file name here, replacing the old one, and set the version it was taken on. The page takes each picture's size from the file, and the build makes its WebP copy (`webp/`, not committed), so nothing else needs to change; if what a picture shows changes, update its alt text and caption in the page that shows it (`site/index.html`, or `site/faq/index.html`) to match.

## The shots

| File | What it shows | Size | Taken on |
| --- | --- | --- | --- |
| `hero.png` | The minimap on a hunting map (Henesys Hunting Ground I) and the panel under it naming the same map beside its Map Mark, down through its Map (its spawn points dotted) and Mobs sections to the panel's foot, with a strip of the game beside it. Also the picture links to the site show (Discord, Reddit). | 600×960, the game window's top-left corner, its title bar included | 0.11.0 |
| `game-window.png` | The game on the same map, with the panel inside the game window under the minimap showing its Map and Mobs sections, every spawn point dotted and every mob listed, and the Grind Tracker under it. Also the hero's film, which finds and reads the minimap's title and drops the read into the panel's title: a retake that moves either title, or changes the map, means setting where they are and what the title says at the top of `site/reel.ts`. | 1922×1232, the whole window | 0.11.0 |
| `map.png` | The Map section on a map with a Hidden Portal, every spawn point dotted: the pointer resting on the Hidden Portal's row, its dashed ring lit, and the card under it opened on the close-up of the portal's spot, with its destination's Map Mark. | 320×410, the panel's Map section and the card | 0.10.0 |
| `mobs.png` | The panel on the hero's hunting map (Henesys Hunting Ground I), the pointer on a mob's name (Green Mushroom): its row lit, its spawn points ringed on the Map Preview, and its Mob Card over the list. | 369×578, the whole panel | 0.11.0 |
| `navigation.png` | The Map section with the next portal's magenta ring and every spawn point dotted, and Navigation with a Hunting Target: the mob, the figures, and the route from the map you're on, each map with its Map Mark. | 349×510, the panel from its title down through its Navigation section | 0.11.0 |
| `grind-tracker.png` | The Grind Tracker: Time to Level, the rates, a Best row, the Pace Trace under them, and the EXP gauge at its foot with its forecast. | 280×261, the Grind Tracker's window | 0.10.0 |
| `item-card.png` | An Item Card at the pointer over the inventory (Claw Attack Scroll: Intermediate): its window bar, the item's picture beside its name, Shop pays, Your price under Market, and the credit's info icon at its foot, with the game's tooltip under it. | 475×400, the inventory, the card and the tooltip | 0.10.0 |
| `item-lookup.png` | An item's tooltip in the inventory, and the Item Lookup alert above it saying the page it opened. | 611×742, the inventory, the tooltip and the alert | 0.7.0 |
| `setup.png` | Setup's step for marking the minimap title, reopened from Help & Setup (step 3 of 6), with Keep the current title. | 522×592, the setup window | 0.9.0 |
| `wip.png` | Not a shot: Minerva's owl in a hard hat beside a sign, "WIP, screenshot coming soon". The page shows it in place of a shot listed under Retake that is too far out of date to show, until it is retaken. | 1448×1086 | |

## Retake

When a change alters something a shot shows, the PR that makes it lists the shot here, with what changed. At release, the smoke test retakes every shot listed (`docs/releasing.md`) and empties the list.

A release may go out without retaking them. Then the page shows `wip.png` in place of each shot too far out of date to show, and the shot stays listed. Since 0.14.0 that is `grind-tracker.png` and `item-card.png` in `site/index.html`, and `setup.png` in `site/faq/index.html`. Retaking one means pointing its `<img>` back at its own file and writing its alt text afresh, from what it shows and the entries below. For `setup.png` it was: "Setup at its step Finding your map: the map Minerva found in the minimap's title, beside its Map Mark, tagged Found, with the hotkeys to use while you play."

- `grind-tracker.png`: the solo EXP/h comparison has a ? help icon explaining the calculation. Update its alt text in `site/index.html` to say so.
- `item-card.png`: each line under Market now has the Deal Line's ring before its name, filled on the Deal Line (Your price). Update its alt text in `site/index.html` to say so.
- `grind-tracker.png`: the EXP bar's reading at its foot has the character playing beside it, by the name and level the status bar shows ("Egbert · Lv. 13"). Update its alt text in `site/index.html` to say so.
- `setup.png`: a re-setup has one step fewer, as the EXP bar needs no mark (step 3 of 5), and its notice says "Mark the minimap title again, or keep it as it is." Update its row above.
- `grind-tracker.png`: the Best row moves up to a new best as soon as it is set, and says "this visit" after "Best" while this visit set it (ADR 0018). If the shot shows a best set on the visit, update its alt text in `site/index.html` to say so.
- `setup.png`: Help & Setup's About & Credits has an updates line and a Check for updates button under the version, and a "Minerva X.Y.Z is out. What's new" line while a newer version is out (ADR 0019). The mark step it shows is unchanged; if the retake shows About & Credits instead, update its row above and its alt text in `site/index.html`.
- `setup.png`: About & Credits has an Update button beside Check for updates while a newer version is out (in a build that can install), says when it's downloading or installing, and asks "Updating restarts Minerva and ends this Session." with Update now, Update when I quit and Later while a Session is running (#128). The mark step it shows is unchanged; if the retake shows About & Credits instead, update its row above and its alt text in `site/index.html`.
- `setup.png`: there is no step for marking the minimap title any more (ADR 0021). Setup's steps are the warning, the game window, setting up the game, Finding your map and the Grind Tracker: retake it at Finding your map on a first run (step 4 of 5), showing the map found beside its Map Mark, tagged Found, with the hotkeys to use while you play. Its alt text in `site/faq/index.html` already says so; update its row above.
- `game-window.png`: the hero's film (`site/reel.ts`) no longer shows the title being marked: Minerva's box appears around the minimap's title by itself, with no veil. A retake that moves the minimap's title means setting `FOUND` at the top of `site/reel.ts`.
