// The link page (account/link/index.html): the token Minerva's Link Discord opened it with,
// signing in with Discord if need be, and the question before linking. What it says is
// worked out in account.ts.
import { type AccountAnswer, linkQuestion, signInFailed, signInHref, type TokenAnswer, tokenIn } from "./account.ts";
import { call, send, UNREACHABLE } from "./api.ts";

const byId = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const said = byId("said");
const ask = byId("ask");

/** What the page says when the token has gone: used, or past its 5 minutes. */
const GONE = "This link has expired or was already used. In Minerva, click Link Discord again.";

async function load() {
  const token = tokenIn(location.search);
  if (token === null) {
    said.textContent = "This page links a PC from Minerva. In Minerva, open Personal Bests and click Link Discord.";
    return;
  }
  const account = await call<AccountAnswer>("/api/account");
  if (account.status === 401) {
    said.textContent = signInFailed(location.search)
      ? "Signing in with Discord didn't go through. Try again."
      : "Sign in with Discord to link this PC. Minerva only learns your Discord username.";
    byId<HTMLAnchorElement>("sign-in").href = signInHref(`${location.pathname}?t=${token}`);
    byId("signed-out").hidden = false;
    return;
  }
  const shown = await call<TokenAnswer>(`/api/link/tokens/${token}`);
  if (account.status !== 200 || (shown.status !== 200 && shown.status !== 404)) {
    said.textContent = UNREACHABLE;
    return;
  }
  if (shown.status === 404 || shown.body === null) {
    said.textContent = GONE;
    return;
  }
  if (shown.body.linked === "this") {
    said.textContent = `This PC is already linked to your Discord account ${account.body!.username}.`;
    return;
  }
  const question = linkQuestion(account.body!.username, shown.body);
  said.textContent = "";
  byId("ask-question").textContent = question.ask;
  byId("ask-holds").textContent = question.holds;
  byId("ask-warn").textContent = question.warn;
  ask.hidden = false;
  byId<HTMLButtonElement>("confirm").addEventListener("click", async (event) => {
    (event.currentTarget as HTMLButtonElement).disabled = true;
    const linked = await send(`/api/link/tokens/${token}`, "POST");
    ask.hidden = true;
    said.textContent =
      linked.status === 200
        ? "Linked. Minerva shows it in Personal Bests in a few seconds. You can close this page."
        : linked.status === 404
          ? GONE
          : UNREACHABLE;
  });
}

load();
