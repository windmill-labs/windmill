import type {
  AlterColumnOperation,
  AlterTableOperation,
  AlterTableValues,
} from "./alterTable";
import type {
  CreateForeignKey,
  CreateTableValues,
  CreateTableValuesColumn,
} from "./createTable";

function byName<T extends { name: string }>(items: T[]) {
  const map = new Map<string, T>();
  for (const item of items) {
    map.set(item.name, item);
  }
  return map;
}

function columnSignature(c: CreateTableValuesColumn) {
  return {
    datatype: c.datatype,
    datatype_length: c.datatype_length,
    nullable: !!c.nullable,
    defaultValue: c.defaultValue ?? null,
    primaryKey: !!c.primaryKey,
  };
}

function diffColumns(
  oldCols: CreateTableValuesColumn[],
  newCols: CreateTableValuesColumn[],
): AlterTableOperation[] {
  const ops: AlterTableOperation[] = [];

  const oldMap = byName(oldCols);
  const newMap = byName(newCols);

  // Added columns
  for (const [name, col] of newMap) {
    if (!oldMap.has(name)) {
      ops.push({
        kind: "addColumn",
        column: col,
      });
    }
  }

  // Dropped columns
  for (const [name] of oldMap) {
    if (!newMap.has(name)) {
      ops.push({
        kind: "dropColumn",
        name,
      });
    }
  }

  // Modified columns
  for (const [name, newCol] of newMap) {
    const oldCol = oldMap.get(name);
    if (!oldCol) continue;

    const before = columnSignature(oldCol);
    const after = columnSignature(newCol);

    const changes: AlterColumnOperation["changes"] = {};

    if (
      before.datatype !== after.datatype ||
      before.datatype_length !== after.datatype_length
    ) {
      changes.datatype = newCol.datatype;
      changes.datatype_length = newCol.datatype_length;
    }

    if (before.nullable !== after.nullable) {
      changes.nullable = after.nullable;
    }

    if (before.defaultValue !== after.defaultValue) {
      changes.defaultValue = after.defaultValue;
    }

    // Primary keys intentionally ignored here
    // PK changes are table-level operations and should be handled separately

    if (Object.keys(changes).length > 0) {
      ops.push({
        kind: "alterColumn",
        name,
        changes,
      });
    }
  }

  return ops;
}

function diffForeignKeys(
  oldFks: CreateForeignKey[],
  newFks: CreateForeignKey[],
  options: {
    tableName: string;
    useSchema: boolean;
  },
): AlterTableOperation[] {
  const ops: AlterTableOperation[] = [];
  const oldMap = new Map<string, CreateForeignKey>();
  const newMap = new Map<string, CreateForeignKey>();

  // Fill old Map
  for (const fk of oldFks) {
    const sourceColumns = fk.columns.map((c) => c.sourceColumn).filter(Boolean);
    const targetColumns = fk.columns.map((c) => c.targetColumn).filter(Boolean);
    const targetTable = options.useSchema || !fk.targetTable?.includes(".")
      ? fk.targetTable
      : fk.targetTable?.split(".").pop();

    const name = [
      options.tableName,
      ...sourceColumns,
      targetTable,
      ...targetColumns,
    ].join("_");

    oldMap.set(name, fk);
  }

  // Fill new Map
  for (const fk of newFks) {
    const sourceColumns = fk.columns.map((c) => c.sourceColumn).filter(Boolean);
    const targetColumns = fk.columns.map((c) => c.targetColumn).filter(Boolean);
    const targetTable = options.useSchema || !fk.targetTable?.includes(".")
      ? fk.targetTable
      : fk.targetTable?.split(".").pop();

    const name = [
      options.tableName,
      ...sourceColumns,
      targetTable,
      ...targetColumns,
    ].join("_");

    newMap.set(name, fk);
  }

  // Added FKs
  for (const [name, fk] of newMap) {
    if (!oldMap.has(name)) {
      ops.push({
        kind: "addForeignKey",
        foreignKey: fk,
      });
    }
  }

  // Dropped FKs
  for (const [name] of oldMap) {
    if (!newMap.has(name)) {
      ops.push({
        kind: "dropForeignKey",
        name,
      });
    }
  }

  // Modified FKs: drop + recreate
  for (const [name, newFk] of newMap) {
    const oldFk = oldMap.get(name);
    if (!oldFk) continue;

    if (JSON.stringify(oldFk) !== JSON.stringify(newFk)) {
      ops.push(
        { kind: "dropForeignKey", name },
        {
          kind: "addForeignKey",
          foreignKey: newFk,
        },
      );
    }
  }

  return ops;
}

export function diffTables(
  oldTable: CreateTableValues,
  newTable: CreateTableValues,
  options?: {
    useSchema?: boolean
  }
): AlterTableValues | null {
  const operations: AlterTableOperation[] = [];

  // Columns
  operations.push(
    ...diffColumns(oldTable.columns, newTable.columns),
  );

  // Foreign keys
  operations.push(
    ...diffForeignKeys(
      oldTable.foreignKeys,
      newTable.foreignKeys,
      {
        tableName: oldTable.name,
        useSchema: !!options?.useSchema
      },
    ),
  );

  if (operations.length === 0) return null;

  return {
    name: oldTable.name,
    operations,
  };
}
