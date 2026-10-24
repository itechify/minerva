// Level Bands (GLOSSARY): 10 levels each, 1–9, 10–19, 20–29 and so on, named by their first
// level. Shared by the Worker (`site/worker`) and the Map Leaderboards' pages
// (`site/leaderboards`), which rank and show entries by them.

/** The Level Band holding `level`, by its first level: 1, 10, 20… */
export function bandOf(level: number): number {
  return level < 10 ? 1 : Math.floor(level / 10) * 10;
}

/** The first and last levels of the Level Band whose first level is `band`. */
export function levelsOf(band: number): [number, number] {
  return [band, band === 1 ? 9 : band + 9];
}

/** The Level Band an API path names by its first level ("20"), or `null` if `first` isn't
 *  one's first level. */
export function bandFrom(first: string): number | null {
  const band = Number(first);
  return /^\d+$/.test(first) && band >= 1 && bandOf(band) === band ? band : null;
}
