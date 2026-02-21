/**
 * Unit tests for settings.ts pure functions.
 * Tests migrateToGroupedFormat which converts legacy flat settings to grouped format.
 */

import { expect, test, describe } from "bun:test";
import { migrateToGroupedFormat } from "../src/core/settings.ts";

// =============================================================================
// migrateToGroupedFormat
// =============================================================================

describe("migrateToGroupedFormat", () => {
  test("migrates legacy auto_invite fields to grouped format", () => {
    const legacy = {
      name: "my-workspace",
      auto_invite_enabled: true,
      auto_invite_as: "operator",
      auto_invite_mode: "add",
    };
    const result = migrateToGroupedFormat(legacy);
    expect(result.auto_invite).toEqual({
      enabled: true,
      operator: true,
      mode: "add",
    });
  });

  test("migrates legacy auto_invite with non-operator role", () => {
    const legacy = {
      name: "ws",
      auto_invite_enabled: true,
      auto_invite_as: "developer",
      auto_invite_mode: "invite",
    };
    const result = migrateToGroupedFormat(legacy);
    expect(result.auto_invite).toEqual({
      enabled: true,
      operator: false,
      mode: "invite",
    });
  });

  test("migrates legacy auto_invite when disabled", () => {
    const legacy = {
      name: "ws",
      auto_invite_enabled: false,
      auto_invite_as: "operator",
    };
    const result = migrateToGroupedFormat(legacy);
    expect(result.auto_invite!.enabled).toBe(false);
  });

  test("preserves already-grouped auto_invite", () => {
    const grouped = {
      name: "ws",
      auto_invite: { enabled: true, operator: false, mode: "invite" as const },
    };
    const result = migrateToGroupedFormat(grouped);
    expect(result.auto_invite).toEqual({
      enabled: true,
      operator: false,
      mode: "invite",
    });
  });

  test("migrates legacy error_handler string to grouped format", () => {
    const legacy = {
      name: "ws",
      error_handler: "u/admin/error_handler",
      error_handler_extra_args: { notify: true },
      error_handler_muted_on_cancel: true,
    };
    const result = migrateToGroupedFormat(legacy);
    expect(result.error_handler).toEqual({
      path: "u/admin/error_handler",
      extra_args: { notify: true },
      muted_on_cancel: true,
    });
  });

  test("preserves already-grouped error_handler", () => {
    const grouped = {
      name: "ws",
      error_handler: {
        path: "u/admin/handler",
        extra_args: {},
        muted_on_cancel: false,
      },
    };
    const result = migrateToGroupedFormat(grouped);
    expect(result.error_handler).toEqual({
      path: "u/admin/handler",
      extra_args: {},
      muted_on_cancel: false,
    });
  });

  test("migrates legacy success_handler string to grouped format", () => {
    const legacy = {
      name: "ws",
      success_handler: "u/admin/on_success",
      success_handler_extra_args: { channel: "#deploys" },
    };
    const result = migrateToGroupedFormat(legacy);
    expect(result.success_handler).toEqual({
      path: "u/admin/on_success",
      extra_args: { channel: "#deploys" },
    });
  });

  test("preserves already-grouped success_handler", () => {
    const grouped = {
      name: "ws",
      success_handler: { path: "u/admin/handler", extra_args: {} },
    };
    const result = migrateToGroupedFormat(grouped);
    expect(result.success_handler).toEqual({
      path: "u/admin/handler",
      extra_args: {},
    });
  });

  test("copies non-legacy fields through", () => {
    const settings = {
      name: "my-workspace",
      webhook: "https://example.com/hook",
      deploy_to: "staging",
      default_app: "u/admin/dashboard",
      mute_critical_alerts: true,
      color: "#ff0000",
    };
    const result = migrateToGroupedFormat(settings);
    expect(result.name).toBe("my-workspace");
    expect(result.webhook).toBe("https://example.com/hook");
    expect(result.deploy_to).toBe("staging");
    expect(result.default_app).toBe("u/admin/dashboard");
    expect(result.mute_critical_alerts).toBe(true);
    expect(result.color).toBe("#ff0000");
  });

  test("handles minimal settings with only name", () => {
    const result = migrateToGroupedFormat({ name: "ws" });
    expect(result.name).toBe("ws");
    expect(result.auto_invite).toBeUndefined();
    expect(result.error_handler).toBeUndefined();
    expect(result.success_handler).toBeUndefined();
  });

  test("defaults name to empty string when missing", () => {
    const result = migrateToGroupedFormat({});
    expect(result.name).toBe("");
  });

  test("defaults auto_invite_mode to invite when missing", () => {
    const legacy = {
      name: "ws",
      auto_invite_enabled: true,
      auto_invite_as: "operator",
    };
    const result = migrateToGroupedFormat(legacy);
    expect(result.auto_invite!.mode).toBe("invite");
  });

  test("defaults error_handler_muted_on_cancel to false when missing", () => {
    const legacy = {
      name: "ws",
      error_handler: "u/admin/handler",
    };
    const result = migrateToGroupedFormat(legacy);
    expect(result.error_handler!.muted_on_cancel).toBe(false);
  });

  test("preserves ai_config, large_file_storage, git_sync, default_scripts, operator_settings", () => {
    const settings = {
      name: "ws",
      ai_config: { provider: "openai" },
      large_file_storage: { type: "s3" },
      git_sync: { enabled: true },
      default_scripts: { python: "template.py" },
      operator_settings: { hideCode: true },
    };
    const result = migrateToGroupedFormat(settings);
    expect(result.ai_config).toEqual({ provider: "openai" });
    expect(result.large_file_storage).toEqual({ type: "s3" });
    expect(result.git_sync).toEqual({ enabled: true });
    expect(result.default_scripts).toEqual({ python: "template.py" });
    expect(result.operator_settings).toEqual({ hideCode: true });
  });

  test("does not include undefined fields in result", () => {
    const result = migrateToGroupedFormat({ name: "ws" });
    expect("webhook" in result).toBe(false);
    expect("deploy_to" in result).toBe(false);
    expect("color" in result).toBe(false);
  });
});
