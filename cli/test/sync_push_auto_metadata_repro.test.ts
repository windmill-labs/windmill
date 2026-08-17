/**
 * Reproduction for `sync push --auto-metadata` cross-folder relative-import bug.
 *
 * `--auto-metadata` regenerates lockfiles client-side before pushing, but on a
 * fresh workspace none of the imported scripts are deployed yet. The fix must
 * route lockgen through `DoubleLinkedDependencyTree` + `uploadScripts` so the
 * dep job can resolve relative imports via `temp_script_refs`.
 *
 * Without the fix, `wmill sync push --auto-metadata --yes` aborts with
 * "Failed to find relative import" / "Non-zero exit status for bun build".
 */

import { expect, test } from "bun:test";
import { writeFile, readFile } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";
import { createLocalScript } from "./test_fixtures.ts";

const wmillYaml = `defaultTs: bun
includes: ["**"]
excludes: []
`;

// The importer path (f/aaa/...) sorts before the imported path (f/bbb/...)
// alphabetically. The CLI sorts changes by path within the script bucket, so
// without the fix the importer is processed first and its lockgen tries to
// fetch a not-yet-uploaded helper from the server.
//
// One `../` from f/aaa/consumer.ts steps out of f/aaa/ to f/, then `bbb/helper.ts`
// resolves to f/bbb/helper.ts.
const consumerScript = `import { helper } from "../bbb/helper.ts";
export async function main() { return helper(); }
`;

const helperScript = `export function helper() { return "ok"; }
`;

test(
  "sync push --auto-metadata succeeds for cross-folder relative imports on a fresh workspace",
  { timeout: 120000 },
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(`${tempDir}/wmill.yaml`, wmillYaml);

      // Cross-folder relative import where the importer's path comes
      // alphabetically before the import target.
      await createLocalScript(tempDir, "f/aaa", "consumer", "bun", consumerScript);
      await createLocalScript(tempDir, "f/bbb", "helper", "bun", helperScript);

      const result = await backend.runCLICommand(
        ["sync", "push", "--yes", "--auto-metadata"],
        tempDir,
      );

      if (result.code !== 0) {
        console.log("STDOUT:", result.stdout);
        console.log("STDERR:", result.stderr);
      }

      // The exit code must be zero — `--auto-metadata` should not abort on a
      // fresh workspace just because the importer is alphabetically first.
      expect(result.code).toBe(0);

      const combined = result.stdout + result.stderr;
      expect(combined).not.toContain("Failed to find relative import");
      expect(combined).not.toContain("Failed to generate lockfile");

      // Consumer's lockfile should exist and be non-empty (i.e. lockgen
      // actually produced a valid lock, not a sentinel/error string).
      const consumerLock = await readFile(
        `${tempDir}/f/aaa/consumer.script.lock`,
        "utf-8",
      ).catch(() => "");
      expect(consumerLock.length).toBeGreaterThan(0);
    });
  },
);

// Helper: build a multi-folder topology mimicking the customer's failure
// shape. Importers (analytics, webhooks) reach helpers in f/lib via deep
// cross-folder relative imports.
async function setupCustomerLikeTopology(tempDir: string) {
  await writeFile(`${tempDir}/wmill.yaml`, wmillYaml);
  await createLocalScript(
    tempDir,
    "f/lib",
    "log_event",
    "bun",
    `export function logEvent(msg: string) { return msg; }\n`,
  );
  await createLocalScript(
    tempDir,
    "f/lib",
    "errors",
    "bun",
    `export class AppError extends Error {}\n`,
  );
  await createLocalScript(
    tempDir,
    "f/integrations/snowflake",
    "client",
    "bun",
    `export function client() { return "snowflake"; }\n`,
  );
  await createLocalScript(
    tempDir,
    "f/analytics/claims_operations",
    "bulk",
    "bun",
    `import { client } from "../../integrations/snowflake/client.ts";
import { AppError } from "../../lib/errors.ts";
export async function main() { try { return client(); } catch (e) { throw new AppError(); } }
`,
  );
  await createLocalScript(
    tempDir,
    "f/webhooks/stripe",
    "handle_webhook",
    "bun",
    `import { logEvent } from "../../lib/log_event.ts";
export async function main() { return logEvent("ok"); }
`,
  );
}

test(
  "generate-metadata succeeds across many folders with deep cross-folder relative imports",
  { timeout: 180000 },
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupCustomerLikeTopology(tempDir);

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir,
      );

      if (result.code !== 0) {
        console.log("STDOUT:", result.stdout);
        console.log("STDERR:", result.stderr);
      }

      expect(result.code).toBe(0);

      const combined = result.stdout + result.stderr;
      expect(combined).not.toContain("Failed to find relative import");
      expect(combined).not.toContain("Failed to generate lockfile");

      const bulkLock = await readFile(
        `${tempDir}/f/analytics/claims_operations/bulk.script.lock`,
        "utf-8",
      ).catch(() => "");
      const webhookLock = await readFile(
        `${tempDir}/f/webhooks/stripe/handle_webhook.script.lock`,
        "utf-8",
      ).catch(() => "");
      expect(bulkLock.length).toBeGreaterThan(0);
      expect(webhookLock.length).toBeGreaterThan(0);
    });
  },
);

// `wmill script generate-metadata` is a deprecated alias defined in
// commands/script/script.ts. Its action handler used to be a separate
// implementation that didn't go through DoubleLinkedDependencyTree +
// uploadScripts, so on a fresh DB it hit the same out-of-order failure as
// `sync push --auto-metadata`. The fix delegates the alias to the canonical
// generateMetadata implementation.
// Customer scenario: a barrel file (f/lib/errors/index.ts) re-exports from
// siblings (./types.ts, ./WorkflowError.ts, ...). An importer in a different
// folder imports from the barrel. On a fresh DB, the dep job for the importer
// fetches index.ts via raw_unpinned + temp_script_hash, but bun's resolver
// then has to resolve the barrel's *sibling* imports — and those need to be
// in TEMP_SCRIPT_REFS too.
test(
  "generate-metadata succeeds when importer reaches helpers via a barrel re-exporter",
  { timeout: 180000 },
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(`${tempDir}/wmill.yaml`, wmillYaml);

      await createLocalScript(
        tempDir,
        "f/lib/errors",
        "types",
        "bun",
        `export type ErrorKind = "fatal" | "warn";\n` +
          `export function _typesAnchor() { return null as unknown; }\n`,
      );
      await createLocalScript(
        tempDir,
        "f/lib/errors",
        "WorkflowError",
        "bun",
        `export class WorkflowError extends Error { kind = "fatal" as const; }\n` +
          `export function _wfeAnchor() { return new WorkflowError(); }\n`,
      );
      await createLocalScript(
        tempDir,
        "f/lib/errors",
        "index",
        "bun",
        `export * from "./types.ts";\n` +
          `export * from "./WorkflowError.ts";\n` +
          `export function main() { return "barrel"; }\n`,
      );
      await createLocalScript(
        tempDir,
        "f/analytics/claims_operations",
        "bulk",
        "bun",
        `import { WorkflowError } from "../../lib/errors/index.ts";
export async function main() { return new WorkflowError().message; }
`,
      );

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir,
      );

      if (result.code !== 0) {
        console.log("STDOUT:", result.stdout);
        console.log("STDERR:", result.stderr);
      }

      expect(result.code).toBe(0);

      const combined = result.stdout + result.stderr;
      expect(combined).not.toContain("Failed to find relative import");
      expect(combined).not.toContain("Failed to generate lockfile");
    });
  },
);

test(
  "deprecated `wmill script generate-metadata` succeeds for cross-folder imports on a fresh workspace",
  { timeout: 180000 },
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupCustomerLikeTopology(tempDir);

      const result = await backend.runCLICommand(
        ["script", "generate-metadata", "--yes"],
        tempDir,
      );

      if (result.code !== 0) {
        console.log("STDOUT:", result.stdout);
        console.log("STDERR:", result.stderr);
      }

      expect(result.code).toBe(0);

      const combined = result.stdout + result.stderr;
      expect(combined).not.toContain("Failed to find relative import");
      expect(combined).not.toContain("Failed to generate lockfile");

      const bulkLock = await readFile(
        `${tempDir}/f/analytics/claims_operations/bulk.script.lock`,
        "utf-8",
      ).catch(() => "");
      const webhookLock = await readFile(
        `${tempDir}/f/webhooks/stripe/handle_webhook.script.lock`,
        "utf-8",
      ).catch(() => "");
      expect(bulkLock.length).toBeGreaterThan(0);
      expect(webhookLock.length).toBeGreaterThan(0);
    });
  },
);
