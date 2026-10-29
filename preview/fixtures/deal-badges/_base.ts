import type { DealBadge } from "../../../src/deal-badges";

const gloves = { id: "1082002", name: "Work Gloves", rechargeable: false };
const stars = { id: "2070000", name: "Subi Throwing-Stars", rechargeable: true };
const potion = { id: "2000000", name: "Red Potion", rechargeable: false };

/** A shop's listings, one a band `band` physical px tall: a steal, a great and a good deal,
 *  no deal, one past twice the line, no price, one the pointer covers, and one unread. */
export function listings(band: number): DealBadge[] {
  const at = (row: number) => ({ row, top: row * band, height: band, covered: false, open: false });
  return [
    { ...at(0), verdict: "deal", item: gloves, listed: { quantity: 1, mesos: 500_000 }, percent: -60, lineMesos: 1_250_000, grade: "steal" },
    { ...at(1), verdict: "deal", item: gloves, listed: { quantity: 1, mesos: 875_000 }, percent: -30, lineMesos: 1_250_000, grade: "great", open: true },
    { ...at(2), verdict: "deal", item: stars, listed: { quantity: 800, mesos: 9_000 }, percent: -10, lineMesos: 10_000, grade: "good" },
    { ...at(3), verdict: "noDeal", item: gloves, listed: { quantity: 1, mesos: 1_400_000 }, percent: 12, lineMesos: 1_250_000 },
    { ...at(4), verdict: "noDeal", item: potion, listed: { quantity: 100, mesos: 9_000 }, percent: 260, lineMesos: 25 },
    { ...at(5), verdict: "noPrice", item: potion, listed: { quantity: 50, mesos: 2_500 } },
    { ...at(6), verdict: "deal", item: gloves, listed: { quantity: 1, mesos: 1_000_000 }, percent: -20, lineMesos: 1_250_000, grade: "good", covered: true },
    { ...at(7), verdict: "unknown" },
  ];
}
