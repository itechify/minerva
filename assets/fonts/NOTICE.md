# Maplestory fonts

`Maplestory-Light.ttf` and `Maplestory-Bold.ttf` are NEXON's Maplestory typeface,
unmodified (version 1.0.0).

- Copyright (c) 2017 NEXON-Maplestory. All rights reserved. Designed by FONTRIX Inc.
  (Park Young-rak, Kim Jung-Jin). "Maplestory" is a trademark of NEXON-Maplestory.
- The files carry no licence text; their embedding permission (OS/2 `fsType` 8) allows
  editable embedding. NEXON publishes the typeface for free use.
- Bundled in Minerva's public releases (from 0.7.0) and credited in Help & Setup's
  About & Credits and in `THIRD-PARTY-NOTICES.md`. Bundling it does not mean NEXON made
  or approved Minerva.
- The Minerva site (`site/`) serves WOFF2 copies of these two files, in `site/fonts`:
  the same fonts without their Korean (the Hangul syllables and jamo, and the CJK
  punctuation, symbols and ideographs, U+1100–11FF, 3000–303F, 3130–318F, 3200–33FF,
  4E00–9FFF, A960–A97F, AC00–D7FF and FF00–FFEF), which the site never shows, keeping
  every other character (735) and the kerning, at about a twentieth of the TTF's size.
  The fonts allow it: their embedding permission doesn't forbid subsetting. They were cut
  with HarfBuzz's subsetter (the `subset-font` package) from the TTFs here. The site
  credits the typeface in its Credits.
