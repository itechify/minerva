import { solo, view } from "./_base";

/** No EXP gained yet this Session: the waiting line, the map's Best and the bar. */
export default view({
  ...solo,
  state: "waiting",
  timeToLevelS: null,
  timeToLevelBadge: null,
  rolling: null,
  sessionAverage: null,
  activeS: 0,
  pace: null,
  forecast: [],
  loot: { mesos: 0, mesosPerHour: null, items: [], itemsTotal: 0 },
});
