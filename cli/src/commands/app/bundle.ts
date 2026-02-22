import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";
import * as log from "../../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import * as windmillUtils from "@windmill-labs/shared-utils";
export interface BundleOptions {
  entryPoint?: string;
  outDir?: string;
  sourcemap?: boolean;
  minify?: boolean;
  production?: boolean;
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
  logLevel: "info" as const,
  write: true,
};

/**
 * Detects which frontend frameworks are present in package.json
 */
export function detectFrameworks(appDir: string): { svelte: boolean; vue: boolean } {
  const packageJsonPath = path.join(appDir, "package.json");
  if (!fs.existsSync(packageJsonPath)) {
    return { svelte: false, vue: false };
  }

  try {
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf-8"));
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

/**
 * Creates a Svelte esbuild plugin
 * Uses the svelte compiler from the project's node_modules
 */
function createSveltePlugin(appDir: string): any {
  return {
    name: "svelte",
    setup(build: any) {
      build.onLoad({ filter: /\.svelte$/ }, async (args: any) => {
        // Import svelte compiler from the project's node_modules
        const svelte = await import("svelte/compiler");

        // Load the file from the file system
        const source = await fs.promises.readFile(args.path, "utf8");
        const filename = path.relative(process.cwd(), args.path);

        // This converts a message in Svelte's format to esbuild's format
        const convertMessage = ({ message, start, end }: any) => {
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

        // Convert Svelte syntax to JavaScript
        try {
          const { js, warnings } = svelte.compile(source, { filename });
          const contents = js.code + `//# sourceMappingURL=` + js.map.toUrl();
          return { contents, warnings: warnings.map(convertMessage) };
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
  // Dynamically import esbuild
  const esbuild = await import("esbuild");

  // Detect frameworks to determine default entry point
  const frameworks = detectFrameworks(process.cwd());
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
  const appDir = path.dirname(entryPoint) || process.cwd();
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

  const buildOptions = {
    ...DEFAULT_BUILD_OPTIONS,
    entryPoints: [entryPoint],
    outfile,
    sourcemap,
    minify,
    define: {
      "process.env.NODE_ENV": production ? '"production"' : '"development"',
    },
    plugins: [...frameworkPlugins, wmillPlugin],
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

    // Read the generated files
    const jsPath = path.join(process.cwd(), outfile);
    const cssPath = path.join(process.cwd(), outDir, "bundle.css");

    if (!fs.existsSync(jsPath)) {
      throw new Error(`Expected JS bundle at ${jsPath} but file not found`);
    }

    const jsContent = fs.readFileSync(jsPath, "utf-8");
    const cssContent = fs.existsSync(cssPath)
      ? fs.readFileSync(cssPath, "utf-8")
      : "";

    try {
      fs.rmSync(distDir, { recursive: true });
    } catch {
      //ignore
    }
    return { js: jsContent, css: cssContent };
    
  } finally {
    // Stop esbuild
    await esbuild.stop();
  }
}

/**
 * Gets the esbuild build options for use in watch mode (dev server)
 * @param entryPoint Entry point file
 * @returns esbuild build options
 */
export function getDevBuildOptions(entryPoint: string = "index.tsx") {
  return {
    ...DEFAULT_BUILD_OPTIONS,
    entryPoints: [entryPoint],
    outfile: "dist/bundle.js",
    sourcemap: true,
    define: {
      "process.env.NODE_ENV": '"development"',
    },
  };
}
