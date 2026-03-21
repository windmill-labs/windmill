/**
 * Tests for relative import resolution:
 * 1. WASM parser unit tests — verify parse_ts/py_relative_imports work correctly
 * 2. E2E tests — verify dependency propagation through scripts, flows, apps, and raw apps
 *    using the CLI generate-metadata command against a real backend
 */

import { expect, test, describe, beforeAll } from "bun:test";
import { readFile, readdir, writeFile, mkdir } from "node:fs/promises";
import { loadParser } from "../src/utils/metadata.ts";
import { extractRelativeImports } from "../src/utils/relative_imports.ts";
import { withTestBackend, type TestBackend, createRemoteWorkspaceDeps } from "./test_backend.ts";
import {
  createLocalScript,
  createLocalFlow,
  createLocalApp,
  createLocalRawApp,
} from "./test_fixtures.ts";
import { setNonDottedPaths } from "../src/utils/resource_folders.ts";

// =============================================================================
// WASM Parser Unit Tests
// =============================================================================

describe("WASM TS parser exports parse_ts_relative_imports", () => {
  test("parse_ts_relative_imports function exists in WASM module", async () => {
    const mod = await loadParser("windmill-parser-wasm-ts");
    expect(typeof mod.parse_ts_relative_imports).toBe("function");
  });

  test("resolves dot-relative import", async () => {
    const code = `import { helper } from "./helper";\nexport async function main() { return helper(); }`;
    const result = await extractRelativeImports(code, "f/folder/script", "bun");
    expect(result).toEqual(["f/folder/helper"]);
  });

  test("resolves double-dot-relative import", async () => {
    const code = `import { utils } from "../utils/helper";\nexport async function main() { return utils(); }`;
    const result = await extractRelativeImports(code, "f/folder/sub/script", "bun");
    expect(result).toEqual(["f/folder/utils/helper"]);
  });

  test("resolves absolute windmill import", async () => {
    const code = `import { shared } from "/f/shared/utils";\nexport async function main() { return shared(); }`;
    const result = await extractRelativeImports(code, "f/folder/script", "bun");
    expect(result).toEqual(["f/shared/utils"]);
  });

  test("ignores external package imports", async () => {
    const code = `import lodash from "lodash";\nimport axios from "axios";\nexport async function main() { return lodash.map([]); }`;
    const result = await extractRelativeImports(code, "f/folder/script", "bun");
    expect(result).toEqual([]);
  });

  test("strips .ts extension from imports", async () => {
    const code = `import { helper } from "./helper.ts";\nexport async function main() { return helper(); }`;
    const result = await extractRelativeImports(code, "f/folder/script", "bun");
    expect(result).toEqual(["f/folder/helper"]);
  });

  test("resolves mixed relative and external imports", async () => {
    const code = `import { helper } from "./helper";\nimport { utils } from "../utils";\nimport lodash from "lodash";\nexport async function main() { return helper(); }`;
    const result = await extractRelativeImports(code, "f/folder/script", "bun");
    expect(result).toEqual(["f/folder/helper", "f/utils"]);
  });

  test("works with named imports", async () => {
    const code = `import { slugify, capitalize } from "./string_helpers";\nexport async function main() { return slugify("test"); }`;
    const result = await extractRelativeImports(code, "f/utils/http_client", "bun");
    expect(result).toEqual(["f/utils/string_helpers"]);
  });
});

describe("WASM Python parser exports parse_py_relative_imports", () => {
  test("parse_py_relative_imports function exists in WASM module", async () => {
    const mod = await loadParser("windmill-parser-wasm-py-imports");
    expect(typeof mod.parse_py_relative_imports).toBe("function");
  });

  test("resolves python relative import", async () => {
    const code = `from f.utils.formatter import format_stats\ndef main(values: list):\n    return format_stats(values)`;
    const result = await extractRelativeImports(code, "f/data/process", "python3");
    expect(result).toEqual(["f/utils/formatter"]);
  });
});

// =============================================================================
// Helper: find all .lock files recursively in a directory
// =============================================================================

async function findLockFiles(dir: string): Promise<string[]> {
  const entries = await readdir(dir, { recursive: true });
  return entries
    .filter((e) => e.endsWith(".lock"))
    .map((e) => `${dir}/${e}`);
}

async function anyLockContains(dir: string, needle: string): Promise<boolean> {
  const lockFiles = await findLockFiles(dir);
  for (const lockFile of lockFiles) {
    const content = await readFile(lockFile, "utf-8").catch(() => "");
    if (content.includes(needle)) return true;
  }
  return false;
}

async function createRemoteScript(
  backend: TestBackend,
  scriptPath: string,
  content: string = 'export async function main() { return "hello"; }',
  language: string = "bun"
): Promise<void> {
  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/scripts/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path: scriptPath,
        content,
        language,
        summary: "Test script",
        description: "Created by integration test",
        schema: {
          $schema: "https://json-schema.org/draft/2020-12/schema",
          type: "object",
          properties: {},
          required: [],
        },
      }),
    }
  );
  const respText = await resp.text();
  if (resp.status >= 300) {
    console.log(`createRemoteScript ${scriptPath} (${language}) failed: ${resp.status} ${respText}`);
  }
  expect(resp.status).toBeLessThan(300);
}

// =============================================================================
// E2E Tests: Dependency propagation through relative imports
// =============================================================================

const helperScript = `import _ from "lodash";
export function helper() { return _.VERSION; }
`;

const importerScript = `import { helper } from "/f/test/helper";
export async function main() { return helper(); }
`;

const pyHelperScript = `import requests

def helper():
    return requests.__version__
`;

const pyImporterScript = `from f.test.py_helper import helper

def main():
    return helper()
`;

for (const nonDotted of [false, true]) {
describe(`E2E: relative import dependency propagation via generate-metadata (${nonDotted ? "non-dotted" : "dotted"} paths)`, () => {
  const inlineSuffix = nonDotted ? "" : ".inline_script";
  const flowSuffix = nonDotted ? "__flow" : ".flow";
  const appSuffix = nonDotted ? "__app" : ".app";
  const rawAppSuffix = nonDotted ? "__raw_app" : ".raw_app";
  const wmillYaml = nonDotted
    ? `defaultTs: bun\nincludes: ["**"]\nexcludes: []\nnonDottedPaths: true`
    : `defaultTs: bun\nincludes: ["**"]\nexcludes: []`;

  beforeAll(() => {
    setNonDottedPaths(nonDotted);
  });
  test("script importing another script gets transitive npm deps in lock", { timeout: 60000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // helper has lodash dep
      await createLocalScript(tempDir, "f/test", "helper", "bun", helperScript);
      // consumer imports helper
      await createLocalScript(
        tempDir,
        "f/test",
        "consumer",
        "bun",
        `import { helper } from "./helper";\nexport async function main() { return helper(); }`
      );

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      if (result.code !== 0) {
        console.log("STDOUT:", result.stdout);
        console.log("STDERR:", result.stderr);
      }
      expect(result.code).toBe(0);

      const helperLock = await readFile(
        `${tempDir}/f/test/helper.script.lock`,
        "utf-8"
      ).catch(() => "");
      const consumerLock = await readFile(
        `${tempDir}/f/test/consumer.script.lock`,
        "utf-8"
      ).catch(() => "");

      expect(helperLock).toContain("lodash");
      expect(consumerLock).toContain("lodash");
    });
  });

  test("flow inline script importing a script gets transitive npm deps in lock", { timeout: 60000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      await createLocalScript(tempDir, "f/test", "helper", "bun", helperScript);
      await createLocalFlow(tempDir, "f/test", "my_flow", importerScript);

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      if (result.code !== 0) {
        console.log("STDOUT:", result.stdout);
        console.log("STDERR:", result.stderr);
      }
      expect(result.code).toBe(0);

      // Helper script lock should have lodash
      const helperLock = await readFile(
        `${tempDir}/f/test/helper.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(helperLock).toContain("lodash");

      // Flow inline script lock should also have lodash (transitive via helper)
      const flowDir = `${tempDir}/f/test/my_flow${flowSuffix}`;
      const flowHasLodash = await anyLockContains(flowDir, "lodash");
      expect(flowHasLodash).toBe(true);
    });
  });

  test("app inline script importing a script gets transitive npm deps in lock", { timeout: 60000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      await createLocalScript(tempDir, "f/test", "helper", "bun", helperScript);
      await createLocalApp(tempDir, "f/test", "my_app", importerScript);

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      if (result.code !== 0) {
        console.log("STDOUT:", result.stdout);
        console.log("STDERR:", result.stderr);
      }
      expect(result.code).toBe(0);

      const helperLock = await readFile(
        `${tempDir}/f/test/helper.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(helperLock).toContain("lodash");

      // App inline script lock should have lodash (transitive via helper)
      const appDir = `${tempDir}/f/test/my_app${appSuffix}`;
      const appHasLodash = await anyLockContains(appDir, "lodash");
      expect(appHasLodash).toBe(true);
    });
  });

  test("raw app inline script importing a script gets transitive npm deps in lock", { timeout: 60000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      await createLocalScript(tempDir, "f/test", "helper", "bun", helperScript);
      await createLocalRawApp(tempDir, "f/test", "my_raw_app", importerScript);

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      if (result.code !== 0) {
        console.log("STDOUT:", result.stdout);
        console.log("STDERR:", result.stderr);
      }
      expect(result.code).toBe(0);

      const helperLock = await readFile(
        `${tempDir}/f/test/helper.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(helperLock).toContain("lodash");

      // Raw app inline script lock should have lodash (transitive via helper)
      const rawAppDir = `${tempDir}/f/test/my_raw_app${rawAppSuffix}`;
      const rawAppHasLodash = await anyLockContains(rawAppDir, "lodash");
      expect(rawAppHasLodash).toBe(true);
    });
  });

  test("modifying leaf script marks all dependents as stale", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      await createLocalScript(tempDir, "f/test", "helper", "bun", helperScript);
      await createLocalScript(
        tempDir,
        "f/test",
        "consumer",
        "bun",
        `import { helper } from "./helper";\nexport async function main() { return helper(); }`
      );
      await createLocalFlow(tempDir, "f/test", "my_flow", importerScript);
      await createLocalApp(tempDir, "f/test", "my_app", importerScript);
      await createLocalRawApp(tempDir, "f/test", "my_raw_app", importerScript);

      // Generate initial metadata
      const initial = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      expect(initial.code).toBe(0);

      // Verify all up to date
      const check1 = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir
      );
      expect(check1.stdout).toContain("up-to-date");

      // Modify the leaf helper script (change content but keep lodash dep)
      await createLocalScript(
        tempDir,
        "f/test",
        "helper",
        "bun",
        `import _ from "lodash";\nexport function helper() { return _.VERSION + " v2"; }\n`
      );

      // All dependents should now be detected as stale
      const check2 = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir
      );
      expect(check2.code).toBe(0);
      expect(check2.stdout).toContain("helper");
      expect(check2.stdout).toContain("consumer");
      expect(check2.stdout).toContain("my_flow");
      expect(check2.stdout).toContain("my_app");
      expect(check2.stdout).toContain("my_raw_app");
    });
  });

  test("new script importing locally modified helper gets local deps not remote", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Deploy helper (lodash) to backend, then pull locally
      // Content includes literal \n to exercise Postgres bytea cast bug (content::bytea fails on backslash)
      const helperWithBackslash = `import _ from "lodash";\nexport function helper() { return "line1\\nline2"; }\n`;
      await createRemoteScript(backend, "f/test/helper", helperWithBackslash);
      const pull = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      if (pull.code !== 0) {
        console.log("PULL STDOUT:", pull.stdout);
        console.log("PULL STDERR:", pull.stderr);
      }
      expect(pull.code).toBe(0);

      // Modify helper locally to use axios instead of lodash (NOT pushed)
      await createLocalScript(
        tempDir,
        "f/test",
        "helper",
        "bun",
        `import axios from "axios";\nexport function helper() { return axios.VERSION; }\n`
      );

      // Regenerate helper metadata — helper is stale (content changed from deployed)
      const run1 = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      expect(run1.code).toBe(0);

      // Create a new consumer that imports helper
      await createLocalScript(
        tempDir,
        "f/test",
        "consumer",
        "bun",
        `import { helper } from "./helper";\nexport async function main() { return helper(); }`
      );

      // Consumer is stale (new), helper is NOT stale (metadata up-to-date).
      // Helper differs from deployed (axios vs lodash).
      // Diff endpoint should detect mismatch, upload local helper.
      // Consumer's lock must have axios (local), not lodash (deployed).
      const run2 = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      if (run2.code !== 0) {
        console.log("STDOUT:", run2.stdout);
        console.log("STDERR:", run2.stderr);
      }
      expect(run2.code).toBe(0);

      const consumerLock = await readFile(
        `${tempDir}/f/test/consumer.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(consumerLock).toContain("axios");
      expect(consumerLock).not.toContain("lodash");
    });
  });

  test("new script importing unpushed helper gets transitive deps in lock", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Create helper (lodash dep) and generate its metadata
      await createLocalScript(tempDir, "f/test", "helper", "bun", helperScript);
      const run1 = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      expect(run1.code).toBe(0);

      // Now create a NEW consumer that imports helper
      // Helper is not stale (metadata up-to-date) and was never pushed to remote
      await createLocalScript(
        tempDir,
        "f/test",
        "consumer",
        "bun",
        `import { helper } from "./helper";\nexport async function main() { return helper(); }`
      );

      // Run 2: only consumer is stale (new). Helper is NOT stale.
      // Consumer's lock must include lodash (transitive dep from local helper)
      const run2 = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      if (run2.code !== 0) {
        console.log("STDOUT:", run2.stdout);
        console.log("STDERR:", run2.stderr);
      }
      expect(run2.code).toBe(0);

      const consumerLock = await readFile(
        `${tempDir}/f/test/consumer.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(consumerLock).toContain("lodash");
    });
  });

  test("dependency change triggers lock regeneration for flows and apps", { timeout: 180000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Step 1: Create helper scripts locally and deploy them to remote via push
      await createLocalScript(tempDir, "f/test", "helper", "bun", helperScript);
      await createLocalScript(tempDir, "f/test", "py_helper", "python3", pyHelperScript);

      // Generate metadata for scripts only, then push to deploy them on remote
      const genScripts = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--skip-flows", "--skip-apps"],
        tempDir
      );
      expect(genScripts.code).toBe(0);

      const push = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      if (push.code !== 0) {
        console.log("PUSH STDOUT:", push.stdout);
        console.log("PUSH STDERR:", push.stderr);
      }
      expect(push.code).toBe(0);

      // Step 2: Now create flows/apps that import the deployed helpers
      await createLocalFlow(tempDir, "f/test", "my_flow", importerScript);
      await createLocalFlow(tempDir, "f/test", "my_py_flow", pyImporterScript, "python3");
      await createLocalApp(tempDir, "f/test", "my_app", importerScript);
      await createLocalRawApp(tempDir, "f/test", "my_raw_app", importerScript);

      // Step 3: Generate initial metadata for everything
      const initial = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      if (initial.code !== 0) {
        console.log("INITIAL STDOUT:", initial.stdout);
        console.log("INITIAL STDERR:", initial.stderr);
      }
      expect(initial.code).toBe(0);

      // Verify flow directories have correct structure — no extra files created
      const flowDir = `${tempDir}/f/test/my_flow${flowSuffix}`;
      expect((await readdir(flowDir)).sort()).toEqual([`a${inlineSuffix}.lock`, `a${inlineSuffix}.ts`, "flow.yaml"]);
      const pyFlowDir = `${tempDir}/f/test/my_py_flow${flowSuffix}`;
      expect((await readdir(pyFlowDir)).sort()).toEqual([`a${inlineSuffix}.lock`, `a${inlineSuffix}.py`, "flow.yaml"]);

      // Verify TS flow/app locks have lodash
      expect(await anyLockContains(flowDir, "lodash")).toBe(true);
      const appDir = `${tempDir}/f/test/my_app${appSuffix}`;
      expect(await anyLockContains(appDir, "lodash")).toBe(true);
      const rawAppDir = `${tempDir}/f/test/my_raw_app${rawAppSuffix}`;
      expect(await anyLockContains(rawAppDir, "lodash")).toBe(true);

      // Verify Python flow lock has requests
      expect(await anyLockContains(pyFlowDir, "requests")).toBe(true);

      // Step 5: Modify helpers LOCALLY (NOT pushed to remote — this is the key scenario)
      await createLocalScript(
        tempDir,
        "f/test",
        "helper",
        "bun",
        `import axios from "axios";\nexport function helper() { return axios.VERSION; }\n`
      );
      await createLocalScript(
        tempDir,
        "f/test",
        "py_helper",
        "python3",
        `import pandas\n\ndef helper():\n    return pandas.__version__\n`
      );

      // Step 6: Regenerate — flow locks must use LOCAL helper content, not remote deployed version
      const regen = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      if (regen.code !== 0) {
        console.log("REGEN STDOUT:", regen.stdout);
        console.log("REGEN STDERR:", regen.stderr);
      }
      expect(regen.code).toBe(0);

      // TS flow lock should now have axios (from local helper), not lodash (from remote)
      expect(await anyLockContains(flowDir, "axios")).toBe(true);
      expect(await anyLockContains(flowDir, "lodash")).toBe(false);

      // Python flow lock should now have pandas (from local helper), not requests (from remote)
      const pyFlowLockFiles = await findLockFiles(pyFlowDir);
      for (const f of pyFlowLockFiles) {
        const c = await readFile(f, "utf-8").catch(() => "");
        console.log(`PY FLOW LOCK [${f}]: ${c.substring(0, 500)}`);
      }
      expect(await anyLockContains(pyFlowDir, "pandas")).toBe(true);
      expect(await anyLockContains(pyFlowDir, "requests")).toBe(false);

      // TS app lock should now have axios, not lodash
      expect(await anyLockContains(appDir, "axios")).toBe(true);
      expect(await anyLockContains(appDir, "lodash")).toBe(false);

      // Raw app lock should now have axios, not lodash
      expect(await anyLockContains(rawAppDir, "axios")).toBe(true);
      expect(await anyLockContains(rawAppDir, "lodash")).toBe(false);
    });
  });

  test("locally modified workspace deps are used for lock generation instead of remote", { timeout: 180000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Deploy named Python workspace dep "test" with requests on the remote
      await createRemoteWorkspaceDeps(backend, "python3", "requests", "test");

      // Pull to get remote state locally
      const pull = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pull.code).toBe(0);

      // Empty the workspace dep locally (NOT pushed) — simulates removing all deps
      await mkdir(`${tempDir}/dependencies`, { recursive: true });
      await writeFile(
        `${tempDir}/dependencies/test.requirements.in`,
        ""
      );

      // Create a Python script that uses the named workspace dep via #requirements: test
      await createLocalScript(
        tempDir,
        "f/test",
        "my_script",
        "python3",
        `#requirements: test\n\ndef main():\n    return "hello"\n`
      );

      // Generate metadata — local dep is empty, so lock must NOT contain requests (from remote)
      const gen = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      if (gen.code !== 0) {
        console.log("STDOUT:", gen.stdout);
        console.log("STDERR:", gen.stderr);
      }
      expect(gen.code).toBe(0);

      const scriptLock = await readFile(
        `${tempDir}/f/test/my_script.script.lock`,
        "utf-8"
      ).catch(() => "");
      // Lock must use local (empty) workspace deps, not remote (requests)
      expect(scriptLock).not.toContain("requests");

      // Second run should be idempotent — no stale items
      const gen2 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      expect(gen2.code).toBe(0);
      const output2 = gen2.stdout + gen2.stderr;
      expect(output2).toContain("All metadata up-to-date");
    });
  });

  test("unchanged workspace deps do not cause dependents to be stale on subsequent runs", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Create local workspace deps
      await mkdir(`${tempDir}/dependencies`, { recursive: true });
      await writeFile(
        `${tempDir}/dependencies/package.json`,
        JSON.stringify({ dependencies: { axios: "^1" } })
      );

      // Create a script that uses workspace deps
      await createLocalScript(
        tempDir,
        "f/test",
        "my_script",
        "bun",
        `export async function main() { return "hello"; }`
      );

      // First generate-metadata — everything is stale, locks get generated
      const gen1 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      if (gen1.code !== 0) {
        console.log("STDOUT:", gen1.stdout);
        console.log("STDERR:", gen1.stderr);
      }
      expect(gen1.code).toBe(0);

      const scriptLock = await readFile(
        `${tempDir}/f/test/my_script.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(scriptLock).toContain("axios");

      // Second generate-metadata — nothing changed, should report "All metadata up-to-date"
      const gen2 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      if (gen2.code !== 0) {
        console.log("STDOUT:", gen2.stdout);
        console.log("STDERR:", gen2.stderr);
      }
      expect(gen2.code).toBe(0);

      const output = gen2.stdout + gen2.stderr;
      expect(output).toContain("All metadata up-to-date");
      // Should NOT show workspace deps or scripts as stale
      expect(output).not.toContain("stale metadata");
    });
  });

  test("diff endpoint correctly identifies mismatched scripts and workspace deps", { timeout: 60000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      const { generateHash } = await import("../src/utils/utils.ts");
      const { setClient } = await import("../src/core/client.ts");
      const { diffRawScriptsWithDeployed } = await import("../gen/services.gen.ts");

      setClient(backend.token, backend.baseUrl);

      // Deploy a script and two workspace deps (bun + python)
      const scriptContent = `export async function main() { return "hello"; }`;
      await createRemoteScript(backend, "f/test/deployed_script", scriptContent);

      const bunDepsContent = JSON.stringify({ dependencies: { lodash: "^4" } });
      await createRemoteWorkspaceDeps(backend, "bun", bunDepsContent);

      const scriptHash = await generateHash(scriptContent);
      const bunDepsHash = await generateHash(bunDepsContent);
      const wrongHash = await generateHash("totally different content");

      // Call 1: matching hash, same path → should NOT be mismatched
      const call1 = await diffRawScriptsWithDeployed({
        workspace: backend.workspace,
        requestBody: {
          scripts: { "f/test/deployed_script": scriptHash },
          workspace_deps: [
            { path: "dependencies/package.json", language: "bun", hash: bunDepsHash },
          ],
        },
      });
      expect(call1).not.toContain("f/test/deployed_script");
      expect(call1).not.toContain("dependencies/package.json");

      // Call 2: wrong hash same path + right hash wrong path → both should be mismatched
      const call2 = await diffRawScriptsWithDeployed({
        workspace: backend.workspace,
        requestBody: {
          scripts: {
            "f/test/deployed_script": wrongHash,
            "f/test/nonexistent_script": scriptHash,
          },
          workspace_deps: [
            { path: "dependencies/package.json", language: "bun", hash: wrongHash },
            { path: "dependencies/requirements.in", language: "python3", hash: bunDepsHash },
          ],
        },
      });
      // Same path, wrong hash → mismatched
      expect(call2).toContain("f/test/deployed_script");
      expect(call2).toContain("dependencies/package.json");
      // Wrong path, right hash → mismatched (endpoint should not match by hash alone)
      expect(call2).toContain("f/test/nonexistent_script");
      expect(call2).toContain("dependencies/requirements.in");
    });
  });

  test("folder arg includes importers outside the folder by default", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Script A in f/lib — has lodash dep
      await createLocalScript(
        tempDir,
        "f/lib",
        "helper",
        "bun",
        `import _ from "lodash";\nexport function helper() { return _.VERSION; }`
      );

      // Script B in f/app — imports A from a different directory
      await createLocalScript(
        tempDir,
        "f/app",
        "consumer",
        "bun",
        `import { helper } from "/f/lib/helper";\nexport async function main() { return helper(); }`
      );

      // First: generate-metadata globally to establish baseline locks
      const gen1 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      if (gen1.code !== 0) {
        console.log("STDOUT:", gen1.stdout);
        console.log("STDERR:", gen1.stderr);
      }
      expect(gen1.code).toBe(0);

      const consumerLock1 = await readFile(
        `${tempDir}/f/app/consumer.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(consumerLock1).toContain("lodash");

      // Now modify helper to use axios instead
      await createLocalScript(
        tempDir,
        "f/lib",
        "helper",
        "bun",
        `import axios from "axios";\nexport function helper() { return axios; }`
      );

      // Run generate-metadata for f/lib only — consumer (in f/app) should also be updated
      const gen2 = await backend.runCLICommand(
        ["generate-metadata", "--yes", "f/lib"],
        tempDir
      );
      if (gen2.code !== 0) {
        console.log("STDOUT:", gen2.stdout);
        console.log("STDERR:", gen2.stderr);
      }
      expect(gen2.code).toBe(0);

      // Consumer's lock should now have axios (updated even though it's outside f/lib)
      const consumerLock2 = await readFile(
        `${tempDir}/f/app/consumer.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(consumerLock2).toContain("axios");
    });
  });

  test("--strict-folder-boundaries skips importers outside the folder and warns", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Script A in f/lib — has lodash dep
      await createLocalScript(
        tempDir,
        "f/lib",
        "helper",
        "bun",
        `import _ from "lodash";\nexport function helper() { return _.VERSION; }`
      );

      // Script B in f/app — imports A
      await createLocalScript(
        tempDir,
        "f/app",
        "consumer",
        "bun",
        `import { helper } from "/f/lib/helper";\nexport async function main() { return helper(); }`
      );

      // Establish baseline
      const gen1 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      expect(gen1.code).toBe(0);

      const consumerLock1 = await readFile(
        `${tempDir}/f/app/consumer.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(consumerLock1).toContain("lodash");

      // Modify helper to use axios
      await createLocalScript(
        tempDir,
        "f/lib",
        "helper",
        "bun",
        `import axios from "axios";\nexport function helper() { return axios; }`
      );

      // Run with --strict-folder-boundaries — consumer should NOT be updated
      const gen2 = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--strict-folder-boundaries", "f/lib"],
        tempDir
      );
      if (gen2.code !== 0) {
        console.log("STDOUT:", gen2.stdout);
        console.log("STDERR:", gen2.stderr);
      }
      expect(gen2.code).toBe(0);

      // Consumer lock should still have lodash (not updated)
      const consumerLock2 = await readFile(
        `${tempDir}/f/app/consumer.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(consumerLock2).toContain("lodash");
      expect(consumerLock2).not.toContain("axios");

      // Output should contain a warning about the skipped importer
      const output = gen2.stdout + gen2.stderr;
      expect(output).toContain("Warning");
      expect(output).toContain("f/app/consumer");

      // Running again with same args should report up-to-date (not stuck in loop)
      const gen3 = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--strict-folder-boundaries", "f/lib"],
        tempDir
      );
      expect(gen3.code).toBe(0);
      const output3 = gen3.stdout + gen3.stderr;
      expect(output3).toContain("All metadata up-to-date");
    });
  });

  // TODO: consider adding --skip-workspace-dependencies flag to generate-metadata
  test("folder arg includes workspace dependencies in lock generation", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Deploy workspace deps with lodash on remote
      const remotePackageJson = JSON.stringify({ dependencies: { lodash: "^4" } });
      await createRemoteWorkspaceDeps(backend, "bun", remotePackageJson);

      // Pull to get remote state
      const pull = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pull.code).toBe(0);

      // Modify workspace deps locally to use axios
      await mkdir(`${tempDir}/dependencies`, { recursive: true });
      await writeFile(
        `${tempDir}/dependencies/package.json`,
        JSON.stringify({ dependencies: { axios: "^1" } })
      );

      // Create a script that uses workspace deps
      await createLocalScript(
        tempDir,
        "f/mydir",
        "my_script",
        "bun",
        `export async function main() { return "hello"; }`
      );

      // Run generate-metadata for f/mydir only — should still include workspace deps content
      // TODO: consider adding --skip-workspace-dependencies flag
      const gen = await backend.runCLICommand(
        ["generate-metadata", "--yes", "f/mydir"],
        tempDir
      );
      if (gen.code !== 0) {
        console.log("STDOUT:", gen.stdout);
        console.log("STDERR:", gen.stderr);
      }
      expect(gen.code).toBe(0);

      const scriptLock = await readFile(
        `${tempDir}/f/mydir/my_script.script.lock`,
        "utf-8"
      ).catch(() => "");
      // Lock should use local workspace deps (axios), not remote (lodash)
      expect(scriptLock).toContain("axios");
      expect(scriptLock).not.toContain("lodash");

      // Running again should report up-to-date (not stuck in loop)
      const gen2 = await backend.runCLICommand(
        ["generate-metadata", "--yes", "f/mydir"],
        tempDir
      );
      expect(gen2.code).toBe(0);
      const output2 = gen2.stdout + gen2.stderr;
      expect(output2).toContain("All metadata up-to-date");
    });
  });

  test("strict folder boundaries with workspace deps does not loop", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Create local workspace deps
      await mkdir(`${tempDir}/dependencies`, { recursive: true });
      await writeFile(
        `${tempDir}/dependencies/package.json`,
        JSON.stringify({ dependencies: { axios: "^1" } })
      );

      // Create a script that uses workspace deps
      await createLocalScript(
        tempDir,
        "f/mydir",
        "my_script",
        "bun",
        `export async function main() { return "hello"; }`
      );

      // First run with strict + folder
      const gen1 = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--strict-folder-boundaries", "f/mydir"],
        tempDir
      );
      if (gen1.code !== 0) {
        console.log("STDOUT:", gen1.stdout);
        console.log("STDERR:", gen1.stderr);
      }
      expect(gen1.code).toBe(0);

      // Second run — should report up-to-date, not stuck in loop
      const gen2 = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--strict-folder-boundaries", "f/mydir"],
        tempDir
      );
      expect(gen2.code).toBe(0);
      const output2 = gen2.stdout + gen2.stderr;
      expect(output2).toContain("All metadata up-to-date");
    });
  });

  // Bug #1: flow/app with mismatched workspace deps — hash inconsistency causes perpetual staleness
  test("flow/app/raw app with workspace deps does not loop across runs", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Deploy workspace deps with lodash on remote
      const remotePackageJson = JSON.stringify({ dependencies: { lodash: "^4" } });
      await createRemoteWorkspaceDeps(backend, "bun", remotePackageJson);

      // Pull to get remote state
      const pull = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pull.code).toBe(0);

      // Modify workspace deps locally to use axios
      await mkdir(`${tempDir}/dependencies`, { recursive: true });
      await writeFile(
        `${tempDir}/dependencies/package.json`,
        JSON.stringify({ dependencies: { axios: "^1" } })
      );

      // Create a flow, app, and raw app with inline scripts
      await createLocalFlow(
        tempDir,
        "f/test",
        "my_flow",
        `export async function main() { return "hello from flow"; }`
      );
      await createLocalApp(
        tempDir,
        "f/test",
        "my_app",
        `export async function main() { return "hello from app"; }`
      );
      await createLocalRawApp(
        tempDir,
        "f/test",
        "my_raw_app",
        `export async function main() { return "hello from raw app"; }`
      );

      // First run — generates locks
      const gen1 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      if (gen1.code !== 0) {
        console.log("STDOUT:", gen1.stdout);
        console.log("STDERR:", gen1.stderr);
      }
      expect(gen1.code).toBe(0);

      // Second run — should report up-to-date, not loop
      const gen2 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      expect(gen2.code).toBe(0);
      const output2 = gen2.stdout + gen2.stderr;
      expect(output2).toContain("All metadata up-to-date");
    });
  });

  // Bug #2: flow/app/raw app importing locally-modified helper uses local content not remote
  test("flow/app/raw app importing locally modified helper uses local content", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Helper with lodash dep — only exists locally, never pushed
      await createLocalScript(
        tempDir,
        "f/test",
        "helper",
        "bun",
        `import _ from "lodash";\nexport function helper() { return _.VERSION; }`
      );

      // Flow, app, and raw app inline scripts import the helper
      await createLocalFlow(
        tempDir,
        "f/test",
        "my_flow",
        `import { helper } from "/f/test/helper";\nexport async function main() { return helper(); }`
      );
      await createLocalApp(
        tempDir,
        "f/test",
        "my_app",
        `import { helper } from "/f/test/helper";\nexport async function main() { return helper(); }`
      );
      await createLocalRawApp(
        tempDir,
        "f/test",
        "my_raw_app",
        `import { helper } from "/f/test/helper";\nexport async function main() { return helper(); }`
      );

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      if (result.code !== 0) {
        console.log("STDOUT:", result.stdout);
        console.log("STDERR:", result.stderr);
      }
      expect(result.code).toBe(0);

      // All locks should contain lodash (transitive dep from local helper)
      const flowDir = `${tempDir}/f/test/my_flow${flowSuffix}`;
      expect(await anyLockContains(flowDir, "lodash")).toBe(true);

      const appDir = `${tempDir}/f/test/my_app${appSuffix}`;
      expect(await anyLockContains(appDir, "lodash")).toBe(true);

      const rawAppDir = `${tempDir}/f/test/my_raw_app${rawAppSuffix}`;
      expect(await anyLockContains(rawAppDir, "lodash")).toBe(true);
    });
  });

  // Cross-directory relative imports with ../
  test("cross-directory relative import with ../ propagates deps", { timeout: 60000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // Helper in f/shared with lodash dep
      await createLocalScript(
        tempDir,
        "f/shared",
        "utils",
        "bun",
        `import _ from "lodash";\nexport function utils() { return _.VERSION; }`
      );

      // Script in f/app imports via ../shared/utils
      await createLocalScript(
        tempDir,
        "f/app",
        "consumer",
        "bun",
        `import { utils } from "../shared/utils";\nexport async function main() { return utils(); }`
      );

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      if (result.code !== 0) {
        console.log("STDOUT:", result.stdout);
        console.log("STDERR:", result.stderr);
      }
      expect(result.code).toBe(0);

      const consumerLock = await readFile(
        `${tempDir}/f/app/consumer.script.lock`,
        "utf-8"
      ).catch(() => "");
      expect(consumerLock).toContain("lodash");
    });
  });

  // Multi-level transitive chain: A -> B -> C, C changes, A must update
  test("3-level transitive chain propagates staleness", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        wmillYaml
      );

      // C has dayjs dep
      await createLocalScript(
        tempDir,
        "f/chain",
        "c",
        "bun",
        `import dayjs from "dayjs";\nexport function c() { return dayjs(); }`
      );

      // B imports C
      await createLocalScript(
        tempDir,
        "f/chain",
        "b",
        "bun",
        `import { c } from "./c";\nexport function b() { return c(); }`
      );

      // A imports B
      await createLocalScript(
        tempDir,
        "f/chain",
        "a",
        "bun",
        `import { b } from "/f/chain/b";\nexport async function main() { return b(); }`
      );

      // First run — establish baseline
      const gen1 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      expect(gen1.code).toBe(0);

      const aLock1 = await readFile(`${tempDir}/f/chain/a.script.lock`, "utf-8").catch(() => "");
      expect(aLock1).toContain("dayjs");

      // Modify C to use uuid instead
      await createLocalScript(
        tempDir,
        "f/chain",
        "c",
        "bun",
        `import { v4 } from "uuid";\nexport function c() { return v4(); }`
      );

      // Second run — A should be updated transitively (C changed -> B stale -> A stale)
      const gen2 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      expect(gen2.code).toBe(0);

      const aLock2 = await readFile(`${tempDir}/f/chain/a.script.lock`, "utf-8").catch(() => "");
      expect(aLock2).toContain("uuid");
    });
  });
});
} // end for nonDotted
