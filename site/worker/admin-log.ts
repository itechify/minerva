// The maintainer's log (ADR 0031): every write under /api/admin/ that went through, who made
// it and when, kept for good.
import { type Answer, type Logged, badRequest } from "./http";

/** How many lines one read of the log gives. */
const PAGE = 50;

/** Keep a line of the log. */
export async function keepLine(env: Env, actor: string, action: string, logged: Logged = {}, now = Date.now()): Promise<void> {
  await env.DB.prepare("INSERT INTO admin_log (at, actor, action, name_key, account_id, detail) VALUES (?, ?, ?, ?, ?, ?)")
    .bind(now, actor, action, logged.nameKey ?? null, logged.accountId ?? null, logged.detail ?? "")
    .run();
}

interface LineRow {
  id: number;
  at: number;
  actor: string;
  action: string;
  name: string | null;
  account_id: number | null;
  username: string | null;
  detail: string;
}

/** The lines of the log, each with its Character's name and its Account's username as they
 *  are now. */
const LINES = `SELECT g.id, g.at, g.actor, g.action, COALESCE(c.name, g.name_key) AS name, g.account_id, l.username, g.detail
  FROM admin_log g LEFT JOIN characters c ON c.name_key = g.name_key
  LEFT JOIN logins l ON l.account_id = g.account_id AND l.provider = 'discord'`;

function line(r: LineRow) {
  return {
    id: r.id,
    at: r.at,
    by: r.actor,
    action: r.action,
    name: r.name,
    account: r.account_id === null ? null : { id: r.account_id, username: r.username },
    detail: r.detail,
  };
}

/** The log, newest first, PAGE lines at a time: those before line `before`, if given. */
export async function readLog(env: Env, before: string | null): Promise<Answer> {
  if (before !== null && !/^\d+$/.test(before)) return badRequest("before is a line's number");
  const rows = await env.DB.prepare(`${LINES} WHERE ?1 IS NULL OR g.id < ?1 ORDER BY g.id DESC LIMIT ?2`)
    .bind(before === null ? null : Number(before), PAGE + 1)
    .all<LineRow>();
  return { status: 200, body: { lines: rows.results.slice(0, PAGE).map(line), more: rows.results.length > PAGE } };
}

/** `nameKey`'s lines in the log, newest first. */
export async function linesOf(env: Env, nameKey: string) {
  const rows = await env.DB.prepare(`${LINES} WHERE g.name_key = ? ORDER BY g.id DESC LIMIT ?`).bind(nameKey, PAGE).all<LineRow>();
  return rows.results.map(line);
}
