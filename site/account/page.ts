// The account page (account/index.html): who's signed in, their linked PCs with an Unlink
// each, deleting the Account, and signing out. What it says is worked out in account.ts.
import { make } from "../../src/dom.ts";
import { type AccountAnswer, holds, pcDates, signInFailed } from "./account.ts";
import { armed, call, send, signOut, UNREACHABLE } from "./api.ts";

const byId = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const said = byId("said");

byId("sign-out").addEventListener("click", signOut);
armed(byId<HTMLButtonElement>("delete"), "Delete? Click again", async () => {
  const deleted = await send("/api/account", "DELETE");
  if (deleted.status === 204) {
    location.reload();
    return;
  }
  const line = byId("delete-said");
  line.textContent = UNREACHABLE;
  line.hidden = false;
});

/** Draw the linked PCs, the most recently used first, each with its Unlink. */
function drawPcs(account: AccountAnswer) {
  const list = byId<HTMLUListElement>("pcs");
  byId("pcs-said").textContent =
    account.pcs.length === 0
      ? "No PC linked yet. In Minerva, open Personal Bests and click Link Discord."
      : "Unlinking a PC takes its Characters to your most recently used other PC. With no other PC, they stay with it, out of your Account.";
  list.replaceChildren(
    ...account.pcs.map((pc) => {
      const item = make("li", "account-item");
      const words = make("div", "account-words");
      words.append(make("strong", "", pcDates(pc)), make("span", "", holds(pc.characters)));
      const unlink = make("button", "button small secondary", "Unlink");
      unlink.type = "button";
      armed(unlink, "Unlink? Click again", async () => {
        const unlinked = await send(`/api/account/pcs/${pc.id}`, "DELETE");
        if (unlinked.status === 204) await load();
        else byId("pcs-said").textContent = UNREACHABLE;
      });
      item.append(words, unlink);
      return item;
    }),
  );
}

async function load() {
  const read = await call<AccountAnswer>("/api/account");
  const signedIn = read.status === 200 && read.body !== null;
  byId("signed-out").hidden = signedIn || read.status !== 401;
  byId("signed-in-actions").hidden = !signedIn;
  byId("pcs-section").hidden = !signedIn;
  byId("delete-section").hidden = !signedIn;
  if (read.status === 401) {
    said.textContent = signInFailed(location.search)
      ? "Signing in with Discord didn't go through. Try again."
      : "You're not signed in. Minerva signs you in with Discord, which tells it only your Discord username.";
    return;
  }
  if (!signedIn) {
    said.textContent = UNREACHABLE;
    return;
  }
  const account = read.body!;
  byId("account-title").textContent = "Signed in";
  said.textContent = `Signed in with Discord as ${account.username}.`;
  byId("admin-link").hidden = !account.maintainer;
  drawPcs(account);
}

load();
