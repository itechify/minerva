// The Podiums Pedestal, on the Map Leaderboards page (leaderboards/index.html): the three
// Characters with the most places in a Level Band's top 3 (ADR 0024, amended). What it shows
// is worked out in leaderboards.ts.
import { byId, drawPedestal, read, unreachable } from "./api.ts";
import { type PodiumsAnswer, podiumsPedestal } from "./leaderboards.ts";

const steps = byId<HTMLOListElement>("podiums-steps");
const said = byId("podiums-said");
const credit = byId("podiums-credit");

function load() {
  read<PodiumsAnswer>("/api/podiums").then((answer) => {
    steps.classList.remove("lb-waiting");
    if (answer === null) {
      steps.hidden = true;
      return unreachable(said, () => {
        said.hidden = true;
        steps.classList.add("lb-waiting");
        steps.hidden = false;
        load();
      });
    }
    drawPedestal(steps, credit, podiumsPedestal(answer));
  });
}
load();
