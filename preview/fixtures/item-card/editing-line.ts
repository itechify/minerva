import type { CardView } from "../../../src/item-card";

/** A line's price field open, holding what isn't a price. */
export default {
  shown: true,
  card: {
    card: 1,
    status: "card",
    id: "1082002",
    name: "Work Gloves",
    icon: null,
    shop: { status: "sells", mesos: 50 },
    marketRows: {
      goingRate: { status: "noPrice" },
      cheapest: { status: "noneListed" },
      anyRoll: false,
    },
    market: [
      { id: "line-1", name: "Lowest ask", price: { mesos: 1_200_000, setAtUnixMs: 1_792_900_000_000 } },
      { id: "line-2", name: "My sell price", price: null },
    ],
    marketEdit: { field: "price", line: "line-2", notAPrice: "1.2 mil" },
    savedCopyFromUnixMs: null,
    credit: "Data: OSMS Data Explorer · Pages: NiaMeowDB, maplestory.market",
  },
} satisfies CardView;
