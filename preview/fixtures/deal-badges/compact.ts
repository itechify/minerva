import type { BadgesView } from "../../../src/deal-badges";
import { listings } from "./_base";

const BAND = 36;

/** Every verdict, at the game's 100% UI: bands too short for the roomy badge. */
export default { badges: listings(BAND), dealLine: "Going rate", fromMarket: true } satisfies BadgesView;

/** The core places the window beside the listings, as tall as they are. */
export const height = 8 * BAND;
