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

/** How soon after arming a second click is a double-click's, and ignored, as the app's are. */
const DOUBLE_CLICK_MS = 400;

/** The page's polite line for screen readers, made the first time a button arms. */
function announcer(): HTMLElement {
  let line = document.getElementById("armed-said");
  if (line === null) {
    line = document.createElement("p");
    line.id = "armed-said";
    line.className = "sr-only";
    line.setAttribute("aria-live", "polite");
    document.body.append(line);
  }
  return line;
}

/** A button that asks before it acts: the first click arms it, in the problem palette, its
 *  label `ask` ("Unlink? Click again"), said for screen readers too; the second runs `act`.
 *  A second click within 400ms of the first is a double-click's, and is ignored, so a
 *  double-click only ever arms. It disarms when it loses focus. Clicks while `act` runs are
 *  ignored, and whether the button is enabled after is left to `act`, which may redraw it.
 *  `ask` may be worked out as it arms, to name what it will act on as it stands then. */
export function armed(button: HTMLButtonElement, ask: string | (() => string), act: () => Promise<void>) {
  const label = button.textContent ?? "";
  let isArmed = false;
  let armedAt = 0;
  let busy = false;
  const disarm = () => {
    isArmed = false;
    button.textContent = label;
    button.classList.remove("armed");
  };
  button.addEventListener("click", async () => {
    if (busy) return;
    if (!isArmed) {
      isArmed = true;
      armedAt = performance.now();
      const words = typeof ask === "string" ? ask : ask();
      button.textContent = words;
      button.classList.add("armed");
      announcer().textContent = words;
      return;
    }
    if (performance.now() - armedAt < DOUBLE_CLICK_MS) return;
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
