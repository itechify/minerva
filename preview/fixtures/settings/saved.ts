import { open, page } from "./_base";

/** A hotkey just saved, swapped with the one that had its keys; the game data can't be
 *  refreshed. */
export default page(
  { ...open, notice: { kind: "saved", hotkey: "search", swappedWith: "grindTracker" }, atDefaults: false },
  { gameData: { fetchedAtUnixMs: 1_792_400_000_000, refresh: "unreachable" } },
);
