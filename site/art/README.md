# The site's drawings

Pictures of Minerva's owl in the game's pixel-art style, drawn for the site and supplied by the owner (October 2026). They aren't screenshots, so `site/screenshots/README.md`'s rules don't apply to them. Each is the supplied drawing trimmed to what it shows, on a transparent ground, at twice the size the page shows it at. The build makes its WebP copy (`site/vite.config.ts`).

## The titles

They head a page in place of its title's words: the owl with a Slime and an Orange Mushroom on a wooden sign lettered with the title. The page shows each 440px wide, gives the `<img>` its size, and the heading's words as its alt text.

| File | Heads | What it shows | Size |
| --- | --- | --- | --- |
| `map-leaderboards.png` | `site/leaderboards/index.html` | The owl, crowned, on the first place of a podium, the Slime on the second and the Orange Mushroom on the third, over "Map Leaderboards" | 880×556 |
| `faq.png` | `site/faq/index.html` | The owl reading a book between two question marks, the Slime and the Orange Mushroom beside it, a stack of books and a notice board, over "FAQ" | 880×482 |

## The FAQ's groups

One over each group's heading in `site/faq/index.html`, the owl holding what the group is about. The page shows each 120px tall (`width` is its own), and as decoration (`alt=""`): the heading under it names the group.

| File | Group | What it shows | Size |
| --- | --- | --- | --- |
| `faq-safety.png` | Safety & your account | The owl holding a gold padlock | 227×240 |
| `faq-install.png` | Install & updates | The owl holding a download and a gear | 267×240 |
| `faq-setup.png` | Setup & hotkeys | The owl holding a gear and the keys Ctrl and K | 292×240 |
| `faq-panel.png` | The panel & the map | The owl reading a map with a dotted route, a signboard showing a minimap beside it, a blue portal and a signpost on its other side | 403×240 |
| `faq-hunting.png` | Hunting | The owl with a map and a magnifying glass beside a signpost | 326×240 |
| `faq-grinding.png` | Grinding | The owl in a headband, stars in its eyes, with an EXP board, coins, an hourglass and a trophy | 252×240 |
| `faq-leaderboards.png` | Map Leaderboards | The owl over a board of five ranked players, the Slime, the Orange Mushroom and a pink horned mob cheering under it | 217×240 |
| `faq-items.png` | Items | The owl in a headband carrying a shopping bag of coins and a potion | 301×240 |
| `faq-trouble.png` | Troubleshooting & Bug Reports | The owl in a hard hat with a wrench at a laptop, a speech bubble with a warning, a wrench and a bug, the Orange Mushroom worried beside it | 331×240 |

## The owl lost

Over a page with nothing to show: the site's 404 (`site/404.html`), heading it in place of a heading's words, which are its alt text; a Character's page with no entries, or a link naming none (`site/character/index.html`); and a map's page naming no map, or one there isn't (`site/leaderboards/map/index.html`). The page shows it 440px wide (300px on a phone), and its alt text is the sign's words. Trimmed of the supplied drawing's faint stray pixels under the ground.

| File | Shows on | What it shows | Size |
| --- | --- | --- | --- |
| `not-found.png` | The 404, a Character's page and a map's page with nothing to show | The owl puzzled, a question mark over it and a torn map with a red cross in its wing, beside a wooden signpost lettered "404" over "Not Found" | 880×574 |

## The missing portrait

Beside a Character's name where ClassicRank has no photo of them, or one that won't load (ADR 0029, `site/leaderboards/api.ts` draws it): a character's figure in black pixel art, a white question mark for its face, supplied by the owner (October 2026), trimmed to the figure on its transparent ground, brought down to its own pixel grid (24×40) and drawn at twice that, 48×80, in a 96×96 square as a character stands in a ClassicRank photo (its feet 8px from the bottom), so it reads at their size. It shows at its own 96px in a band's table and on a Character's page. Decoration: the name beside it says whose it is.

| File | Shows | What it shows | Size |
| --- | --- | --- | --- |
| `portrait-none.png` | A Character with no portrait | A character's figure in black pixels, a white question mark for its face | 96×96 |

## The job icons

Beside each job the site writes (`site/jobs.ts` gives a job's class, `site/leaderboards/api.ts` draws it): the game's class icons, supplied by the owner (October 2026) as GIFs with white corners, kept here as PNGs with those corners made transparent and nothing else changed. They show pixel for pixel, at their own size. A Pirate has no icon yet, and shows none.

| File | Class | What it shows | Size |
| --- | --- | --- | --- |
| `jobs/beginner.png` | Beginner | A figure in white on a teal square, sparkles around it | 26×26 |
| `jobs/warrior.png` | Warrior | A figure swinging a sword, in white on red to yellow | 27×27 |
| `jobs/magician.png` | Magician | A figure casting, in white on blue | 26×26 |
| `jobs/bowman.png` | Bowman | A figure drawing a bow, in white on green | 26×26 |
| `jobs/thief.png` | Thief | A figure throwing, in white on brown to cream | 27×27 |
