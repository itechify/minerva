import { owlbert, view } from "./_base";

/** Two Characters, the one playing sharing without a linked Account (the prompt shows), and
 *  an entry the Map Leaderboards kept off. */
export default view(
  [
    {
      ...owlbert,
      leaderboard: {
        ...owlbert.leaderboard!,
        rejected: [{ map: "100000005", lowestLevel: 30, highestLevel: 39, party: false, reason: { overCap: { cap: 60_000 } } }],
      },
    },
    { ...owlbert, name: "Hootsworth", current: false, maps: owlbert.maps.slice(1, 3), leaderboard: null },
  ],
  { discord: { linkedAs: null, offered: true, linking: false, failed: null, prompt: true } },
);
