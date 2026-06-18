/**
 * Datatable settings sync tests
 *
 * Tests that datatable config is correctly synced via settings.yaml during pull/push operations.
 */

import { expect, test } from "bun:test";
import { writeFile, readFile } from "node:fs/promises";
import { parse, stringify } from "yaml";
import { withTestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";
import { addWorkspace } from "../workspace.ts";

test.skipIf(shouldSkipOnCI())("Datatable config: included in sync pull settings.yaml", async () => {
  await withTestBackend(async (backend, tempDir) => {
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
    expect(configResp.ok).toBe(true);

    // Create wmill.yaml with includeSettings
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: true`, "utf-8");

    // Pull settings
    const result = await backend.runCLICommand(['sync', 'pull', '--yes'], tempDir);
    expect(result.code).toEqual(0);

    // Verify settings.yaml was created and contains datatable
    const settingsContent = await readFile(`${tempDir}/settings.yaml`, "utf-8");
    expect(settingsContent).toContain("datatable:");
    expect(settingsContent).toContain("u/test/test_db");
    expect(settingsContent).toContain("postgresql");
  });
});

test.skipIf(shouldSkipOnCI())("Datatable config: pushed correctly from settings.yaml", async () => {
  await withTestBackend(async (backend, tempDir) => {
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "datatable_push_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Create wmill.yaml with includeSettings
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: true`, "utf-8");

    // First pull to get baseline settings
    const pullResult = await backend.runCLICommand(['sync', 'pull', '--yes'], tempDir);
    expect(pullResult.code).toEqual(0);

    // Read existing settings and add datatable config
    const settingsContent = await readFile(`${tempDir}/settings.yaml`, "utf-8");
    const existingSettings = parse(settingsContent) as Record<string, unknown>;

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

    await writeFile(`${tempDir}/settings.yaml`, stringify(existingSettings), "utf-8");

    // Push the modified settings
    const pushResult = await backend.runCLICommand(['sync', 'push', '--yes'], tempDir);
    expect(pushResult.code).toEqual(0);

    // Verify the backend has the updated datatable config
    const settingsResp = await backend.apiRequest!(
      `/api/w/${backend.workspace}/workspaces/get_settings`
    );
    expect(settingsResp.ok).toBe(true);
    const settings = await settingsResp.json();

    expect(settings.datatable).toBeDefined();
    expect(settings.datatable.datatables).toBeDefined();
    expect(settings.datatable.datatables.analytics).toBeDefined();
    expect(settings.datatable.datatables.analytics.database.resource_path).toEqual("u/admin/analytics_db");
    expect(settings.datatable.datatables.analytics.database.resource_type).toEqual("postgresql");
  });
});

test.skipIf(shouldSkipOnCI())("Datatable config: empty/undefined doesn't cause errors", async () => {
  await withTestBackend(async (backend, tempDir) => {
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
    expect(clearResp.ok).toBe(true);

    // Create wmill.yaml with includeSettings
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: true`, "utf-8");

    // Pull should succeed even with empty datatable config
    const pullResult = await backend.runCLICommand(['sync', 'pull', '--yes'], tempDir);
    expect(pullResult.code).toEqual(0);

    // Settings.yaml should exist
    const settingsContent = await readFile(`${tempDir}/settings.yaml`, "utf-8");
    expect(settingsContent.length).toBeGreaterThan(0);

    // Push should also succeed
    const pushResult = await backend.runCLICommand(['sync', 'push', '--yes'], tempDir);
    expect(pushResult.code).toEqual(0);
  });
});

test.skipIf(shouldSkipOnCI())("Datatable config: round-trip preserves structure", async () => {
  await withTestBackend(async (backend, tempDir) => {
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "datatable_roundtrip_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Set up a complex datatable config on backend with multiple datatables
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
    expect(configResp.ok).toBe(true);

    // Create wmill.yaml with includeSettings
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: true`, "utf-8");

    // Pull
    const pullResult = await backend.runCLICommand(['sync', 'pull', '--yes'], tempDir);
    expect(pullResult.code).toEqual(0);

    // Push back without modification
    const pushResult = await backend.runCLICommand(['sync', 'push', '--yes'], tempDir);
    expect(pushResult.code).toEqual(0);

    // Verify the config is preserved
    const settingsResp = await backend.apiRequest!(
      `/api/w/${backend.workspace}/workspaces/get_settings`
    );
    expect(settingsResp.ok).toBe(true);
    const settings = await settingsResp.json();

    expect(settings.datatable).toBeDefined();
    expect(settings.datatable.datatables.users).toBeDefined();
    expect(settings.datatable.datatables.logs).toBeDefined();
    expect(settings.datatable.datatables.users.database.resource_path).toEqual("f/shared/users_db");
    expect(settings.datatable.datatables.logs.database.resource_type).toEqual("instance");
  });
});
