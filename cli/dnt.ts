// ex. scripts/build_npm.ts
import { build, emptyDir } from "jsr:@deno/dnt@0.41.3";

await emptyDir("./npm");

await build({
  entryPoints: [
    {
      kind: "bin",
      name: "wmill", // command name
      path: "./main.ts",
    },
  ],
  outDir: "./npm",
  shims: {
    // see JS docs for overview and more options
    deno: true,
  },
  typeCheck: false,
  scriptModule: false,
  package: {
    // package.json properties
    name: "windmill-cli",
    version: Deno.args[0],
    description: "CLI for Windmill",
    license: "Apache 2.0",
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

    Deno.copyFileSync("../LICENSE", "npm/LICENSE");
    Deno.copyFileSync("README.md", "npm/README.md");
    Deno.copyFileSync(
      "wasm/windmill_parser_wasm_bg.wasm",
      "npm/esm/wasm/windmill_parser_wasm_bg.wasm"
    );
  },
});
