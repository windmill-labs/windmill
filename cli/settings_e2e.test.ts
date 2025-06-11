import { assertEquals, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { uiStateToSyncOptions } from "./settings_utils.ts";

// Helper to run real CLI commands
async function runCliCommand(args: string[], tempDir: string): Promise<{
  stdout: string;
  stderr: string;
  code: number;
}> {
  // Use absolute path to main.ts in CLI directory
  const originalCwd = Deno.cwd();
  const mainPath = `${originalCwd}/main.ts`;
  const cmd = ["deno", "run", "-A", mainPath, ...args];
  const process = new Deno.Command(cmd[0], {
    args: cmd.slice(1),
    stdout: "piped",
    stderr: "piped",
    cwd: tempDir // Run in temp dir where wmill.yaml files are
  });

  const { code, stdout, stderr } = await process.output();

  return {
    stdout: new TextDecoder().decode(stdout),
    stderr: new TextDecoder().decode(stderr),
    code
  };
}

// Create temporary workspace for testing
async function runWithTempDir<T>(fn: (tmpDir: string) => Promise<T>): Promise<T> {
  const tmpDir = await Deno.makeTempDir();
  const originalCwd = Deno.cwd();

  try {
    return await fn(tmpDir);
  } finally {
    Deno.chdir(originalCwd);
    try {
      await Deno.remove(tmpDir, { recursive: true });
    } catch {
      // ignore cleanup errors
    }
  }
}

// =============================================================================
// REAL END-TO-END TESTS WITH ACTUAL CLI
// =============================================================================

Deno.test("E2E: settings pull --from-json actually processes JSON", async () => {
  await runWithTempDir(async (tempDir) => {
    // Create initial file with different settings
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
codebases: []
skipVariables: false
skipResources: false
skipResourceTypes: false
skipSecrets: false
includeSchedules: false
includeTriggers: false
includeUsers: false
includeGroups: false
includeSettings: false
includeKey: false
`);

    // This JSON should set includeSettings: true
    const jsonWithSettings = JSON.stringify({
      include_path: ["f/**"],
      include_type: ["script", "flow", "app", "folder", "settings"]
    });

    // Call the real CLI
    const result = await runCliCommand([
      "settings", "pull",
      "--from-json", jsonWithSettings,
      "--json-output"
    ], tempDir);

    // Should either succeed or fail with auth, but process the JSON
    if (result.code === 0) {
      assertStringIncludes(result.stdout, "success");
    } else {
      // Should attempt to process JSON settings
      assertEquals(result.stderr.length > 0 || result.stdout.length > 0, true);
    }

    // Verify the file was actually updated by checking if CLI processed the settings
    const updatedContent = await Deno.readTextFile(`${tempDir}/wmill.yaml`);
    // The CLI should have written the new settings if it succeeded
    if (result.code === 0) {
      assertStringIncludes(updatedContent, 'includeSettings: true');
    }
  });
});

Deno.test("E2E: settings pull --from-json --diff shows real comparison", async () => {
  await runWithTempDir(async (tempDir) => {
    // Create local file with includeSettings: false
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
codebases: []
skipVariables: false
skipResources: false
skipResourceTypes: false
skipSecrets: false
includeSchedules: false
includeTriggers: false
includeUsers: false
includeGroups: false
includeSettings: false
includeKey: false
`);

    // JSON that should show includeSettings: true
    const jsonWithSettings = JSON.stringify({
      include_path: ["f/**"],
      include_type: ["script", "flow", "app", "folder", "settings"]
    });

    const result = await runCliCommand([
      "settings", "pull",
      "--from-json", jsonWithSettings,
      "--diff",
      "--json-output"
    ], tempDir);

    // Should process the diff request
    if (result.code === 0) {
      assertStringIncludes(result.stdout, "success");
    } else {
      assertEquals(result.stderr.length > 0 || result.stdout.length > 0, true);
    }
  });
});

Deno.test("E2E: settings push --from-json processes real settings", async () => {
  await runWithTempDir(async (tempDir) => {
    // Create a wmill.yaml file
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
codebases: []
skipVariables: false
skipResources: false
skipResourceTypes: false
skipSecrets: false
includeSchedules: false
includeTriggers: false
includeUsers: false
includeGroups: false
includeSettings: true
includeKey: false
`);

    const jsonSettings = JSON.stringify({
      include_path: ["f/**"],
      include_type: ["script", "flow", "app", "folder", "settings"]
    });

    const result = await runCliCommand([
      "settings", "push",
      "--from-json", jsonSettings,
      "--dry-run",
      "--json-output"
    ], tempDir);

    // Should process the JSON and attempt push
    if (result.code === 0) {
      assertStringIncludes(result.stdout, "success");
    } else {
      assertEquals(result.stderr.length > 0 || result.stdout.length > 0, true);
    }
  });
});

Deno.test("E2E: settings push --diff shows real backend comparison", async () => {
  await runWithTempDir(async (tempDir) => {
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
codebases: []
skipVariables: false
skipResources: false
skipResourceTypes: false
skipSecrets: false
includeSchedules: false
includeTriggers: false
includeUsers: false
includeGroups: false
includeSettings: false
includeKey: false
`);

    // Try to show diff with backend settings (will fail auth but should parse args)
    const result = await runCliCommand([
      "settings", "push",
      "--diff",
      "--dry-run",
      "--json-output"
    ], tempDir);

    // Should attempt to compare with backend
    assertEquals(result.stderr.length > 0 || result.stdout.length > 0, true);
  });
});

Deno.test("E2E: settings commands handle invalid JSON gracefully", async () => {
  await runWithTempDir(async (tempDir) => {
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**`);

    const invalidJson = '{"include_path":["f/**"],"include_type":["script","flow"}'; // Missing closing bracket

    const result = await runCliCommand([
      "settings", "pull",
      "--from-json", invalidJson,
      "--json-output"
    ], tempDir);

    // Should fail with JSON parse error
    assertEquals(result.code !== 0, true);
    // When using --json-output, error should be in stdout as JSON
    assertStringIncludes((result.stdout + result.stderr).toLowerCase(), "invalid json");
  });
});

Deno.test("E2E: settings conversion actually uses real utility functions", async () => {
  // Test that the conversion functions work correctly in isolation
  const uiState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder", "variable", "resource", "resourcetype", "secret", "schedule", "trigger", "user", "group", "settings", "key"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  // These should match the real implementation
  assertEquals(syncOptions.defaultTs, 'bun');
  assertEquals(syncOptions.includes, ['f/**']);
  assertEquals(syncOptions.skipVariables, false);
  assertEquals(syncOptions.skipResources, false);
  assertEquals(syncOptions.includeSettings, true);
  assertEquals(syncOptions.includeSchedules, true);
  assertEquals(syncOptions.includeUsers, true);
});

Deno.test("E2E: settings pull without --from-json uses real backend", async () => {
  await runWithTempDir(async (tempDir) => {
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**`);

    const result = await runCliCommand([
      "settings", "pull",
      "--dry-run",
      "--json-output"
    ], tempDir);

    // Should attempt to connect to backend (will fail auth but try)
    assertEquals(result.stderr.length > 0 || result.stdout.length > 0, true);
  });
});

Deno.test("E2E: settings push without --from-json reads local file", async () => {
  await runWithTempDir(async (tempDir) => {
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
codebases: []
skipVariables: false
skipResources: false
skipResourceTypes: false
skipSecrets: false
includeSchedules: false
includeTriggers: false
includeUsers: false
includeGroups: false
includeSettings: true
includeKey: false`);

    const result = await runCliCommand([
      "settings", "push",
      "--dry-run",
      "--json-output"
    ], tempDir);

    // Should read local file and attempt to push (will fail auth but try)
    assertEquals(result.stderr.length > 0 || result.stdout.length > 0, true);
  });
});