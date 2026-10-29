import type { OwlView } from "../../../src/owl-bubble";

/** Several MeowDB items share the name read: the player picks one. */
export default {
  bubble: {
    line: {
      says: "itemLookup",
      lookup: 1,
      hotkey: "itemLookup",
      status: "choose",
      read: "Feather Cap",
      items: [
        { id: "1002014", name: "Feather Cap", category: "Hat", url: "https://example.com/1002014" },
        { id: "1002015", name: "Feather Cap", category: "Hat", url: "https://example.com/1002015" },
        { id: "4000999", name: "Feather Cap", category: null, url: "https://example.com/4000999" },
      ],
    },
    mood: "news",
    clickable: true,
    lastsMs: 12_000,
  },
  hotkey: "ctrl+shift+u",
} satisfies OwlView;
