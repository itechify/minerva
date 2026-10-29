import type { BadgesView } from "../../../src/deal-badges";
import { listings } from "./_base";

const BAND = 54;

/** Every verdict, against one of the player's Market Lines, in bands with room for the
 *  roomy badge. */
export default { badges: listings(BAND), dealLine: "Lowest ask", fromMarket: false } satisfies BadgesView;

/** The core places the window beside the listings, as tall as they are. */
export const height = 8 * BAND;
