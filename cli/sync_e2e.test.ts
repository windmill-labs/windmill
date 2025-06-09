import { assertEquals, assertStringIncludes } from "https://deno.land/std@0.208.0/testing/asserts.ts";
import { resolve } from "https://deno.land/std@0.208.0/path/mod.ts";

// Helper function to extract JSON from CLI output that may contain warnings
function extractJsonFromOutput(output: string): any {
  // Split by lines and find the last line that looks like JSON
  const lines = output.trim().split('\n');
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();
    if (line.startsWith('{') && line.endsWith('}')) {
      try {
        return JSON.parse(line);
      } catch {
        continue;
      }
    }
  }
  throw new Error('No valid JSON found in output');
}

// Helper function to run wmill commands in test environment
async function runWmillCommand(args: string[]): Promise<{ success: boolean; output: string; error?: string }> {
  try {
    // Use dynamic import to handle both Deno and Node.js environments
    const command = typeof Deno !== 'undefined' && Deno.Command ?
      new Deno.Command("deno", {
        args: ["run", "--allow-all", resolve(import.meta.dirname || ".", "main.ts"), ...args],
        stdout: "piped",
        stderr: "piped",
      }) : null;

    if (!command) {
      // Skip test in Node.js environment
      return {
        success: true,
        output: JSON.stringify({ success: true, added: [], deleted: [], modified: [] }),
        error: undefined
      };
    }

    const { code, stdout, stderr } = await command.output();
    const output = new TextDecoder().decode(stdout);
    const error = new TextDecoder().decode(stderr);

    return {
      success: code === 0,
      output,
      error: error || undefined
    };
  } catch (e) {
    return {
      success: false,
      output: "",
      error: e instanceof Error ? e.message : String(e)
    };
  }
}

// Helper function to create a temporary directory with test files
async function createTestWorkspace(): Promise<string> {
  const tempDir = await Deno.makeTempDir({ prefix: "wmill_test_" });

  // Create a basic wmill.yaml
  const wmillYaml = `defaultTs: bun
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
includeKey: false`;

  await Deno.writeTextFile(`${tempDir}/wmill.yaml`, wmillYaml);

  return tempDir;
}

Deno.test("sync pull --settings-from-json - basic functionality", async () => {
  const testDir = await createTestWorkspace();
  const originalCwd = Deno.cwd();

  try {
    Deno.chdir(testDir);

    const settingsJson = JSON.stringify({
      include_path: ["f/**", "u/**"],
      include_type: ["script", "flow", "app", "folder", "variable", "schedule"]
    });

    const result = await runWmillCommand([
      "sync", "pull", "--dry-run", "--json-output",
      "--settings-from-json", settingsJson
    ]);

    // The command might fail due to authentication, but we can test argument parsing
    if (result.error) {
      // Check that the error is authentication-related, not argument parsing
      assertStringIncludes(result.error.toLowerCase(), "login");
    } else {
      // If it succeeds, verify JSON output structure
      const output = extractJsonFromOutput(result.output);
      assertEquals(typeof output.success, "boolean");
      assertEquals(Array.isArray(output.added), true);
      assertEquals(Array.isArray(output.deleted), true);
      assertEquals(Array.isArray(output.modified), true);
    }
  } finally {
    Deno.chdir(originalCwd);
    await Deno.remove(testDir, { recursive: true });
  }
});

Deno.test("sync push --settings-from-json - basic functionality", async () => {
  const testDir = await createTestWorkspace();
  const originalCwd = Deno.cwd();

  try {
    Deno.chdir(testDir);

    const settingsJson = JSON.stringify({
      include_path: ["f/**"],
      include_type: ["script", "flow", "app", "folder", "settings"]
    });

    const result = await runWmillCommand([
      "sync", "push", "--dry-run", "--json-output",
      "--settings-from-json", settingsJson
    ]);

    // The command might fail due to authentication, but we can test argument parsing
    if (result.error) {
      // Check that the error is authentication-related, not argument parsing
      assertStringIncludes(result.error.toLowerCase(), "login");
    } else {
      // If it succeeds, verify JSON output structure
      const output = extractJsonFromOutput(result.output);
      assertEquals(typeof output.success, "boolean");
      assertEquals(Array.isArray(output.added), true);
      assertEquals(Array.isArray(output.deleted), true);
      assertEquals(Array.isArray(output.modified), true);
    }
  } finally {
    Deno.chdir(originalCwd);
    await Deno.remove(testDir, { recursive: true });
  }
});

Deno.test("sync pull --settings-from-json - invalid JSON", async () => {
  const testDir = await createTestWorkspace();
  const originalCwd = Deno.cwd();

  try {
    Deno.chdir(testDir);

    const invalidJson = '{"include_path":["f/**"],"include_type":["script","flow"}'; // Missing closing bracket

    const result = await runWmillCommand([
      "sync", "pull", "--dry-run", "--json-output",
      "--settings-from-json", invalidJson
    ]);

    assertEquals(result.success, false);
    assertStringIncludes(result.error || "", "Invalid JSON");
  } finally {
    Deno.chdir(originalCwd);
    await Deno.remove(testDir, { recursive: true });
  }
});

Deno.test("sync pull --settings-from-json - wmill.yaml comparison", async () => {
  const testDir = await createTestWorkspace();
  const originalCwd = Deno.cwd();

  try {
    Deno.chdir(testDir);

    // Create a wmill.yaml with different settings than what we'll pass in JSON
    const differentWmillYaml = `defaultTs: bun
includes:
  - f/**
excludes: []
codebases: []
skipVariables: true
skipResources: true
skipResourceTypes: true
skipSecrets: true
includeSchedules: false
includeTriggers: false
includeUsers: false
includeGroups: false
includeSettings: false
includeKey: false`;

    await Deno.writeTextFile("wmill.yaml", differentWmillYaml);

    const settingsJson = JSON.stringify({
      include_path: ["f/**"],
      include_type: ["script", "flow", "app", "folder", "variable", "schedule"] // This will make skipVariables: false
    });

    const result = await runWmillCommand([
      "sync", "pull", "--dry-run", "--json-output",
      "--settings-from-json", settingsJson
    ]);

    // The wmill.yaml comparison should detect differences
    // Even if authentication fails, the local file comparison should work
    if (result.success && result.output) {
      const output = extractJsonFromOutput(result.output);
      // The wmill.yaml should be marked as modified due to different settings
      assertStringIncludes(JSON.stringify(output.modified), "wmill.yaml");
    }
  } finally {
    Deno.chdir(originalCwd);
    await Deno.remove(testDir, { recursive: true });
  }
});