import type { DbType } from "$lib/components/dbTypes";
import { dbSupportsSchemas } from "../utils";
import {
  type CreateForeignKey,
  type CreateTableValuesColumn,
} from "./createTable";
import {
  formatDefaultValue,
  renderColumn,
  renderForeignKey,
} from "./dbQueriesUtils";

export type AlterTableValues = {
  name: string;
  operations: AlterTableOperation[];
};

type AddColumnOperation = {
  kind: "addColumn";
  column: CreateTableValuesColumn;
};

type DropColumnOperation = {
  kind: "dropColumn";
  name: string;
};

type RenameColumnOperation = {
  kind: "renameColumn";
  from: string;
  to: string;
};

export type AlterColumnOperation = {
  kind: "alterColumn";
  name: string;
  changes: Partial<Omit<CreateTableValuesColumn, "name">>;
};

type AddForeignKeyOperation = {
  kind: "addForeignKey";
  foreignKey: CreateForeignKey;
};

type DropForeignKeyOperation = {
  kind: "dropForeignKey";
  name: string;
};

export type AlterTableOperation =
  | AddColumnOperation
  | DropColumnOperation
  | RenameColumnOperation
  | AlterColumnOperation
  | AddForeignKeyOperation
  | DropForeignKeyOperation;

export function makeAlterTableQuery(
  values: AlterTableValues,
  dbType: DbType,
  schema?: string,
): string[] {
  const useSchema = dbSupportsSchemas(dbType);
  const tableRef = useSchema && schema
    ? `${schema.trim()}.${values.name}`
    : values.name;

  const queries: string[] = [];

  for (const op of values.operations) {
    switch (op.kind) {
      case "addColumn":
        queries.push(
          `ALTER TABLE ${tableRef} ADD COLUMN ${
            // TODO: handle primary key updates
            renderColumn(op.column, dbType)};`,
        );
        break;

      case "dropColumn":
        queries.push(
          `ALTER TABLE ${tableRef} DROP COLUMN ${op.name};`,
        );
        break;

      case "renameColumn":
        queries.push(
          `ALTER TABLE ${tableRef} RENAME COLUMN ${op.from} TO ${op.to};`,
        );
        break;

      case "alterColumn":
        queries.push(
          ...renderAlterColumn(tableRef, op, dbType),
        );
        break;

      case "addForeignKey":
        queries.push(
          `ALTER TABLE ${tableRef} ADD ${
            renderForeignKey(op.foreignKey, {
              useSchema,
              dbType,
              tableName: values.name,
            })
          };`,
        );
        break;

      case "dropForeignKey":
        queries.push(renderDropForeignKey(tableRef, op.name, dbType));
        break;

      default:
        throw new Error("Unimplemented case");
    }
  }

  return queries;
}

function renderAlterColumn(
  tableRef: string,
  op: AlterColumnOperation,
  dbType: DbType,
): string[] {
  const queries: string[] = [];
  const { name, changes } = op;

  if (changes.datatype) {
    const datatype = changes.datatype_length
      ? `${changes.datatype}(${changes.datatype_length})`
      : changes.datatype;

    queries.push(
      `ALTER TABLE ${tableRef} ALTER COLUMN ${name} TYPE ${datatype};`,
    );
  }

  if ("defaultValue" in changes) {
    if (!changes.defaultValue) {
      queries.push(
        `ALTER TABLE ${tableRef} ALTER COLUMN ${name} DROP DEFAULT;`,
      );
    } else {
      const def = formatDefaultValue(
        changes.defaultValue,
        changes.datatype ?? "",
        dbType,
      );
      queries.push(
        `ALTER TABLE ${tableRef} ALTER COLUMN ${name} SET DEFAULT ${def};`,
      );
    }
  }

  if (typeof changes.nullable === "boolean") {
    queries.push(
      `ALTER TABLE ${tableRef} ALTER COLUMN ${name} ${
        changes.nullable ? "DROP" : "SET"
      } NOT NULL;`,
    );
  }

  return queries;
}

function renderDropForeignKey(
  tableRef: string,
  name: string,
  dbType: DbType,
): string {
  if (dbType === "postgresql") {
    return `ALTER TABLE ${tableRef} DROP CONSTRAINT ${name};`;
  }
  return `ALTER TABLE ${tableRef} DROP FOREIGN KEY ${name};`;
}
