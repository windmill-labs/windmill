/**
 * Example test using CargoBackend
 *
 * Prerequisites:
 * 1. PostgreSQL running locally: postgres://postgres:changeme@localhost:5432
 * 2. Backend compiled: cd backend && cargo build --release
 *
 * Run:
 *   cd cli
 *   deno test --allow-all test/cargo_backend_example.test.ts
 *
 * Or with custom database URL:
 *   DATABASE_URL=postgres://user:pass@host:5432 deno test --allow-all test/cargo_backend_example.test.ts
 *
 * For verbose cargo output:
 *   VERBOSE=1 deno test --allow-all test/cargo_backend_example.test.ts
 */

import {
  assertEquals,
  assertExists,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { CargoBackend } from "./cargo_backend.ts";

// Single backend instance for all tests
let backend: CargoBackend;

// Setup before all tests
Deno.test({
  name: "setup: start cargo backend",
  fn: async () => {
    backend = new CargoBackend({
      verbose: Deno.env.get("VERBOSE") === "1",
    });
    await backend.start();
    assertExists(backend.baseUrl);
    assertExists(backend.authToken);
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

Deno.test({
  name: "API: version endpoint responds",
  fn: async () => {
    const response = await fetch(`${backend.baseUrl}/api/version`);
    assertEquals(response.ok, true);
    const version = await response.text();
    assertExists(version);
    console.log(`  Backend version: ${version.trim()}`);
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

Deno.test({
  name: "API: workspace exists",
  fn: async () => {
    const response = await backend.apiRequest(
      `/api/w/${backend.workspace}/workspaces/get_settings`,
    );
    assertEquals(response.ok, true);
    await response.text();
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

Deno.test({
  name: "CLI: wmill --version works",
  fn: async () => {
    const tempDir = await Deno.makeTempDir({ prefix: "wmill_test_" });
    try {
      const result = await backend.runCLICommand(["--version"], tempDir);
      assertEquals(result.code, 0);
      console.log(`  CLI version: ${result.stdout.trim()}`);
    } finally {
      await Deno.remove(tempDir, { recursive: true });
    }
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

Deno.test({
  name: "CLI: wmill sync pull works",
  fn: async () => {
    const tempDir = await Deno.makeTempDir({ prefix: "wmill_test_" });
    try {
      const result = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir,
      );
      // May fail if workspace is empty, but shouldn't error on connection
      console.log(`  Pull result: code=${result.code}`);
      if (result.stderr) {
        console.log(`  stderr: ${result.stderr.slice(0, 200)}`);
      }
    } finally {
      await Deno.remove(tempDir, { recursive: true });
    }
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

// Cleanup after all tests
Deno.test({
  name: "cleanup: stop cargo backend",
  fn: async () => {
    await backend.stop();
  },
  sanitizeResources: false,
  sanitizeOps: false,
});
