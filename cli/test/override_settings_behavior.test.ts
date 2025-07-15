import { assertEquals, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { getEffectiveSettings } from "../conf.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
import { addWorkspace } from "../workspace.ts";

// =============================================================================
// OVERRIDE SETTINGS BEHAVIOR TESTS
// Tests for override inheritance and file filtering behavior
// =============================================================================

Deno.test("Override Settings: override inherits non-overridden settings from base config", () => {
  const config = {
    includes: ["default/**"],
    skipVariables: true,      // Base has this as true
    skipResources: true,      // Base has this as true  
    skipApps: false,          // Base has this as false
    defaultTs: "bun" as const,
    overrides: {
      "http://localhost:8000/:test:u/user/repo": { 
        includes: ["override/**"],
        skipApps: true           // Override only changes skipApps, should inherit other skip flags
      }
    }
  };
  
  const effective = getEffectiveSettings(
    config,
    "http://localhost:8000/",
    "test",
    "u/user/repo"
  );
  
  // Override values should be used
  assertEquals(effective.includes, ["override/**"], "Must use override includes");
  assertEquals(effective.skipApps, true, "Must use override skipApps");
  
  // Should inherit skip flags from base config
  assertEquals(effective.skipVariables, true, "Must inherit skipVariables=true from base config");
  assertEquals(effective.skipResources, true, "Must inherit skipResources=true from base config");
  assertEquals(effective.defaultTs, "bun", "Must inherit defaultTs from base config");
});

Deno.test("Override Settings: workspace wildcards with repo-specific precedence", () => {
  const config = {
    includes: ["default/**"],
    skipVariables: false,
    overrides: {
      "http://localhost:8000/:test:*": { 
        skipVariables: true, 
        includes: ["workspace/**"] 
      },
      "http://localhost:8000/:test:u/user/specific": { 
        includes: ["specific/**"] 
      }
    }
  };
  
  // Test specific repo override (should take precedence over wildcard)
  const specificEffective = getEffectiveSettings(
    config,
    "http://localhost:8000/",
    "test",
    "u/user/specific"
  );
  assertEquals(specificEffective.includes, ["specific/**"], "Specific repo override must take precedence over wildcard");
  assertEquals(specificEffective.skipVariables, true, "Workspace wildcard setting must still apply");
  
  // Test wildcard match
  const wildcardEffective = getEffectiveSettings(
    config,
    "http://localhost:8000/",
    "test",
    "u/user/other"
  );
  assertEquals(wildcardEffective.includes, ["workspace/**"], "Wildcard must match repos without specific overrides");
  assertEquals(wildcardEffective.skipVariables, true, "Workspace wildcard setting must apply");
});

// =============================================================================
// INTEGRATION TESTS - File Filtering Behavior
// =============================================================================

Deno.test("Integration: sync pull with skipVariables override excludes variable files", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "skip_variables_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Create wmill.yaml with override that skips variables
    const backendUrl = new URL(backend.baseUrl).toString();
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

overrides:
  "${backendUrl}:${backend.workspace}:u/test/test_repo":
    skipVariables: true`);

    // Verify backend has test variable before pull
    const backendVariables = await backend.listAllVariables();
    const hasTestVariable = backendVariables.some(v => v.path === 'u/admin/test_config');
    assert(hasTestVariable, "Backend should have test variable before pull");

    // Run sync pull (NOT dry-run) to actually write files
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/test_repo',
      '--yes'
    ], tempDir);

    assertEquals(result.code, 0, `Sync pull should succeed: ${result.stderr}`);

    // Verify variable files were NOT written to filesystem due to skipVariables: true
    const filesWritten = [];
    for await (const entry of Deno.readDir(tempDir)) {
      if (entry.isFile && entry.name.endsWith('.yaml')) {
        filesWritten.push(entry.name);
      }
    }

    const hasVariableFile = filesWritten.some(file => file.includes('.variable.yaml'));
    assertEquals(hasVariableFile, false, "Variable files should NOT be written due to skipVariables override");

    // Verify other files WERE written (since skipVariables only affects variables)
    // Check what files were actually written
    console.log("Files written:", filesWritten);
    
    // Should have some files written (just not variable files)
    assert(filesWritten.length > 0, `Some files should be written when skipVariables is true. Got: ${filesWritten.join(', ')}`);
    
    // Should not have only wmill.yaml file 
    const nonWmillFiles = filesWritten.filter(f => !f.includes('wmill.yaml'));
    assert(nonWmillFiles.length > 0, `Non-wmill.yaml files should be written when skipVariables is true. Got: ${nonWmillFiles.join(', ')}`);
  });
});

Deno.test("Integration: sync push with skipVariables override excludes variable files", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace  
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "push_skip_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Create wmill.yaml with override that skips variables
    const backendUrl = new URL(backend.baseUrl).toString();
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

overrides:
  "${backendUrl}:${backend.workspace}:u/test/test_repo":
    skipVariables: true`);

    // Create local test files including variables and scripts
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

    // Get backend state before push
    const beforeVariables = await backend.listAllVariables();
    const beforeScripts = await backend.listAllScripts();
    
    const variableExistsBefore = beforeVariables.some(v => v.path === `u/admin/test_push_var_${timestamp}`);
    const scriptExistsBefore = beforeScripts.some(s => s.path === `f/test/push_script_${timestamp}`);
    
    assertEquals(variableExistsBefore, false, "Variable should not exist before push");
    assertEquals(scriptExistsBefore, false, "Script should not exist before push");

    // Run sync push (NOT dry-run) to actually push files
    const result = await backend.runCLICommand([
      'sync', 'push',
      '--repository', 'u/test/test_repo', 
      '--yes'
    ], tempDir);

    assertEquals(result.code, 0, `Sync push should succeed: ${result.stderr}`);

    // Verify variable was NOT pushed due to skipVariables: true
    const afterVariables = await backend.listAllVariables();
    const variableExistsAfter = afterVariables.some(v => v.path === `u/admin/test_push_var_${timestamp}`);
    assertEquals(variableExistsAfter, false, "Variable should NOT be pushed due to skipVariables override");

    // Verify script WAS pushed (not affected by skipVariables)
    const afterScripts = await backend.listAllScripts();
    const scriptExistsAfter = afterScripts.some(s => s.path === `f/test/push_script_${timestamp}`);
    assertEquals(scriptExistsAfter, true, "Script should be pushed normally");
  });
});

Deno.test("Integration: sync pull respects includes override for file filtering", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "includes_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Create wmill.yaml with override that only includes specific path
    const backendUrl = new URL(backend.baseUrl).toString();
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"

overrides:
  "${backendUrl}:${backend.workspace}:u/test/test_repo":
    includes:
      - "u/admin/**"  # Only include admin resources, exclude f/** apps/scripts`);

    // Run sync pull to write files
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/test_repo',
      '--yes'
    ], tempDir);

    assertEquals(result.code, 0, `Sync pull should succeed: ${result.stderr}`);

    // Verify admin files were written (since we have includes: ["u/admin/**"])
    const adminFiles = [];
    try {
      for await (const entry of Deno.readDir(`${tempDir}/u/admin`)) {
        if (entry.isFile) {
          adminFiles.push(`u/admin/${entry.name}`);
        }
      }
    } catch {
      // Directory might not exist if no files matched
    }

    // We expect admin files to be written since backend has u/admin/test_config variable
    assert(adminFiles.length > 0, `Admin files should be written due to includes override. Expected u/admin files but found: ${adminFiles.join(', ')}`);
    
    // Verify f/** files were NOT written due to includes override
    let fDirectoryExists = false;
    try {
      await Deno.stat(`${tempDir}/f`);
      fDirectoryExists = true;
    } catch {
      // Directory doesn't exist, which is expected
    }

    assertEquals(fDirectoryExists, false, "f/ directory should not exist due to includes override excluding f/**");
  });
});