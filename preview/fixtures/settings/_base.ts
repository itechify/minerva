import type { SettingsPage } from "../../../src/settings";

type Window = NonNullable<SettingsPage["window"]>;

export const open: Window = {
  placement: { x: 200, y: 120, width: 420, height: 500 },
  takesKeyboard: false,
  opened: 1,
  rebinding: null,
  problem: null,
  notice: null,
  atDefaults: true,
};

export const page = (window: Window, more: Partial<SettingsPage> = {}) =>
  ({
    window,
    hotkeys: {
      togglePanel: "ctrl+shift+c",
      cyclePlacement: "ctrl+shift+p",
      openMeowdbPage: "ctrl+shift+m",
      setup: "ctrl+shift+s",
      chooseCandidate: "ctrl+shift+n",
      search: "ctrl+shift+f",
      grindTracker: "ctrl+shift+g",
      itemCard: "ctrl+shift+i",
      itemLookup: "ctrl+shift+u",
      dealFinder: "ctrl+shift+d",
    },
    unavailableHotkeys: [],
    invalidHotkeys: [],
    marginPercent: 10,
    grindTrackerOn: true,
    checksOn: true,
    marketPrices: true,
    dealLines: [
      { id: "goingRate", name: "Going rate", chosen: true },
      { id: "cheapest", name: "Cheapest now", chosen: false },
      { id: "line-1", name: "Lowest ask", chosen: false },
    ],
    gameData: { fetchedAtUnixMs: 1_792_990_000_000, refresh: null },
    ...more,
  }) satisfies SettingsPage;
