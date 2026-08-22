import { dirname } from "node:path";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";

const jobDir = "JOB_DIR";
const nodeModulesDir = jobDir + "/node_modules";
const cjsShimDir = jobDir + "/.wm_node_cjs";

let installedPackages = [];
try {
  installedPackages = readdirSync(nodeModulesDir);
} catch (e) {}

// The shim reaches the package by require() and the classifier by bun's resolution, which agree
// only where no condition can steer them apart. Which conditions are live is unknowable here (bun
// applies its own, and NODE_OPTIONS `--conditions` adds any name at runtime), so any condition at
// all disqualifies the package; subpaths and `default` select one entry whatever is live.
export function entryDependsOnCondition(exports) {
  if (Array.isArray(exports)) {
    return exports.some(entryDependsOnCondition);
  }
  if (exports && typeof exports === "object") {
    return Object.keys(exports).some(
      (key) =>
        (key !== "default" && !key.startsWith(".")) ||
        entryDependsOnCondition(exports[key])
    );
  }
  return false;
}

const shimmable = new Map();
function isShimmableCjs(specifier) {
  if (!shimmable.has(specifier)) {
    shimmable.set(specifier, classifyAsCjs(specifier));
  }
  return shimmable.get(specifier);
}

function classifyAsCjs(specifier) {
  const segments = specifier.split("/");
  const pkg = specifier.startsWith("@")
    ? segments.slice(0, 2).join("/")
    : segments[0];
  try {
    const manifest = JSON.parse(
      readFileSync(nodeModulesDir + "/" + pkg + "/package.json", "utf8")
    );
    if (manifest.bun !== undefined || entryDependsOnCondition(manifest.exports)) {
      return false;
    }
    const file = Bun.resolveSync(specifier, jobDir);
    if (file.endsWith(".cjs") || file.endsWith(".node")) {
      return true;
    }
    if (file.endsWith(".js")) {
      // Same nearest-package.json walk node does to decide how to load a bare .js
      for (let dir = dirname(file); dir !== dirname(dir); dir = dirname(dir)) {
        try {
          return (
            JSON.parse(readFileSync(dir + "/package.json", "utf8")).type !==
            "module"
          );
        } catch (e) {}
      }
    }
    return false;
  } catch (e) {
    console.log(
      "could not inspect '" +
        specifier +
        "' to pick its module format, leaving it external: " +
        e
    );
    return false;
  }
}

const cjsShims = new Map();
function cjsShim(specifier) {
  if (!cjsShims.has(specifier)) {
    // Truncated so that a deep subpath cannot flatten into a basename over the filesystem's
    // limit; the hash is what keeps two specifiers from sharing a shim.
    const shim =
      cjsShimDir +
      "/" +
      specifier.replace(/[^a-zA-Z0-9]/g, "_").slice(0, 64) +
      "_" +
      Bun.hash(specifier).toString(36) +
      ".cjs";
    mkdirSync(cjsShimDir, { recursive: true });
    // The local binding is load-bearing: bun collapses a bare `module.exports = require(x)` back
    // into a passthrough external import, which is the shape that breaks node.
    writeFileSync(
      shim,
      "const mod = require(" + JSON.stringify(specifier) + ");\nmodule.exports = mod;\n"
    );
    cjsShims.set(specifier, shim);
  }
  return cjsShims.get(specifier);
}

// Node only sees the named exports of a CommonJS dependency that cjs-module-lexer finds
// statically, which fails on lodash and the like, so a plain external breaks `import { x }` and
// `import * as x` from it. Routing it through a CommonJS shim makes bun synthesize the interop.
// Only proven-CommonJS packages get one: requiring an ESM entry throws without require(esm).
export const nodeExternals = {
  name: "windmill-node-externals",
  setup(build) {
    build.onResolve({ filter: /^[^./]/ }, (args) => {
      if (args.importer.replace(/\\/g, "/").includes("/.wm_node_cjs/")) {
        return { path: args.path, external: true };
      }
      if (!installedPackages.includes(args.path.split("/")[0])) {
        return undefined;
      }
      if (!isShimmableCjs(args.path)) {
        return { path: args.path, external: true };
      }
      return { path: cjsShim(args.path) };
    });
  },
};
