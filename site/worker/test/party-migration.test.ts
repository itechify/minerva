// Migration 0005 gives every row from before party entries the kind of a best with no known
// kind: Party (ADR 0015, ADR 0026). It runs here on a second database that has only 0001 to
// 0004 applied and rows in them, as production has.
import { applyD1Migrations } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";

describe("migration 0005", () => {
  it("marks every row from before it Party, in each table that has a kind", async () => {
    const db = env.OLD_DB;
    const before = env.TEST_MIGRATIONS.filter((m) => !m.name.startsWith("0005"));
    const party = env.TEST_MIGRATIONS.filter((m) => m.name.startsWith("0005"));
    expect([before.length, party.length]).toEqual([env.TEST_MIGRATIONS.length - 1, 1]);
    await applyD1Migrations(db, before);

    await db.batch([
      db.prepare("INSERT INTO characters (name_key, name, key_hash, claimed_at) VALUES ('ada', 'Ada', 'h', 1)"),
      db.prepare(
        "INSERT INTO entries (name_key, map_id, band, level, job, exp_per_hour, set_at) VALUES ('ada', '100000000', 20, 23, 'Magician', 40000, 5)",
      ),
      db.prepare("INSERT INTO removed_entries (name_key, map_id, band, exp_per_hour) VALUES ('ada', '100000000', 30, 90000)"),
      db.prepare(
        "INSERT INTO reports (name_key, map_id, band, exp_per_hour, reason, reported_at) VALUES ('ada', '100000000', 20, 40000, '', 6)",
      ),
      db.prepare(
        "INSERT INTO taken_tops (name_key, map_id, band, exp_per_hour, from_name_key, from_exp_per_hour, taken_at) VALUES ('ada', '100000000', 20, 40000, NULL, 30000, 7)",
      ),
    ]);
    await applyD1Migrations(db, party);

    for (const table of ["entries", "removed_entries", "reports", "taken_tops"]) {
      const rows = await db.prepare(`SELECT party FROM ${table}`).all<{ party: number }>();
      expect({ table, parties: rows.results.map((r) => r.party) }).toEqual({ table, parties: [1] });
    }
    // The row's other figures came through.
    const kept = await db.prepare("SELECT level, exp_per_hour, set_at FROM entries").first();
    expect(kept).toEqual({ level: 23, exp_per_hour: 40000, set_at: 5 });
  });
});
