import type { OwlView } from "../../../src/owl-bubble";

/** No tooltip under the pointer when the player pressed the Item Card's hotkey. */
export default {
  bubble: {
    line: { says: "itemLookup", lookup: 1, hotkey: "itemCard", status: "noTooltip" },
    mood: "problem",
    clickable: false,
    lastsMs: 6_000,
  },
  hotkey: "ctrl+shift+i",
} satisfies OwlView;
