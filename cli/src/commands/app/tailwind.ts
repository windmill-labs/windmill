import * as fs from "node:fs";
import * as path from "node:path";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import * as log from "../../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import { readTextFileSync } from "../../utils/utils.ts";
import { APP_BACKEND_FOLDER } from "./app_metadata.ts";
import {
  compileTailwind,
  declaresTailwindV4,
  installedTailwindMajor,
  staticPrefix,
  TAILWIND_SOURCE_EXTENSIONS,
  type TailwindCompile,
  type TailwindHost,
  type TailwindSource,
} from "./tailwind_core.ts";

const MAX_SOURCE_BYTES = 1024 * 1024;

/** `src/**\/*.vue` scans `.vue` files; a pattern naming no extension scans the default set. */
function sourceExtensions(pattern: string): string[] {
  const ext = pattern.match(/\*(\.[\w-]+)$/);
  return ext ? [ext[1]] : TAILWIND_SOURCE_EXTENSIONS;
}

/**
 * `app dev` recompiles in this process, so an edited `@config` or `@plugin`
 * must not come back from the module cache. Node keys its ES module cache by
 * URL, query included, but a CommonJS file imported that way is still served
 * from `require.cache`; Bun ignores the query and honours only that deletion.
 */
async function freshImport(file: string): Promise<unknown> {
  let mod: any;
  const req = createRequire(file);
  delete req.cache[file];
  if ((process.versions as Record<string, string | undefined>).bun) {
    mod = req(file);
  } else {
    const url = pathToFileURL(file);
    url.searchParams.set("mtime", String(fs.statSync(file).mtimeMs));
    mod = await import(url.href);
  }
  return mod.default ?? mod;
}

function readJson(file: string): string | undefined {
  try {
    return readTextFileSync(file);
  } catch {
    return undefined;
  }
}

/**
 * Walks `dir` for scannable files. The default walk skips build output
 * (`dist`); an explicit `@source` keeps it, since a component library
 * (`@source "../node_modules/some-ui-lib"`) ships its classes only there.
 * Neither descends into a nested `node_modules`.
 */
function walk(
  dir: string,
  extensions: string[],
  skipTop: Set<string>,
  out: { files: Map<string, string>; dirs: Set<string> },
  explicit: boolean,
) {
  let entries: fs.Dirent[];
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  out.dirs.add(dir);
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name.startsWith(".") || skipTop.has(entry.name)) continue;
      if (entry.name === "node_modules" || (!explicit && entry.name === "dist")) continue;
      walk(full, extensions, new Set(), out, explicit);
    } else if (entry.isFile()) {
      if (!extensions.includes(path.extname(entry.name))) continue;
      addFile(full, out);
    }
  }
}

function addFile(file: string, out: { files: Map<string, string> }) {
  let real: string;
  try {
    real = fs.realpathSync(file);
    if (out.files.has(real) || fs.statSync(real).size > MAX_SOURCE_BYTES) return;
    out.files.set(real, fs.readFileSync(real, "utf-8"));
  } catch {
    // A file that vanished mid-walk has no classes to contribute.
  }
}

/**
 * The esbuild plugin compiling Tailwind CSS v4 stylesheets, or undefined when
 * the app does not declare `tailwindcss` v4+: then nothing is registered and
 * the build is exactly what it is without Tailwind. The compiler is the app's
 * own, loaded on the first stylesheet that uses a Tailwind directive.
 */
export function createTailwindPlugin(appDir: string, sharedUiDir?: string): any | undefined {
  const resolvedApp = path.resolve(appDir);
  if (!declaresTailwindV4(readJson(path.join(resolvedApp, "package.json")))) {
    return undefined;
  }
  const requireFromApp = createRequire(path.join(resolvedApp, "package.json"));
  let pkgDir: string;
  try {
    pkgDir = path.dirname(requireFromApp.resolve("tailwindcss/package.json"));
  } catch {
    log.warn(
      colors.yellow(
        "⚠️  package.json declares tailwindcss but it is not installed; Tailwind stylesheets will not be compiled",
      ),
    );
    return undefined;
  }
  const pkgJson = readJson(path.join(pkgDir, "package.json"));
  const major = installedTailwindMajor(pkgJson);
  if (major === undefined || major < 4) {
    log.warn(
      colors.yellow(
        `⚠️  installed tailwindcss is v${major ?? "?"}; only v4+ is compiled by the bundler`,
      ),
    );
    return undefined;
  }
  log.info(colors.blue("🎨 Tailwind CSS v4 detected, adding tailwind plugin..."));

  const uiDir = sharedUiDir && fs.existsSync(sharedUiDir) ? fs.realpathSync(sharedUiDir) : undefined;
  let compilePromise: Promise<TailwindCompile> | undefined;
  const loadCompile = () => {
    compilePromise ??= (async () => {
      const exportsMap = JSON.parse(pkgJson!)?.exports?.["."];
      const entry = typeof exportsMap === "object" && typeof exportsMap.import === "string"
        ? path.resolve(pkgDir, exportsMap.import)
        : requireFromApp.resolve("tailwindcss");
      const mod = await import(pathToFileURL(entry).href);
      const compile = mod.compile ?? mod.default?.compile;
      if (typeof compile !== "function") {
        throw new Error(`tailwindcss at ${pkgDir} has no compile() export`);
      }
      return compile as TailwindCompile;
    })();
    return compilePromise;
  };

  return {
    name: "wmill-tailwind",
    setup(build: any) {
      const host: TailwindHost = {
        loadCompile,
        // Through esbuild's own resolver, so `/ui/...`, the "style" export
        // condition and bare imports from `ui/` resolve as a plain `@import` would.
        async loadStylesheet(id, base) {
          const resolved = await build.resolve(id, {
            kind: "import-rule",
            resolveDir: base,
            importer: path.join(base, "__tailwind__.css"),
          });
          if (resolved.errors.length > 0) {
            throw new Error(resolved.errors[0].text);
          }
          return {
            path: resolved.path,
            base: path.dirname(resolved.path),
            content: fs.readFileSync(resolved.path, "utf-8"),
          };
        },
        rebaseUrl(url, from, to) {
          const cut = url.search(/[?#]/);
          const file = cut === -1 ? url : url.slice(0, cut);
          const suffix = cut === -1 ? "" : url.slice(cut);
          const rel = path.relative(to, path.resolve(from, file)).split(path.sep).join("/");
          return (rel.startsWith("../") ? rel : "./" + rel) + suffix;
        },
        // `@plugin` / `@config` run in this process, as they would under the
        // Tailwind CLI: resolved from the stylesheet, then from the app.
        async loadModule(id, base) {
          let resolved: string;
          try {
            resolved = createRequire(path.join(base, "__tailwind__.js")).resolve(id);
          } catch {
            resolved = requireFromApp.resolve(id);
          }
          return { path: resolved, base: path.dirname(resolved), module: await freshImport(resolved) };
        },
        async scan(sources: TailwindSource[], includeDefault: boolean) {
          const out = { files: new Map<string, string>(), dirs: new Set<string>() };
          if (includeDefault) {
            walk(resolvedApp, TAILWIND_SOURCE_EXTENSIONS, new Set([APP_BACKEND_FOLDER]), out, false);
            if (uiDir) walk(uiDir, TAILWIND_SOURCE_EXTENSIONS, new Set(), out, false);
          }
          for (const source of sources) {
            const target = path.resolve(source.base, staticPrefix(source.pattern));
            let stat: fs.Stats;
            try {
              stat = fs.statSync(target);
            } catch {
              continue;
            }
            if (stat.isFile()) addFile(target, out);
            else walk(target, sourceExtensions(source.pattern), new Set(), out, true);
          }
          return {
            files: [...out.files].map(([p, content]) => ({ path: p, content })),
            dirs: [...out.dirs],
          };
        },
      };

      build.onLoad({ filter: /\.css$/, namespace: "file" }, async (args: any) => {
        const css = fs.readFileSync(args.path, "utf-8");
        try {
          const result = await compileTailwind(css, args.path, path.dirname(args.path), host);
          if (!result) return undefined;
          return {
            contents: result.css,
            loader: "css",
            resolveDir: path.dirname(args.path),
            watchFiles: result.files,
            watchDirs: result.dirs,
          };
        } catch (e: any) {
          return {
            errors: [{ text: `Tailwind CSS: ${e?.message ?? e}` }],
            watchFiles: [args.path],
          };
        }
      });
    },
  };
}
