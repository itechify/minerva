import { owlbert, view } from "./_base";

/** One Character whose uploads are held: the bests file didn't read at a launch since they
 *  were shown, and getting their bests back from the site failed. Linked to Discord, so the
 *  note doesn't offer it. */
export default view(
  [{ ...owlbert, maps: owlbert.maps.slice(0, 1), leaderboard: { ...owlbert.leaderboard!, maps: 1, status: "held", problem: true } }],
  { discord: { linkedAs: "owlbert", offered: false, linking: false, failed: null, prompt: false } },
);
