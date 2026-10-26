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
| `grind-tracker.png` | The Grind Tracker: Time to Level, the rates for the last 5 minutes and the Session, a Best row, the Pace Trace under them, the map's solo EXP/h with its ? help icon, and the EXP gauge at its foot with the character playing ("Egbert · Lv. 36") and its forecast. | 280×279, the Grind Tracker's window | 0.14.0 |
| `item-card.png` | An Item Card at the pointer under the game's tooltip (One-Handed Blunt Weapon Attack Scroll: Greater), over the inventory: its window bar, the item's picture beside its name, Shop pays, and under Market its Price line with the Deal Line's filled ring, the buttons to add a line and open MesoWatch, and the credit's info icon at its foot. | 602×473, the inventory, the tooltip and the card | 0.14.0 |
| `item-lookup.png` | An item's tooltip in the inventory, and the Item Lookup alert above it saying the page it opened. | 611×742, the inventory, the tooltip and the alert | 0.7.0 |
| `setup.png` | Setup reopened, at Finding your map (step 3 of 4), its notice saying there's nothing to redo: the map found in the minimap's title (Kerning City) beside its Map Mark, tagged Found, with the hotkeys to use while you play. | 522×592, the setup window | 0.14.0 |
| `deal-finder.png` | A Player Shop's first four listings with the Deal Finder's badges beside them: two pale (0%, +3%), a blue deal (−1% · 99,999) and a purple one (−25% · 6,632). Cut to the badges and the list, so no seller's name, chat or other player is in it. | 481×250, the badges and the shop's list | 0.14.0 |
| `wip.png` | Not a shot: Minerva's owl in a hard hat beside a sign, "WIP, screenshot coming soon". The page shows it in place of a shot listed under Retake that is too far out of date to show, until it is retaken. | 1448×1086 | |

## The link card

`site/public/card.png` is the picture a link to the site shows (Open Graph and Twitter cards, so Discord and Reddit): the site's window, 1200×630, with the logo, the hero's title and a sentence beside the top of `hero.png`. It is `card.html` here as Chrome draws it. When `hero.png` is retaken, draw it again from the repo's root:

```sh
chrome --headless=new --hide-scrollbars --allow-file-access-from-files --window-size=1200,630 --screenshot=site/public/card.png site/screenshots/card.html
```

## Retake

When a change alters something a shot shows, the PR that makes it lists the shot here, with what changed. At release, the smoke test retakes every shot listed (`docs/releasing.md`) and empties the list.

- `item-lookup.png`: Item Lookup's alert is now the owl saying it in a speech bubble (ADR 0025). Retaken, the table's row and the home page's alt text say the owl in place of the alert.

- `grind-tracker.png`: the Grind Tracker has a Meso row under its rates ("Mesos", the Session's mesos and mesos per hour, with a ? help icon beside the label). Retake it from a Session with some mesos picked up, and add the row to the table's row and the home page's alt text.
- `game-window.png`: the Grind Tracker in it gains the same Meso row, if its Session has started.

A release may go out without retaking them. Then the page shows `wip.png` in place of each shot too far out of date to show, and the shot stays listed. None does now. Retaking one means pointing its `<img>` back at its own file (a scene from the game, as the Item Card's, on its own stage again: `shot stage scene`, where the sign stood on the panel's), and writing its alt text afresh, from what it shows and the entries below.

- `map.png`: the Mobs section's figures show down the right edge of the card, past the panel's edge beside it ("hare", "2%", "30%"). Retake it with the Mobs section folded, so only the Map section and the card show.
- `game-window.png`: the hero's film (`site/reel.ts`) no longer shows the title being marked: Minerva's box appears around the minimap's title by itself, with no veil. A retake that moves the minimap's title means setting `FOUND` at the top of `site/reel.ts`.
