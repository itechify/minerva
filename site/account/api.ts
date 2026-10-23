// The account pages' calls to the site's API (ADR 0023), signed in by the session's cookie,
// which the browser sends with every call to the site itself.

/** What a call came to: its status, and its JSON body (`null` with none, or when the API
 *  can't be reached, as status 0). */
export interface Called<T> {
  status: number;
  body: T | null;
}

/** Call the API at `path`. */
export async function call<T>(path: string, init: RequestInit = {}): Promise<Called<T>> {
  try {
    const response = await fetch(path, { ...init, headers: { accept: "application/json", ...(init.headers ?? {}) } });
    const text = await response.text();
    return { status: response.status, body: text === "" ? null : (JSON.parse(text) as T) };
  } catch {
    return { status: 0, body: null };
  }
}

/** Call the API at `path` with a JSON body. */
export function send<T>(path: string, method: string, body?: unknown): Promise<Called<T>> {
  return call<T>(path, {
    method,
    headers: body === undefined ? {} : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
}

/** What the pages say when the API can't be reached, or answers wrong. */
export const UNREACHABLE = "Can't reach owlofminerva.app right now. Try again in a minute.";

/** Sign out, and reload the page as it is signed out. */
export async function signOut(): Promise<void> {
  await send("/api/auth/sign-out", "POST");
  location.reload();
}

/** A button that asks before it acts: the first click arms it ("Unlink?"), the second runs
 *  `act`; it disarms when it loses focus. Clicks while `act` runs are ignored, and whether
 *  the button is enabled after is left to `act`, which may redraw it. */
export function armed(button: HTMLButtonElement, ask: string, act: () => Promise<void>) {
  const label = button.textContent ?? "";
  let isArmed = false;
  let busy = false;
  const disarm = () => {
    isArmed = false;
    button.textContent = label;
  };
  button.addEventListener("click", async () => {
    if (busy) return;
    if (!isArmed) {
      isArmed = true;
      button.textContent = ask;
      return;
    }
    busy = true;
    button.setAttribute("aria-busy", "true");
    try {
      await act();
    } finally {
      busy = false;
      button.removeAttribute("aria-busy");
      disarm();
    }
  });
  button.addEventListener("blur", disarm);
}
