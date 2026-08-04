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
  const frameworks = detectFrameworks(appDir);
  const entryFile = frameworks.svelte || frameworks.vue
    ? "index.ts"
    : "index.tsx";

  const { js, css } = await createBundle({
    entryPoint: path.join(appDir, entryFile),
    production: true,
    minify: opts.minify ?? true,
    // Matches `wmill app push`: shared UI lives in `ui/` next to the app.
    sharedUiDir: path.join(appDir, "ui"),
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
