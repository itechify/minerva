// A made-up Discord webhook (ADR 0022), set up before every test file (vitest.config.ts): the
// Worker's posts to the test config's DISCORD_WEBHOOK_URL are taken off the global fetch, kept
// here and answered as Discord does, with a 204. Any upload that takes a top spot posts, so
// without it every test file's posts would go out to discord.invalid and fail there. A failed
// DNS lookup is reported by workerd as an uncaught exception even when the Worker catches it.
// Nothing leaves the test.
import { env } from "cloudflare:workers";
import type { Post } from "../announce";

/** What the made-up webhook was sent: each post, in order. */
interface State {
  posts: Post[];
}

/** One made-up webhook per isolate, however many times this file is set up. */
const global = globalThis as typeof globalThis & { madeUpDiscordWebhook?: State };

export const discordWebhook: State = (global.madeUpDiscordWebhook ??= { posts: [] });

const realFetch = globalThis.fetch;
const wrapped = globalThis as typeof globalThis & { madeUpDiscordWebhookFetch?: boolean };
if (!wrapped.madeUpDiscordWebhookFetch) {
  wrapped.madeUpDiscordWebhookFetch = true;
  globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = input instanceof Request ? input.url : String(input);
    if (url !== env.DISCORD_WEBHOOK_URL) return realFetch(input as RequestInfo, init);
    const body = init?.body ?? (input instanceof Request ? await input.text() : null);
    discordWebhook.posts.push(JSON.parse(String(body)) as Post);
    return new Response(null, { status: 204 });
  }) as typeof globalThis.fetch;
}
