// deno-lint-ignore-file no-explicit-any
import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { log, colors } from "../../../deps.ts";

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
 * Creates an esbuild bundle for the app
 * @param options Bundle configuration options
 * @returns Bundle result containing JS and CSS blobs
 */
export async function createBundle(
  options: BundleOptions = {}
): Promise<BundleResult> {
  // Dynamically import esbuild
  const esbuild = await import("npm:esbuild@0.24.2");

  const entryPoint = options.entryPoint ?? "index.tsx";
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

  // Ensure output directory exists
  const distDir = path.join(process.cwd(), outDir);
  if (!fs.existsSync(distDir)) {
    fs.mkdirSync(distDir, { recursive: true });
  }

  const outfile = path.join(outDir, "bundle.js");

  const buildOptions = {
    ...DEFAULT_BUILD_OPTIONS,
    entryPoints: [entryPoint],
    outfile,
    sourcemap,
    minify,
    define: {
      "process.env.NODE_ENV": production ? '"production"' : '"development"',
    },
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
