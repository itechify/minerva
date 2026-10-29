import type { CardView } from "../../../src/item-card";

/** Several items share the name read: the player picks theirs by its picture. */
export default {
  shown: true,
  card: {
    card: 1,
    status: "choose",
    name: "Blue Bandana",
    items: [
      { id: "1002014", kind: "Hat", icon: null },
      { id: "1002015", kind: "Hat (female)", icon: null },
    ],
  },
} satisfies CardView;
