import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { dirname, relative } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";
import { defineConfig, type Plugin } from "vite";
import pkg from "../package.json" with { type: "json" };
import { hotkeyRows } from "./hotkeys.ts";
import { fillFaqPage, fillQuestionLists, readFaq } from "./questions.ts";
import { STAR } from "./star.ts";

// The Minerva site for players: the home page (site/index.html), the FAQ
// (site/faq/index.html) and the Map Leaderboards (site/leaderboards/index.html, and a map's
// page, site/leaderboards/map/index.html, both reading the Worker's API), and the account
// pages (site/account, its link page and site/admin, which no tab names), published on
// Cloudflare from a release's tag (.github/workflows/site.yml, site/wrangler.jsonc). `pnpm site:dev` serves it,
// `pnpm site:build` builds it into site/dist.

const HOME = fileURLToPath(new URL("./index.html", import.meta.url));
const FAQ = fileURLToPath(new URL("./faq/index.html", import.meta.url));
const LEADERBOARDS = fileURLToPath(new URL("./leaderboards/index.html", import.meta.url));
const MAP_LEADERBOARD = fileURLToPath(new URL("./leaderboards/map/index.html", import.meta.url));
const ACCOUNT = fileURLToPath(new URL("./account/index.html", import.meta.url));
const LINK = fileURLToPath(new URL("./account/link/index.html", import.meta.url));
const ADMIN = fileURLToPath(new URL("./admin/index.html", import.meta.url));
const REPO = fileURLToPath(new URL("..", import.meta.url));
const same = (a: string, b: string) => a.replace(/\\/g, "/") === b.replace(/\\/g, "/");
/** The folder `to` from the folder of the page `page`, as an address: "", "..", "../.."… */
const upTo = (page: string, to: string) => relative(dirname(page), to).replace(/\\/g, "/");
/** Whether a page being built is the home page. */
const isHome = (file: string) => same(file, HOME);
/** Whether a page being built is the FAQ, where the questions are written. */
const isFaq = (file: string) => same(file, FAQ);

/** Where the site is published, for the links that must be absolute (Open Graph). */
const SITE_URL = "https://owlofminerva.app/";
const RELEASES = "https://github.com/itechify/minerva/releases";

/**
 * The pages' placeholders, filled at build: the version they describe (package.json's, which
 * at a release's tag is the release's), that version's downloads, the hotkey table, the
 * star's path (site/star.ts), the window's head and the credits.
 */
function fillPlaceholders(): Plugin {
  const version = pkg.version;
  const values: Record<string, string> = {
    "@VERSION@": version,
    "@SETUP_URL@": `${RELEASES}/download/v${version}/Minerva_${version}_x64-setup.exe`,
    "@RELEASE_URL@": `${RELEASES}/tag/v${version}`,
    "@SITE_URL@": SITE_URL,
    "@HOTKEY_ROWS@": hotkeyRows(),
    "@STAR@": STAR,
  };
  // The window's head opens every page and the credits close it, each written once.
  const windowHead = fileURLToPath(new URL("./window-head.html", import.meta.url));
  const credits = fileURLToPath(new URL("./credits.html", import.meta.url));
  /** The head as `page` shows it: its icon from the repo's root, and its own tab marked open.
   *  The account pages have no tab. */
  function headFor(page: string) {
    const isLeaderboards = same(page, LEADERBOARDS) || same(page, MAP_LEADERBOARD);
    const tab = isHome(page) ? "/" : isFaq(page) ? "/faq/" : isLeaderboards ? "/leaderboards/" : null;
    const head = readFileSync(windowHead, "utf8")
      .replace(/^<!--[\s\S]*?-->\s*/, "")
      .trim()
      .replace("@ROOT@", () => upTo(page, REPO));
    return tab === null ? head : head.replace(`<a class="tab" href="${tab}"`, (open) => `${open} aria-current="page"`);
  }
  return {
    name: "minerva-site:placeholders",
    transformIndexHtml: {
      order: "pre",
      handler: (html, ctx) =>
        html
          .replace("@WINDOW_HEAD@", () => headFor(ctx.filename))
          .replace("@CREDITS@", () => readFileSync(credits, "utf8").trim())
          .replace(/@[A-Z_]+@/g, (token) => values[token] ?? token),
    },
  };
}

/**
 * The FAQ's questions, each written once on the FAQ page: drawn there, and copied into the
 * other pages where they name them (site/questions.ts). An edit to the FAQ reloads them all.
 */
function questions(): Plugin {
  return {
    name: "minerva-site:questions",
    transformIndexHtml: {
      order: "pre",
      handler(html, ctx) {
        const faq = readFaq(readFileSync(FAQ, "utf8"));
        if (isFaq(ctx.filename)) return fillFaqPage(html, faq);
        // The site's folder from the page's: "./" for the home page, "../" a folder down.
        const root = upTo(ctx.filename, dirname(HOME));
        return fillQuestionLists(html, faq, root ? `${root}/` : "./");
      },
    },
    handleHotUpdate({ file, server }) {
      if (isFaq(file)) server.ws.send({ type: "full-reload" });
    },
  };
}

/**
 * The screenshots are taken by hand in the game (site/screenshots/README.md), so the build
 * fails while one is missing rather than publish the page without it. Vite leaves the
 * address of a picture it couldn't find as written. Open Graph and Twitter cards then get
 * the hero shot's absolute address, known only once the build has named the file.
 */
function screenshots(): Plugin {
  return {
    name: "minerva-site:screenshots",
    apply: "build",
    transformIndexHtml: {
      order: "post",
      handler(html, ctx) {
        const missing = [...html.matchAll(/<img[^>]*\bsrc="(?:\.\.?\/)*\/?(screenshots\/[^"]+)"/g)].map((m) => `site/${m[1]}`);
        if (missing.length > 0) {
          throw new Error(`site: not taken yet: ${missing.join(", ")} (see site/screenshots/README.md)`);
        }
        if (isHome(ctx.filename) && !/<img[^>]*\bid="hero-shot"/.test(html)) {
          throw new Error("site: the hero shot (img#hero-shot) is missing from the home page");
        }
        // The hero shot is every page's card. Its address as Vite writes it from the site's
        // root (it names the file last), after the site's origin.
        const hero = Object.values(ctx.bundle ?? {}).find(
          (file) => file.type === "asset" && file.originalFileNames.some((name) => name.endsWith("screenshots/hero.png")),
        );
        if (!hero) throw new Error("site: the hero shot (site/screenshots/hero.png) isn't in the build");
        return html.replace(/@CARD_IMAGE@/g, `${new URL(SITE_URL).origin}/${hero.fileName}`);
      },
    },
  };
}

/**
 * Each screenshot's own size on its <img>, read from the PNG's header, so the page keeps
 * its room before the picture arrives and a crop shows at its full size.
 */
function screenshotSizes(): Plugin {
  return {
    name: "minerva-site:screenshot-sizes",
    transformIndexHtml: {
      order: "pre",
      handler: (html) =>
        html.replace(/<img\b([^>]*?)\bsrc="((?:\.\.?\/)+)(screenshots\/[^"]+\.png)"/g, (tag, before: string, up: string, path: string) => {
          const file = fileURLToPath(new URL(`./${path}`, import.meta.url));
          if (!existsSync(file)) return tag;
          // A PNG's width and height are its IHDR chunk's first eight bytes, after the
          // eight-byte signature and the chunk's length and type.
          const header = readFileSync(file).subarray(16, 24);
          const size = `width="${header.readUInt32BE(0)}" height="${header.readUInt32BE(4)}" `;
          return `<img${before}${size}src="${up}${path}"`;
        }),
    },
  };
}

/**
 * Each screenshot also as WebP, a fraction of the PNG's size, made with sharp when the page
 * is served or built (into site/screenshots/webp, which isn't committed): the PNGs stay the
 * shots as taken, and a browser takes the WebP. A picture whose <img> says how wide it
 * shows (`sizes`) gets a half-size copy too, for a phone. A shot not taken yet gets none,
 * so the build still fails on it.
 */
function webpScreenshots(): Plugin {
  const dir = fileURLToPath(new URL("./screenshots/webp/", import.meta.url));
  /** Writes the PNG as WebP, at `width` if given, unless a copy newer than the PNG is there. */
  async function webp(png: string, name: string, width?: number) {
    const file = `${dir}${name}`;
    if (existsSync(file) && statSync(file).mtimeMs >= statSync(png).mtimeMs) return;
    mkdirSync(dir, { recursive: true });
    await sharp(png).resize(width ? { width } : undefined).webp({ quality: 90 }).toFile(file);
  }
  return {
    name: "minerva-site:webp-screenshots",
    transformIndexHtml: {
      order: "pre",
      async handler(html) {
        for (const [tag, up, name] of html.matchAll(/<img\b[^>]*\bsrc="((?:\.\.?\/)+)screenshots\/([\w-]+)\.png"[^>]*>/g)) {
          const png = fileURLToPath(new URL(`./screenshots/${name}.png`, import.meta.url));
          if (!existsSync(png)) continue;
          await webp(png, `${name}.webp`);
          let srcset = `${up}screenshots/webp/${name}.webp`;
          const sizes = /\bsizes="([^"]+)"/.exec(tag)?.[1];
          if (sizes) {
            const { width } = await sharp(png).metadata();
            const half = Math.round(width / 2);
            await webp(png, `${name}-${half}.webp`, half);
            srcset = `${up}screenshots/webp/${name}-${half}.webp ${half}w, ${srcset} ${width}w`;
          }
          const source = `<source type="image/webp" srcset="${srcset}"${sizes ? ` sizes="${sizes}"` : ""} />`;
          html = html.replace(tag, `<picture>${source}${tag}</picture>`);
        }
        return html;
      },
    },
  };
}

/**
 * The page names the app's theme and icon outside site/ (../src, ../assets), as the build
 * takes them. The dev server serves only its root, so it serves those from the repo.
 */
function serveAppFiles(): Plugin {
  return {
    name: "minerva-site:app-files",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use((req, _res, next) => {
        if (req.url && /^\/(src|assets)\//.test(req.url)) {
          const file = fileURLToPath(new URL(`..${req.url}`, import.meta.url)).replace(/\\/g, "/");
          req.url = `/@fs${file.startsWith("/") ? "" : "/"}${file}`;
        }
        next();
      });
    },
  };
}

export default defineConfig({
  root: "site",
  base: "/",
  // Copied into the build as they are: Cloudflare's _headers.
  publicDir: "public",
  // The Map Leaderboards pages read the Worker's API. While developing, it is the Worker
  // run beside the dev server with `wrangler dev` (README.md).
  server: { proxy: { "/api": "http://localhost:8787" } },
  plugins: [
    questions(),
    fillPlaceholders(),
    screenshotSizes(),
    webpScreenshots(),
    screenshots(),
    serveAppFiles(),
  ],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rolldownOptions: {
      input: { home: HOME, faq: FAQ, leaderboards: LEADERBOARDS, map: MAP_LEADERBOARD, account: ACCOUNT, link: LINK, admin: ADMIN },
    },
    // A screenshot keeps a file of its own, however small, so the cards can address it.
    assetsInlineLimit: (file) => (file.includes("/screenshots/") ? false : undefined),
  },
});
