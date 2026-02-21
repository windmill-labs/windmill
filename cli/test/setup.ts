/**
 * Global test setup — preloaded before all test files.
 *
 * Ensures the backend binary is compiled so individual tests
 * don't time out waiting for cargo build. Each test file
 * starts its own backend instance with its own database.
 */

import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { statSync } from "node:fs";

const __dirname = resolve(fileURLToPath(import.meta.url), "..");

function findBackendDir(): string {
  const candidates = [
    resolve(__dirname, "..", "..", "backend"),
    resolve(__dirname, "..", "..", "..", "backend"),
    resolve(".", "backend"),
    resolve("..", "backend"),
  ];

  for (const candidate of candidates) {
    try {
      const cargoPath = resolve(candidate, "Cargo.toml");
      const stat = statSync(cargoPath);
      if (stat.isFile()) {
        return candidate;
      }
    } catch {
      // Continue searching
    }
  }

  throw new Error("Could not find backend directory.");
}

// Build the backend binary so `cargo run` is fast for all tests
const backendDir = findBackendDir();

const isCI = process.env["CI_MINIMAL_FEATURES"] === "true";
const hasLicenseKey = !!process.env["EE_LICENSE_KEY"];
const features = isCI
  ? ["zip"]
  : hasLicenseKey
    ? ["zip", "private", "enterprise", "license"]
    : ["zip"];

const cargoArgs = ["build", "--features", features.join(",")];
console.log(`Pre-building backend: cargo ${cargoArgs.join(" ")}`);

const proc = Bun.spawn(["cargo", ...cargoArgs], {
  cwd: backendDir,
  stdout: "inherit",
  stderr: "inherit",
  env: {
    ...process.env as Record<string, string>,
    SQLX_OFFLINE: "true",
  },
});

const exitCode = await proc.exited;
if (exitCode !== 0) {
  throw new Error(`cargo build failed with exit code ${exitCode}`);
}
console.log("Backend build complete.");

// When TEST_CLI_RUNTIME=node, also build the npm package so tests
// can invoke `node npm/esm/main.js` instead of `bun run src/main.ts`
if (process.env["TEST_CLI_RUNTIME"] === "node") {
  const cliDir = resolve(__dirname, "..");
  console.log("Building npm package for Node runtime testing...");
  const npmBuild = Bun.spawn(["bun", "run", "build-npm.ts"], {
    cwd: cliDir,
    stdout: "inherit",
    stderr: "inherit",
    env: process.env as Record<string, string>,
  });
  const npmExit = await npmBuild.exited;
  if (npmExit !== 0) {
    throw new Error(`npm build failed with exit code ${npmExit}`);
  }
  console.log("npm package built — tests will use Node runtime.");
}
