import { assertEquals, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withTestBackend, cleanupTestBackend } from "./test_backend.ts";

// =============================================================================
// PREVIEW COMMAND INTEGRATION TESTS
// Tests for `wmill script preview` and `wmill flow preview` commands
//
// These tests require a backend to run. Run with:
//   deno test --allow-all test/preview.test.ts
//
// The tests cover:
// - Regular script preview (non-codebase)
// - Codebase script preview (CJS format)
// - Codebase script preview (ESM format)
// - Codebase script preview with assets (tar bundle)
// - Codebase script preview with ESM + tar
// - Flow preview
// =============================================================================

// Helper to create a wmill.yaml config file
async function createWmillConfig(
  tempDir: string,
  config: {
    defaultTs?: "bun" | "deno";
    codebases?: Array<{
      relative_path: string;
      includes?: string[];
      format?: "cjs" | "esm";
      assets?: Array<{ from: string; to: string }>;
    }>;
  }
): Promise<void> {
  let yamlContent = `defaultTs: ${config.defaultTs ?? "bun"}\n`;

  if (config.codebases && config.codebases.length > 0) {
    yamlContent += "codebases:\n";
    for (const cb of config.codebases) {
      yamlContent += `  - relative_path: ${cb.relative_path}\n`;
      yamlContent += "    includes:\n";
      for (const inc of cb.includes ?? ["**"]) {
        yamlContent += `      - "${inc}"\n`;
      }
      if (cb.format) {
        yamlContent += `    format: ${cb.format}\n`;
      }
      if (cb.assets && cb.assets.length > 0) {
        yamlContent += "    assets:\n";
        for (const asset of cb.assets) {
          yamlContent += `      - from: ${asset.from}\n`;
          yamlContent += `        to: ${asset.to}\n`;
        }
      }
    }
  }

  await Deno.writeTextFile(`${tempDir}/wmill.yaml`, yamlContent);
}

// Helper to create a script file with metadata
async function createScript(
  tempDir: string,
  path: string,
  content: string,
  metadata?: {
    summary?: string;
    description?: string;
  }
): Promise<void> {
  const dir = `${tempDir}/${path.substring(0, path.lastIndexOf("/"))}`;
  await Deno.mkdir(dir, { recursive: true });
  await Deno.writeTextFile(`${tempDir}/${path}`, content);

  // Create metadata file
  const metaPath = path.replace(/\.[^.]+$/, ".script.yaml");
  const metaContent = `summary: "${metadata?.summary ?? "Test script"}"
description: "${metadata?.description ?? "Test script description"}"
lock: ""
schema:
  $schema: "https://json-schema.org/draft/2020-12/schema"
  type: object
  properties:
    name:
      type: string
      default: "World"
  required: []
`;
  await Deno.writeTextFile(`${tempDir}/${metaPath}`, metaContent);
}

// Helper to create a flow directory with flow.yaml
async function createFlow(
  tempDir: string,
  flowPath: string,
  options: {
    summary: string;
    scriptContent: string;
  }
): Promise<void> {
  const dir = `${tempDir}/${flowPath}`;
  await Deno.mkdir(dir, { recursive: true });

  const flowYaml = `summary: "${options.summary}"
description: "Test flow"
value:
  modules:
    - id: "a"
      value:
        type: "rawscript"
        language: "bun"
        content: |
${options.scriptContent.split("\n").map(line => `          ${line}`).join("\n")}
schema:
  $schema: "https://json-schema.org/draft/2020-12/schema"
  type: object
  properties:
    name:
      type: string
      default: "World"
  required: []
`;
  await Deno.writeTextFile(`${dir}/flow.yaml`, flowYaml);
}

// =============================================================================
// SCRIPT PREVIEW TESTS
// =============================================================================

Deno.test({
  name: "script preview: regular script (non-codebase)",
  async fn() {
    await withTestBackend(async (backend, tempDir) => {
      await createWmillConfig(tempDir, { defaultTs: "bun" });
      await createScript(
        tempDir,
        "f/test/simple_script.ts",
        `export function main(name: string = "World") {
  return \`Hello, \${name}!\`;
}`
      );

      const result = await backend.runCLICommand(
        ["script", "preview", "f/test/simple_script.ts"],
        tempDir
      );

      assertEquals(result.code, 0, `Preview failed: ${result.stderr}\n${result.stdout}`);
      assertStringIncludes(result.stdout + result.stderr, "Hello, World!");
    });
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

Deno.test({
  name: "script preview: codebase script (CJS)",
  async fn() {
    await withTestBackend(async (backend, tempDir) => {
      await createWmillConfig(tempDir, {
        defaultTs: "bun",
        codebases: [{ relative_path: "f/codebase", includes: ["**"] }],
      });

      await createScript(
        tempDir,
        "f/codebase/cjs_script.ts",
        `export function main(name: string = "World") {
  console.log("CJS codebase script running");
  return \`Hello from CJS codebase, \${name}!\`;
}`
      );

      const result = await backend.runCLICommand(
        ["script", "preview", "f/codebase/cjs_script.ts"],
        tempDir
      );

      assertEquals(result.code, 0, `Preview failed: ${result.stderr}\n${result.stdout}`);
      assertStringIncludes(result.stdout + result.stderr, "Hello from CJS codebase, World!");
    });
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

Deno.test({
  name: "script preview: codebase script (ESM)",
  async fn() {
    await withTestBackend(async (backend, tempDir) => {
      await createWmillConfig(tempDir, {
        defaultTs: "bun",
        codebases: [{ relative_path: "f/codebase_esm", includes: ["**"], format: "esm" }],
      });

      await createScript(
        tempDir,
        "f/codebase_esm/esm_script.ts",
        `export function main(name: string = "World") {
  console.log("ESM codebase script running");
  return \`Hello from ESM codebase, \${name}!\`;
}`
      );

      const result = await backend.runCLICommand(
        ["script", "preview", "f/codebase_esm/esm_script.ts"],
        tempDir
      );

      assertEquals(result.code, 0, `Preview failed: ${result.stderr}\n${result.stdout}`);
      assertStringIncludes(result.stdout + result.stderr, "Hello from ESM codebase, World!");
    });
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

Deno.test({
  name: "script preview: codebase script with assets (tar)",
  async fn() {
    await withTestBackend(async (backend, tempDir) => {
      await createWmillConfig(tempDir, {
        defaultTs: "bun",
        codebases: [{
          relative_path: "f/codebase_tar",
          includes: ["**"],
          assets: [{ from: "f/codebase_tar/data.json", to: "data.json" }],
        }],
      });

      // Create asset file
      await Deno.mkdir(`${tempDir}/f/codebase_tar`, { recursive: true });
      await Deno.writeTextFile(
        `${tempDir}/f/codebase_tar/data.json`,
        JSON.stringify({ message: "Hello from asset!" })
      );

      await createScript(
        tempDir,
        "f/codebase_tar/tar_script.ts",
        `import * as fs from "fs";

export function main(name: string = "World") {
  console.log("Tar codebase script running");
  const data = fs.readFileSync("data.json", "utf-8");
  const parsed = JSON.parse(data);
  return \`Hello \${name}! Asset says: \${parsed.message}\`;
}`
      );

      const result = await backend.runCLICommand(
        ["script", "preview", "f/codebase_tar/tar_script.ts"],
        tempDir
      );

      assertEquals(result.code, 0, `Preview failed: ${result.stderr}\n${result.stdout}`);
      assertStringIncludes(result.stdout + result.stderr, "Hello World! Asset says: Hello from asset!");
    });
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

Deno.test({
  name: "script preview: codebase script ESM + tar (assets)",
  async fn() {
    await withTestBackend(async (backend, tempDir) => {
      await createWmillConfig(tempDir, {
        defaultTs: "bun",
        codebases: [{
          relative_path: "f/codebase_esm_tar",
          includes: ["**"],
          format: "esm",
          assets: [{ from: "f/codebase_esm_tar/config.json", to: "config.json" }],
        }],
      });

      // Create asset file
      await Deno.mkdir(`${tempDir}/f/codebase_esm_tar`, { recursive: true });
      await Deno.writeTextFile(
        `${tempDir}/f/codebase_esm_tar/config.json`,
        JSON.stringify({ setting: "esm_tar_value" })
      );

      await createScript(
        tempDir,
        "f/codebase_esm_tar/esm_tar_script.ts",
        `import * as fs from "fs";

export function main(name: string = "World") {
  console.log("ESM + tar codebase script running");
  const config = fs.readFileSync("config.json", "utf-8");
  const parsed = JSON.parse(config);
  return \`Hello \${name}! Config setting: \${parsed.setting}\`;
}`
      );

      const result = await backend.runCLICommand(
        ["script", "preview", "f/codebase_esm_tar/esm_tar_script.ts"],
        tempDir
      );

      assertEquals(result.code, 0, `Preview failed: ${result.stderr}\n${result.stdout}`);
      assertStringIncludes(result.stdout + result.stderr, "Hello World! Config setting: esm_tar_value");
    });
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

// =============================================================================
// FLOW PREVIEW TESTS
// =============================================================================

Deno.test({
  name: "flow preview: simple flow",
  async fn() {
    await withTestBackend(async (backend, tempDir) => {
      await createWmillConfig(tempDir, { defaultTs: "bun" });
      await createFlow(tempDir, "f/test/simple_flow.flow", {
        summary: "Test flow",
        scriptContent: `export function main(name: string = "World") { return \`Flow says: Hello, \${name}!\`; }`,
      });

      const result = await backend.runCLICommand(
        ["flow", "preview", "f/test/simple_flow.flow"],
        tempDir
      );

      assertEquals(result.code, 0, `Flow preview failed: ${result.stderr}\n${result.stdout}`);
      assertStringIncludes(result.stdout + result.stderr, "Flow says: Hello, World!");
    });
  },
  sanitizeResources: false,
  sanitizeOps: false,
});

// =============================================================================
// CLEANUP
// =============================================================================

Deno.test({
  name: "cleanup test backend",
  async fn() {
    await cleanupTestBackend();
  },
  sanitizeResources: false,
  sanitizeOps: false,
});
