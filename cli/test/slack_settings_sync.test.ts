/**
 * Slack settings sync tests.
 *
 * Covers the "omit = clear" normalization applied to error_handler,
 * success_handler, slack_oauth_client_id, and slack_oauth_client_secret:
 *
 *   - pull always emits these 4 fields (null when remote is NULL) so that
 *     settings.yaml is a complete snapshot of remote state.
 *   - push treats the YAML as canonical: absence and explicit null both clear;
 *     a populated value upserts.
 *   - Round-trip (pull → push with no edits) is idempotent.
 *
 * Mirrors the structure of `datatable_settings_sync.test.ts`. Requires a live
 * backend (skipped on CI by `shouldSkipOnCI()`).
 */

import { expect, test } from "bun:test";
import { writeFile, readFile } from "node:fs/promises";
import { parse, stringify } from "yaml";
import { withTestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";
import { addWorkspace } from "../workspace.ts";

// When the remote has no error_handler / success_handler / slack_oauth_client_*
// configured, a sync pull should still emit the keys — as `null` — so that the
// YAML is self-describing and round-trip is bijective.
test.skipIf(shouldSkipOnCI())(
  "Slack sync: pull emits null for unset handlers + oauth override",
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "slack_pull_null_test",
        token: backend.token,
      };
      await addWorkspace(testWorkspace, {
        force: true,
        configDir: backend.testConfigDir,
      });

      // Clear any prior state from other tests.
      await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/edit_error_handler`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({}),
        },
      );
      await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/edit_success_handler`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({}),
        },
      );
      await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/slack_oauth_config`,
        { method: "DELETE" },
      );

      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
includeSettings: true`,
        "utf-8",
      );

      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir,
      );
      expect(pullResult.code).toEqual(0);

      const settingsYaml = await readFile(`${tempDir}/settings.yaml`, "utf-8");
      const settings = parse(settingsYaml) as Record<string, unknown>;

      // All four keys MUST be present and null — not absent.
      expect("error_handler" in settings).toBe(true);
      expect(settings.error_handler).toBeNull();
      expect("success_handler" in settings).toBe(true);
      expect(settings.success_handler).toBeNull();
      expect("slack_oauth_client_id" in settings).toBe(true);
      expect(settings.slack_oauth_client_id).toBeNull();
      expect("slack_oauth_client_secret" in settings).toBe(true);
      expect(settings.slack_oauth_client_secret).toBeNull();
    });
  },
);

// After pull, pushing back without any edits must not mutate remote state
// (no churn on muted_on_* booleans, no spurious editErrorHandler calls).
test.skipIf(shouldSkipOnCI())(
  "Slack sync: round-trip with all-null handlers is idempotent",
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "slack_roundtrip_null_test",
        token: backend.token,
      };
      await addWorkspace(testWorkspace, {
        force: true,
        configDir: backend.testConfigDir,
      });

      await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/slack_oauth_config`,
        { method: "DELETE" },
      );

      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
includeSettings: true`,
        "utf-8",
      );

      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir,
      );
      expect(pullResult.code).toEqual(0);

      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
      );
      expect(pushResult.code).toEqual(0);

      // Remote state must match the pre-push snapshot.
      const settingsResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/get_settings`,
      );
      expect(settingsResp.ok).toBe(true);
      const settings = await settingsResp.json();
      expect(settings.error_handler ?? null).toBeNull();
      expect(settings.success_handler ?? null).toBeNull();
      expect(settings.slack_oauth_client_id ?? null).toBeNull();
      expect(settings.slack_oauth_client_secret ?? null).toBeNull();
    });
  },
);

// Push with an explicit slack_oauth_client_id / _secret pair upserts the
// workspace-level OAuth override; the subsequent pull reflects the new state.
test.skipIf(shouldSkipOnCI())(
  "Slack sync: push of populated slack_oauth_config upserts",
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "slack_push_oauth_test",
        token: backend.token,
      };
      await addWorkspace(testWorkspace, {
        force: true,
        configDir: backend.testConfigDir,
      });

      await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/slack_oauth_config`,
        { method: "DELETE" },
      );

      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
includeSettings: true`,
        "utf-8",
      );

      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir,
      );
      expect(pullResult.code).toEqual(0);

      const settingsYaml = await readFile(`${tempDir}/settings.yaml`, "utf-8");
      const localSettings = parse(settingsYaml) as Record<string, unknown>;
      localSettings.slack_oauth_client_id = "1234567890.1234567890";
      localSettings.slack_oauth_client_secret = "deadbeefcafebabe";
      await writeFile(
        `${tempDir}/settings.yaml`,
        stringify(localSettings),
        "utf-8",
      );

      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
      );
      expect(pushResult.code).toEqual(0);

      const settingsResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/get_settings`,
      );
      expect(settingsResp.ok).toBe(true);
      const settings = await settingsResp.json();
      expect(settings.slack_oauth_client_id).toEqual("1234567890.1234567890");
      expect(settings.slack_oauth_client_secret).toEqual("deadbeefcafebabe");
    });
  },
);

// Deleting the two slack_oauth keys from settings.yaml (leaving them absent)
// must clear the remote — same rule as every other workspace setting.
test.skipIf(shouldSkipOnCI())(
  "Slack sync: omitting slack_oauth_config from YAML clears remote",
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "slack_omit_clear_test",
        token: backend.token,
      };
      await addWorkspace(testWorkspace, {
        force: true,
        configDir: backend.testConfigDir,
      });

      // Pre-seed the workspace with a workspace-level OAuth override.
      const setResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/slack_oauth_config`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            slack_oauth_client_id: "1111111111.2222222222",
            slack_oauth_client_secret: "feedfacefeedfacefeedface",
          }),
        },
      );
      expect(setResp.ok).toBe(true);

      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
includeSettings: true`,
        "utf-8",
      );

      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir,
      );
      expect(pullResult.code).toEqual(0);

      const settingsYaml = await readFile(`${tempDir}/settings.yaml`, "utf-8");
      const localSettings = parse(settingsYaml) as Record<string, unknown>;
      // Remove both keys entirely — "omit = clear" under the universal rule.
      delete localSettings.slack_oauth_client_id;
      delete localSettings.slack_oauth_client_secret;
      await writeFile(
        `${tempDir}/settings.yaml`,
        stringify(localSettings),
        "utf-8",
      );

      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
      );
      expect(pushResult.code).toEqual(0);

      const settingsResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/get_settings`,
      );
      expect(settingsResp.ok).toBe(true);
      const settings = await settingsResp.json();
      expect(settings.slack_oauth_client_id ?? null).toBeNull();
      expect(settings.slack_oauth_client_secret ?? null).toBeNull();
    });
  },
);

// Explicit null in YAML must also clear — same outcome as omission; the two
// representations converge on the "clear remote" semantic.
test.skipIf(shouldSkipOnCI())(
  "Slack sync: explicit null error_handler clears remote",
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "slack_null_clear_test",
        token: backend.token,
      };
      await addWorkspace(testWorkspace, {
        force: true,
        configDir: backend.testConfigDir,
      });

      // Pre-seed an error handler on the workspace.
      const setResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/edit_error_handler`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: "script/hub/19741/workspace-or-schedule-error-handler-slack",
            extra_args: null,
            muted_on_cancel: false,
            muted_on_user_path: false,
          }),
        },
      );
      expect(setResp.ok).toBe(true);

      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
includeSettings: true`,
        "utf-8",
      );

      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir,
      );
      expect(pullResult.code).toEqual(0);

      const settingsYaml = await readFile(`${tempDir}/settings.yaml`, "utf-8");
      const localSettings = parse(settingsYaml) as Record<string, unknown>;
      expect(localSettings.error_handler).not.toBeNull(); // sanity: it was populated
      // Explicit null — must clear.
      localSettings.error_handler = null;
      await writeFile(
        `${tempDir}/settings.yaml`,
        stringify(localSettings),
        "utf-8",
      );

      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
      );
      expect(pushResult.code).toEqual(0);

      const settingsResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/get_settings`,
      );
      expect(settingsResp.ok).toBe(true);
      const settings = await settingsResp.json();
      expect(settings.error_handler ?? null).toBeNull();
    });
  },
);

// Round-trip for a populated error_handler: pull should capture it exactly
// (including the always-persisted muted_on_* booleans), and re-push without
// edits should be a no-op.
test.skipIf(shouldSkipOnCI())(
  "Slack sync: round-trip preserves populated error_handler",
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "slack_handler_roundtrip_test",
        token: backend.token,
      };
      await addWorkspace(testWorkspace, {
        force: true,
        configDir: backend.testConfigDir,
      });

      const handlerPath =
        "script/hub/19741/workspace-or-schedule-error-handler-slack";
      const extraArgs = { channel: "test", slack: "$res:f/slack_bot/bot_token" };
      const setResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/edit_error_handler`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: handlerPath,
            extra_args: extraArgs,
            muted_on_cancel: false,
            muted_on_user_path: false,
          }),
        },
      );
      expect(setResp.ok).toBe(true);

      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
includeSettings: true`,
        "utf-8",
      );

      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir,
      );
      expect(pullResult.code).toEqual(0);

      const settingsYaml = await readFile(`${tempDir}/settings.yaml`, "utf-8");
      const localSettings = parse(settingsYaml) as Record<string, unknown>;
      const localHandler = localSettings.error_handler as Record<string, unknown>;
      expect(localHandler).not.toBeNull();
      expect(localHandler.path).toEqual(handlerPath);
      expect(localHandler.extra_args).toEqual(extraArgs);
      // Always-persist booleans (fix 4b) ensure round-trip stability.
      expect(localHandler.muted_on_cancel).toEqual(false);
      expect(localHandler.muted_on_user_path).toEqual(false);

      // Push unchanged — remote must be byte-identical.
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
      );
      expect(pushResult.code).toEqual(0);

      const settingsResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/get_settings`,
      );
      expect(settingsResp.ok).toBe(true);
      const settings = await settingsResp.json();
      expect(settings.error_handler).toEqual({
        path: handlerPath,
        extra_args: extraArgs,
        muted_on_cancel: false,
        muted_on_user_path: false,
      });
    });
  },
);
