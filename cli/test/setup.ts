/**
 * Global test setup — preloaded before all test files.
 *
 * When UNIT_ONLY=1, skips all backend setup (cargo build, database, etc.)
 * so that unit tests can run instantly without any external dependencies.
 *
 * Otherwise:
 * 1. Builds the backend binary so `cargo run` starts instantly.
 * 2. Starts a shared backend instance so integration tests don't
 *    bear the startup cost inside their per-test timeout window.
 */

if (process.env["UNIT_ONLY"]) {
  // Nothing to do — unit tests don't need backend setup
} else {

const { resolve } = await import("node:path");
const { fileURLToPath } = await import("node:url");
const { statSync } = await import("node:fs");

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

// Start the shared backend instance so it's ready before any test runs.
// This avoids the first integration test timing out while the backend
// creates its database, starts the process, and waits for the health check.
if (process.env["DATABASE_URL"]) {
  // Clean up any stale databases/processes from previous crashed test runs
  const { cleanupStaleTestResources } = await import("./cargo_backend.ts");
  await cleanupStaleTestResources();

  const { getTestBackend, cleanupTestBackend } = await import("./test_backend.ts");
  console.log("Pre-starting test backend...");
  await getTestBackend();
  console.log("Test backend is ready for all tests.");

  // Register afterAll to do full async cleanup (kill processes + drop DB)
  // when all tests complete. The synchronous "exit" handler alone can't
  // drop databases or scan /proc for orphaned child processes.
  const { afterAll } = await import("bun:test");
  afterAll(async () => {
    await cleanupTestBackend();
  });
}

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

}
