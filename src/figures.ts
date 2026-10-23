/** How Minerva writes its figures, wherever they show: the panel, the Grind Tracker, the
 *  Personal Bests window, the Item Card and the Deal Badges, and the site's Map Leaderboards. */

/** Mirror of `minerva_core::LevelRange`. */
export type LevelRange = { lowest: number; highest: number };

/** "−" (U+2212) for a negative figure. */
export const signed = (text: string) => text.replace(/^-/, "−");

/** A whole number as the game writes it: 8,435. */
export const whole = (n: number) => signed(n.toLocaleString("en-US"));

/** 1234 hundredths -> "12.3%" with one decimal, "12%" with none: no finer than the
 *  figure can be trusted, so it doesn't flicker. From 100% on, whole percents. */
export const percent = (hundredths: number, decimals = Math.abs(hundredths) >= 10_000 ? 0 : 1) =>
  signed(`${(hundredths / 100).toFixed(decimals)}%`);

/** A share of a rate (a map's solo ceiling): "73%", and "7.3%" under 10%, where one
 *  decimal still says something. */
export const sharePercent = (hundredths: number) => percent(hundredths, hundredths < 1000 ? 1 : 0);

/** An EXP rate to 3 significant figures, written out as the panel writes its rates:
 *  840, 6,120, 61,200, 1,230,000; halves up, 66,550 is 66,600. It must agree with
 *  `three_figures` in the core's session.rs, through which the Grind Tracker's star
 *  compares Last 5 min with the Best row as written here, so it never shows beside two
 *  that read the same. */
export const rate = (n: number) => whole(Number(n.toPrecision(3)));

/** A step on a scale of EXP/h as its ruler writes it, in the Personal Bests window and on
 *  the Map Leaderboards: 0, 500, 20k, 1.25M. */
export function short(n: number): string {
  if (n >= 1_000_000) return `${Number((n / 1_000_000).toFixed(2))}M`;
  if (n >= 1_000) return `${Number((n / 1_000).toFixed(1))}k`;
  return String(n);
}

/** Mob levels: "Lv 2–14", or "Lv 8" for one level. */
export const levels = ({ lowest, highest }: LevelRange) =>
  lowest === highest ? `Lv ${lowest}` : `Lv ${lowest}–${highest}`;
