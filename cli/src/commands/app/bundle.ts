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
import { getEsbuild, stopEsbuild } from "../../utils/esbuild_loader.ts";
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
export function detectFrameworks(appDir: string): { svelte: boolean; vue: boolean } {
  const packageJsonPath = path.join(appDir, "package.json");
  if (!fs.existsSync(packageJsonPath)) {
    return { svelte: false, vue: false };
  }

  try {
    const packageJson = JSON.parse(readTextFileSync(packageJsonPath));
    const allDeps = {
      ...packageJson.dependencies,
      ...packageJson.devDependencies,
    };

    return {
      svelte: "svelte" in allDeps,
      vue: "vue" in allDeps,
    };
  } catch {
    return { svelte: false, vue: false };
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

/**
 * Creates framework-specific esbuild plugins based on detected dependencies
 */
export async function createFrameworkPlugins(appDir: string): Promise<any[]> {
  const frameworks = detectFrameworks(appDir);
  const plugins: any[] = [];

  if (frameworks.svelte) {
    log.info(colors.blue("🔧 Svelte detected, adding svelte plugin..."));
    plugins.push(createSveltePlugin(appDir));
  }

  if (frameworks.vue) {
    log.info(colors.blue("🔧 Vue detected, adding vue plugin..."));
    throw new Error("Vue plugin not supported yet");
    // try {
    //   const esbuildPluginVue = await import("esbuild-plugin-vue3");
    //   plugins.push(esbuildPluginVue.default());
    // } catch (error: any) {
    //   log.warn(colors.yellow(`Failed to load vue plugin: ${error.message}`));
    // }
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
 * Creates an esbuild bundle for the app
 * @param options Bundle configuration options
 * @returns Bundle result containing JS and CSS blobs
 */
export async function createBundle(
  options: BundleOptions = {}
): Promise<BundleResult> {
  // Native esbuild with a transparent esbuild-wasm fallback on host/binary
  // version mismatch (see esbuild_loader.ts).
  const esbuild = await getEsbuild();

  // Detect frameworks to determine default entry point.
  // Use the entryPoint's directory if provided, otherwise fall back to cwd.
  const appDir = options.entryPoint ? path.dirname(options.entryPoint) : process.cwd();
  const frameworks = detectFrameworks(appDir);
  const defaultEntry = (frameworks.svelte || frameworks.vue) ? "index.ts" : "index.tsx";

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

  // Load framework-specific plugins (svelte, vue) based on package.json
  const frameworkPlugins = await createFrameworkPlugins(appDir);

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

  const sharedUiPlugins: any[] = [];
  if (options.sharedUiDir && fs.existsSync(options.sharedUiDir)) {
    const sharedUiDir = options.sharedUiDir;
    sharedUiPlugins.push({
      name: "wmill-shared-ui",
      setup(build: any) {
        // Intercept imports of /ui/<file> and resolve to the workspace ui/ folder.
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
              return { path: full };
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
      },
    });
  }

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

  try {
    const result = await esbuild.build(buildOptions);

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

  } finally {
    // Stop the native esbuild service so the process can exit (no-op for wasm).
    await stopEsbuild();
  }
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
