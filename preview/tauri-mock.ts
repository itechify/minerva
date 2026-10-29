/// <reference types="vite/client" />
/** Stands in for the shell when `pnpm preview:window` opens a window's page in a browser:
 *  `page_view` answers with the fixture `?fixture=` names, from `preview/fixtures/<window>/`,
 *  listening does nothing, and any other command is logged. What the page reports of its
 *  size lands in `window.__preview` for the script to size the shot by. Only the preview's
 *  Vite server puts this on a page (scripts/preview-window.mjs); the app's build never has it. */
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

/** A fixture: the view (its default export) and, for a window the core places rather than
 *  sizing to its page (the Deal Badges'), the height to shoot it at. */
type Fixture = { default: unknown; height?: number };

type Preview = {
  /** The page has had its view. */
  ready: boolean;
  /** The last height and width the page reported, in CSS px. */
  height: number | null;
  width: number | null;
  fixtureHeight: number | null;
};

declare global {
  interface Window {
    __preview: Preview;
  }
}

const fixtures = import.meta.glob<Fixture>("./fixtures/*/*.ts");
const page = location.pathname.replace(/^\//, "").replace(/\.html$/, "");
const state = new URLSearchParams(location.search).get("fixture");
const preview: Preview = { ready: false, height: null, width: null, fixtureHeight: null };
window.__preview = preview;

async function view(): Promise<unknown> {
  const load = fixtures[`./fixtures/${page}/${state}.ts`];
  if (!load) throw new Error(`no fixture preview/fixtures/${page}/${state}.ts`);
  const fixture = await load();
  preview.fixtureHeight = fixture.height ?? null;
  // Every font the page declares, in before the view is drawn: a page measures itself as it
  // draws, and until a font is used nothing asks for it. This is a window some time after
  // it opened, not its first frame.
  await Promise.all([...document.fonts].map((font) => font.load().catch(() => {})));
  // The page draws the view once this answer resolves.
  setTimeout(() => (preview.ready = true));
  return fixture.default;
}

let listeners = 0;
mockWindows(page);
mockIPC((cmd, args) => {
  const given = (args ?? {}) as Record<string, unknown>;
  switch (cmd) {
    case "page_view":
      return view();
    case "plugin:event|listen":
      return ++listeners;
    case "plugin:event|unlisten":
      return null;
    case "page_height":
      preview.height = given.height as number;
      return null;
    case "page_width":
      preview.width = given.width as number;
      return null;
    case "window_error":
      console.error(`window_error: ${given.message}`);
      return null;
    default:
      console.info(`[tauri-mock] ${cmd}`, given);
      return null;
  }
});
