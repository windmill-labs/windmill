import * as esbuild from "esbuild";


esbuild
  .build({
    entryPoints: ["src/web/test/suite/index.ts"],
    outdir: "dist/web/test/suite/",
    bundle: true,
    format: "cjs",
    minify:false,
    external: ["vscode"],
    sourcemap:"inline",
    platform: "browser",
    treeShaking: false,
    sourcesContent: true,
    // plugins: [moduleShimmer],
    // watch: {
    //   onRebuild(error, result) {
    //     if (error) console.error("watch build test failed:", JSON.stringify(error));
    //     else console.log("watch build test succeeded:", JSON.stringify(result));
    //   },
    // },
  })

