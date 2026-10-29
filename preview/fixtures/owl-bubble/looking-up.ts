import type { OwlView } from "../../../src/owl-bubble";

/** Item Lookup asking MeowDB for the item read. */
export default {
  bubble: {
    line: { says: "itemLookup", lookup: 1, hotkey: "itemLookup", status: "lookingUp", read: "Work Gloves" },
    mood: "waiting",
    clickable: false,
    lastsMs: 4_000,
  },
  hotkey: "ctrl+shift+u",
} satisfies OwlView;
