// The Item names table (osms-item-names.json), which `probe ceilings` writes from OSMS's live item
// files beside the ceilings table: every Item but the Cash Shop's, its name by its ID. Map Loot
// takes only Items it holds, and names each one from it (ADR 0033).
import table from "./osms-item-names.json";

/** The whole table: each Item's name by its ID, written without leading zeros. */
export interface ItemsTable {
  items: Record<string, string>;
}

/** The tests' made-up table, or the committed one. */
function tableOf(env: Env): ItemsTable {
  return env.TEST_ITEMS ?? table;
}

/** Item `id`'s name, or `undefined` for an ID the table doesn't hold. */
export function itemNameOf(env: Env, id: number): string | undefined {
  return Object.hasOwn(tableOf(env).items, id) ? tableOf(env).items[id] : undefined;
}
