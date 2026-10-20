// Map names from the map names fixture (crates/minerva-core/tests/fixtures/osms-map-names.json,
// ADR 0008): the only place the site turns a 9-digit map ID into the name a player reads.
// Shared by the Worker (`site/worker`, which names the map a new #1 was set on) and the Map
// Leaderboards' pages (`site/leaderboards`), as `level-bands.ts` is.

/** A row of the map names fixture, as OSMS writes it: its ID a number without its leading
 *  zeros, its text untrimmed. */
export interface MapNameRow {
  id: number;
  street_name: string;
  name: string;
  region: string;
}

/** A map as the site names it: its Map Name under its Street Name, as the panel does. */
export interface MapPlace {
  /** The 9-digit ID. */
  id: string;
  name: string;
  street: string;
  region: string;
  /** Whether the map names fixture names it. */
  known: boolean;
}

/** The maps the fixture names, by 9-digit ID. */
export type MapNames = Map<string, MapPlace>;

/** The maps the fixture names, trimmed as the core reads them, leaving out any with no name
 *  or street. */
export function mapNames(rows: MapNameRow[]): MapNames {
  const names: MapNames = new Map();
  for (const row of rows) {
    const [name, street] = [row.name.trim(), row.street_name.trim()];
    if (name === "" || street === "") continue;
    const id = String(row.id).padStart(9, "0");
    names.set(id, { id, name, street, region: row.region, known: true });
  }
  return names;
}

/** Map `id` as the site names it: a map the fixture doesn't name (one newer than it) by its
 *  ID, "Not in the map list", as the Personal Bests window names it. */
export function placeOf(names: MapNames, id: string): MapPlace {
  return names.get(id) ?? { id, name: `Map ${id}`, street: "Not in the map list", region: "", known: false };
}
