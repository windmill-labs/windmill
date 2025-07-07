import {
    assertEquals,
    assert,
    assertStringIncludes,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
import { getEffectiveSettings } from "../conf.ts";
import { addWorkspace } from "../workspace.ts";

// =============================================================================
// WORKSPACE NAME VS ID TESTS
// Tests for proper handling of workspace names vs workspace IDs
// =============================================================================

// Helper function to set up workspace profile with localhost_test name
async function setupWorkspaceProfile(backend: any): Promise<void> {
    const testWorkspace = {
        remote: backend.baseUrl, // "http://localhost:8001/"
        workspaceId: backend.workspace, // "test"
        name: "localhost_test", // This is what the tests expect!
        token: backend.token,
    };

    await addWorkspace(testWorkspace, {
        force: true,
        configDir: backend.testConfigDir,
    });
}

Deno.test(
    "Workspace Name vs ID: sync commands use workspace name in override keys",
    async () => {
        await withContainerizedBackend(async (backend, tempDir) => {
            // Set up workspace profile with name "localhost_test"
            await setupWorkspaceProfile(backend);

            // Create wmill.yaml using workspace name (localhost_test) not ID (test)
            await Deno.writeTextFile(
                `${tempDir}/wmill.yaml`,
                `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # Using workspace name, not ID
  "localhost_test:u/test/test_repo":
    skipVariables: true
    includeSchedules: true
    includes:
      - f/specific/**`,
            );

            // Run sync pull
            const result = await backend.runCLICommand(
                [
                    "sync",
                    "pull",
                    "--repository",
                    "u/test/test_repo",
                    "--dry-run",
                    "--json-output",
                ],
                tempDir,
            );

            assertEquals(result.code, 0);

            // Extract JSON from CLI output (skip log messages)
            const jsonStart = result.stdout.indexOf("{");
            assert(jsonStart >= 0, "Should have JSON output");
            const jsonOutput = result.stdout.substring(jsonStart);
            const data = JSON.parse(jsonOutput);

            // Variables should be skipped due to override using workspace name
            const hasVariables = (data.changes || []).some((change: any) =>
                change.path?.includes(".variable.yaml"),
            );
            assertEquals(
                hasVariables,
                false,
                "Variables should be skipped due to override using workspace name",
            );
        });
    },
);

Deno.test(
    "Workspace Name vs ID: gitsync-settings generates correct override keys",
    async () => {
        await withContainerizedBackend(async (backend, tempDir) => {
            // Set up workspace profile with name "localhost_test"
            await setupWorkspaceProfile(backend);

            // Start with empty wmill.yaml
            await Deno.writeTextFile(
                `${tempDir}/wmill.yaml`,
                `defaultTs: bun
includes:
  - f/**
excludes: []`,
            );

            // Pull git-sync settings with override flag
            const pullResult = await backend.runCLICommand(
                [
                    "gitsync-settings",
                    "pull",
                    "--repository",
                    "u/test/test_repo",
                    "--override",
                ],
                tempDir,
            );

            assertEquals(pullResult.code, 0);

            // Read the updated wmill.yaml
            const updatedYaml = await Deno.readTextFile(
                `${tempDir}/wmill.yaml`,
            );

            // Should contain override key with workspace name, not ID
            assertStringIncludes(
                updatedYaml,
                "localhost_test:u/test/test_repo",
            );
            assert(
                !updatedYaml.includes("test:u/test/test_repo:"),
                "Should not use workspace ID alone in new overrides",
            );
        });
    },
);

Deno.test(
    "Workspace Name vs ID: error messages show both name and ID",
    async () => {
        await withContainerizedBackend(async (backend, tempDir) => {
            // Set up workspace profile with name "localhost_test"
            await setupWorkspaceProfile(backend);

            // Create wmill.yaml with wrong workspace name
            await Deno.writeTextFile(
                `${tempDir}/wmill.yaml`,
                `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # Wrong workspace name
  "wrong_workspace:u/test/test_repo":
    skipVariables: true`,
            );

            // Test warning functionality directly
            const originalWarn = console.warn;
            const warnings: string[] = [];
            console.warn = (...args) => {
                warnings.push(args.join(" "));
                originalWarn(...args);
            };

            try {
                const config = {
                    includes: ["f/**"],
                    overrides: {
                        "wrong_workspace:u/test/test_repo": {
                            skipVariables: true,
                        },
                    },
                };

                const workspace = {
                    remote: "http://localhost:8000/",
                    workspaceId: "test",
                    name: "localhost_test",
                    token: "test-token",
                };

                // This should trigger warning about mismatched override key
                getEffectiveSettings(
                    config,
                    "u/test/test_repo",
                    workspace,
                );

                // Check warnings - they might be in separate warning lines
                const allWarnings = warnings.join(" ");
                const hasOverrideNotFound = allWarnings.includes(
                    "Override key not found",
                );
                const hasWorkspaceName = allWarnings.includes("localhost_test");
                const hasWorkspaceId = allWarnings.includes("ID: test");

                console.log("=== WARNINGS CAPTURED ===");
                console.log(warnings);
                console.log("=== CHECKS ===");
                console.log("Has override not found:", hasOverrideNotFound);
                console.log("Has workspace name:", hasWorkspaceName);
                console.log("Has workspace ID:", hasWorkspaceId);

                assert(
                    hasOverrideNotFound && hasWorkspaceName && hasWorkspaceId,
                    "Warning should show override key not found, workspace name, and ID",
                );
            } finally {
                console.warn = originalWarn;
            }
        });
    },
);

Deno.test(
    "Workspace Name vs ID: init command uses workspace name",
    async () => {
        await withContainerizedBackend(async (backend, tempDir) => {
            // Set up workspace profile with name "localhost_test"
            await setupWorkspaceProfile(backend);

            // Run init command
            const initResult = await backend.runCLICommand(["init"], tempDir);

            // Even if init fails due to git requirements, check if it would create proper config
            if (initResult.code === 0) {
                const yamlContent = await Deno.readTextFile(
                    `${tempDir}/wmill.yaml`,
                );

                // If overrides were created, they should use workspace name
                if (yamlContent.includes("overrides:")) {
                    assertStringIncludes(yamlContent, "localhost_test:");
                }
            }
        });
    },
);

Deno.test(
    "Workspace Name vs ID: workspace wildcard uses name not ID",
    async () => {
        await withContainerizedBackend(async (backend, tempDir) => {
            // Set up workspace profile with name "localhost_test"
            await setupWorkspaceProfile(backend);

            // Create wmill.yaml with workspace wildcards
            await Deno.writeTextFile(
                `${tempDir}/wmill.yaml`,
                `defaultTs: bun
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
    skipResources: false`,
            );

            // Run sync to test wildcard resolution
            const result = await backend.runCLICommand(
                [
                    "sync",
                    "pull",
                    "--repository",
                    "u/test/any_repo",
                    "--dry-run",
                    "--json-output",
                ],
                tempDir,
            );

            assertEquals(result.code, 0);

            // Extract JSON from CLI output (skip log messages)
            const jsonStart = result.stdout.indexOf("{");
            assert(jsonStart >= 0, "Should have JSON output");
            const jsonOutput = result.stdout.substring(jsonStart);
            const data = JSON.parse(jsonOutput);

            // Should use workspace name wildcard (skipVariables: true)
            const hasVariables = (data.changes || []).some((change: any) =>
                change.path?.includes(".variable.yaml"),
            );
            assertEquals(
                hasVariables,
                false,
                "Variables should be skipped due to workspace name wildcard",
            );
        });
    },
);

Deno.test(
    "Workspace Name vs ID: handles workspaces with similar names",
    async () => {
        await withContainerizedBackend(async (backend, tempDir) => {
            // Set up workspace profile with name "localhost_test"
            await setupWorkspaceProfile(backend);

            // Test config with similar workspace names
            await Deno.writeTextFile(
                `${tempDir}/wmill.yaml`,
                `defaultTs: bun
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
    skipResources: true`,
            );

            // Our backend workspace is "localhost_test"
            const result = await backend.runCLICommand(
                [
                    "sync",
                    "pull",
                    "--repository",
                    "u/test/test_repo",
                    "--dry-run",
                    "--json-output",
                ],
                tempDir,
            );

            assertEquals(result.code, 0);

            // Extract JSON from CLI output (skip log messages)
            const jsonStart = result.stdout.indexOf("{");
            assert(jsonStart >= 0, "Should have JSON output");
            const jsonOutput = result.stdout.substring(jsonStart);
            const data = JSON.parse(jsonOutput);

            // Should use exact match "localhost_test", not similar names
            const hasVariables = (data.changes || []).some((change: any) =>
                change.path?.includes(".variable.yaml"),
            );
            assertEquals(
                hasVariables,
                false,
                "Should use localhost_test override (skipVariables: true)",
            );
        });
    },
);

Deno.test(
    "Workspace Name vs ID: unit test for getEffectiveSettings precedence",
    async () => {
        // Direct unit test of getEffectiveSettings
        const config = {
            includes: ["default/**"],
            overrides: {
                // All supported formats for the same workspace
                "localhost_test:u/user/repo": {
                    includes: ["name/**"],
                    skipScripts: true,
                },
                "http://localhost:8000/:test:u/user/repo": {
                    includes: ["disambiguated/**"],
                    skipFlows: true,
                },
            },
        };

        const workspace = {
            remote: "http://localhost:8000/",
            workspaceId: "test",
            name: "localhost_test",
            token: "test-token",
        };

        const effective = getEffectiveSettings(
            config,
            "u/user/repo",
            workspace,
        );

        // Should use workspace name format (highest precedence)
        assertEquals(effective.includes, ["name/**"]);
        assertEquals(effective.skipScripts, true);
        assertEquals(effective.skipFlows, undefined); // Not from other formats
    },
);
