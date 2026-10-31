// A made-up Discord webhook (ADR 0022), set up before every test file (vitest.config.ts): the
// Worker's posts to the test config's DISCORD_WEBHOOK_URL are taken off the global fetch, kept
// here with their files and answered as Discord does, with a 204. Any upload that takes a top spot posts, so
// without it every test file's posts would go out to discord.invalid and fail there. A failed
// DNS lookup is reported by workerd as an uncaught exception even when the Worker catches it.
// Nothing leaves the test.
import { env } from "cloudflare:workers";
import type { Post } from "../announce";

/** A file a post was sent with. */
export interface PostedFile {
  /** Its form field (`files[n]`), name and type, and its bytes. */
  field: string;
  name: string;
  type: string;
  bytes: Uint8Array;
}

/** What the made-up webhook was sent: each post's `payload_json`, in order, and each post's
 *  files. */
interface State {
  posts: Post[];
  files: PostedFile[][];
}

/** One made-up webhook per isolate, however many times this file is set up. */
const global = globalThis as typeof globalThis & { madeUpDiscordWebhook?: State };

export const discordWebhook: State = (global.madeUpDiscordWebhook ??= { posts: [], files: [] });

const realFetch = globalThis.fetch;
const wrapped = globalThis as typeof globalThis & { madeUpDiscordWebhookFetch?: boolean };
if (!wrapped.madeUpDiscordWebhookFetch) {
  wrapped.madeUpDiscordWebhookFetch = true;
  globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = input instanceof Request ? input.url : String(input);
    if (url !== env.DISCORD_WEBHOOK_URL) return realFetch(input as RequestInfo, init);
    const form = await new Request(url, { method: "POST", body: init?.body ?? (input instanceof Request ? input.body : null) }).formData();
    const files: PostedFile[] = [];
    for (const [field, value] of form) {
      if (typeof value !== "string") files.push({ field, name: value.name, type: value.type, bytes: new Uint8Array(await value.arrayBuffer()) });
    }
    discordWebhook.posts.push(JSON.parse(String(form.get("payload_json"))) as Post);
    discordWebhook.files.push(files);
    return new Response(null, { status: 204 });
  }) as typeof globalThis.fetch;
}
