/** Mirror of `minerva_core::ExpRate`. */
export type ExpRate = { expPerHour: number; percentPerHourHundredths: number | null };

/** Mirror of `minerva_core::ExpBar`: the EXP bar's figures, "EXP 160 [3.81%]". */
export type ExpBar = { exp: number; percentHundredths: number };

/** Mirror of `minerva_core::Character`: who is playing, as the game's Status Bar names them,
 *  with the job and level it last showed. */
export type Character = { name: string; job: string | null; level: number | null };

/** Mirror of `minerva_core::ExpTracking`. */
export type ExpTracking = {
  state: "off" | "waiting" | "warmingUp" | "active" | "paused" | "unreadable";
  timeToLevelS: number | null;
  timeToLevelBadge: string | null;
  rolling: ExpRate | null;
  sessionAverage: ExpRate | null;
  ceilingShareHundredths: number | null;
  /** The shown map's solo EXP/h ceiling (its Map Summary), while there is a share of it. */
  soloExpCeiling: number | null;
  deaths: { count: number; expLost: number; percentLostHundredths: number };
  /** The Session started afresh on a character switch and hasn't warmed up since. */
  characterSwitched: boolean;
  /** The Session's active time, and the part of it the Rolling Rate covers (seconds). */
  activeS: number;
  rollingS: number;
  /** The player reset the Session and the EXP hasn't changed since. */
  resetUndoable: boolean;
  /** The last EXP bar reading, which moves with every reading. */
  bar: ExpBar | null;
  /** The Best row: the shown map's Personal Best (EXP/h) as it stands, from arriving, this
   *  Visit's once it beats the earlier Visits'; none until the map has one, or where no bests
   *  are kept. */
  personalBest: number | null;
  /** The Best row's figure was set on this Visit: the row says "this visit". */
  personalBestThisVisit: boolean;
  /** The star on the Last 5 min row: the core says when the Rolling Rate is the best this
   *  Visit set on the Best row, as both are written. */
  newBest: boolean;
  /** The Pace Trace; none while waiting. It moves with the EXP, as `bar` does. */
  pace: PaceTrace | null;
  /** Where the bar will be at the Last 5 min pace, short of the level-up, nearest first. */
  forecast: Forecast[];
  /** The bar's percentage before the last death in this level, while it reads less. */
  lostFromHundredths: number | null;
  /** Unreadable because the game's Status Bar isn't found, so there is nowhere to read the
   *  EXP figures. */
  noStatusBar: boolean;
  /** What the Pickup Log says the Session picked up. */
  pickups: Pickups;
};

/** Mirror of `minerva_core::Pickups`: read from the Pickup Log, so possibly fewer than were
 *  picked up, never more. */
export type Pickups = {
  /** The Session's mesos, as they arrive. */
  mesos: number;
  /** Mesos an hour over the Session's active time, held with the other figures; none until
   *  the Session has active time. */
  mesosPerHour: number | null;
};

/** Mirror of `minerva_core::PacePoint`: active time before the last EXP change, and the
 *  EXP gained since the trace's first point. */
export type PacePoint = { agoMs: number; exp: number };

/** Mirror of `minerva_core::PaceTrace`. */
export type PaceTrace = {
  spanMs: number;
  /** Oldest first, the last EXP change last. */
  points: PacePoint[];
  /** The pace of the 5 minutes before, from the first point to the last change. */
  earlier: [PacePoint, PacePoint];
  /** The Best row's pace, from where the race on the map begins to the last change; only
   *  once it is within reach on this Visit. */
  best: [PacePoint, PacePoint] | null;
  /** Arriving on the map, if within the trace. */
  arrivedAgoMs: number | null;
};

/** Mirror of `minerva_core::Forecast`. */
export type Forecast = { afterS: number; percentHundredths: number };
