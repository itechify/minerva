---
status: accepted, amended by ADR-0021
---

# One bundled title recogniser on every platform: ocrs

Use one bundled recogniser, **ocrs** on the pure-Rust rten runtime, across platforms. Native OCR per OS would mean different errors to tune the map-name matcher against. Ship only the recognition model: Minerva finds the text itself. A reading is snapped to known street and map names.

Preprocessing does most of the work: keep the open minimap's neutral-grey title pixels as black on white, then split its two lines. ocrs tolerated imperfect masking better than bundled Tesseract in the spike. Its pure-Rust build avoids CMake, a C++ toolchain and build-time source downloads. Both engines were fast enough for recognition; accept ocrs's larger bundle. Whole-string templates were faster but would need a reference image for every name.

The ocrs and rten code is MIT OR Apache-2.0; the bundled model is CC-BY-SA-4.0. Attribution and licence notices live in Help & Setup's About & Credits and THIRD-PARTY-NOTICES.md. Our reading is that ShareAlike covers an adapted model, including a fine-tuned one, rather than the app's own code. Tesseract remains the fallback if the model's licence becomes a problem.

ocrs confuses the font's closely spaced Roman numerals II and III. When the map line ends in upright strokes, the core counts them in the mask and replaces the OCR numeral. ADR 0021 adds automatic location at any UI Size and the one-line layout, whose dark text on grey is split at its colon. There is no player-marked Title Region.
