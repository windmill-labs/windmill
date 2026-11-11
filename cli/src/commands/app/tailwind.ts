import * as fs from "node:fs";
import * as path from "node:path";
import type { Plugin } from "esbuild";

/** Compile before esbuild lowers CSS syntax or separates imported theme definitions. */
export function createTailwindPlugin(appDir: string, sharedUiDir?: string): Plugin {
  return {
    name: "wmill-tailwind",
    setup(build) {
      build.onLoad({ filter: /\.css$/ }, async (args) => {
        const source = await fs.promises.readFile(args.path, "utf8");
        // Follow imports too: the v4 entry point can live in a shared stylesheet.
        if (!/@(?:import|theme|utility|reference)\b/.test(source)) return;

        const { compile, Features } = await import("@tailwindcss/node");
        const dependencies = new Set<string>([args.path]);
        const options = {
          base: path.dirname(args.path),
          from: args.path,
          shouldRewriteUrls: true,
          onDependency: (file: string) => { dependencies.add(file); },
          customCssResolver: async (id: string, base: string) => {
            const resolved = await build.resolve(id, {
              kind: "import-rule",
              resolveDir: base,
              importer: path.join(base, "__tailwind.css"),
            });
            return resolved.errors.length === 0 && !resolved.external
              ? resolved.path
              : undefined;
          },
        };
        let compiler = await compile(source, options);
        // Imports alone aren't Tailwind; leave ordinary CSS to esbuild.
        if ((compiler.features & ~Features.AtImport) === Features.None) return;
        if (!(compiler.features & Features.Utilities)) {
          compiler = await compile(source + "\n@tailwind utilities;", options);
        }

        const { Scanner } = await import("@tailwindcss/oxide");
        const sources = [
          ...(compiler.root === "none"
            ? []
            : [{ ...(compiler.root ?? { base: path.resolve(appDir), pattern: "**/*" }), negated: false }]),
          ...(compiler.root !== "none" && sharedUiDir && fs.existsSync(sharedUiDir)
            ? [{ base: fs.realpathSync(sharedUiDir), pattern: "**/*", negated: false }]
            : []),
          ...compiler.sources,
        ];
        const scanner = new Scanner({ sources });
        const contents = compiler.build(scanner.scan());
        // Include directories so adding a file also invalidates the CSS in watch mode.
        const watchDirs = new Set(scanner.globs.map((glob) => glob.base));
        for (const file of scanner.files) {
          dependencies.add(file);
          watchDirs.add(path.dirname(file));
        }
        return {
          contents,
          loader: "css",
          resolveDir: path.dirname(args.path),
          watchFiles: [...dependencies],
          watchDirs: [...watchDirs],
        };
      });
    },
  };
}
