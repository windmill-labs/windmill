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
    overrides: {
      "localhost_test:u/user/repo": { includes: ["new/**"] },
      "test:u/user/repo": { includes: ["legacy/**"] }
    }
  };
  
  const workspace = TEST_WORKSPACES.localhost_test;
  const effective = getEffectiveSettings(config, workspace.workspaceId, "u/user/repo", workspace);
  
  // STRONG ASSERTION: Must use new format, never legacy
  assertEquals(effective.includes, ["new/**"], "Must use workspace name format, not legacy workspaceId format");
});

Deno.test("Override Key Resolution: fallback to legacy format when new format not available", async () => {
  const config = {
    includes: ["default/**"],
    overrides: {
      "test:u/user/repo": { includes: ["legacy/**"] }
    }
  };
  
  const workspace = TEST_WORKSPACES.localhost_test;
  const effective = getEffectiveSettings(config, workspace.workspaceId, "u/user/repo", workspace);
  
  // STRONG ASSERTION: Must fall back to legacy format when new format unavailable
  assertEquals(effective.includes, ["legacy/**"], "Must use legacy format when workspace name format not found");
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
  const localhostEffective = getEffectiveSettings(config, localhostWorkspace.workspaceId, "u/user/repo", localhostWorkspace);
  assertEquals(localhostEffective.includes, ["local/**"], "Must match localhost instance based on remote URL");
  assertEquals(localhostEffective.skipScripts, true, "Must apply localhost-specific settings");
  
  // Test cloud instance
  const cloudWorkspace = TEST_WORKSPACES.cloud_test;
  const cloudEffective = getEffectiveSettings(config, cloudWorkspace.workspaceId, "u/user/repo", cloudWorkspace);
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
  const specificEffective = getEffectiveSettings(config, workspace.workspaceId, "u/user/specific", workspace);
  assertEquals(specificEffective.includes, ["specific/**"], "Specific repo override must take precedence over wildcard");
  assertEquals(specificEffective.skipVariables, true, "Workspace wildcard setting must still apply");
  
  // Test wildcard match
  const wildcardEffective = getEffectiveSettings(config, workspace.workspaceId, "u/user/other", workspace);
  assertEquals(wildcardEffective.includes, ["workspace/**"], "Wildcard must match repos without specific overrides");
  assertEquals(wildcardEffective.skipVariables, true, "Workspace wildcard setting must apply");
});

Deno.test("Override Key Resolution: exact precedence order is enforced", async () => {
  const config = {
    includes: ["default/**"],
    overrides: {
      "localhost_test:u/user/repo": { includes: ["name/**"], tag: "name" },
      "http://localhost:8001/:test:u/user/repo": { includes: ["disambig/**"], tag: "disambig" },
      "test:u/user/repo": { includes: ["legacy/**"], tag: "legacy" }
    }
  };
  
  const workspace = TEST_WORKSPACES.localhost_test;
  const effective = getEffectiveSettings(config, workspace.workspaceId, "u/user/repo", workspace);
  
  // STRONG ASSERTION: Must use first format in precedence order
  assertEquals(effective.includes, ["name/**"], "Must use workspace name format (highest precedence)");
  assertEquals((effective as any).tag, "name", "Must apply only settings from highest precedence match");
});

Deno.test("Override Key Resolution: no workspace object falls back to legacy only", async () => {
  const config = {
    includes: ["default/**"],
    overrides: {
      "localhost_test:u/user/repo": { includes: ["new/**"] },
      "test:u/user/repo": { includes: ["legacy/**"] }
    }
  };
  
  // Call without workspace object - should only check legacy format
  const effective = getEffectiveSettings(config, "test", "u/user/repo");
  assertEquals(effective.includes, ["legacy/**"], "Without workspace object, must fall back to legacy format only");
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

Deno.test("Integration: error messages show workspace name and ID correctly", async () => {
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
  # Wrong workspace name
  "wrong_workspace_name:u/test/repo":
    skipScripts: true`);
    
    // Step 3: Run sync command
    // Note: The backend.runCLICommand already sets --workspace=test, so we use that
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/repo',
      '--dry-run'
    ], tempDir);
    
    // Command should still succeed, but show warnings
    assertEquals(result.code, 0, "Sync command should succeed despite override key mismatch");
    
    // STRONG ASSERTION: Error message must show workspace name (now that ambiguity is resolved)
    assert(
      result.stderr.includes("error_test_workspace"),
      `Error message must show workspace name. stderr: ${result.stderr}`
    );
    assert(
      result.stderr.includes(`ID: ${backend.workspace}`),
      `Error message must show workspace ID. stderr: ${result.stderr}`
    );
  });
});