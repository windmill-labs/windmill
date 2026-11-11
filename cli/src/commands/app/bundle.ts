import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import * as log from "../../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import * as windmillUtils from "@windmill-labs/shared-utils";
import { readTextFile, readTextFileSync } from "../../utils/utils.ts";
import { withEsbuild } from "../../utils/esbuild_loader.ts";
export interface BundleOptions {
  entryPoint?: string;
  outDir?: string;
  sourcemap?: boolean;
  minify?: boolean;
  production?: boolean;
  /**
   * Absolute path to the workspace's shared `ui/` folder. When set, imports
   * starting with `/ui/...` are resolved as files inside this directory.
   * Allows raw apps to reuse components from the workspace-level shared folder.
   */
  sharedUiDir?: string;
}

export interface BundleResult {
  js: string;
  css: string;
}

export const DEFAULT_BUILD_OPTIONS = {
  bundle: true,
  format: "iife" as const,
  platform: "browser" as const,
  target: "es2020",
  jsx: "automatic" as const,
  loader: {
    ".css": "css" as const,
  },
  // esbuild export conditions safe for any app: "style" resolves tailwindcss v4's CSS
  // entry (@import "tailwindcss"); "module" is re-added because esbuild drops its
  // auto-included "module" default once any custom condition is set. The Svelte-only
  // "svelte" condition is gated per-app in conditionsFor().
  conditions: ["style", "module"],
  logLevel: "info" as const,
  write: true,
};

// "svelte" points at raw .svelte sources that only compile with the Svelte plugin, so
// enable it only for Svelte apps — for a plain app a Svelte-dual-published import would
// otherwise resolve to .svelte and hard-fail with no loader configured.
function conditionsFor(svelte: boolean): string[] {
  return svelte
    ? [...DEFAULT_BUILD_OPTIONS.conditions, "svelte"]
    : DEFAULT_BUILD_OPTIONS.conditions;
}

/**
 * Detects which frontend frameworks are present in package.json
 */
export function detectFrameworks(appDir: string): { svelte: boolean } {
  const packageJsonPath = path.join(appDir, "package.json");
  if (!fs.existsSync(packageJsonPath)) {
    return { svelte: false };
  }

  try {
    const packageJson = JSON.parse(readTextFileSync(packageJsonPath));
    const allDeps = {
      ...packageJson.dependencies,
      ...packageJson.devDependencies,
    };

    return {
      svelte: "svelte" in allDeps,
    };
  } catch {
    return { svelte: false };
  }
}

/** What an `import()` matches — the point being that it never matches "require". */
const ESM_CONDITIONS = ["node", "import", "default"];

/**
 * Walks one subpath of an exports map the way Node's ESM resolver would: first
 * key in declaration order whose condition an `import()` matches wins.
 */
function esmConditionTarget(subpath: unknown): string | undefined {
  if (typeof subpath === "string") return subpath;
  if (!subpath || typeof subpath !== "object" || Array.isArray(subpath)) {
    return undefined;
  }
  for (const [condition, target] of Object.entries(subpath)) {
    if (!ESM_CONDITIONS.includes(condition)) continue;
    const entry = esmConditionTarget(target);
    if (entry) return entry;
  }
  return undefined;
}

/**
 * `require.resolve` answers with the `require` condition, which Svelte maps at a
 * minified UMD bundle. Only the CJS loader can read that file's exports, so
 * `import()`ing it yields a namespace holding nothing but `default` and every
 * named export reads undefined. The exports map is the only place to ask for the
 * ESM entry instead — `require.resolve` takes no conditions.
 */
function resolveAppSvelteCompiler(appDir: string): string {
  const requireFromApp = createRequire(
    path.join(path.resolve(appDir), "package.json")
  );
  try {
    const pkgPath = requireFromApp.resolve("svelte/package.json");
    const exportsMap = JSON.parse(readTextFileSync(pkgPath))?.exports;
    const target = esmConditionTarget(exportsMap?.["./compiler"]);
    if (target?.startsWith(".")) {
      const entry = path.resolve(path.dirname(pkgPath), target);
      if (fs.existsSync(entry)) return entry;
    }
  } catch {
    // No exports map to read (or an unexpected shape) — let the CJS resolver try.
  }
  return requireFromApp.resolve("svelte/compiler");
}

/**
 * Loads the Svelte compiler out of the *app's* node_modules.
 *
 * The app brings its own Svelte runtime via package.json, and compiler and
 * runtime have to agree on internals: Svelte 5.52.0 moved delegated event
 * handlers off `element.__click` onto a Symbol-keyed map, so an older
 * compiler's `onclick` output is silently ignored by a newer runtime — the app
 * builds and renders with every handler dead. A bare `import("svelte/compiler")`
 * resolves against this CLI instead, whose own svelte floats independently of
 * the app's, which is exactly how the two drift apart.
 *
 * Falls back to the CLI's own compiler when the app has none resolvable.
 */
export async function loadSvelteCompiler(appDir: string): Promise<any> {
  let mod: any;
  try {
    mod = await import(pathToFileURL(resolveAppSvelteCompiler(appDir)).href);
  } catch {
    mod = await import("svelte/compiler");
  }
  // A CJS entry still imports as a namespace whose only key is `default`.
  return typeof mod?.compile === "function" ? mod : (mod?.default ?? mod);
}

/**
 * Creates a Svelte esbuild plugin
 * Uses the svelte compiler from the project's node_modules
 */
function createSveltePlugin(appDir: string): any {
  // Resolved once per build, not per file.
  let compilerPromise: Promise<any> | undefined;
  const svelteCompiler = () =>
    (compilerPromise ??= loadSvelteCompiler(appDir));

  // This converts a message in Svelte's format to esbuild's format
  const messageConverter =
    (source: string, filename: string) =>
    ({ message, start, end }: any) => {
      let location;
      if (start && end) {
        const lineText = source.split(/\r\n|\r|\n/g)[start.line - 1];
        const lineEnd = start.line === end.line ? end.column : lineText.length;
        location = {
          file: filename,
          line: start.line,
          column: start.column,
          length: lineEnd - start.column,
          lineText,
        };
      }
      return { text: message, location };
    };

  return {
    name: "svelte",
    setup(build: any) {
      build.onLoad({ filter: /\.svelte$/ }, async (args: any) => {
        const svelte = await svelteCompiler();

        // Load the file from the file system
        const source = await readTextFile(args.path);
        const filename = path.relative(process.cwd(), args.path);
        const convertMessage = messageConverter(source, filename);

        // Convert Svelte syntax to JavaScript
        try {
          // The raw-app editor's in-browser bundler compiles with
          // `css: "injected"`, so this must too, or the same app renders
          // styled there and unstyled once the CLI builds it: Svelte's default
          // ("external") hands the <style> back on a `css` field nothing emits.
          const { js, warnings } = svelte.compile(source, {
            filename,
            css: "injected",
          });
          const contents = js.code + `//# sourceMappingURL=` + js.map.toUrl();
          return { contents, warnings: warnings.map(convertMessage) };
        } catch (e: any) {
          return { errors: [convertMessage(e)] };
        }
      });

      // `lib.svelte.ts` / `lib.svelte.js` are plain modules that may use runes.
      // They need `compileModule`, otherwise `$state`/`$derived` sail through
      // esbuild untouched and the bundle throws "$state is not defined".
      build.onLoad({ filter: /\.svelte\.[jt]s$/ }, async (args: any) => {
        const svelte = await svelteCompiler();

        const source = await readTextFile(args.path);
        const filename = path.relative(process.cwd(), args.path);
        const convertMessage = messageConverter(source, filename);

        try {
          // `compileModule` parses with plain acorn and chokes on TypeScript, so
          // types have to come off first (vite-plugin-svelte gets this for free
          // by running after Vite's esbuild transform).
          const code = filename.endsWith(".ts")
            ? (
                await build.esbuild.transform(source, {
                  loader: "ts",
                  sourcefile: filename,
                })
              ).code
            : source;

          const { js, warnings } = svelte.compileModule(code, { filename });
          const contents = js.code + `//# sourceMappingURL=` + js.map.toUrl();
          return {
            contents,
            loader: "js",
            warnings: warnings.map(convertMessage),
          };
        } catch (e: any) {
          return { errors: [convertMessage(e)] };
        }
      });
    },
  };
}

// The Tailwind import (split ones such as `tailwindcss/theme.css` included) or any
// directive only Tailwind defines: a stylesheet that imports the theme from ui/ and
// adds its own `@theme`/`@source` must be compiled as the root, with the import
// inlined into it, or its additions reach the browser raw.
const TAILWIND_ENTRY =
  /@import\s+(?:url\(\s*)?["']tailwindcss["'/]|@(?:tailwind|theme|utility|variant|custom-variant|source|plugin|config|reference|apply)\b/;
const TAILWIND_SCAN_EXTS = new Set([
  ".html", ".js", ".jsx", ".ts", ".tsx", ".svelte", ".vue",
]);

function collectTailwindCandidates(dirs: string[]): string[] {
  const candidates = new Set<string>();
  const walk = (dir: string) => {
    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      if (e.name.startsWith(".") || e.name === "node_modules" || e.name === "dist") continue;
      const full = path.join(dir, e.name);
      if (e.isDirectory()) walk(full);
      else if (TAILWIND_SCAN_EXTS.has(path.extname(e.name))) {
        // Each split keeps a different set of characters that candidates may contain
        // (`content-['x']` or `content-["x"]` inside the other quote, `[&>svg]:hidden`,
        // classes next to markup); the compiler drops tokens that are not utilities,
        // so the union is safe.
        const text = fs.readFileSync(full, "utf-8");
        for (const re of [/[\s"`{};]+/, /[\s'`{};=]+/, /[\s"'`{};=]+/, /[\s"'`{}<>;=]+/]) {
          for (const tok of text.split(re)) {
            // Svelte's `class:hidden={x}` names the utility after the directive.
            if (tok) candidates.add(tok.startsWith("class:") ? tok.slice(6) : tok);
          }
        }
      }
    }
  };
  dirs.forEach(walk);
  return [...candidates];
}

type TailwindKit =
  // The official toolchain, as @tailwindcss/vite uses it: Tailwind's own module
  // loading, URL rewriting and oxide scanner (`@source` included).
  | { kind: "node"; compile: any; Scanner: any; clearRequireCache: (files: string[]) => void }
  // Only the `tailwindcss` package: its compile() with our module loading and scan.
  | { kind: "core"; compile: any };

/**
 * Compiles stylesheets that pull in Tailwind v4 (`@import "tailwindcss"`) with the
 * app's own Tailwind packages, generating the utilities used in the app and the
 * shared `ui/` folder. esbuild alone would inline Tailwind's sources verbatim and
 * ship `@theme`/`@tailwind utilities` to the browser, which ignores them.
 * Stylesheet imports are resolved through esbuild, so `/ui/...` and the "style"
 * export condition work as they do for plain CSS.
 */
function createTailwindPlugin(appDir: string, sharedUiDir?: string): any {
  const requireFromApp = createRequire(path.join(path.resolve(appDir), "package.json"));
  const importFromApp = async (id: string) => {
    const mod: any = await import(pathToFileURL(requireFromApp.resolve(id)).href);
    return mod.default && !mod.compile && !mod.Scanner ? mod.default : mod;
  };
  let kitPromise: Promise<TailwindKit | undefined> | undefined;
  const loadKit = () =>
    (kitPromise ??= (async () => {
      try {
        const [node, oxide, cache] = await Promise.all([
          importFromApp("@tailwindcss/node"),
          importFromApp("@tailwindcss/oxide"),
          importFromApp("@tailwindcss/node/require-cache"),
        ]);
        if (typeof node.compile === "function" && typeof oxide.Scanner === "function") {
          return {
            kind: "node" as const,
            compile: node.compile,
            Scanner: oxide.Scanner,
            clearRequireCache: cache.clearRequireCache,
          };
        }
      } catch {
        // Not installed: fall back to the core package.
      }
      try {
        requireFromApp.resolve("tailwindcss");
      } catch {
        return undefined;
      }
      try {
        const { compile } = await importFromApp("tailwindcss");
        // Tailwind v3 has no compile(); its CSS keeps going through esbuild as before.
        return typeof compile === "function" ? { kind: "core" as const, compile } : undefined;
      } catch (e: any) {
        log.warn(`Could not load tailwindcss, Tailwind CSS is bundled uncompiled: ${e?.message ?? e}`);
        return undefined;
      }
    })());

  const dirs = [appDir, ...(sharedUiDir ? [sharedUiDir] : [])];
  // What the previous compile of each stylesheet loaded, dropped from Node's require
  // cache before the next one so an edited CommonJS config or plugin is re-read.
  const previousDeps = new Map<string, string[]>();

  return {
    name: "wmill-tailwind",
    setup(build: any) {
      build.onLoad({ filter: /\.css$/ }, async (args: any) => {
        // Package stylesheets (tailwindcss's own parts included) are inlined by the
        // app stylesheet that imports them, never compiled on their own.
        if (args.namespace !== "file" || args.path.includes(`${path.sep}node_modules${path.sep}`)) {
          return undefined;
        }
        const source = await readTextFile(args.path);
        if (!TAILWIND_ENTRY.test(source)) return undefined;
        const kit = await loadKit();
        if (!kit) return undefined;

        const watchFiles = new Set<string>();
        const resolveStylesheet = async (id: string, base: string) => {
          // The importer lets the shared-ui plugin send bare imports made from ui/
          // to the app's node_modules; esbuild only reads it as a path prefix.
          const r = await build.resolve(id, {
            resolveDir: base,
            importer: path.join(base, "_"),
            kind: "import-rule",
          });
          if (r.errors.length > 0) throw new Error(r.errors[0].text);
          watchFiles.add(r.path);
          return r.path as string;
        };
        // Plugins and configs run in Node, so they resolve with Node's rules rather
        // than esbuild's browser ones; ui/ falls back to the app's packages.
        const resolveModule = (id: string, base: string) => {
          try {
            return createRequire(path.join(base, "_")).resolve(id);
          } catch {
            return requireFromApp.resolve(id);
          }
        };
        const result = (contents: string) => ({
          contents,
          loader: "css",
          resolveDir: path.dirname(args.path),
          watchFiles: [...watchFiles],
        });

        try {
          if (kit.kind === "node") {
            kit.clearRequireCache(previousDeps.get(args.path) ?? []);
            const compiler = await kit.compile(source, {
              base: path.dirname(args.path),
              from: args.path,
              shouldRewriteUrls: true,
              onDependency: (p: string) => watchFiles.add(p),
              customCssResolver: (id: string, base: string) =>
                resolveStylesheet(id, base).catch(() => undefined),
              customJsResolver: async (id: string, base: string) => {
                try {
                  return resolveModule(id, base);
                } catch {
                  return undefined;
                }
              },
            });
            // `source(none)` scans nothing, `source("dir")` replaces the default
            // roots; `@source` entries add to either.
            const roots =
              compiler.root === "none"
                ? []
                : compiler.root
                  ? [compiler.root]
                  : dirs.map((base) => ({ base, pattern: "**/*" }));
            const scanner = new kit.Scanner({
              sources: [
                ...roots.map((r: any) => ({ ...r, negated: false })),
                ...compiler.sources,
              ],
            });
            previousDeps.set(args.path, [...watchFiles]);
            return result(compiler.build(scanner.scan()));
          }

          const compiler = await kit.compile(source, {
            base: path.dirname(args.path),
            loadStylesheet: async (id: string, base: string) => {
              const p = await resolveStylesheet(id, base);
              return { path: p, base: path.dirname(p), content: await readTextFile(p) };
            },
            loadModule: async (id: string, base: string) => {
              const p = resolveModule(id, base);
              // Bundled and evaluated afresh on every build rather than import()ed: TS, ESM
              // and CJS configs all load under Node, and an edit to the module or a local
              // file it imports applies on the next `app dev` rebuild instead of hitting
              // the runtime's module cache.
              const out = await build.esbuild.build({
                entryPoints: [p],
                bundle: true,
                platform: "node",
                format: "cjs",
                packages: "external",
                write: false,
                metafile: true,
                logLevel: "silent",
              });
              for (const input of Object.keys(out.metafile.inputs)) {
                watchFiles.add(path.resolve(input));
              }
              const module = { exports: {} as any };
              new Function("module", "exports", "require", "__filename", "__dirname",
                out.outputFiles[0].text)(module, module.exports, createRequire(p), p, path.dirname(p));
              const mod = module.exports;
              return { path: p, base: path.dirname(p), module: mod?.default ?? mod };
            },
          });
          return result(compiler.build(collectTailwindCandidates(dirs)));
        } catch (e: any) {
          return { errors: [{ text: `Tailwind CSS: ${e?.message ?? e}` }] };
        }
      });
    },
  };
}

/**
 * Creates framework-specific esbuild plugins based on detected dependencies
 */
export async function createFrameworkPlugins(
  appDir: string,
  sharedUiDir?: string,
): Promise<any[]> {
  const frameworks = detectFrameworks(appDir);
  const plugins: any[] = [createTailwindPlugin(appDir, sharedUiDir)];

  if (frameworks.svelte) {
    log.info(colors.blue("🔧 Svelte detected, adding svelte plugin..."));
    plugins.push(createSveltePlugin(appDir));
  }

  return plugins;
}

/**
 * Ensures node_modules exists in the specified directory
 * Runs npm install if node_modules is missing
 * @param appDir Directory to check for node_modules (defaults to entry point directory)
 */
export async function ensureNodeModules(appDir?: string): Promise<void> {
  const targetDir = appDir ?? process.cwd();
  const nodeModulesPath = path.join(targetDir, "node_modules");

  if (!fs.existsSync(nodeModulesPath)) {
    log.info(colors.yellow("📦 node_modules not found, running npm install..."));
    const code = await new Promise<number>((resolve, reject) => {
      const npmInstall = spawn("npm", ["install"], {
        cwd: targetDir,
        stdio: "inherit",
        shell: true,
      });
      npmInstall.on("close", (code) => resolve(code ?? 0));
      npmInstall.on("error", reject);
    });
    if (code !== 0) {
      throw new Error(`npm install failed with exit code ${code}`);
    }
    log.info(colors.green("✅ npm install completed"));
  }
}

/**
 * The workspace's shared `ui/` folder for an app: `ui/` next to the nearest
 * wmill.yaml above `appDir`, the root `wmill app push` and `wmill app bundle`
 * run from. Undefined when no wmill.yaml is found.
 */
export function findSharedUiDir(appDir: string): string | undefined {
  let dir = path.resolve(appDir);
  while (true) {
    if (fs.existsSync(path.join(dir, "wmill.yaml"))) {
      return path.join(dir, "ui");
    }
    const parent = path.dirname(dir);
    if (parent === dir) return undefined;
    dir = parent;
  }
}

/**
 * Resolves `/ui/<file>` imports to the workspace's shared `ui/` folder, and bare
 * package imports made from those files against the app's own node_modules: the
 * server-side bundler places `ui/` inside the app, so walking up from the shared
 * folder instead would miss the app's packages or load a second copy of them
 * (two Reacts break hooks at runtime).
 */
export function createSharedUiPlugin(sharedUiDir: string, appDir: string): any {
  // Real paths throughout: esbuild follows symlinks for relative imports, so a
  // symlinked ui/ would otherwise give its files two different prefixes.
  const uiPrefix = fs.realpathSync(sharedUiDir) + path.sep;
  const resolveDir = path.resolve(appDir);
  return {
    name: "wmill-shared-ui",
    setup(build: any) {
      build.onResolve({ filter: /^\/ui\// }, (args: any) => {
        const rel = args.path.slice("/ui/".length);
        const candidates = [rel];
        if (!path.extname(rel)) {
          candidates.push(
            rel + ".tsx",
            rel + ".ts",
            rel + ".jsx",
            rel + ".js",
            rel + ".css",
            path.join(rel, "index.tsx"),
            path.join(rel, "index.ts"),
          );
        }
        for (const c of candidates) {
          const full = path.join(sharedUiDir, c);
          if (fs.existsSync(full)) {
            return { path: fs.realpathSync(full) };
          }
        }
        return {
          errors: [
            {
              text: `Could not resolve shared UI import "${args.path}" in ${sharedUiDir}`,
            },
          ],
        };
      });
      build.onResolve({ filter: /^[^./]/ }, async (args: any) => {
        if (!args.importer.startsWith(uiPrefix)) return undefined;
        // CSS reads `@import "base.css"` as relative first; keep that when the
        // sibling resolves, and only send package names to the app.
        if (args.kind === "import-rule" || args.kind === "url-token") {
          const sibling = await build.resolve("./" + args.path, {
            resolveDir: args.resolveDir,
            kind: args.kind,
          });
          if (sibling.errors.length === 0) return sibling;
        }
        // No importer is passed, so this nested resolve does not re-enter here.
        return build.resolve(args.path, { resolveDir, kind: args.kind });
      });
    },
  };
}

/**
 * Creates an esbuild bundle for the app
 * @param options Bundle configuration options
 * @returns Bundle result containing JS and CSS blobs
 */
export async function createBundle(
  options: BundleOptions = {}
): Promise<BundleResult> {
  // Detect frameworks to determine default entry point.
  // Use the entryPoint's directory if provided, otherwise fall back to cwd.
  const appDir = options.entryPoint ? path.dirname(options.entryPoint) : process.cwd();
  const frameworks = detectFrameworks(appDir);
  const defaultEntry = frameworks.svelte ? "index.ts" : "index.tsx";

  const entryPoint = options.entryPoint ?? defaultEntry;
  const outDir = options.outDir ?? "dist";
  const sourcemap = options.sourcemap ?? false;
  const minify = options.minify ?? true;
  const production = options.production ?? true;

  // Verify entry point exists
  if (!fs.existsSync(entryPoint)) {
    throw new Error(
      `Entry point "${entryPoint}" not found. Please ensure the file exists.`
    );
  }

  // Ensure node_modules exists in the app directory
  await ensureNodeModules(appDir);

  // Load framework-specific plugins (svelte) based on package.json
  const frameworkPlugins = await createFrameworkPlugins(appDir, options.sharedUiDir);

  // Ensure output directory exists
  const distDir = path.join(process.cwd(), outDir);
  if (!fs.existsSync(distDir)) {
    fs.mkdirSync(distDir, { recursive: true });
  }

  const outfile = path.join(outDir, "bundle.js");

  // log.info("FOO")
  // log.info("wmillTs" + JSON.stringify(wmillTs));
  // Plugin to provide /wmill.ts as a virtual module
  const wmillTs = (windmillUtils.wmillTsRaw as any).default ?? windmillUtils.wmillTsRaw;

  const wmillPlugin = {
    name: "wmill-virtual",
    setup(build: any) {
      // Intercept imports of wmill with various path formats:
      // - wmill, wmill.ts (bare import)
      // - /wmill, /wmill.ts (absolute)
      // - ./wmill, ./wmill.ts (same directory)
      // - ../wmill, ../../wmill, etc. (parent directories)
      build.onResolve(
        { filter: /^(\.\.\/)+wmill(\.ts)?$|^(\.\/|\/)?wmill(\.ts)?$/ },
        (args: any) => {
          log.info(colors.yellow(`[wmill-virtual] Intercepted: ${args.path}`));
          return {
            path: args.path,
            namespace: "wmill-virtual",
          };
        }
      );

      // Provide the virtual module content
      build.onLoad({ filter: /.*/, namespace: "wmill-virtual" }, (args: any) => {
        log.info(colors.yellow(`[wmill-virtual] Loading virtual module: ${args.path}`));
        return {
          contents: wmillTs,
          loader: "ts",
        };
      });
    },
  };

  const sharedUiPlugins: any[] =
    options.sharedUiDir && fs.existsSync(options.sharedUiDir)
      ? [createSharedUiPlugin(options.sharedUiDir, appDir)]
      : [];

  const buildOptions = {
    ...DEFAULT_BUILD_OPTIONS,
    conditions: conditionsFor(frameworks.svelte),
    entryPoints: [entryPoint],
    outfile,
    sourcemap,
    minify,
    // Keep outputs in memory: esbuild-wasm cannot write to the filesystem
    // ("write" option unavailable), and the dist files were discarded after the
    // read anyway. Native esbuild supports write:false + outputFiles too.
    write: false as const,
    define: {
      "process.env.NODE_ENV": production ? '"production"' : '"development"',
    },
    plugins: [...frameworkPlugins, wmillPlugin, ...sharedUiPlugins],
  };

  log.info(colors.blue("📦 Building bundle..."));

  // Native esbuild with a transparent esbuild-wasm fallback on host/binary
  // version mismatch (see esbuild_loader.ts).
  const result = await withEsbuild((esbuild) => esbuild.build(buildOptions));

  if (result.errors.length > 0) {
    log.error(colors.red("❌ Build failed:"));
    result.errors.forEach((error: any) => {
      log.error(colors.red(error.text));
    });
    throw new Error("Build failed with errors");
  }

  log.info(colors.green("✅ Bundle created successfully"));

  const outputFiles = result.outputFiles ?? [];
  const jsFile = outputFiles.find((f) => f.path.endsWith(".js"));
  const cssFile = outputFiles.find((f) => f.path.endsWith(".css"));

  if (!jsFile) {
    throw new Error("Expected a JS bundle in esbuild output but none found");
  }

  try {
    fs.rmSync(distDir, { recursive: true });
  } catch {
    //ignore
  }
  return { js: jsFile.text, css: cssFile?.text ?? "" };
}

/**
 * Gets the esbuild build options for use in watch mode (dev server)
 * @param entryPoint Entry point file
 * @param svelte Whether the app is a Svelte app (enables the "svelte" condition)
 * @returns esbuild build options
 */
export function getDevBuildOptions(entryPoint: string = "index.tsx", svelte = false) {
  return {
    ...DEFAULT_BUILD_OPTIONS,
    conditions: conditionsFor(svelte),
    entryPoints: [entryPoint],
    outfile: "dist/bundle.js",
    sourcemap: true,
    define: {
      "process.env.NODE_ENV": '"development"',
    },
  };
}
