// Who's signed in, in the window's bar (ADR 0023): only when the marker cookie the sign-in
// set says a session is there, so a page view by anyone else never asks the API.
import { make } from "../src/dom.ts";
import { call, signOut } from "./account/api.ts";

/** Put "Signed in as X · Sign out" in the window's bar, if a session is there. */
export async function signedIn() {
  if (!document.cookie.split(";").some((c) => c.trim() === "signed_in=1")) return;
  const account = await call<{ username: string }>("/api/account");
  if (account.status !== 200 || account.body === null) return;
  const line = make("p", "window-signed-in", "Signed in as ");
  const to = make("a", "", account.body.username);
  to.href = "/account/";
  const out = make("button", "", "Sign out");
  out.type = "button";
  out.addEventListener("click", signOut);
  line.append(to, " · ", out);
  document.querySelector(".window-version")?.after(line);
}
