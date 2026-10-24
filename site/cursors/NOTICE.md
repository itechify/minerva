# The game's pointer

`normal.png`, `hover.png` and `press.png` are MapleStory's own mouse cursor, NEXON's art,
as MapleStory Classic World's files hold it (`UI.wz/Basic.img/Cursor/0/0`, `1/0` and
`1/1`), unmodified, taken from maplestory.io's copy of those files
(`https://maplestory.io/api/wz/MCW/CBT2/UI/Basic.img/Cursor/<n>/<frame>`). Each one's
hotspot in `site/site.css` is its `origin` there: 0,3, 3,6 and 4,4.

The `@2x` copies are the same drawings at twice the size, each pixel made four, so a
high-DPI screen shows them sharp rather than smoothed.

The site credits them in its Credits.
