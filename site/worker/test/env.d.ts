declare namespace Cloudflare {
  interface Env {
    TEST_MIGRATIONS: import("cloudflare:test").D1Migration[];
    /** A second database, for the test that runs migration 0005 on rows from before it. */
    OLD_DB: D1Database;
  }
}
