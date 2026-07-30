// `wmill lint` imports windmill-yaml-validator from source, and its schemas are generated
// from this checkout's OpenAPI specs. Regenerating them on every install is what keeps lint
// from validating against a schema older than the code being linted.

import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import path from "node:path";

const validatorDir = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "windmill-yaml-validator"
);

function npm(args) {
  const result = spawnSync("npm", args, {
    cwd: validatorDir,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

const require = createRequire(path.join(validatorDir, "package.json"));
const hasDeps = ["js-yaml", "ajv", "@stoplight/yaml"].every((pkg) => {
  try {
    require.resolve(pkg);
    return true;
  } catch {
    return false;
  }
});

// --omit=dev keeps the validator's test toolchain off the CLI's install path; skipping the
// install entirely when the deps already resolve keeps it from pruning a full install made
// by someone working on the validator itself.
if (!hasDeps) {
  npm(["install", "--omit=dev", "--no-audit", "--no-fund", "--loglevel=error"]);
}

npm(["run", "gen"]);
