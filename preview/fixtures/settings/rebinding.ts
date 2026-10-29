import { open, page } from "./_base";

/** Changing a hotkey: the keys pressed were another hotkey's, and another app holds one. */
export default page(
  { ...open, takesKeyboard: true, rebinding: "search", problem: { kind: "inUse", pressed: "ctrl+shift+g", by: "grindTracker" }, atDefaults: false },
  { unavailableHotkeys: ["ctrl+shift+u"] },
);
