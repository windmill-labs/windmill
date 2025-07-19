// ex. scripts/build_npm.ts
import { build, emptyDir } from "jsr:@deno/dnt@0.41.3";
import { VERSION } from "./main.ts";
await emptyDir("./npm");

await build({
  entryPoints: [
    "main.ts",
    {
      kind: "bin",
      name: "wmill", // command name
      path: "./main.ts",
    },
  ],
  outDir: "./npm",
  test: false, // Disable all tests in npm build since they use Deno-specific APIs
  shims: {
    // see JS docs for overview and more options
    deno: true,
    // shims to only use in the tests
    customDev: [{
      // this is what `timers: "dev"` does internally
      package: {
        name: "@deno/shim-timers",
        version: "~0.1.0",
      },
      globalNames: ["setTimeout", "setInterval"],
    }],
  },
  scriptModule: false,
  filterDiagnostic(diagnostic) {
    if (
      diagnostic.file?.fileName.includes("node_modules/") ||
      diagnostic.file?.fileName.includes("src/deps/") ||
      diagnostic.file?.fileName.includes("src/deps.ts") ||
      diagnostic.file?.fileName.includes("src/utils.ts")
    ) {
      return false; // ignore all diagnostics in this file
    }
    // console.log(diagnostic.file?.fileName);
    return true;
  },
  declaration: "separate",
  package: {
    // package.json properties
    name: "windmill-cli",
    version: VERSION,
    description: "CLI for Windmill",
    license: "Apache 2.0",
    main: "esm/main.js",
    repository: {
      type: "git",
      url: "git+https://github.com/windmill-labs/windmill.git",
    },
    bugs: {
      url: "https://github.com/windmill-labs/windmill/issues",
    },
  },

  postBuild() {
    // steps to run after building and before running the tests
    // add shebang to npm/esm/main.js
    const dirs = [
      "nu",
      "ts",
      "regex",
      "py",
      "go",
      "php",
      "rust",
      "yaml",
      "csharp",
      "java",
    ];

    for (const l of dirs) {
      Deno.copyFileSync(
        "wasm/" + l + "/windmill_parser_wasm_bg.wasm",
        "npm/esm/wasm/" + l + "/windmill_parser_wasm_bg.wasm"
      );
    }
    Deno.copyFileSync("../LICENSE", "npm/LICENSE");
    Deno.copyFileSync("README.md", "npm/README.md");
  },
});
