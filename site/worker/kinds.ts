// The two kinds of entry on the Map Leaderboards (ADR 0015, amended): a Character's Solo Best
// and its Party Best on a map in a Level Band (ADR 0026) are entries of their own. A read
// ranks the entries of one kind (`?kind=solo` or `?kind=party`), or, with no kind, each
// Character's higher entry in a place.
import type { Kind } from "../entries";
import { type Answer, badRequest } from "./http";

/** The entries a read ranks, as SQL to put where `entries` would go: one kind's, or, for no
 *  kind, each Character's higher entry on a map in a band (the Solo Best on a tie, so a
 *  Character is never ranked twice and never moves between reads). */
export function boardOf(kind: Kind | null): string {
  switch (kind) {
    case "solo":
      return "(SELECT * FROM entries WHERE party = 0)";
    case "party":
      return "(SELECT * FROM entries WHERE party = 1)";
    case null:
      return `(SELECT * FROM entries higher WHERE NOT EXISTS (
        SELECT 1 FROM entries other
        WHERE other.name_key = higher.name_key AND other.map_id = higher.map_id AND other.band = higher.band
          AND (other.exp_per_hour > higher.exp_per_hour
               OR (other.exp_per_hour = higher.exp_per_hour AND other.party < higher.party))))`;
  }
}

/** The kind a read asks for in its query (`?kind=`): `null` for both, or the answer refusing
 *  one that isn't solo or party. */
export function kindAsked(url: URL): Kind | null | Answer {
  const asked = url.searchParams.get("kind");
  if (asked === null) return null;
  if (asked === "solo" || asked === "party") return asked;
  return badRequest("a kind is solo or party");
}

/** Whether `kind` is a kind, not the answer refusing one. */
export function isKind(kind: Kind | null | Answer): kind is Kind | null {
  return kind === null || typeof kind === "string";
}

/** Validate a route's kind once and pass it to the read or moderation action. */
export async function withKind(url: URL, action: (kind: Kind | null) => Promise<Answer>): Promise<Answer> {
  const kind = kindAsked(url);
  return isKind(kind) ? action(kind) : kind;
}

/** The database's representation of the existing wire boolean. */
export function sqlParty(party: boolean): 0 | 1 { return party ? 1 : 0; }
export function isParty(value: number): boolean { return value === 1; }

/** An absent kind means both; otherwise select just its stored kind. */
export function sqlKind(kind: Kind | null): number | null {
  return kind === null ? null : sqlParty(kind === "party");
}
export function kindCondition(parameter: string): string {
  return `(${parameter} IS NULL OR party = ${parameter})`;
}
