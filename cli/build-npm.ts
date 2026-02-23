import { VERSION } from "./src/main.ts";
import { readFileSync, writeFileSync, rmSync, cpSync } from "node:fs";
import { join } from "node:path";

const outDir = "./npm";

// Parser npm packages — used as externals and added to generated package.json
const parserPackages = [
  "windmill-parser-wasm-py", "windmill-parser-wasm-ts",
  "windmill-parser-wasm-regex", "windmill-parser-wasm-go",
  "windmill-parser-wasm-php", "windmill-parser-wasm-rust",
  "windmill-parser-wasm-yaml", "windmill-parser-wasm-csharp",
  "windmill-parser-wasm-nu", "windmill-parser-wasm-java",
  "windmill-parser-wasm-ruby",
];
const parserExternals = parserPackages.flatMap(p => ["--external", p]);

// Clean output directory
rmSync(outDir, { recursive: true, force: true });

// Build with bun — bundle everything except esbuild (platform-specific binary),
// svelte (optional, only needed for `wmill app bundle/dev`), and parser packages
// (loaded at runtime via init() with readFileSync for the .wasm binary).
console.log("Bundling with bun build...");
const buildResult = Bun.spawnSync([
  "bun", "build", "src/main.ts",
  "--outdir", join(outDir, "esm"),
  "--target", "node",
  "--format", "esm",
  "--external", "esbuild",
  "--external", "svelte",
  "--external", "svelte/compiler",
  ...parserExternals,
], { cwd: import.meta.dir, stdout: "inherit", stderr: "inherit" });

if (buildResult.exitCode !== 0) {
  console.error("Build failed");
  process.exit(1);
}

// Add shebang to main.js
const mainJsPath = join(outDir, "esm", "main.js");
const mainJs = readFileSync(mainJsPath, "utf-8");
writeFileSync(mainJsPath, "#!/usr/bin/env node\n" + mainJs, "utf-8");

// Copy LICENSE and README
cpSync("../LICENSE", join(outDir, "LICENSE"));
cpSync("README.md", join(outDir, "README.md"));

// Generate package.json
const packageJson = {
  name: "windmill-cli",
  version: VERSION,
  description: "CLI for Windmill",
  license: "Apache 2.0",
  type: "module",
  main: "esm/main.js",
  bin: {
    wmill: "esm/main.js",
  },
  repository: {
    type: "git",
    url: "git+https://github.com/windmill-labs/windmill.git",
  },
  bugs: {
    url: "https://github.com/windmill-labs/windmill/issues",
  },
  dependencies: {
    esbuild: "^0.24.2",
    ...Object.fromEntries(parserPackages.map(p => [p, "*"])),
  },
  optionalDependencies: {
    svelte: "^5.0.0",
  },
};

writeFileSync(
  join(outDir, "package.json"),
  JSON.stringify(packageJson, null, 2) + "\n",
  "utf-8"
);

console.log(`Built npm package v${VERSION} to ${outDir}/`);
