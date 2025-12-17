import { describe, expect, it } from "vitest";
import { buildCreateTableValues } from "./mergeMetaData";
import type { ForeignKeyMetadata, TableMetadata } from "../utils";

const tableMetadata: TableMetadata = [
  {
    field: "id",
    datatype: "integer",
    defaultvalue: "",
    isprimarykey: true,
    isidentity: "By Default" as any,
    isnullable: "NO",
    isenum: false,
  },
  {
    field: "user_id",
    datatype: "integer",
    defaultvalue: "",
    isprimarykey: false,
    isidentity: "No" as any,
    isnullable: "NO",
    isenum: false,
  },
  {
    field: "role",
    datatype: "varchar(255)",
    defaultvalue: "'user'",
    isprimarykey: false,
    isidentity: "No" as any,
    isnullable: "YES",
    isenum: false,
  },
];

const foreignKeys: ForeignKeyMetadata[] = [
  {
    constraint_name: "fk_user",
    table_name: "accounts",
    column_name: "user_id",
    referenced_table_name: "users",
    referenced_column_name: "id",
    delete_rule: "CASCADE",
    update_rule: "NO ACTION",
  },
];

const compositeForeignKeys: ForeignKeyMetadata[] = [
  {
    constraint_name: "fk_membership",
    table_name: "memberships",
    column_name: "user_id",
    referenced_table_name: "users",
    referenced_column_name: "id",
    delete_rule: "CASCADE",
    update_rule: "CASCADE",
  },
  {
    constraint_name: "fk_membership",
    table_name: "memberships",
    column_name: "org_id",
    referenced_table_name: "orgs",
    referenced_column_name: "id",
    delete_rule: "CASCADE",
    update_rule: "CASCADE",
  },
];

describe("buildCreateTableValues – columns", () => {
  it("maps columns correctly", () => {
    const result = buildCreateTableValues(
      "accounts",
      tableMetadata,
      [],
    );

    expect(result.columns).toEqual([
      {
        name: "id",
        datatype: "integer",
        datatype_length: undefined,
        primaryKey: true,
        defaultValue: null,
        nullable: false,
      },
      {
        name: "user_id",
        datatype: "integer",
        datatype_length: undefined,
        primaryKey: undefined,
        defaultValue: null,
        nullable: false,
      },
      {
        name: "role",
        datatype: "varchar",
        datatype_length: 255,
        primaryKey: undefined,
        defaultValue: "'user'",
        nullable: true,
      },
    ]);
  });
});

describe("buildCreateTableValues – foreign keys", () => {
  it("creates a single-column foreign key", () => {
    const result = buildCreateTableValues(
      "accounts",
      tableMetadata,
      foreignKeys,
    );

    expect(result.foreignKeys).toEqual([
      {
        targetTable: "users",
        onDelete: "CASCADE",
        onUpdate: "NO ACTION",
        columns: [
          {
            sourceColumn: "user_id",
            targetColumn: "id",
          },
        ],
      },
    ]);
  });
});

describe("buildCreateTableValues – composite foreign keys", () => {
  it("groups columns by constraint name", () => {
    const result = buildCreateTableValues(
      "memberships",
      tableMetadata,
      compositeForeignKeys,
    );

    expect(result.foreignKeys).toEqual([
      {
        targetTable: "users",
        onDelete: "CASCADE",
        onUpdate: "CASCADE",
        columns: [
          { sourceColumn: "user_id", targetColumn: "id" },
          { sourceColumn: "org_id", targetColumn: "id" },
        ],
      },
    ]);
  });
});

describe("buildCreateTableValues – rule normalization", () => {
  it("defaults to NO ACTION when rules are missing", () => {
    const result = buildCreateTableValues(
      "accounts",
      tableMetadata,
      [
        {
          constraint_name: "fk_test",
          table_name: "accounts",
          column_name: "user_id",
          referenced_table_name: "users",
          referenced_column_name: "id",
        },
      ],
    );

    expect(result.foreignKeys[0].onDelete).toBe("NO ACTION");
    expect(result.foreignKeys[0].onUpdate).toBe("NO ACTION");
  });
});
