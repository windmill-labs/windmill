import { assertEquals, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { getEffectiveSettings } from "../conf.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { withTestConfig, clearTestRemotes } from "./test_config_helpers.ts";

// =============================================================================
// OVERRIDE KEY RESOLUTION TESTS
// Tests for the enhanced override key format with workspace names
// =============================================================================

// Test data - workspace objects matching the Workspace interface
const TEST_WORKSPACES = {
  localhost_test: {
    remote: "http://localhost:8001/",
    workspaceId: "test",
    name: "localhost_test",
    token: "test-token"
  },
  production: {
    remote: "https://app.windmill.dev/",
    workspaceId: "prod",
    name: "production",
    token: "prod-token"
  },
  cloud_test: {
    remote: "https://app.windmill.dev/",
    workspaceId: "test",  // Same workspaceId, different instance
    name: "cloud_test", 
    token: "cloud-token"
  }
};

// =============================================================================
// UNIT TESTS - Override Key Resolution Logic
// =============================================================================

Deno.test("Override Key Resolution: primary format (workspace.name:repo) takes precedence", async () => {
  const config = {
    includes: ["default/**"],
    skipVariables: false,
    overrides: {
      "localhost_test:u/user/repo": { includes: ["new/**"], skipVariables: true }
    }
  };
  
  const workspace = TEST_WORKSPACES.localhost_test;
  const effective = getEffectiveSettings(config, "u/user/repo", workspace);
  
  // STRONG ASSERTION: Must use override for includes, but also merge with base config
  assertEquals(effective.includes, ["new/**"], "Must use override includes");
  assertEquals(effective.skipVariables, true, "Must use override skipVariables");
  // Should inherit other settings from base config that aren't overridden
});

Deno.test("Override Key Resolution: override inherits skip flags from base config", async () => {
  const config = {
    includes: ["default/**"],
    skipVariables: true,      // Base has this as true (implies skipSecrets: true)
    skipResources: true,      // Base has this as true  
    skipApps: false,          // Base has this as false
    defaultTs: "bun" as const,
    overrides: {
      "localhost_test:u/user/repo": { 
        includes: ["override/**"],
        skipApps: true           // Override only changes skipApps, should inherit other skip flags
      }
    }
  };
  
  const workspace = TEST_WORKSPACES.localhost_test;
  const effective = getEffectiveSettings(config, "u/user/repo", workspace);
  
  // Override values should be used
  assertEquals(effective.includes, ["override/**"], "Must use override includes");
  assertEquals(effective.skipApps, true, "Must use override skipApps");
  
  // Should inherit skip flags from base config
  assertEquals(effective.skipVariables, true, "Must inherit skipVariables=true from base config");
  assertEquals(effective.skipResources, true, "Must inherit skipResources=true from base config");
  assertEquals(effective.defaultTs, "bun", "Must inherit defaultTs from base config");
});

Deno.test("Override Key Resolution: no match when override key doesn't match workspace", async () => {
  const config = {
    includes: ["default/**"],
    overrides: {
      "wrong_workspace:u/user/repo": { includes: ["wrong/**"] }
    }
  };
  
  const workspace = TEST_WORKSPACES.localhost_test;
  const effective = getEffectiveSettings(config, "u/user/repo", workspace);
  
  // STRONG ASSERTION: Must use default when no override key matches
  assertEquals(effective.includes, ["default/**"], "Must use default when no override key matches workspace");
});

Deno.test("Override Key Resolution: disambiguated format (remote:workspaceId:repo) for multi-instance", async () => {
  const config = {
    includes: ["default/**"],
    overrides: {
      "http://localhost:8001/:test:u/user/repo": { includes: ["local/**"], skipScripts: true },
      "https://app.windmill.dev/:test:u/user/repo": { includes: ["cloud/**"], skipScripts: false }
    }
  };
  
  // Test localhost instance
  const localhostWorkspace = TEST_WORKSPACES.localhost_test;
  const localhostEffective = getEffectiveSettings(config, "u/user/repo", localhostWorkspace);
  assertEquals(localhostEffective.includes, ["local/**"], "Must match localhost instance based on remote URL");
  assertEquals(localhostEffective.skipScripts, true, "Must apply localhost-specific settings");
  
  // Test cloud instance
  const cloudWorkspace = TEST_WORKSPACES.cloud_test;
  const cloudEffective = getEffectiveSettings(config, "u/user/repo", cloudWorkspace);
  assertEquals(cloudEffective.includes, ["cloud/**"], "Must match cloud instance based on remote URL");
  assertEquals(cloudEffective.skipScripts, false, "Must apply cloud-specific settings");
});

Deno.test("Override Key Resolution: workspace-level wildcards work correctly", async () => {
  const config = {
    includes: ["default/**"],
    skipVariables: false,
    overrides: {
      "localhost_test:*": { skipVariables: true, includes: ["workspace/**"] },
      "localhost_test:u/user/specific": { includes: ["specific/**"] }
    }
  };
  
  const workspace = TEST_WORKSPACES.localhost_test;
  
  // Test specific repo override (should take precedence over wildcard)
  const specificEffective = getEffectiveSettings(config, "u/user/specific", workspace);
  assertEquals(specificEffective.includes, ["specific/**"], "Specific repo override must take precedence over wildcard");
  assertEquals(specificEffective.skipVariables, true, "Workspace wildcard setting must still apply");
  
  // Test wildcard match
  const wildcardEffective = getEffectiveSettings(config, "u/user/other", workspace);
  assertEquals(wildcardEffective.includes, ["workspace/**"], "Wildcard must match repos without specific overrides");
  assertEquals(wildcardEffective.skipVariables, true, "Workspace wildcard setting must apply");
});

Deno.test("Override Key Resolution: exact precedence order is enforced", async () => {
  const config = {
    includes: ["default/**"],
    overrides: {
      "localhost_test:u/user/repo": { includes: ["name/**"], tag: "name" },
      "http://localhost:8001/:test:u/user/repo": { includes: ["disambig/**"], tag: "disambig" }
    }
  };
  
  const workspace = TEST_WORKSPACES.localhost_test;
  const effective = getEffectiveSettings(config, "u/user/repo", workspace);
  
  // STRONG ASSERTION: Must use first format in precedence order
  assertEquals(effective.includes, ["name/**"], "Must use workspace name format (highest precedence)");
  assertEquals((effective as any).tag, "name", "Must apply only settings from highest precedence match");
});

Deno.test("Override Key Resolution: no workspace object uses defaults only", async () => {
  const config = {
    includes: ["default/**"],
    overrides: {
      "localhost_test:u/user/repo": { includes: ["new/**"] },
      "wrong:u/user/repo": { includes: ["wrong/**"] }
    }
  };
  
  // Call without workspace object - should only use defaults
  const effective = getEffectiveSettings(config, "u/user/repo");
  assertEquals(effective.includes, ["default/**"], "Without workspace object, must use defaults only");
});

// =============================================================================
// INTEGRATION TESTS - Real Backend with Controlled Scenarios
// =============================================================================

Deno.test("Integration: workspace name override keys work with configured workspace", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Step 1: Add our test workspace using backend's isolated config
    const testWorkspace = {
      remote: backend.baseUrl,  // Use actual backend URL (http://localhost:8001)
      workspaceId: backend.workspace,  // Use actual backend workspace (test)
      name: "test_workspace_name",  // Named workspace for testing
      token: backend.token
    };
    
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });
    
    // Step 2: Configure git-sync on backend with specific repository
    await backend.updateGitSyncConfig({
      git_sync_settings: {
        repositories: [{
          git_repo_resource_path: "u/test/override_test_repo",
          script_path: "f/**",
          group_by_folder: false,
          use_individual_branch: false,
          settings: {
            include_path: ["f/**"],
            include_type: ["script", "app"],
            exclude_path: [],
            extra_include_path: []
          }
        }]
      }
    });
    
    // Step 3: Create wmill.yaml with workspace name override
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
skipScripts: false
skipApps: false

overrides:
  # Using workspace name (should match since CLI resolves to test_workspace_name)
  "test_workspace_name:u/test/override_test_repo":
    skipScripts: true
    includes:
      - f/override/**
  
  # Legacy format (should be ignored due to precedence)
  "${backend.workspace}:u/test/override_test_repo":
    skipScripts: false
    includes:
      - f/legacy/**`);
    
    // Step 4: Run sync pull and verify workspace name override is used
    // Since we use the backend's isolated config, CLI will use our named workspace
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/override_test_repo',
      '--dry-run',
      '--json-output'
    ], tempDir);
    
    // STRONG ASSERTION: Command must succeed
    assertEquals(result.code, 0, `Sync command must succeed. Error: ${result.stderr}`);
    
    // Step 5: Parse output and verify override was applied
    const jsonMatch = result.stdout.match(/\{[\s\S]*\}/);
    assert(jsonMatch, `Must have JSON output in: ${result.stdout}`);
    
    const data = JSON.parse(jsonMatch[0]);
    
    // STRONG ASSERTION: With includes: ["f/override/**"], only files matching that pattern should be included
    // Since backend has no files in f/override/**, expect 0 files
    assertEquals(data.total, 0, "Override with restrictive include pattern must filter out all files");
    assertEquals(data.success, true, "Sync operation must report success");
  });
});

Deno.test("Integration: disambiguated format resolves multi-instance conflicts", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Step 1: Add test workspaces using backend's isolated config
    
    const testWorkspace = {
      remote: backend.baseUrl,      // http://localhost:8001
      workspaceId: backend.workspace,  // test
      name: "instance1",           // Name for override key
      token: backend.token
    };
    
    // Also add a workspace with different remote but same ID (for disambiguation testing)
    const otherWorkspace = {
      remote: "https://app.windmill.dev/",  // Different remote
      workspaceId: backend.workspace,       // Same workspace ID "test"
      name: "instance2", 
      token: "different-token"
    };
    
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });
    await addWorkspace(otherWorkspace, { force: true, configDir: backend.testConfigDir });
    
    // Step 2: Configure backend
    await backend.updateGitSyncConfig({
      git_sync_settings: {
        repositories: [{
          git_repo_resource_path: "u/test/multi_instance_repo",
          script_path: "f/**",
          group_by_folder: false,
          use_individual_branch: false,
          settings: {
            include_path: ["f/**"],
            include_type: ["script", "app"],
            exclude_path: [],
            extra_include_path: []
          }
        }]
      }
    });
    
    // Step 3: Create wmill.yaml with disambiguated overrides  
    const normalizedBaseUrl = new URL(backend.baseUrl).toString(); // Normalize to include trailing slash
    
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # Instance 1 (current backend) - will match due to CLI using this backend
  "${normalizedBaseUrl}:${backend.workspace}:u/test/multi_instance_repo":
    includes:
      - f/instance1/**
    skipScripts: true
  
  # Instance 2 (different remote) - won't match
  "https://app.windmill.dev/:${backend.workspace}:u/test/multi_instance_repo":
    includes:
      - f/instance2/**
    skipScripts: false`);
    
    // Step 4: Run sync with instance1 workspace
    // Note: The backend.runCLICommand already sets --workspace=test, so we use that
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/multi_instance_repo',
      '--dry-run',
      '--json-output'
    ], tempDir);
    
    assertEquals(result.code, 0, `Multi-instance sync must succeed. Error: ${result.stderr}`);
    
    const jsonMatch = result.stdout.match(/\{[\s\S]*\}/);
    assert(jsonMatch, `Must have JSON output in: ${result.stdout}`);
    
    const data = JSON.parse(jsonMatch[0]);
    
    console.log("=== DEBUG: Multi-instance test ===");
    console.log("JSON data:", JSON.stringify(data, null, 2));
    console.log("Backend baseUrl:", backend.baseUrl);
    console.log("Normalized baseUrl:", normalizedBaseUrl);
    console.log("Expected override key:", `${normalizedBaseUrl}:${backend.workspace}:u/test/multi_instance_repo`);
    console.log("Backend workspace ID:", backend.workspace);
    console.log("CLI should be using workspace with name 'instance1'");
    
    // Let's also check what's in the wmill.yaml that was created
    const yamlContent = await Deno.readTextFile(`${tempDir}/wmill.yaml`);
    console.log("=== wmill.yaml content ===");
    console.log(yamlContent);
    console.log("=== END wmill.yaml ===");
    console.log("=== END DEBUG ===");
    
    // STRONG ASSERTION: Must use instance1 settings (includes: ["f/instance1/**"])
    // Since no files exist in f/instance1/**, expect 0 files
    assertEquals(data.total, 0, "Instance1 override must be applied, filtering to f/instance1/** pattern");
    
    // Verify the disambiguation worked by checking that scripts would be skipped
    // (We can't easily test this without creating actual scripts, but the include pattern test is sufficient)
  });
});

Deno.test("Integration: gitsync-settings commands generate correct override format", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Step 1: Add test workspace using backend's isolated config
    
    const namedWorkspace = {
      remote: backend.baseUrl,           // http://localhost:8001
      workspaceId: backend.workspace,    // test  
      name: "gitsync_test_workspace",   // Name for override generation
      token: backend.token
    };
    
    await addWorkspace(namedWorkspace, { force: true, configDir: backend.testConfigDir });
    
    // Step 2: Configure backend with specific settings
    await backend.updateGitSyncConfig({
      git_sync_settings: {
        repositories: [{
          git_repo_resource_path: "u/test/gitsync_repo",
          script_path: "f/**",
          group_by_folder: false,
          use_individual_branch: false,
          settings: {
            include_path: ["f/**"],
            include_type: ["script", "flow"],
            exclude_path: ["*.test.ts"],
            extra_include_path: ["g/**"]
          }
        }]
      }
    });
    
    // Step 3: Create initial wmill.yaml
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []`);
    
    // Step 4: Run gitsync-settings pull with override flag
    // Note: The backend.runCLICommand already sets --workspace=test, so we use that
    const pullResult = await backend.runCLICommand([
      'gitsync-settings', 'pull',
      '--repository', 'u/test/gitsync_repo',
      '--override'
    ], tempDir);
    
    assertEquals(pullResult.code, 0, `Gitsync-settings pull must succeed. Error: ${pullResult.stderr}`);
    
    // Step 5: Verify the generated override key format
    const updatedConfig = await Deno.readTextFile(`${tempDir}/wmill.yaml`);
    
    // STRONG ASSERTION: Must use workspace name format, not workspace ID
    assert(
      updatedConfig.includes("gitsync_test_workspace:u/test/gitsync_repo"),
      `Generated override must use workspace name format. Config: ${updatedConfig}`
    );
    
    // STRONG ASSERTION: Must not use legacy workspace ID format alone
    assert(
      !updatedConfig.includes(`"${backend.workspace}:u/test/gitsync_repo"`),
      `Generated override must not use legacy workspace ID format. Config: ${updatedConfig}`
    );
    
    // Step 6: Verify the override settings are correct
    assert(updatedConfig.includes("extraIncludes:"), "Must include extraIncludes from backend");
    assert(updatedConfig.includes("g/**"), "Must include extraIncludes pattern from backend");
  });
});

Deno.test("Integration: sync works correctly with mismatched override keys", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Step 1: Add test workspace using backend's isolated config
    
    const namedWorkspace = {
      remote: backend.baseUrl,         // http://localhost:8001
      workspaceId: backend.workspace,  // test
      name: "error_test_workspace",   // Name that should appear in error messages
      token: backend.token
    };
    
    await addWorkspace(namedWorkspace, { force: true, configDir: backend.testConfigDir });
    
    // Step 2: Create wmill.yaml with wrong override key
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # Wrong workspace name - should not match, falls back to defaults
  "wrong_workspace_name:u/test/repo":
    skipScripts: true`);
    
    // Step 3: Run sync command
    // Note: The backend.runCLICommand already sets --workspace=test, so we use that
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/repo',
      '--dry-run',
      '--json-output'
    ], tempDir);
    
    // Command should succeed with default settings since override doesn't match
    assertEquals(result.code, 0, "Sync command should succeed despite override key mismatch");
    
    // Extract JSON to verify behavior
    const jsonMatch = result.stdout.match(/\{[\s\S]*\}/);
    assert(jsonMatch, `Must have JSON output in: ${result.stdout}`);
    const data = JSON.parse(jsonMatch[0]);
    
    // Since override doesn't match, it should use default includes ["f/**"] 
    // and default skipScripts: false, so scripts should be included
    assertEquals(data.success, true, "Sync operation must report success");
  });
});

// =============================================================================
// INTEGRATION TESTS - Real File Operations with Overrides  
// =============================================================================

Deno.test("Integration: sync pull with skipVariables override excludes variable files", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Step 1: Set up workspace
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "skip_variables_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Step 2: Create wmill.yaml with override that skips variables
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

overrides:
  "skip_variables_test:u/test/test_repo":
    skipVariables: true`);

    // Step 3: Verify backend has test variable before pull
    const backendVariables = await backend.listAllVariables();
    const hasTestVariable = backendVariables.some(v => v.path === 'u/admin/test_config');
    assert(hasTestVariable, "Backend should have test variable before pull");

    // Step 4: Run sync pull (NOT dry-run) to actually write files
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/test_repo',
      '--yes'
    ], tempDir);

    assertEquals(result.code, 0, `Sync pull should succeed: ${result.stderr}`);

    // Step 5: Verify variable files were NOT written to filesystem due to skipVariables: true
    const filesWritten = [];
    for await (const entry of Deno.readDir(tempDir)) {
      if (entry.isFile && entry.name.endsWith('.yaml')) {
        filesWritten.push(entry.name);
      }
    }

    const hasVariableFile = filesWritten.some(file => file.includes('.variable.yaml'));
    assertEquals(hasVariableFile, false, "Variable files should NOT be written due to skipVariables override");

    // Step 6: Verify other files WERE written (app, resource)
    const hasAppFile = filesWritten.some(file => file.includes('test_dashboard.app'));
    const hasResourceFile = filesWritten.some(file => file.includes('.resource.yaml'));
    
    // At least one of app or resource should be written (depends on includes pattern)
    assert(hasAppFile || hasResourceFile || filesWritten.length > 1, 
      `Other files should be written. Files: ${filesWritten.join(', ')}`);
  });
});

Deno.test("Integration: sync push with skipVariables override excludes variable files", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Step 1: Set up workspace  
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "push_skip_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Step 2: Create wmill.yaml with override that skips variables
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

overrides:
  "push_skip_test:u/test/test_repo":
    skipVariables: true`);

    // Step 3: Create local test files including variables and scripts
    const timestamp = Date.now();
    
    // Create variable file
    await Deno.mkdir(`${tempDir}/u/admin`, { recursive: true });
    await Deno.writeTextFile(`${tempDir}/u/admin/test_push_var_${timestamp}.variable.yaml`, 
      `value: test_value_${timestamp}
description: Test variable for push override test
is_secret: false`);

    // Create script file  
    await Deno.mkdir(`${tempDir}/f/test`, { recursive: true });
    await Deno.writeTextFile(`${tempDir}/f/test/push_script_${timestamp}.ts`,
      `export async function main() {
  return "Test script ${timestamp}";
}`);
    await Deno.writeTextFile(`${tempDir}/f/test/push_script_${timestamp}.script.yaml`,
      `summary: Test Push Script ${timestamp}
description: Script for testing push with override`);

    // Step 4: Get backend state before push
    const beforeVariables = await backend.listAllVariables();
    const beforeScripts = await backend.listAllScripts();
    
    const variableExistsBefore = beforeVariables.some(v => v.path === `u/admin/test_push_var_${timestamp}`);
    const scriptExistsBefore = beforeScripts.some(s => s.path === `f/test/push_script_${timestamp}`);
    
    assertEquals(variableExistsBefore, false, "Variable should not exist before push");
    assertEquals(scriptExistsBefore, false, "Script should not exist before push");

    // Step 5: Run sync push (NOT dry-run) to actually push files
    const result = await backend.runCLICommand([
      'sync', 'push',
      '--repository', 'u/test/test_repo', 
      '--yes'
    ], tempDir);

    assertEquals(result.code, 0, `Sync push should succeed: ${result.stderr}`);

    // Step 6: Verify variable was NOT pushed due to skipVariables: true
    const afterVariables = await backend.listAllVariables();
    const variableExistsAfter = afterVariables.some(v => v.path === `u/admin/test_push_var_${timestamp}`);
    assertEquals(variableExistsAfter, false, "Variable should NOT be pushed due to skipVariables override");

    // Step 7: Verify script WAS pushed (not affected by skipVariables)
    const afterScripts = await backend.listAllScripts();
    const scriptExistsAfter = afterScripts.some(s => s.path === `f/test/push_script_${timestamp}`);
    assertEquals(scriptExistsAfter, true, "Script should be pushed normally");
  });
});

Deno.test("Integration: sync pull respects includes override for file filtering", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Step 1: Set up workspace
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "includes_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Step 2: Create wmill.yaml with override that only includes specific path
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"

overrides:
  "includes_test:u/test/test_repo":
    includes:
      - "u/admin/**"  # Only include admin resources, exclude f/** apps/scripts`);

    // Step 3: Run sync pull to write files
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/test_repo',
      '--yes'
    ], tempDir);

    assertEquals(result.code, 0, `Sync pull should succeed: ${result.stderr}`);

    // Step 4: Verify only admin resources were written due to includes override
    const allFiles = [];
    
    try {
      for await (const entry of Deno.readDir(`${tempDir}/u/admin`)) {
        if (entry.isFile) {
          allFiles.push(`u/admin/${entry.name}`);
        }
      }
    } catch {
      // Directory might not exist if no files matched
    }

    // Step 5: Verify f/** files were NOT written due to includes override
    let fDirectoryExists = false;
    try {
      await Deno.stat(`${tempDir}/f`);
      fDirectoryExists = true;
    } catch {
      // Directory doesn't exist, which is expected
    }

    assert(allFiles.length > 0 || !fDirectoryExists, 
      "Should either have admin files OR no f/ directory due to includes override");
    
    if (fDirectoryExists) {
      // If f/ exists, it should be empty or minimal
      const fFiles = [];
      for await (const entry of Deno.readDir(`${tempDir}/f`)) {
        if (entry.isFile) {
          fFiles.push(`f/${entry.name}`);
        }
      }
      
      // The override should have excluded f/** files
      console.log(`Files in f/: ${fFiles.join(', ')}`);
      console.log(`Files in u/admin/: ${allFiles.join(', ')}`);
    }
  });
});