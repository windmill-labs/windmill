import { Command } from "@cliffy/command";
import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import * as log from "../../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import { createBundle, detectFrameworks } from "./bundle.ts";

interface BundleOptions {
  out?: string;
  minify?: boolean;
}

/** Every entry point a raw app may have, most specific first, with the
 * framework's preferred one promoted — Svelte and Vue mount from a `.ts`. */
function entryPointFor(appDir: string): string {
  const frameworks = detectFrameworks(appDir);
  const candidates = frameworks.svelte || frameworks.vue
    ? ["index.ts", "index.tsx", "index.js"]
    : ["index.tsx", "index.ts", "index.js"];
  const entry = candidates.find((c) => fs.existsSync(path.join(appDir, c)));
  if (!entry) {
    throw new Error(
      `No entry point in ${appDir}: expected one of ${candidates.join(", ")}`,
    );
  }
  return path.join(appDir, entry);
}

/**
 * Bundle a raw app folder without deploying it. `wmill app push` bundles as part
 * of deploying; this exposes the same build on its own, so anything that needs a
 * raw app's js/css — the server-side bundler behind `/apps/update_raw_source`,
 * most of all — runs this exact build rather than reimplementing it.
 *
 * The result goes to files, not stdout: the build logs to stdout as it runs, so
 * a caller can't read the bundle off the pipe.
 */
async function bundleApp(opts: BundleOptions, appFolder?: string) {
  const appDir = path.resolve(appFolder ?? process.cwd());

  const { js, css } = await createBundle({
    entryPoint: entryPointFor(appDir),
    production: true,
    minify: opts.minify ?? true,
    // Same as `wmill app push`: the workspace's shared UI is `ui/` under the
    // directory the command runs from, not under the app.
    sharedUiDir: path.join(process.cwd(), "ui"),
    // createBundle removes its own outDir when it is done, and it resolves the
    // path against cwd — so name one that is ours to delete rather than let it
    // default to `dist`, which is very likely the caller's.
    outDir: `.wmill-bundle-${process.pid}`,
  });

  const outDir = path.resolve(opts.out ?? path.join(appDir, "dist"));
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(path.join(outDir, "bundle.js"), js);
  fs.writeFileSync(path.join(outDir, "bundle.css"), css);
  log.info(colors.green(`Wrote bundle.js and bundle.css to ${outDir}`));
}

const command = new Command()
  .description("Bundle a raw app folder to js/css without deploying it")
  .arguments("[app_folder:string]")
  .option(
    "--out <dir:string>",
    "Directory to write bundle.js and bundle.css into (default: <app_folder>/dist)",
  )
  .option("--no-minify", "Skip minification")
  .action(bundleApp as any);

export default command;
