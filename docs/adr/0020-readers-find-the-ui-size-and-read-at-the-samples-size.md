---
status: accepted
---

# Readers find the game's UI Size, and read at the samples' size

The readers that find the game's UI in a capture of the client area (the Player Shop's list, ADR 0014; the Status Bar and its EXP figures, ADRs 0016 and 0017) were written on the premise that the game draws its UI at a fixed pixel size, whatever the client area's size. It doesn't. The game sizes its UI from a setting and the window's size together, the UI Size: its Default Ratio keeps the share of the screen the UI had at 1366×768 (the window's height over 768: ×1.5625 at 1920×1200), sharpened or not (Default Ratio (Filter Applied)), and its Ideal Ratio picks a size for the resolution (×1 from 1366×768 to 1920×1200, ×2 at 2560×1440, about ×1.8 at 2560×1600). The scales aren't whole numbers, and the game offers no list of them. Every sample so far is at one UI Size, the maintainer's: a Player Shop listing's band 253 px wide. A player at another found no shop, and no Status Bar, at either of two window sizes (#130).

## The decision

- **Each reader measures the UI Size from what it finds**, by a part of the UI the game draws at a known size: the Player Shop's listing band (253 px at the samples' size). It accepts that part at any size in a wide range (for the band, 140 to 480 px, past Default Ratio at 3840×2160), at one size within a list: the list is where the most band rows of one width start at one column.
- **It then reads at the samples' size.** The found area is scaled to the size the samples were taken at with a smooth filter (Catmull-Rom), and read by the reader as it stands, its distances and tolerances unchanged; what it found is put back at the capture's size. A smooth filter weighs neighbouring pixels by weights that add up to one, so pixels on the line from the band to black make a pixel on that line, which is what tells clean text from covered (ADR 0014). On #130's frame, at ×1, Catmull-Rom read the three listings in sight and the one under the pointer as covered; nearest-neighbour missed the covered one, and bilinear read one listing as unreadable.
- **A UI Size is covered by a sample at it**, as every reading is. The samples' tests also feed each sample scaled up and down, so the measuring and the putting back are tested at sizes no sample is at; there a listing may read as unreadable, never as another.

## Considered options

- **Scaling every distance and tolerance by the UI Size**, and reading at the capture's own size. No resampling, but each tolerance was tuned at one size and would need tuning again at each, and small text reaches the recogniser with fewer pixels than it was tuned on.
- **A list of known UI Sizes.** The game's sizes aren't whole numbers and depend on the window's size, so no list holds.
- **A UI Size the player sets in Minerva.** A setup step that most players get wrong without knowing, and a reading that fails silently when they change the game's setting.

## Consequences

- The Status Bar follows this decision since #132, measured by its LV box (47 rows at the samples' size), and so do its EXP figures, measured by their brackets (11 rows). The strip captured for the bar is 128 rows, room for it at ×1.9 of the samples'.
- Sharpening leaves a dark fringe along the UI's edges, where the game's filter puts it and where Catmull-Rom does. The Status Bar's checks for its box and for covered text take a fringe with the slate's tint, and a pixel or two of black, beside the text. Shrinking blurs the pointer's black outline into the white inside it, which then looks like text: so the pointer is looked for in the capture as it was captured, before any scaling, as a few pixels of untinted black over a line (the game's filter leaves no more than a stray one or two), and the EXP's label must be the label's width with nothing above it, or the pointer's white may be taken for it.
- A sample the tests scale with a bilinear filter often reads as unreadable (the blend from a tile's orange into the box isn't the box's grey): unreadable, never wrong, as this decision allows. The game's own sizes read whole.
- A reader that finds nothing saves no crop in a Bug Report, so a UI Size it can't find leaves no frame to make a sample of: such frames come from `probe item` or `probe record` until it does.
