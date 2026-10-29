import { owlbert, view } from "./_base";

/** One Character whose bests on three maps came back from the site this launch, uploaded
 *  since. Linked to Discord. */
export default view([{ ...owlbert, leaderboard: { ...owlbert.leaderboard!, mapsFromSite: 3 } }], {
  discord: { linkedAs: "owlbert", offered: false, linking: false, failed: null, prompt: false },
});
