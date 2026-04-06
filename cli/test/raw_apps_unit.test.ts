import { describe, expect, test } from "bun:test";
import {
  applyRawAppValueMetadata,
  populateRawAppMetadataFromValue,
  type AppFile,
} from "../src/commands/app/raw_apps.ts";

function createRawAppFile(overrides: Partial<AppFile> = {}): AppFile {
  return {
    summary: "Test raw app",
    policy: {
      execution_mode: "publisher",
      triggerables: {},
      triggerables_v2: {},
    } as any,
    ...overrides,
  };
}

describe("raw app metadata mapping", () => {
  test("maps hide_edit_button into the raw app value payload", () => {
    const value: Record<string, any> = { runnables: {}, files: {} };

    applyRawAppValueMetadata(
      value,
      createRawAppFile({ hide_edit_button: true }),
    );

    expect(value.hideEditButton).toBe(true);
  });

  test("preserves explicit false when mapping hide_edit_button", () => {
    const value: Record<string, any> = { runnables: {}, files: {} };

    applyRawAppValueMetadata(
      value,
      createRawAppFile({ hide_edit_button: false }),
    );

    expect(value.hideEditButton).toBe(false);
  });

  test("hydrates raw_app.yaml metadata from the stored raw app value", () => {
    const rawApp: Record<string, any> = {
      value: {
        hideEditButton: false,
        data: {
          datatable: "main",
          tables: ["main/users"],
        },
      },
    };

    populateRawAppMetadataFromValue(rawApp);

    expect(rawApp.hide_edit_button).toBe(false);
    expect(rawApp.data).toEqual({
      datatable: "main",
      tables: ["main/users"],
    });
  });
});
