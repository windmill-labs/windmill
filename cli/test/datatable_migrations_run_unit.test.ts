/**
 * A git-sync push (CI, no TTY) is how merged migrations reach the workspace, so
 * `--yes` must apply them without a prompt — and the no-TTY, no-`--yes` case must
 * still leave the data tables untouched.
 */

import { expect, test, describe, beforeEach, mock } from "bun:test";

let runCalls: { workspace: string; datatableName: string }[] = [];

mock.module("../gen/services.gen.ts", () => ({
  runDatatableMigrations: async (args: {
    workspace: string;
    datatableName: string;
  }) => {
    runCalls.push(args);
    return { applied: [{ version: 1, name: "m" }] };
  },
}));

const { offerToRunNewMigrations } = await import(
  "../src/commands/datatable_migrations.ts"
);

const NEW_MIGRATIONS = [
  { datatable: "orders", timestamp: 1, name: "create_orders" },
  { datatable: "orders", timestamp: 2, name: "add_status" },
  { datatable: "analytics", timestamp: 3, name: "create_events" },
];

describe("offerToRunNewMigrations", () => {
  beforeEach(() => {
    runCalls = [];
  });

  test("--yes runs each affected data table once, without prompting", async () => {
    await offerToRunNewMigrations("ws", NEW_MIGRATIONS, { yes: true });
    expect(runCalls.map((c) => c.datatableName)).toEqual([
      "orders",
      "analytics",
    ]);
    expect(runCalls.every((c) => c.workspace === "ws")).toBe(true);
  });

  test("runs nothing when nobody can answer the prompt", async () => {
    await offerToRunNewMigrations("ws", NEW_MIGRATIONS, { jsonOutput: true });
    await offerToRunNewMigrations("ws", NEW_MIGRATIONS, {});
    expect(runCalls).toEqual([]);
  });

  test("runs nothing when the push introduced no migrations", async () => {
    await offerToRunNewMigrations("ws", [], { yes: true });
    expect(runCalls).toEqual([]);
  });
});
