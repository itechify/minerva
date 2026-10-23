/** Whether minimap titles can be recognised as real maps, as the panel and setup say it. */

/** Mirror of `minerva_core::Recognition`. */
export type Recognition =
  | { status: "starting" | "ready" | "unavailable" | "formatChanged" }
  | { status: "savedCopy"; savedAtUnixMs: number };

/** While OSMS's file set hasn't arrived yet. */
export const LOADING_MAP_LIST = "Loading the map list from OSMS Data Explorer…";

/** Why no map can be recognised, and what happens next; null while maps can be (or the map list is loading). */
export function recognitionProblem(recognition: Recognition): string | null {
  switch (recognition.status) {
    case "unavailable":
      return "Couldn't load the map list from OSMS Data Explorer. Check your connection. The map list retries every minute.";
    case "formatChanged":
      return "OSMS Data Explorer's data has changed format. Minerva needs an update to read it.";
    default:
      return null;
  }
}

/** No minimap title has been found for a while (`PanelState::title_lost`): what the panel and
 *  setup say. The FAQ answer it links to says more. */
export const TITLE_LOST = "Can't find the minimap's title: is the minimap open?";
