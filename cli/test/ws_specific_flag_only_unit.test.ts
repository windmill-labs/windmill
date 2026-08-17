import { expect, test } from "bun:test";

import { computeWsSpecificFlagOnlyPushes } from "../src/commands/sync/sync.ts";
import type { SpecificItemsConfig } from "../src/core/specific_items.ts";

// =============================================================================
// computeWsSpecificFlagOnlyPushes
// Verifies which (kind, path) pairs are emitted as ws_specific_flag changes
// during sync push when local config flags an item ws_specific but the server
// does not yet.
// =============================================================================

test("emits a flag-only push for a resource missing from the server list", () => {
  const localMap: Record<string, string> = {
    "u/admin/db.resource.yaml": "value: foo",
  };
  const local: SpecificItemsConfig = { resources: ["u/admin/db.resource.yaml"] };
  const server: Array<{ item_kind: string; path: string }> = [];

  const out = computeWsSpecificFlagOnlyPushes(localMap, local, server);
  expect(out).toEqual([
    {
      kind: "resource",
      serverPath: "u/admin/db",
      filePath: "u/admin/db.resource.yaml",
    },
  ]);
});

test("emits a flag-only push for a variable missing from the server list", () => {
  const localMap: Record<string, string> = {
    "u/admin/v.variable.yaml": "value: bar",
  };
  const local: SpecificItemsConfig = { variables: ["u/admin/v.variable.yaml"] };

  const out = computeWsSpecificFlagOnlyPushes(localMap, local, []);
  expect(out).toEqual([
    {
      kind: "variable",
      serverPath: "u/admin/v",
      filePath: "u/admin/v.variable.yaml",
    },
  ]);
});

test("does not emit a flag-only push when the server already has the row", () => {
  const localMap: Record<string, string> = {
    "u/admin/db.resource.yaml": "value: foo",
  };
  const local: SpecificItemsConfig = { resources: ["u/admin/db.resource.yaml"] };
  const server = [{ item_kind: "resource", path: "u/admin/db" }];

  expect(computeWsSpecificFlagOnlyPushes(localMap, local, server)).toEqual([]);
});

test("does not emit anything for schedules in specificItems", () => {
  // The backend's list_ws_specific_versions endpoint only handles
  // resource + variable today; schedules have their ws_specific concept
  // expressed through workspace-specific files but the server has no
  // ws_specific row for them, so flag-only pushes don't apply.
  const localMap: Record<string, string> = {
    "u/admin/cron.schedule.yaml": "schedule: '* * * * *'",
  };
  const local: SpecificItemsConfig = { schedules: ["u/admin/cron.schedule.yaml"] };

  expect(computeWsSpecificFlagOnlyPushes(localMap, local, [])).toEqual([]);
});

test("does not emit anything for triggers in specificItems", () => {
  // Same reason as schedules — the backend doesn't track ws_specific for
  // any *_trigger kind.
  const localMap: Record<string, string> = {
    "u/admin/hook.http_trigger.yaml": "method: GET",
  };
  const local: SpecificItemsConfig = { triggers: ["u/admin/hook.http_trigger.yaml"] };

  expect(computeWsSpecificFlagOnlyPushes(localMap, local, [])).toEqual([]);
});

test("returns empty when serverItems is null (server doesn't expose the endpoint)", () => {
  const localMap: Record<string, string> = {
    "u/admin/db.resource.yaml": "value: foo",
  };
  const local: SpecificItemsConfig = { resources: ["u/admin/db.resource.yaml"] };

  expect(computeWsSpecificFlagOnlyPushes(localMap, local, null)).toEqual([]);
});

test("handles .json resource files (extension is preserved on filePath)", () => {
  const localMap: Record<string, string> = {
    "u/admin/db.resource.json": "{\"value\":\"foo\"}",
  };
  const local: SpecificItemsConfig = {
    // Patterns are conventionally .yaml; isSpecificItem normalizes .json -> .yaml
    resources: ["u/admin/db.resource.yaml"],
  };

  const out = computeWsSpecificFlagOnlyPushes(localMap, local, []);
  expect(out).toEqual([
    {
      kind: "resource",
      serverPath: "u/admin/db",
      filePath: "u/admin/db.resource.json",
    },
  ]);
});

test("ignores files not in specificItems", () => {
  const localMap: Record<string, string> = {
    "u/admin/db.resource.yaml": "value: foo",
    "u/admin/other.resource.yaml": "value: bar",
  };
  const local: SpecificItemsConfig = { resources: ["u/admin/db.resource.yaml"] };

  const out = computeWsSpecificFlagOnlyPushes(localMap, local, []);
  expect(out.map((p) => p.filePath)).toEqual(["u/admin/db.resource.yaml"]);
});
