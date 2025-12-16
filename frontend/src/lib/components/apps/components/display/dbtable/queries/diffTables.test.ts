import { describe, expect, it } from "vitest";
import { diffTables } from "./diffTables";
import type { CreateTableValues } from "./createTable";

function opKinds(ops: { kind: string }[]) {
  return ops.map((o) => o.kind);
}

describe("diffTables", () => {
  it("returns null when tables are identical", () => {
    const table = {
      name: "users",
      columns: [
        { name: "id", datatype: "INTEGER", primaryKey: true },
        { name: "email", datatype: "TEXT", nullable: true },
      ],
      foreignKeys: [],
    };

    const diff = diffTables(table, table);

    expect(diff).toBeNull();
  });

  it("detects added columns", () => {
    const oldTable = {
      name: "users",
      columns: [{ name: "id", datatype: "INTEGER" }],
      foreignKeys: [],
    };

    const newTable = {
      name: "users",
      columns: [
        { name: "id", datatype: "INTEGER" },
        { name: "age", datatype: "INTEGER" },
      ],
      foreignKeys: [],
    };

    const diff = diffTables(oldTable, newTable)!;

    expect(opKinds(diff.operations)).toEqual(["addColumn"]);
    expect(diff.operations[0]).toMatchObject({
      kind: "addColumn",
      column: { name: "age" },
    });
  });

  it("detects dropped columns", () => {
    const oldTable = {
      name: "users",
      columns: [
        { name: "id", datatype: "INTEGER" },
        { name: "age", datatype: "INTEGER" },
      ],
      foreignKeys: [],
    };

    const newTable = {
      name: "users",
      columns: [{ name: "id", datatype: "INTEGER" }],
      foreignKeys: [],
    };

    const diff = diffTables(oldTable, newTable)!;

    expect(opKinds(diff.operations)).toEqual(["dropColumn"]);
    expect(diff.operations[0]).toMatchObject({
      kind: "dropColumn",
      name: "age",
    });
  });

  it("detects column nullability changes", () => {
    const oldTable = {
      name: "users",
      columns: [
        { name: "email", datatype: "TEXT", nullable: true },
      ],
      foreignKeys: [],
    };

    const newTable = {
      name: "users",
      columns: [
        { name: "email", datatype: "TEXT", nullable: false },
      ],
      foreignKeys: [],
    };

    const diff = diffTables(oldTable, newTable)!;

    expect(opKinds(diff.operations)).toEqual(["alterColumn"]);
    expect(diff.operations[0]).toMatchObject({
      kind: "alterColumn",
      name: "email",
      changes: { nullable: false },
    });
  });

  it("detects column datatype changes", () => {
    const oldTable = {
      name: "users",
      columns: [
        { name: "age", datatype: "INTEGER" },
      ],
      foreignKeys: [],
    };

    const newTable = {
      name: "users",
      columns: [
        { name: "age", datatype: "BIGINT" },
      ],
      foreignKeys: [],
    };

    const diff = diffTables(oldTable, newTable)!;

    expect(diff.operations[0]).toMatchObject({
      kind: "alterColumn",
      name: "age",
      changes: { datatype: "BIGINT" },
    });
  });

  it("detects default value changes", () => {
    const oldTable = {
      name: "users",
      columns: [
        { name: "active", datatype: "BOOLEAN" },
      ],
      foreignKeys: [],
    };

    const newTable = {
      name: "users",
      columns: [
        {
          name: "active",
          datatype: "BOOLEAN",
          defaultValue: "true",
        },
      ],
      foreignKeys: [],
    };

    const diff = diffTables(oldTable, newTable)!;

    expect(diff.operations[0]).toMatchObject({
      kind: "alterColumn",
      name: "active",
      changes: { defaultValue: "true" },
    });
  });

  it("detects added foreign keys", () => {
    const oldTable: CreateTableValues = {
      name: "posts",
      columns: [
        { name: "id", datatype: "INTEGER" },
        { name: "user_id", datatype: "INTEGER" },
      ],
      foreignKeys: [],
    };

    const newTable: CreateTableValues = {
      name: "posts",
      columns: [
        { name: "id", datatype: "INTEGER" },
        { name: "user_id", datatype: "INTEGER" },
      ],
      foreignKeys: [
        {
          targetTable: "users",
          columns: [
            { sourceColumn: "user_id", targetColumn: "id" },
          ],
          onDelete: "CASCADE",
          onUpdate: "NO ACTION",
        },
      ],
    };

    const diff = diffTables(oldTable, newTable)!;

    expect(opKinds(diff.operations)).toEqual(["addForeignKey"]);
  });

  it("recreates foreign keys when modified", () => {
    const oldTable: CreateTableValues = {
      name: "posts",
      columns: [],
      foreignKeys: [
        {
          targetTable: "users",
          columns: [
            { sourceColumn: "user_id", targetColumn: "id" },
          ],
          onDelete: "NO ACTION",
          onUpdate: "NO ACTION",
        },
      ],
    };

    const newTable: CreateTableValues = {
      name: "posts",
      columns: [],
      foreignKeys: [
        {
          targetTable: "users",
          columns: [
            { sourceColumn: "user_id", targetColumn: "id" },
          ],
          onDelete: "CASCADE",
          onUpdate: "NO ACTION",
        },
      ],
    };

    const diff = diffTables(oldTable, newTable)!;

    expect(opKinds(diff.operations)).toEqual([
      "dropForeignKey",
      "addForeignKey",
    ]);
  });

  it("combines column and foreign key changes", () => {
    const oldTable: CreateTableValues = {
      name: "users",
      columns: [{ name: "id", datatype: "INTEGER" }],
      foreignKeys: [],
    };

    const newTable: CreateTableValues = {
      name: "users",
      columns: [
        { name: "id", datatype: "INTEGER" },
        { name: "age", datatype: "INTEGER" },
      ],
      foreignKeys: [
        {
          targetTable: "orgs",
          columns: [
            { sourceColumn: "org_id", targetColumn: "id" },
          ],
          onDelete: "SET NULL",
          onUpdate: "NO ACTION",
        },
      ],
    };

    const diff = diffTables(oldTable, newTable)!;

    expect(opKinds(diff.operations)).toEqual([
      "addColumn",
      "addForeignKey",
    ]);
  });
});
