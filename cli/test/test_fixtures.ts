/**
 * Test Fixtures
 *
 * Shared helpers for creating test data (scripts, flows, apps, raw apps) in tests.
 *
 * Two types of helpers:
 * - Fixture functions: Return data structures with paths and contents (no disk I/O)
 * - Local creation functions: Create fixtures AND write them to disk
 *
 * CROSS-LINKS - Related test helper locations (keep in sync when adding new helpers):
 * @see test_backend.ts - API-based creation helpers (createTestApp, createTestResource,
 *   createAppWithInlineScript, createFlowWithInlineScript, etc.)
 * @see sync_pull_push.test.ts - Local fixtures + createRemoteScript (API-based)
 *
 * This file contains: Shared local fixtures (createLocalScript, createLocalFlow, etc.)
 * If you add new helpers, update cross-links in the files above.
 *
 * @example
 * // Using fixtures (data only)
 * const fixture = createScriptFixture("my_script", "bun");
 *
 * // Using local creation (writes to disk)
 * await createLocalScript(tempDir, "f/test", "my_script", "bun");
 *
 * @keywords createLocal, local script, local flow, local app, raw app, fixture, test data
 */

import { writeFile, mkdir } from "node:fs/promises";
import {
  getFolderSuffix,
  getMetadataFileName,
  getModuleFolderSuffix,
} from "../src/utils/resource_folders.ts";

// =============================================================================
// Fixture Types
// =============================================================================

export interface FileFixture {
  path: string;
  content: string;
}

export interface ScriptFixture {
  contentFile: FileFixture;
  metadataFile: FileFixture;
}

export interface FlowFixture {
  metadata: FileFixture;
  inlineScript: FileFixture;
}

export interface AppFixture {
  metadata: FileFixture;
}

export interface RawAppFixture {
  metadata: FileFixture;
  indexHtml: FileFixture;
  indexJs: FileFixture;
  [key: string]: FileFixture;
}

// =============================================================================
// Script Fixtures
// =============================================================================

/**
 * Creates a script fixture (data structure, no disk I/O).
 * See file header for cross-links to related helpers.
 *
 * Use this when you need fine-grained control over the script structure.
 * For simple cases, use {@link createLocalScript} instead.
 *
 * @param name - Script name (without extension)
 * @param language - Script language
 * @param content - Optional custom script content
 * @returns Script fixture with content and metadata files
 *
 * @example
 * const fixture = createScriptFixture("my_script", "bun");
 * const fixture = createScriptFixture("custom", "python3", "def main(): return 42");
 *
 * @keywords script fixture, create script, local script
 */
export function createScriptFixture(
  name: string,
  language: "python3" | "deno" | "bun" | "bash" | "go" | "postgresql" = "bun",
  content?: string
): ScriptFixture {
  const extensions: Record<string, string> = {
    python3: ".py",
    deno: ".ts",
    bun: ".ts",
    bash: ".sh",
    go: ".go",
    postgresql: ".sql",
  };

  const ext = extensions[language];
  const defaultContent: Record<string, string> = {
    python3: `def main():\n    return "Hello from ${name}"`,
    deno: `export async function main() {\n  return "Hello from ${name}";\n}`,
    bun: `export async function main() {\n  return "Hello from ${name}";\n}`,
    bash: `#!/bin/bash\necho "Hello from ${name}"`,
    go: `package inner\n\nfunc main() string {\n  return "Hello from ${name}"\n}`,
    postgresql: `-- ${name}\nSELECT 'Hello from ${name}';`,
  };

  return {
    contentFile: {
      path: `${name}${ext}`,
      content: content ?? defaultContent[language],
    },
    metadataFile: {
      path: `${name}.script.yaml`,
      content: `summary: "${name} script"
description: "A ${language} script for testing"
schema:
  $schema: "https://json-schema.org/draft/2020-12/schema"
  type: object
  properties: {}
  required: []
is_template: false
lock: ""
kind: script
`,
    },
  };
}

// =============================================================================
// Flow Fixtures
// =============================================================================

/**
 * Creates a flow fixture (data structure, no disk I/O).
 * See file header for cross-links to related helpers.
 *
 * TODO: Add optional params: language, summary, description
 *
 * Use this when you need fine-grained control over the flow structure.
 * For simple cases, use {@link createLocalFlow} instead.
 *
 * @param name - Flow name
 * @param inlineScriptContent - Optional custom inline script content
 * @returns Flow fixture with metadata and inline script
 *
 * @example
 * const fixture = createFlowFixture("my_flow");
 *
 * @keywords flow fixture, create flow, local flow
 */
export function createFlowFixture(
  name: string,
  inlineScriptContent?: string
): FlowFixture {
  const flowSuffix = getFolderSuffix("flow");
  const metadataFile = getMetadataFileName("flow", "yaml");

  const scriptContent =
    inlineScriptContent ??
    `export async function main() {\n  return "Hello from flow ${name}";\n}`;

  return {
    metadata: {
      path: `${name}${flowSuffix}/${metadataFile}`,
      content: `summary: "${name} flow"
description: "A flow for testing"
value:
  modules:
    - id: a
      value:
        type: rawscript
        content: |
          ${scriptContent.split("\n").join("\n          ")}
        language: bun
        input_transforms: {}
schema:
  $schema: "https://json-schema.org/draft/2020-12/schema"
  type: object
  properties: {}
  required: []
`,
    },
    inlineScript: {
      path: `${name}${flowSuffix}/a.inline_script.ts`,
      content: scriptContent,
    },
  };
}

// =============================================================================
// App Fixtures
// =============================================================================

/**
 * Creates an app fixture (data structure, no disk I/O).
 * See file header for cross-links to related helpers.
 *
 * TODO: Add optional params: inlineScriptContent, summary, grid
 *
 * Use this when you need fine-grained control over the app structure.
 * For simple cases, use {@link createLocalApp} instead.
 *
 * @param name - App name
 * @returns App fixture with metadata
 *
 * @example
 * const fixture = createAppFixture("my_app");
 *
 * @keywords app fixture, create app, local app
 */
export function createAppFixture(name: string): AppFixture {
  const appSuffix = getFolderSuffix("app");
  const metadataFile = getMetadataFileName("app", "yaml");

  return {
    metadata: {
      path: `${name}${appSuffix}/${metadataFile}`,
      content: `summary: "${name} app"
value:
  type: app
  grid:
    - id: button1
      data:
        type: buttoncomponent
        componentInput:
          type: runnable
          runnable:
            type: runnableByName
            inlineScript:
              content: |
                export async function main() {
                  return "hello from app";
                }
              language: bun
  hiddenInlineScripts: []
  css: {}
  norefreshbar: false
policy:
  on_behalf_of: null
  on_behalf_of_email: null
  triggerables: {}
  execution_mode: viewer
`,
    },
  };
}

// =============================================================================
// Raw App Fixtures
// =============================================================================

/**
 * Creates a raw app fixture (data structure, no disk I/O).
 * See file header for cross-links to related helpers.
 *
 * TODO: Add optional params: inlineScriptContent, htmlContent, jsContent
 *
 * Raw apps are React/frontend apps with separate inline scripts.
 * Use this when you need fine-grained control over the raw app structure.
 * For simple cases, use {@link createLocalRawApp} instead.
 *
 * @param name - Raw app name
 * @returns Raw app fixture with metadata and frontend files
 *
 * @example
 * const fixture = createRawAppFixture("my_raw_app");
 *
 * @keywords raw app fixture, create raw app, local raw app, react app
 */
export function createRawAppFixture(name: string): RawAppFixture {
  const rawAppSuffix = getFolderSuffix("raw_app");
  const metadataFile = getMetadataFileName("raw_app", "yaml");

  return {
    metadata: {
      path: `${name}${rawAppSuffix}/${metadataFile}`,
      content: `summary: "${name} raw app"
policy:
  execution_mode: publisher
  triggerables: {}
`,
    },
    indexHtml: {
      path: `${name}${rawAppSuffix}/index.html`,
      content: `<!DOCTYPE html>
<html>
<head><title>${name}</title></head>
<body><div id="root"></div></body>
</html>`,
    },
    indexJs: {
      path: `${name}${rawAppSuffix}/index.tsx`,
      content: `import React from 'react'
import { createRoot } from 'react-dom/client'

const App = () => <div><h1>${name}</h1></div>

const root = createRoot(document.getElementById('root')!)
root.render(<App/>)
`,
    },
    packageJson: {
      path: `${name}${rawAppSuffix}/package.json`,
      content: `{
  "dependencies": {
    "react": "19.0.0",
    "react-dom": "19.0.0"
  }
}`,
    },
    inlineScript: {
      path: `${name}${rawAppSuffix}/inline_scripts/a.inline_script.ts`,
      content: `export async function main(x: string) {
  return x
}
`,
    },
    inlineScriptLock: {
      path: `${name}${rawAppSuffix}/inline_scripts/a.inline_script.lock`,
      content: ``,
    },
  };
}

// =============================================================================
// Local Creation Functions (Fixture + Write to Disk)
// =============================================================================

/**
 * Creates a script on the local filesystem.
 * See file header for cross-links to related helpers.
 *
 * This is a convenience function that creates a script fixture and writes it to disk.
 *
 * @param tempDir - Base directory for the test workspace
 * @param path - Relative path within the workspace (e.g., "f/test")
 * @param name - Script name (without extension)
 * @param language - Script language (default: "bun")
 * @param content - Optional custom script content
 *
 * @example
 * await createLocalScript(tempDir, "f/test", "my_script");
 * await createLocalScript(tempDir, "f/test", "custom", "python3", "def main(): return 42");
 *
 * @keywords create local script, local script, write script, script on disk
 */
export async function createLocalScript(
  tempDir: string,
  path: string,
  name: string,
  language: "python3" | "deno" | "bun" | "bash" | "go" | "postgresql" = "bun",
  content?: string
): Promise<void> {
  const fixture = createScriptFixture(name, language, content);
  await mkdir(`${tempDir}/${path}`, { recursive: true });
  await writeFile(
    `${tempDir}/${path}/${fixture.contentFile.path}`,
    fixture.contentFile.content,
    "utf-8"
  );
  await writeFile(
    `${tempDir}/${path}/${fixture.metadataFile.path}`,
    fixture.metadataFile.content,
    "utf-8"
  );
}

/**
 * Creates a flow on the local filesystem.
 * See file header for cross-links to related helpers.
 *
 * This is a convenience function that creates a flow fixture and writes it to disk.
 *
 * @param tempDir - Base directory for the test workspace
 * @param path - Relative path within the workspace (e.g., "f/test")
 * @param name - Flow name
 * @param inlineScriptContent - Optional custom inline script content
 *
 * @example
 * await createLocalFlow(tempDir, "f/test", "my_flow");
 *
 * @keywords create local flow, local flow, write flow, flow on disk
 */
export async function createLocalFlow(
  tempDir: string,
  path: string,
  name: string,
  inlineScriptContent?: string
): Promise<void> {
  const fixture = createFlowFixture(name, inlineScriptContent);
  const flowDir = `${tempDir}/${path}/${name}${getFolderSuffix("flow")}`;
  await mkdir(flowDir, { recursive: true });

  for (const file of Object.values(fixture)) {
    const fullPath = `${tempDir}/${path}/${file.path}`;
    await writeFile(fullPath, file.content, "utf-8");
  }
}

/**
 * Creates an app on the local filesystem.
 * See file header for cross-links to related helpers.
 *
 * This is a convenience function that creates an app fixture and writes it to disk.
 *
 * @param tempDir - Base directory for the test workspace
 * @param path - Relative path within the workspace (e.g., "f/test")
 * @param name - App name
 *
 * @example
 * await createLocalApp(tempDir, "f/test", "my_app");
 *
 * @keywords create local app, local app, write app, app on disk
 */
export async function createLocalApp(
  tempDir: string,
  path: string,
  name: string
): Promise<void> {
  const fixture = createAppFixture(name);
  const appDir = `${tempDir}/${path}/${name}${getFolderSuffix("app")}`;
  await mkdir(appDir, { recursive: true });

  for (const file of Object.values(fixture)) {
    const fullPath = `${tempDir}/${path}/${file.path}`;
    await writeFile(fullPath, file.content, "utf-8");
  }
}

/**
 * Creates a raw app on the local filesystem.
 * See file header for cross-links to related helpers.
 *
 * Raw apps are React/frontend apps with separate inline scripts.
 * This is a convenience function that creates a raw app fixture and writes it to disk.
 *
 * @param tempDir - Base directory for the test workspace
 * @param path - Relative path within the workspace (e.g., "f/test")
 * @param name - Raw app name
 *
 * @example
 * await createLocalRawApp(tempDir, "f/test", "my_raw_app");
 *
 * @keywords create local raw app, local raw app, write raw app, raw app on disk, react app
 */
export async function createLocalRawApp(
  tempDir: string,
  path: string,
  name: string
): Promise<void> {
  const fixture = createRawAppFixture(name);
  const rawAppSuffix = getFolderSuffix("raw_app");
  const appDir = `${tempDir}/${path}/${name}${rawAppSuffix}`;
  await mkdir(`${appDir}/inline_scripts`, { recursive: true });

  for (const file of Object.values(fixture)) {
    const fullPath = `${tempDir}/${path}/${file.path}`;
    const dir = fullPath.substring(0, fullPath.lastIndexOf("/"));
    await mkdir(dir, { recursive: true });
    await writeFile(fullPath, file.content, "utf-8");
  }
}

// =============================================================================
// Script with Modules Fixtures
// =============================================================================

export interface ModuleFile {
  path: string;
  content: string;
  lock?: string;
}

/**
 * Creates a script with module files on the local filesystem.
 *
 * Creates the main script, its metadata, and module files in a __mod/ folder.
 * Optionally includes lock files for modules.
 *
 * @param tempDir - Base directory for the test workspace
 * @param dir - Relative path within the workspace (e.g., "f/test")
 * @param name - Script name (without extension)
 * @param language - Script language (default: "bun")
 * @param modules - Module files to create
 *
 * @example
 * await createLocalScriptWithModules(tempDir, "f/test", "my_script", "bun", [
 *   { path: "helper.ts", content: "export const x = 1;" },
 *   { path: "utils/math.ts", content: "export function add(a, b) { return a + b; }", lock: "lodash@4.0.0\n" },
 * ]);
 */
export async function createLocalScriptWithModules(
  tempDir: string,
  dir: string,
  name: string,
  language: "python3" | "deno" | "bun" | "bash" | "go" | "postgresql" = "bun",
  modules: ModuleFile[]
): Promise<void> {
  // Create the main script
  await createLocalScript(tempDir, dir, name, language);

  // Create module files in __mod/ folder
  const modSuffix = getModuleFolderSuffix();
  const modDir = `${tempDir}/${dir}/${name}${modSuffix}`;

  for (const mod of modules) {
    const fullPath = `${modDir}/${mod.path}`;
    const parentDir = fullPath.substring(0, fullPath.lastIndexOf("/"));
    await mkdir(parentDir, { recursive: true });
    await writeFile(fullPath, mod.content, "utf-8");

    // Write lock file if provided
    if (mod.lock) {
      const baseName = mod.path.substring(0, mod.path.indexOf("."));
      const lockPath = `${modDir}/${baseName}.lock`;
      const lockDir = lockPath.substring(0, lockPath.lastIndexOf("/"));
      await mkdir(lockDir, { recursive: true });
      await writeFile(lockPath, mod.lock, "utf-8");
    }
  }
}

// =============================================================================
// Resource Fixtures (Variables, Resources, Schedules, etc.)
// =============================================================================

/**
 * Creates a resource fixture.
 *
 * @keywords resource fixture, create resource
 */
export function createResourceFixture(
  name: string,
  resourceType: string,
  value: Record<string, unknown>
): FileFixture {
  return {
    path: `${name}.resource.yaml`,
    content: `resource_type: "${resourceType}"
value:
${Object.entries(value)
  .map(([k, v]) => `  ${k}: ${JSON.stringify(v)}`)
  .join("\n")}
`,
  };
}

/**
 * Creates a variable fixture.
 *
 * @keywords variable fixture, create variable
 */
export function createVariableFixture(
  name: string,
  value: string,
  isSecret: boolean = false
): FileFixture {
  return {
    path: `${name}.variable.yaml`,
    content: `value: "${value}"
is_secret: ${isSecret}
description: "Variable ${name} for testing"
`,
  };
}

/**
 * Creates a schedule fixture.
 *
 * @keywords schedule fixture, create schedule
 */
export function createScheduleFixture(
  name: string,
  scriptPath: string,
  schedule: string = "0 * * * *"
): FileFixture {
  return {
    path: `${name}.schedule.yaml`,
    content: `path: "${name}"
schedule: "${schedule}"
script_path: "${scriptPath}"
is_flow: false
args: {}
enabled: true
timezone: "UTC"
`,
  };
}
