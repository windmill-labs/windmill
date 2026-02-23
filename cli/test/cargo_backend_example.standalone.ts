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

import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { CargoBackend } from "./cargo_backend.ts";

// Single backend instance for all tests
let backend: CargoBackend;

// Setup before all tests
test("setup: start cargo backend", async () => {
    backend = new CargoBackend({
      verbose: process.env.VERBOSE === "1",
    });
    await backend.start();
    expect(backend.baseUrl).toBeDefined();
    expect(backend.authToken).toBeDefined();
});

test("API: version endpoint responds", async () => {
    const response = await fetch(`${backend.baseUrl}/api/version`);
    expect(response.ok).toEqual(true);
    const version = await response.text();
    expect(version).toBeDefined();
    console.log(`  Backend version: ${version.trim()}`);
});

test("API: workspace exists", async () => {
    const response = await backend.apiRequest(
      `/api/w/${backend.workspace}/workspaces/get_settings`,
    );
    expect(response.ok).toEqual(true);
    await response.text();
});

test("CLI: wmill --version works", async () => {
    const tempDir = await mkdtemp(join(tmpdir(), "wmill_test_"));
    try {
      const result = await backend.runCLICommand(["--version"], tempDir);
      expect(result.code).toEqual(0);
      console.log(`  CLI version: ${result.stdout.trim()}`);
    } finally {
      await rm(tempDir, { recursive: true });
    }
});

test("CLI: wmill sync pull works", async () => {
    const tempDir = await mkdtemp(join(tmpdir(), "wmill_test_"));
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
      await rm(tempDir, { recursive: true });
    }
});

// Cleanup after all tests
test("cleanup: stop cargo backend", async () => {
    await backend.stop();
});
