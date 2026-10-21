---
version: 1
slug: "site-index-html"
primary_target: "site/index.html"
related_targets: ["site/faq/index.html","site/site.css","site/site.ts","site/questions.ts"]
---

# Surface: the Minerva site

## Scope and mode
Pages for players at owlofminerva.app: the home page (`site/index.html`) and the FAQ (`site/faq/index.html`), beside the Map Leaderboards (their own brief), styled by `site/site.css` and run by `site/site.ts`. Mode: **Persuade** for the home page (decide, then download), **Read** for the FAQ (find the one answer). Spec: GitHub issue #64; restructured in October 2026 from a six-tab manual into a short home page and a FAQ, after poeoverlay.com.

## Audience, job and constraints
- The average MapleStory Classic player: deciding whether to install Minerva, then learning what each part of the panel does.
- The reader's questions, in order: what is it, what does it show, is it safe for my account, how do I start, what are the keys; then, later, the one detail they came back for.
- Short on the home page: a sentence or two per feature, its most-asked questions folded under it. Every detail lives in the FAQ, written once and copied to the home page at build.
- Content comes from the app as it ships (the release the page describes), in the panel's own words and GLOSSARY.md's terms. The setup warning and the NEXON disclaimer are word for word.
- Real screenshots only, taken by hand (`site/screenshots/README.md`); the build fails while one is missing.
- No analytics, cookies or third-party requests; fonts served from the site.

## Direction contract
THESIS: The site is one of the game's own windows, opened wide: its window bar carries the name and Download, and the game's raised window tabs are its pages (Home, FAQ). The home page shows each feature as a crop of the app on its stage beside a sentence or two, the details folded beneath as the panel folds its sections; the owl speaks as the game's NPCs do. It refuses both the docs-site default of a sidebar tree on white and the landing-page default of icon cards.
OWN-WORLD: Setup's opaque Window Lavender body under the Mini Map's grey window bar, framed in Window Edge, standing on Quest Helper Glass over white as the Personal Bests page does. Raised tabs in Light Steel, the open one joined to the lavender. Steel primary and light-steel secondary buttons, Tag Periwinkle tags, questions folded behind the panel's fold button in the settings view's inset rows, the owl in a white NPC chat box on the Mini Map's sky, the minimap legend marks on the Map, Mobs and Navigation headings. Maplestory throughout; nothing moves but by the scroll: the minimap's dot (overdrive, October 2026: the page's minimap, docked at the top left from 1440px), and the hero's film (overdrive, October 2026, "Watch it read": the hero holds under the head while the scroll draws the camera back from the hero shot to the whole game window, then in to the minimap's title, marks it with the title-marking window's veil and dashed box, reads it into Minerva's lettering, drops the read into the panel's title, and draws back to the hero shot; `site/reel.ts`). With reduced motion, both step or hold still.
STORY: A player reads in the hero that Minerva only looks at the screen, sees what it shows in eight short rows, believes it, downloads it, and later comes back to the FAQ for the one answer they need.
FIRST VIEWPORT: The window fills the width (up to 1200px), 28px down. Bar: 32px owl, "Minerva", version and platform, Download (steel) at the right. Under it the three tabs (Home, Map Leaderboards, FAQ), "Home" raised. Body: left, the logo (the owl over "Minerva", 300px wide), the title, a one-sentence lede, the line that it only looks at the screen and never reads the game's memory or presses keys, the trust line, Download beside "See what it does", and the fine print with the NEXON disclaimer and a link to How it stays safe; right, the hero's frame on the hero shot, the panel under the minimap, with the shot's own caption; as the reader scrolls, the hero holds while its film draws back to the whole game window, reads the title, and ends on the hero shot again.
FORM: Inventory tabs, position 5 of 7 on the ordered list; seed key c9497a89. Signature interaction: the bar and tabs stick as the window's head; each question unfolds in place behind the panel's fold button, and an address naming one opens it.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Unresolved
- The screenshots: taken by the owner before the first deploy.
