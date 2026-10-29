import type { OwlView } from "../../../src/owl-bubble";

/** A new version is out: the line stays until the player acts on it. */
export default {
  bubble: { line: { says: "update", version: "0.17.0" }, mood: "news", clickable: true, lastsMs: null },
  hotkey: "ctrl+shift+i",
} satisfies OwlView;
