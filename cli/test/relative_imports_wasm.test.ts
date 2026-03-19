/**
 * Tests for relative import resolution:
 * 1. WASM parser unit tests — verify parse_ts/py_relative_imports work correctly
 * 2. E2E tests — verify dependency propagation through scripts, flows, apps, and raw apps
 *    using the CLI generate-metadata command against a real backend
 */

import { expect, test, describe } from "bun:test";
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
  content: string = 'export async function main() { return "hello"; }'
): Promise<void> {
  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/scripts/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path: scriptPath,
        content,
        language: "bun",
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
  expect(resp.status).toBeLessThan(300);
  await resp.text();
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

describe("E2E: relative import dependency propagation via generate-metadata", () => {
  test("script importing another script gets transitive npm deps in lock", { timeout: 60000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
      const flowDir = `${tempDir}/f/test/my_flow.flow`;
      const flowHasLodash = await anyLockContains(flowDir, "lodash");
      expect(flowHasLodash).toBe(true);
    });
  });

  test("app inline script importing a script gets transitive npm deps in lock", { timeout: 60000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
      const appDir = `${tempDir}/f/test/my_app.app`;
      const appHasLodash = await anyLockContains(appDir, "lodash");
      expect(appHasLodash).toBe(true);
    });
  });

  test("raw app inline script importing a script gets transitive npm deps in lock", { timeout: 60000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
      const rawAppDir = `${tempDir}/f/test/my_raw_app.raw_app`;
      const rawAppHasLodash = await anyLockContains(rawAppDir, "lodash");
      expect(rawAppHasLodash).toBe(true);
    });
  });

  test("modifying leaf script marks all dependents as stale", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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

  test("dependency change triggers lock regeneration for flows and apps", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
      );

      // helper uses lodash
      await createLocalScript(tempDir, "f/test", "helper", "bun", helperScript);
      await createLocalFlow(tempDir, "f/test", "my_flow", importerScript);
      await createLocalApp(tempDir, "f/test", "my_app", importerScript);
      await createLocalRawApp(tempDir, "f/test", "my_raw_app", importerScript);

      // Generate initial metadata — all should get lodash in locks
      const initial = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      expect(initial.code).toBe(0);

      // Verify flow/app locks have lodash
      const flowDir = `${tempDir}/f/test/my_flow.flow`;
      expect(await anyLockContains(flowDir, "lodash")).toBe(true);
      const appDir = `${tempDir}/f/test/my_app.app`;
      expect(await anyLockContains(appDir, "lodash")).toBe(true);
      const rawAppDir = `${tempDir}/f/test/my_raw_app.raw_app`;
      expect(await anyLockContains(rawAppDir, "lodash")).toBe(true);

      // Modify helper to use axios instead of lodash
      await createLocalScript(
        tempDir,
        "f/test",
        "helper",
        "bun",
        `import axios from "axios";\nexport function helper() { return axios.VERSION; }\n`
      );

      // Regenerate — dependents should get new locks with axios
      const regen = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir
      );
      expect(regen.code).toBe(0);

      // Flow lock should now have axios, not lodash
      const flowHasAxios = await anyLockContains(flowDir, "axios");
      expect(flowHasAxios).toBe(true);

      // App lock should now have axios
      const appHasAxios = await anyLockContains(appDir, "axios");
      expect(appHasAxios).toBe(true);

      // Raw app lock should now have axios
      const rawAppHasAxios = await anyLockContains(rawAppDir, "axios");
      expect(rawAppHasAxios).toBe(true);
    });
  });

  test("locally modified workspace deps are used for lock generation instead of remote", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
      );

      // Deploy workspace deps with lodash on the remote
      const remotePackageJson = JSON.stringify({ dependencies: { lodash: "^4" } });
      await createRemoteWorkspaceDeps(backend, "bun", remotePackageJson);

      // Pull to get remote state locally
      const pull = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pull.code).toBe(0);

      // Modify workspace deps locally to use axios instead of lodash (NOT pushed)
      await mkdir(`${tempDir}/dependencies`, { recursive: true });
      await writeFile(
        `${tempDir}/dependencies/package.json`,
        JSON.stringify({ dependencies: { axios: "^1" } })
      );

      // Generate metadata to persist workspace deps hash — after this, deps are "up-to-date"
      // in wmill-lock.yaml even though they differ from the remote deployed version
      const gen1 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      expect(gen1.code).toBe(0);

      // Verify wmill-lock.yaml has the workspace deps entry with correct hash
      const { generateHash } = await import("../src/utils/utils.ts");
      const localDepsContent = JSON.stringify({ dependencies: { axios: "^1" } });
      const expectedHash = await generateHash(localDepsContent + "dependencies/package.json");
      const lockYaml = await readFile(`${tempDir}/wmill-lock.yaml`, "utf-8");
      expect(lockYaml).toContain("dependencies/package.json");
      expect(lockYaml).toContain(expectedHash);

      // Create a new script that uses workspace deps (no annotation = uses default package.json)
      await createLocalScript(
        tempDir,
        "f/test",
        "my_script",
        "bun",
        `export async function main() { return "hello"; }`
      );

      // Generate metadata — workspace deps are NOT stale (hash matches lockfile),
      // but they differ from the remote deployed version.
      // The script's lock should use the local workspace deps (axios), not remote (lodash).
      const gen2 = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
      if (gen2.code !== 0) {
        console.log("STDOUT:", gen2.stdout);
        console.log("STDERR:", gen2.stderr);
      }
      expect(gen2.code).toBe(0);

      const scriptLock = await readFile(
        `${tempDir}/f/test/my_script.script.lock`,
        "utf-8"
      ).catch(() => "");
      // Lock should use local workspace deps (axios), not remote (lodash)
      expect(scriptLock).toContain("axios");
      expect(scriptLock).not.toContain("lodash");
    });
  });

  test("unchanged workspace deps do not cause dependents to be stale on subsequent runs", { timeout: 120000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
        `defaultTs: bun\nincludes: ["**"]\nexcludes: []`
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
});
