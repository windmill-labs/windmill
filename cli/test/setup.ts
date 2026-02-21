/**
 * Global test setup — preloaded before all test files.
 *
 * Starts the backend (compiling if needed) so individual tests
 * don't time out waiting for cargo build.
 *
 * Also registers a global afterAll hook to clean up the backend
 * after all test files complete, so individual files don't need
 * their own cleanup tests.
 */

import { afterAll } from "bun:test";
import { getTestBackend, cleanupTestBackend } from "./test_backend.ts";

await getTestBackend();

afterAll(async () => {
  await cleanupTestBackend();
});
