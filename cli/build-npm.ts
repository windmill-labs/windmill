import { VERSION } from "./src/main.ts";
import { mkdirSync, cpSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";

const outDir = "./npm";

// Clean output directory
rmSync(outDir, { recursive: true, force: true });

// Build with bun
console.log("Bundling with bun build...");
const buildResult = Bun.spawnSync([
  "bun", "build", "src/main.ts",
  "--outdir", join(outDir, "esm"),
  "--target", "node",
  "--format", "esm",
  "--packages", "external",
], { cwd: import.meta.dir, stdout: "inherit", stderr: "inherit" });

if (buildResult.exitCode !== 0) {
  console.error("Build failed");
  process.exit(1);
}

// Add shebang to main.js
const mainJsPath = join(outDir, "esm", "main.js");
const mainJs = readFileSync(mainJsPath, "utf-8");
writeFileSync(mainJsPath, "#!/usr/bin/env node\n" + mainJs, "utf-8");

// Copy WASM files
const wasmDirs = [
  "nu", "ts", "regex", "py", "go", "php", "rust", "yaml",
  "csharp", "java", "ruby",
  // for related places search: ADD_NEW_LANG
];

for (const lang of wasmDirs) {
  const destDir = join(outDir, "esm", "wasm", lang);
  mkdirSync(destDir, { recursive: true });
  cpSync(
    join("wasm", lang, "windmill_parser_wasm_bg.wasm"),
    join(destDir, "windmill_parser_wasm_bg.wasm")
  );
  cpSync(
    join("wasm", lang, "windmill_parser_wasm.js"),
    join(destDir, "windmill_parser_wasm.js")
  );
}

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
    ws: "8.18.0",
    esbuild: "^0.24.2",
    "get-port": "^7.1.0",
    open: "^10.0.0",
    "es-main": "^1.3.0",
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
