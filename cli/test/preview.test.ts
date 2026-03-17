import { expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";

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

  await writeFile(`${tempDir}/wmill.yaml`, yamlContent, "utf-8");
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
  await mkdir(dir, { recursive: true });
  await writeFile(`${tempDir}/${path}`, content, "utf-8");

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
  await writeFile(`${tempDir}/${metaPath}`, metaContent, "utf-8");
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
  await mkdir(dir, { recursive: true });

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
  await writeFile(`${dir}/flow.yaml`, flowYaml, "utf-8");
}

async function createPathScriptFlow(
  tempDir: string,
  flowPath: string,
  options: {
    summary: string;
    scriptPath: string;
    inputTransforms?: string;
  }
): Promise<void> {
  const dir = `${tempDir}/${flowPath}`;
  await mkdir(dir, { recursive: true });
  const inputTransforms = options.inputTransforms
    ? `        input_transforms:\n${options.inputTransforms}`
    : "        input_transforms: {}\n";

  const flowYaml = `summary: "${options.summary}"
description: "Test flow"
value:
  modules:
    - id: "a"
      value:
        type: "script"
        path: "${options.scriptPath}"
${inputTransforms}
schema:
  $schema: "https://json-schema.org/draft/2020-12/schema"
  type: object
  properties: {}
  required: []
`;
  await writeFile(`${dir}/flow.yaml`, flowYaml, "utf-8");
}

// =============================================================================
// SCRIPT PREVIEW TESTS
// =============================================================================

test("script preview: regular script (non-codebase)", async () => {
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

    expect(result.code).toEqual(0);
    expect(result.stdout + result.stderr).toContain("Hello, World!");
  });
});

test("script preview: codebase script (CJS)", async () => {
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

    expect(result.code).toEqual(0);
    expect(result.stdout + result.stderr).toContain("Hello from CJS codebase, World!");
  });
});

test("script preview: codebase script (ESM)", async () => {
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

    expect(result.code).toEqual(0);
    expect(result.stdout + result.stderr).toContain("Hello from ESM codebase, World!");
  });
});

test("script preview: codebase script with assets (tar)", async () => {
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
    await mkdir(`${tempDir}/f/codebase_tar`, { recursive: true });
    await writeFile(
      `${tempDir}/f/codebase_tar/data.json`,
      JSON.stringify({ message: "Hello from asset!" }),
      "utf-8"
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

    expect(result.code).toEqual(0);
    expect(result.stdout + result.stderr).toContain("Hello World! Asset says: Hello from asset!");
  });
});

test("script preview: codebase script ESM + tar (assets)", async () => {
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
    await mkdir(`${tempDir}/f/codebase_esm_tar`, { recursive: true });
    await writeFile(
      `${tempDir}/f/codebase_esm_tar/config.json`,
      JSON.stringify({ setting: "esm_tar_value" }),
      "utf-8"
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

    expect(result.code).toEqual(0);
    expect(result.stdout + result.stderr).toContain("Hello World! Config setting: esm_tar_value");
  });
});

test("script preview: codebase with imports (simulates ../shared layout)", async () => {
  await withTestBackend(async (backend, tempDir) => {
    // This test simulates a codebase that could be in a parent directory.
    // The structure is:
    //   tempDir/
    //     wmill.yaml (codebase at ".")
    //     f/
    //       lib/
    //         helper.ts       (shared module)
    //         main_script.ts  (imports helper)
    //
    // This tests that codebase bundling correctly includes imported modules,
    // which is the key functionality needed for ../shared codebases during sync.
    // Note: Preview requires valid windmill paths (u/, g/, f/), so we run
    // from within the codebase directory.

    await createWmillConfig(tempDir, {
      defaultTs: "bun",
      codebases: [{ relative_path: ".", includes: ["**"] }],
    });

    // Create helper module
    await mkdir(`${tempDir}/f/lib`, { recursive: true });
    await writeFile(
      `${tempDir}/f/lib/helper.ts`,
      `export function greet(name: string): string {
  return \`Hello from shared codebase, \${name}!\`;
}`,
      "utf-8"
    );

    // Create main script that imports the helper
    await writeFile(
      `${tempDir}/f/lib/main_script.ts`,
      `import { greet } from "./helper";

export function main(name: string = "World") {
  console.log("Running codebase script with imports");
  return greet(name);
}`,
      "utf-8"
    );

    // Create script metadata
    await writeFile(
      `${tempDir}/f/lib/main_script.script.yaml`,
      `summary: "Test script with imports"
description: "Test script that imports from helper module"
lock: ""
schema:
  $schema: "https://json-schema.org/draft/2020-12/schema"
  type: object
  properties:
    name:
      type: string
      default: "World"
  required: []
`,
      "utf-8"
    );

    // Run preview - the script should be bundled with the helper module
    const result = await backend.runCLICommand(
      ["script", "preview", "f/lib/main_script.ts"],
      tempDir
    );

    expect(result.code).toEqual(0);
    // The script should be bundled (includes the helper) and run successfully
    expect(
      result.stdout + result.stderr,
    ).toContain("Hello from shared codebase, World!");
  });
});

// =============================================================================
// FLOW PREVIEW TESTS
// =============================================================================

test("flow preview: simple flow", async () => {
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

    expect(result.code).toEqual(0);
    expect(result.stdout + result.stderr).toContain("Flow says: Hello, World!");
  });
});

test("flow preview: uses local PathScript by default and remote PathScript with --remote", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await createWmillConfig(tempDir, { defaultTs: "bun" });

    await createScript(
      tempDir,
      "f/test/helper_script.ts",
      `export function main(name: string = "World") { return \`Remote script says: \${name}!\`; }`
    );

    const pushResult = await backend.runCLICommand(
      ["script", "push", "f/test/helper_script.ts"],
      tempDir
    );
    expect(pushResult.code).toEqual(0);

    await writeFile(
      `${tempDir}/f/test/helper_script.ts`,
      `export function main(name: string = "World") { return \`Local script says: \${name}!\`; }`,
      "utf-8"
    );

    await createPathScriptFlow(tempDir, "f/test/path_flow.flow", {
      summary: "Flow with PathScript",
      scriptPath: "f/test/helper_script",
      inputTransforms: `          name:
            type: "static"
            value: "PathTest"
`,
    });

    const localResult = await backend.runCLICommand(
      ["flow", "preview", "f/test/path_flow.flow"],
      tempDir
    );

    expect(localResult.code).toEqual(0);
    expect(localResult.stdout + localResult.stderr).toContain(
      "Local script says: PathTest!"
    );
    expect(localResult.stdout + localResult.stderr).toContain(
      "Using local PathScript files for flow preview."
    );
    expect(localResult.stdout + localResult.stderr).toContain(
      "These workspace scripts differ from the deployed version:\n- f/test/helper_script"
    );

    const remoteResult = await backend.runCLICommand(
      ["flow", "preview", "--remote", "f/test/path_flow.flow"],
      tempDir
    );

    expect(remoteResult.code).toEqual(0);
    expect(remoteResult.stdout + remoteResult.stderr).toContain(
      "Remote script says: PathTest!"
    );
    expect(remoteResult.stdout + remoteResult.stderr).not.toContain(
      "Using local PathScript files for flow preview."
    );
  });
});

test.skipIf(shouldSkipOnCI())("flow preview: respects defaultTs when resolving local PathScripts", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await createWmillConfig(tempDir, { defaultTs: "deno" });

    await createScript(
      tempDir,
      "f/test/deno_helper.ts",
      `export function main() { return Deno.version.deno ? "deno-runtime" : "missing"; }`
    );

    await createPathScriptFlow(tempDir, "f/test/deno_path_flow.flow", {
      summary: "Flow with Deno PathScript",
      scriptPath: "f/test/deno_helper",
    });

    const result = await backend.runCLICommand(
      ["flow", "preview", "f/test/deno_path_flow.flow"],
      tempDir
    );

    expect(result.code).toEqual(0);
    expect(result.stdout + result.stderr).toContain("deno-runtime");
  });
});

test("flow preview: bundles local PathScripts with local imports", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await createWmillConfig(tempDir, {
      defaultTs: "bun",
      codebases: [{ relative_path: "f/flow_codebase", includes: ["**"] }],
    });

    await mkdir(`${tempDir}/f/flow_codebase`, { recursive: true });
    await writeFile(
      `${tempDir}/f/flow_codebase/helper.ts`,
      `export function greet(name: string): string {
  return \`Hello from local flow codebase, \${name}!\`;
}`,
      "utf-8"
    );

    await createScript(
      tempDir,
      "f/flow_codebase/main_script.ts",
      `import { greet } from "./helper";

export function main(name: string = "World") {
  return greet(name);
}`
    );

    await createPathScriptFlow(tempDir, "f/test/importing_path_flow.flow", {
      summary: "Flow with imported PathScript",
      scriptPath: "f/flow_codebase/main_script",
      inputTransforms: `          name:
            type: "static"
            value: "FlowTest"
`,
    });

    const result = await backend.runCLICommand(
      ["flow", "preview", "f/test/importing_path_flow.flow"],
      tempDir
    );

    expect(result.code).toEqual(0);
    expect(result.stdout + result.stderr).toContain(
      "Hello from local flow codebase, FlowTest!"
    );
  });
});

test("flow preview: warns when local PathScript is not deployed remotely", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await createWmillConfig(tempDir, { defaultTs: "bun" });

    await createScript(
      tempDir,
      "f/test/undeployed_helper.ts",
      `export function main() { return "Local only script"; }`
    );

    await createPathScriptFlow(tempDir, "f/test/undeployed_path_flow.flow", {
      summary: "Flow with undeployed PathScript",
      scriptPath: "f/test/undeployed_helper",
    });

    const result = await backend.runCLICommand(
      ["flow", "preview", "f/test/undeployed_path_flow.flow"],
      tempDir
    );

    expect(result.code).toEqual(0);
    expect(result.stdout + result.stderr).toContain("Local only script");
    expect(result.stdout + result.stderr).toContain(
      "These scripts do not exist in the workspace yet:\n- f/test/undeployed_helper"
    );
  });
});

test("flow preview: does not warn when local and deployed PathScripts match", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await createWmillConfig(tempDir, { defaultTs: "bun" });

    await createScript(
      tempDir,
      "f/test/matching_helper.ts",
      `export function main() { return "Matching script"; }`
    );

    const pushResult = await backend.runCLICommand(
      ["script", "push", "f/test/matching_helper.ts"],
      tempDir
    );
    expect(pushResult.code).toEqual(0);

    await createPathScriptFlow(tempDir, "f/test/matching_path_flow.flow", {
      summary: "Flow with matching PathScript",
      scriptPath: "f/test/matching_helper",
    });

    const result = await backend.runCLICommand(
      ["flow", "preview", "f/test/matching_path_flow.flow"],
      tempDir
    );

    expect(result.code).toEqual(0);
    expect(result.stdout + result.stderr).toContain("Matching script");
    expect(result.stdout + result.stderr).not.toContain(
      "Using local PathScript files for flow preview."
    );
  });
});

test("flow preview: fails loudly for asset-backed codebase scripts", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await createWmillConfig(tempDir, {
      defaultTs: "bun",
      codebases: [{
        relative_path: "f/codebase_tar",
        includes: ["**"],
        assets: [{ from: "f/codebase_tar/data.json", to: "data.json" }],
      }],
    });

    await mkdir(`${tempDir}/f/codebase_tar`, { recursive: true });
    await writeFile(
      `${tempDir}/f/codebase_tar/data.json`,
      JSON.stringify({ message: "Hello from asset!" }),
      "utf-8"
    );

    await createScript(
      tempDir,
      "f/codebase_tar/main_script.ts",
      `import * as fs from "fs";

export function main() {
  const data = JSON.parse(fs.readFileSync("data.json", "utf-8"));
  return data.message;
}`
    );

    await createPathScriptFlow(tempDir, "f/test/assets_path_flow.flow", {
      summary: "Flow with asset-backed PathScript",
      scriptPath: "f/codebase_tar/main_script",
    });

    const localResult = await backend.runCLICommand(
      ["flow", "preview", "f/test/assets_path_flow.flow"],
      tempDir
    );

    expect(localResult.code).not.toEqual(0);
    expect(localResult.stdout + localResult.stderr).toContain(
      "requires codebase assets"
    );
  });
});
