import { dbSupportsSchemas } from "../utils";
import type { DbType } from "$lib/components/dbTypes";
import { renderColumn, renderForeignKey, type SQLDataType } from "./dbQueriesUtils";

export type CreateTableValues = {
	name: string;
	columns: CreateTableValuesColumn[];
	foreignKeys: CreateForeignKey[];
};

export type CreateForeignKey = {
	targetTable?: string;
	columns: {
		sourceColumn?: string;
		targetColumn?: string;
	}[];
	onDelete: "CASCADE" | "SET NULL" | "NO ACTION";
	onUpdate: "CASCADE" | "SET NULL" | "NO ACTION";
};

export type CreateTableValuesColumn = {
	name: string;
	datatype: SQLDataType;
	primaryKey?: boolean;
	defaultValue?: string | null;
	nullable?: boolean;
	datatype_length?: number; // e.g varchar(255)
};

export function makeCreateTableQuery(
	values: CreateTableValues,
	dbType: DbType,
	schema?: string,
) {
	const pkCount = values.columns.reduce(
		(p, c) => p + (c.primaryKey ? 1 : 0),
		0,
	);

	const useSchema = dbSupportsSchemas(dbType);
	const tableRef = useSchema && schema
		? `${schema.trim()}.${values.name}`
		: values.name;

	const lines = values.columns.map((c) =>
		renderColumn(c, dbType, pkCount === 1 && c.primaryKey)
	);
	lines.push(
		...values.foreignKeys.map((fk) =>
			renderForeignKey(fk, { useSchema, dbType, tableName: values.name })
		),
	);
	if (pkCount > 1) {
		const pks = values.columns.filter((c) => c.primaryKey);
		lines.push(`  PRIMARY KEY (${pks.map((c) => c.name).join(", ")})`);
	}

	return `CREATE TABLE ${tableRef} ( ${lines.join(",\n")} );`;
}
