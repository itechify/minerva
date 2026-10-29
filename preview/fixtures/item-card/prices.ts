import type { CardView } from "../../../src/item-card";

/** A card with maplestory.market's rows and the player's own lines priced. */
export default {
  shown: true,
  card: {
    card: 1,
    status: "card",
    id: "1082002",
    name: "Leather Mitts",
    icon: null,
    shop: { status: "sells", mesos: 50 },
    marketRows: {
      goingRate: {
        status: "goingRate",
        goingRate: { mesos: 1_250_000, basis: "sold", samples: 14 },
        sales: { sales: 14, today: 3 },
        asking: { low: 1_100_000, high: 1_600_000, listings: 9 },
        asOfUnixMs: 1_793_000_000_000,
      },
      cheapest: { status: "cheapest", listed: { cheapest: 1_150_000, listings: 9 }, asOfUnixMs: 1_793_000_000_000 },
      anyRoll: true,
    },
    market: [
      { id: "line-1", name: "Lowest ask", price: { mesos: 1_200_000, setAtUnixMs: 1_792_900_000_000 } },
      { id: "line-2", name: "My sell price", price: null },
    ],
    marketEdit: null,
    savedCopyFromUnixMs: null,
    credit: "Data: OSMS Data Explorer · Pages: NiaMeowDB, maplestory.market",
  },
} satisfies CardView;
