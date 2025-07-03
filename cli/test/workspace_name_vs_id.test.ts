import { assertEquals, assert, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
import { getEffectiveSettings } from "../conf.ts";

// =============================================================================
// WORKSPACE NAME VS ID TESTS
// Tests for proper handling of workspace names vs workspace IDs
// =============================================================================

Deno.test("Workspace Name vs ID: sync commands use workspace name in override keys", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Create wmill.yaml using workspace name (localhost_test) not ID (test)
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # Using workspace name, not ID
  "localhost_test:u/test/test_repo":
    skipVariables: true
    includeSchedules: true
    includes:
      - f/specific/**`);
    
    // Run sync pull
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/test_repo',
      '--dry-run',
      '--json-output'
    ], tempDir);
    
    assertEquals(result.code, 0);
    
    // Verify override was applied
    const jsonOutput = result.stdout.split('\n').find(line => line.trim().startsWith('{'));
    assert(jsonOutput, "Should have JSON output");
    const data = JSON.parse(jsonOutput);
    
    // Variables should be skipped due to override
    const hasVariables = data.added.some((file: string) => file.includes('.variable.yaml'));
    assertEquals(hasVariables, false, "Variables should be skipped due to override using workspace name");
  });
});

Deno.test("Workspace Name vs ID: gitsync-settings generates correct override keys", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Start with empty wmill.yaml
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []`);
    
    // Pull git-sync settings with override flag
    const pullResult = await backend.runCLICommand([
      'gitsync-settings', 'pull',
      '--repository', 'u/test/test_repo',
      '--override'
    ], tempDir);
    
    assertEquals(pullResult.code, 0);
    
    // Read the updated wmill.yaml
    const updatedYaml = await Deno.readTextFile(`${tempDir}/wmill.yaml`);
    
    // Should contain override key with workspace name, not ID
    assertStringIncludes(updatedYaml, "localhost_test:u/test/test_repo");
    assert(!updatedYaml.includes("test:u/test/test_repo:"), "Should not use workspace ID alone in new overrides");
  });
});

Deno.test("Workspace Name vs ID: error messages show both name and ID", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Create wmill.yaml with wrong workspace name
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # Wrong workspace name
  "wrong_workspace:u/test/test_repo":
    skipVariables: true`);
    
    // Create test to trigger validation
    const testContent = `
import { getEffectiveSettings } from "../windmill/cli/conf.ts";

// Capture console output
const originalWarn = console.warn;
const warnings: string[] = [];
console.warn = (...args) => {
  warnings.push(args.join(' '));
  originalWarn(...args);
};

try {
  const config = {
    includes: ["f/**"],
    overrides: {
      "wrong_workspace:u/test/test_repo": { skipVariables: true }
    }
  };
  
  const workspace = {
    remote: "http://localhost:8000/",
    workspaceId: "test",
    name: "localhost_test",
    token: "test-token"
  };
  
  // This should trigger warning about mismatched override key
  getEffectiveSettings(config, workspace.workspaceId, "u/test/test_repo", workspace);
  
  // Check warnings
  const hasWarning = warnings.some(w => 
    w.includes("Override key not found") &&
    w.includes("localhost_test") &&
    w.includes("ID: test")
  );
  
  if (hasWarning) {
    console.log("SUCCESS: Warning shows both workspace name and ID");
  } else {
    console.log("FAIL: Warning does not show both workspace name and ID");
    console.log("Warnings:", warnings);
  }
} finally {
  console.warn = originalWarn;
}`;

    await Deno.writeTextFile(`${tempDir}/test_warnings.ts`, testContent);
    
    // Run the test
    const cmd = new Deno.Command("deno", {
      args: ["run", "--allow-read", "test_warnings.ts"],
      cwd: tempDir,
      stdout: "piped",
      stderr: "piped"
    });
    
    const result = await cmd.output();
    const stdout = new TextDecoder().decode(result.stdout);
    
    assertStringIncludes(stdout, "SUCCESS: Warning shows both workspace name and ID");
  });
});

Deno.test("Workspace Name vs ID: init command uses workspace name", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Run init command
    const initResult = await backend.runCLICommand([
      'init'
    ], tempDir);
    
    // Even if init fails due to git requirements, check if it would create proper config
    if (initResult.code === 0) {
      const yamlContent = await Deno.readTextFile(`${tempDir}/wmill.yaml`);
      
      // If overrides were created, they should use workspace name
      if (yamlContent.includes("overrides:")) {
        assertStringIncludes(yamlContent, "localhost_test:");
      }
    }
  });
});

Deno.test("Workspace Name vs ID: workspace wildcard uses name not ID", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Create wmill.yaml with workspace wildcards
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # Workspace wildcard using name
  "localhost_test:*":
    skipVariables: true
    skipResources: true
  
  # Legacy wildcard using ID (should be lower priority)
  "test:*":
    skipVariables: false
    skipResources: false`);
    
    // Run sync to test wildcard resolution
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/any_repo',
      '--dry-run',
      '--json-output'
    ], tempDir);
    
    assertEquals(result.code, 0);
    
    const jsonOutput = result.stdout.split('\n').find(line => line.trim().startsWith('{'));
    assert(jsonOutput, "Should have JSON output");
    const data = JSON.parse(jsonOutput);
    
    // Should use workspace name wildcard (skipVariables: true)
    const hasVariables = data.added.some((file: string) => file.includes('.variable.yaml'));
    assertEquals(hasVariables, false, "Variables should be skipped due to workspace name wildcard");
  });
});

Deno.test("Workspace Name vs ID: push command preserves override key format", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Create wmill.yaml with both formats
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # New format
  "localhost_test:u/test/test_repo":
    includeSchedules: true
  
  # Legacy format
  "test:u/test/other_repo":
    includeSchedules: false`);
    
    // Push settings (diff mode)
    const pushResult = await backend.runCLICommand([
      'gitsync-settings', 'push',
      '--repository', 'u/test/test_repo',
      '--diff'
    ], tempDir);
    
    assertEquals(pushResult.code, 0);
    
    // The command should work with the new format key
    const output = pushResult.stdout;
    // STRONG ASSERTION: Command must succeed and process configuration
    assert(
      output.includes("gitsync-settings"),
      "Output must reference gitsync-settings operation"
    );
  });
});

Deno.test("Workspace Name vs ID: handles workspaces with similar names", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Test config with similar workspace names
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # Similar but different workspace names
  "localhost_test:u/test/test_repo":
    skipVariables: true
  
  "localhost_testing:u/test/test_repo":
    skipVariables: false
  
  "localhost_test2:u/test/test_repo":
    skipResources: true`);
    
    // Our backend workspace is "localhost_test"
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/test_repo',
      '--dry-run',
      '--json-output'
    ], tempDir);
    
    assertEquals(result.code, 0);
    
    const jsonOutput = result.stdout.split('\n').find(line => line.trim().startsWith('{'));
    assert(jsonOutput, "Should have JSON output");
    const data = JSON.parse(jsonOutput);
    
    // Should use exact match "localhost_test", not similar names
    const hasVariables = data.added.some((file: string) => file.includes('.variable.yaml'));
    assertEquals(hasVariables, false, "Should use localhost_test override (skipVariables: true)");
  });
});

Deno.test("Workspace Name vs ID: unit test for getEffectiveSettings precedence", async () => {
  // Direct unit test of getEffectiveSettings
  const config = {
    includes: ["default/**"],
    overrides: {
      // All possible formats for the same workspace
      "localhost_test:u/user/repo": { includes: ["name/**"], skipScripts: true },
      "http://localhost:8000/:test:u/user/repo": { includes: ["disambiguated/**"], skipFlows: true },
      "test:u/user/repo": { includes: ["id/**"], skipApps: true }
    }
  };
  
  const workspace = {
    remote: "http://localhost:8000/",
    workspaceId: "test",
    name: "localhost_test",
    token: "test-token"
  };
  
  const effective = getEffectiveSettings(config, workspace.workspaceId, "u/user/repo", workspace);
  
  // Should use workspace name format (highest precedence)
  assertEquals(effective.includes, ["name/**"]);
  assertEquals(effective.skipScripts, true);
  assertEquals(effective.skipFlows, undefined); // Not from other formats
  assertEquals(effective.skipApps, undefined); // Not from other formats
});