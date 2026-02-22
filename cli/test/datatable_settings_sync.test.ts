/**
 * Datatable settings sync tests
 *
 * Tests that datatable config is correctly synced via settings.yaml during pull/push operations.
 */

import { assertEquals, assertStringIncludes, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";

Deno.test({
  name: "Datatable config: included in sync pull settings.yaml",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "datatable_pull_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Configure datatable on backend
      const datatableConfig = {
        datatables: {
          main: {
            database: {
              resource_path: "u/test/test_db",
              resource_type: "postgresql"
            }
          }
        }
      };

      const configResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/edit_datatable_config`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ settings: datatableConfig })
        }
      );
      assertEquals(
        configResp.ok,
        true,
        `Failed to set datatable config: ${await configResp.text()}`
      );

      // Create wmill.yaml with includeSettings
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: true`);

      // Pull settings
      const result = await backend.runCLICommand(['sync', 'pull', '--yes'], tempDir);
      assertEquals(result.code, 0, `Pull should succeed: ${result.stderr}`);

      // Verify settings.yaml was created and contains datatable
      const settingsContent = await Deno.readTextFile(`${tempDir}/settings.yaml`);
      assertStringIncludes(settingsContent, "datatable:");
      assertStringIncludes(settingsContent, "u/test/test_db");
      assertStringIncludes(settingsContent, "postgresql");
    });
  }
});

Deno.test({
  name: "Datatable config: pushed correctly from settings.yaml",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "datatable_push_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml with includeSettings
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: true`);

      // First pull to get baseline settings
      const pullResult = await backend.runCLICommand(['sync', 'pull', '--yes'], tempDir);
      assertEquals(pullResult.code, 0, `Initial pull should succeed: ${pullResult.stderr}`);

      // Read existing settings and modify datatable config
      let settingsContent = await Deno.readTextFile(`${tempDir}/settings.yaml`);

      // Parse existing settings as YAML and add datatable config
      const { parse, stringify } = await import("https://deno.land/std@0.224.0/yaml/mod.ts");
      const existingSettings = parse(settingsContent) as Record<string, unknown>;

      // Add datatable config
      existingSettings.datatable = {
        datatables: {
          analytics: {
            database: {
              resource_path: "u/admin/analytics_db",
              resource_type: "postgresql"
            }
          }
        }
      };

      await Deno.writeTextFile(`${tempDir}/settings.yaml`, stringify(existingSettings));

      // Push the modified settings
      const pushResult = await backend.runCLICommand(['sync', 'push', '--yes'], tempDir);
      assertEquals(pushResult.code, 0, `Push should succeed: ${pushResult.stderr}`);

      // Verify the backend has the updated datatable config
      const settingsResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/get_settings`
      );
      assertEquals(settingsResp.ok, true, "Failed to get workspace settings");
      const settings = await settingsResp.json();

      assert(settings.datatable, "Datatable config should exist in settings");
      assert(settings.datatable.datatables, "Datatables should exist");
      assert(
        settings.datatable.datatables.analytics,
        "Analytics datatable should exist"
      );
      assertEquals(
        settings.datatable.datatables.analytics.database.resource_path,
        "u/admin/analytics_db",
        "Resource path should match"
      );
      assertEquals(
        settings.datatable.datatables.analytics.database.resource_type,
        "postgresql",
        "Resource type should match"
      );
    });
  }
});

Deno.test({
  name: "Datatable config: empty/undefined doesn't cause errors",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "datatable_empty_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Ensure datatable config is empty/cleared on backend
      const clearResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/edit_datatable_config`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ settings: { datatables: {} } })
        }
      );
      assertEquals(
        clearResp.ok,
        true,
        `Failed to clear datatable config: ${await clearResp.text()}`
      );

      // Create wmill.yaml with includeSettings
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: true`);

      // Pull should succeed even with empty datatable config
      const pullResult = await backend.runCLICommand(['sync', 'pull', '--yes'], tempDir);
      assertEquals(pullResult.code, 0, `Pull should succeed with empty datatable: ${pullResult.stderr}`);

      // Settings.yaml should exist
      const settingsContent = await Deno.readTextFile(`${tempDir}/settings.yaml`);
      assert(settingsContent.length > 0, "Settings file should have content");

      // Push should also succeed
      const pushResult = await backend.runCLICommand(['sync', 'push', '--yes'], tempDir);
      assertEquals(pushResult.code, 0, `Push should succeed with empty datatable: ${pushResult.stderr}`);
    });
  }
});

Deno.test({
  name: "Datatable config: round-trip preserves structure",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "datatable_roundtrip_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Set up a complex datatable config on backend with multiple datatables
      // Note: resource_type only accepts "postgresql" or "instance"
      const originalConfig = {
        datatables: {
          users: {
            database: {
              resource_path: "f/shared/users_db",
              resource_type: "postgresql"
            }
          },
          logs: {
            database: {
              resource_path: "f/shared/logs_db",
              resource_type: "instance"
            }
          }
        }
      };

      const configResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/edit_datatable_config`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ settings: originalConfig })
        }
      );
      assertEquals(
        configResp.ok,
        true,
        `Failed to set datatable config: ${await configResp.text()}`
      );

      // Create wmill.yaml with includeSettings
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: true`);

      // Pull
      const pullResult = await backend.runCLICommand(['sync', 'pull', '--yes'], tempDir);
      assertEquals(pullResult.code, 0, `Pull should succeed: ${pullResult.stderr}`);

      // Push back without modification
      const pushResult = await backend.runCLICommand(['sync', 'push', '--yes'], tempDir);
      assertEquals(pushResult.code, 0, `Push should succeed: ${pushResult.stderr}`);

      // Verify the config is preserved
      const settingsResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/get_settings`
      );
      assertEquals(settingsResp.ok, true, "Failed to get workspace settings");
      const settings = await settingsResp.json();

      assert(settings.datatable, "Datatable config should exist");
      assert(settings.datatable.datatables.users, "Users datatable should exist");
      assert(settings.datatable.datatables.logs, "Logs datatable should exist");
      assertEquals(
        settings.datatable.datatables.users.database.resource_path,
        "f/shared/users_db",
        "Users resource path should be preserved"
      );
      assertEquals(
        settings.datatable.datatables.logs.database.resource_type,
        "instance",
        "Logs resource type should be preserved"
      );
    });
  }
});
