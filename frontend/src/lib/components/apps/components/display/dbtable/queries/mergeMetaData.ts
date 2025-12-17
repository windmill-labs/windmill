import type { ForeignKeyMetadata, TableMetadata } from "../utils";
import type {
  CreateForeignKey,
  CreateTableValues,
  CreateTableValuesColumn,
} from "./createTable";
import type { SQLDataType } from "./dbQueriesUtils";

function normalizeRule(rule?: string): "CASCADE" | "SET NULL" | "NO ACTION" {
  switch (rule?.toUpperCase()) {
    case "CASCADE":
      return "CASCADE";
    case "SET NULL":
      return "SET NULL";
    default:
      return "NO ACTION";
  }
}

function parseDatatype(
  datatype: string,
): { type: SQLDataType; length?: number } {
  const match = datatype.match(/^(\w+)(?:\((\d+)\))?/i);

  if (!match) {
    return { type: datatype as SQLDataType };
  }

  return {
    type: match[1] as SQLDataType,
    length: match[2] ? Number(match[2]) : undefined,
  };
}

export function buildCreateTableValues(
  tableName: string,
  tableMetadata: TableMetadata,
  foreignKeys: ForeignKeyMetadata[],
): CreateTableValues {
  const columns: CreateTableValuesColumn[] = tableMetadata.map((col) => {
    const { type, length } = parseDatatype(col.datatype);

    return {
      name: col.field,
      datatype: type,
      datatype_length: length,
      primaryKey: col.isprimarykey || undefined,
      defaultValue: col.defaultvalue === ""
        ? null
        : col.defaultvalue ?? undefined,
      nullable: col.isnullable === "YES",
    };
  });

  const tableFKs = foreignKeys.filter((fk) => fk.table_name === tableName);
  const fkByConstraint = new Map<string, ForeignKeyMetadata[]>();

  for (const fk of tableFKs) {
    if (!fkByConstraint.has(fk.constraint_name)) {
      fkByConstraint.set(fk.constraint_name, []);
    }
    fkByConstraint.get(fk.constraint_name)!.push(fk);
  }

  const foreignKeysValues: CreateForeignKey[] = Array.from(
    fkByConstraint.values(),
  ).map((group) => {
    const first = group[0];

    return {
      targetTable: first.referenced_table_name,
      onDelete: normalizeRule(first.delete_rule),
      onUpdate: normalizeRule(first.update_rule),
      columns: group.map((fk) => ({
        sourceColumn: fk.column_name,
        targetColumn: fk.referenced_column_name,
      })),
    };
  });

  return {
    name: tableName,
    columns,
    foreignKeys: foreignKeysValues,
  };
}
