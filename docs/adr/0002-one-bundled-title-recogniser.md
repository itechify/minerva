---
status: accepted. Since 0.7.0, the first public release, the model's attribution and licence are in Help & Setup's About & Credits and in THIRD-PARTY-NOTICES.md
---

# One bundled title recogniser on every platform: ocrs

Minerva targets Windows first but must not block macOS and Linux (Wine/Proton), so we read the minimap title with a single recogniser bundled in the app rather than each OS's native OCR (Windows.Media.Ocr, Apple Vision, Tesseract on Linux). Three engines would mean three different sets of errors to tune the name matching against; one engine behaves the same everywhere. A spike on the real screenshots (issue #2) picked **ocrs** (pure-Rust OCR on the rten runtime), using only its text-recognition model. We find the two text lines ourselves, so its detection model is not shipped. The raw reading is then snapped to the closest known street name and map name.

## Why ocrs

What makes reading work is the preprocessing, not the engine. The title is light grey text with a dark-blue outline on a light-blue panel. We turn it into black text on white by keeping the neutral-grey pixels, then split the two lines with a row profile. With that preprocessing, ocrs and bundled Tesseract (5.5.2, tessdata_fast) both read all 5 samples exactly (street name and map name), including the long-name and Hidden Street samples. Both still snapped all 5 correctly with the capture resized to 0.6–1.25×. Three things decided it:

- **Tolerance of imperfect preprocessing.** Without the colour mask, ocrs still read titles to within a letter or two ("Keming city") and snapped 20/20 across those scales. Tesseract read garbage under every polarity setting ("Vie tories sual"). The player-marked Title Region will meet conditions the samples don't show, and only ocrs has a safety margin there.
- **Same build on every platform.** ocrs is pure Rust: no CMake, no C++ toolchain and no downloads at build time. The tesseract-rs route builds Leptonica and Tesseract from source with CMake, and downloads sources plus about 22 MB of unused traineddata at build time. Cheap cross-platform builds are the point of this ADR.
- **Speed is not a differentiator.** ocrs reads a title in 14–26 ms with 5 ms model load; Tesseract takes 12–24 ms with 19 ms load (release build, Ryzen 7800X3D). Preprocessing takes under 1 ms. Both fit easily in the one-second recognition budget.

A custom matcher for the game's title font, using whole-string template matching on the same mask, was also tried. It is scale-invariant and takes under 0.3 ms, but it needs a reference image harvested for every street name and map name, so it is only worth having later as a cache in front of the recogniser.

## Consequences

- **Download size:** ocrs adds about 11 MB zipped (6 MB of code plus the 9.7 MB recognition model). Tesseract would have added about 4 MB. We accept the difference.
- **Licence:** ocrs and rten code is MIT OR Apache-2.0, but **the ocrs models are CC-BY-SA-4.0**: they were trained on HierText (CC-BY-SA) and synthetic data. Our reading is as follows. Shipping the unmodified model obliges us to credit its creator and to include a licence notice, a warranty-disclaimer notice and a link (§3(a)). ShareAlike (§3(b)) covers only adapted material. So if we ever fine-tune the model on game-font crops, the tuned model must also be released as CC-BY-SA. The app's own code is not affected. Before any public release (already blocked by ADR 0003), add the model attribution and licence to the About screen and to the third-party notices, and have this reading confirmed. If the licence becomes a problem, Tesseract (Apache-2.0, with BSD-2 Leptonica) is a drop-in fallback that uses the same preprocessing and scored the same.
- The spike's preprocessing settings, per-sample results and the harness live outside the repo in the maintainer's notes (`minerva-notes/spike-ocr/RESULTS.md`).
- ocrs can't read the game font's Roman numerals "II" and "III" (bold, 1 px apart): it gives an unknown glyph, or a single "I". So when the map line's last word is nothing but upright strokes, the core counts them in the mask and puts that numeral in place of what ocrs read (The Tree That Grew I–III samples).
- Not yet covered by samples: a second in-game UI size, duplicate map names, an obscured title, a semi-transparent minimap panel, names with punctuation, and numerals from IV up.
- Since ADR 0021, the title is found in the client area by the minimap's WORLD button, at any UI Size, and the two-line preprocessing here is one of two layouts: a collapsed minimap, or a map without one, shows the title on one line ("Street : Map"), which is masked as dark text on the bar's grey and split at its colon. There is no Title Region.
