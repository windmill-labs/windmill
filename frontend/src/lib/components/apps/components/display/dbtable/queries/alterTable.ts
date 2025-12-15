import type { CreateForeignKey, CreateTableValuesColumn } from "./createTable";

export type AlterTableValues = {
  table: string;
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

type AlterColumnOperation = {
  kind: "alterColumn";
  name: string;
  changes: Omit<CreateTableValuesColumn, "name">;
};

type AddForeignKeyOperation = {
  kind: "addForeignKey";
  foreignKey: CreateForeignKey;
};

type DropForeignKeyOperation = {
  kind: "dropForeignKey";
  name: string;
};

type AlterTableOperation =
  | AddColumnOperation
  | DropColumnOperation
  | RenameColumnOperation
  | AlterColumnOperation
  | AddForeignKeyOperation
  | DropForeignKeyOperation;
