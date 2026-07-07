import type {
	TableEditorValues,
	TableEditorValuesColumn,
	TableEditorForeignKey
} from '$lib/components/apps/components/display/dbtable/tableEditor'
import {
	diffTableEditorValues,
	type AlterTableValues,
	makeAlterTableQueries
} from '$lib/components/apps/components/display/dbtable/queries/alterTable'
import { renderForeignKey } from '$lib/components/apps/components/display/dbtable/queries/dbQueriesUtils'
import type { GetDatatableFullSchemaResponse } from '$lib/gen'

export type DatabaseSchema = Record<string, Record<string, TableEditorValues>>

export function apiSchemaToEditorSchema(
	apiSchema: GetDatatableFullSchemaResponse
): DatabaseSchema {
	const result: DatabaseSchema = {}
	for (const [schemaName, tables] of Object.entries(apiSchema)) {
		result[schemaName] = {}
		for (const [tableName, table] of Object.entries(tables as Record<string, any>)) {
			if (!table || typeof table !== 'object') continue
			result[schemaName][tableName] = {
				name: table.name ?? tableName,
				columns: (table.columns ?? []).map(
					(c: any): TableEditorValuesColumn => ({
						name: c.name,
						datatype: c.datatype,
						primaryKey: c.primary_key ?? c.primaryKey,
						defaultValue: c.default_value ?? c.defaultValue,
						nullable: c.nullable
					})
				),
				foreignKeys: (table.foreign_keys ?? table.foreignKeys ?? []).map(
					(fk: any): TableEditorForeignKey => ({
						targetTable: fk.target_table ?? fk.targetTable,
						columns: (fk.columns ?? []).map((col: any) => ({
							sourceColumn: col.source_column ?? col.sourceColumn,
							targetColumn: col.target_column ?? col.targetColumn
						})),
						onDelete: (fk.on_delete ?? fk.onDelete ?? 'NO ACTION') as
							| 'CASCADE'
							| 'SET NULL'
							| 'NO ACTION',
						onUpdate: (fk.on_update ?? fk.onUpdate ?? 'NO ACTION') as
							| 'CASCADE'
							| 'SET NULL'
							| 'NO ACTION',
						fk_constraint_name: fk.fk_constraint_name
					})
				),
				pk_constraint_name: table.pk_constraint_name
			}
		}
	}
	return result
}

export type TableDiff = {
	schemaName: string
	tableName: string
	kind: 'added' | 'removed' | 'modified'
	operations?: AlterTableValues
}

export type DatatableDiff = {
	datatableName: string
	aheadChanges: TableDiff[]
	behindChanges: TableDiff[]
	originalSchema: DatabaseSchema
	parentSchema: DatabaseSchema
	forkSchema: DatabaseSchema
}

export function diffDatabaseSchemas(
	original: DatabaseSchema,
	current: DatabaseSchema
): TableDiff[] {
	const diffs: TableDiff[] = []
	const allSchemas = new Set([...Object.keys(original), ...Object.keys(current)])
	for (const schemaName of allSchemas) {
		const origTables = original[schemaName] ?? {}
		const currTables = current[schemaName] ?? {}
		const allTables = new Set([...Object.keys(origTables), ...Object.keys(currTables)])
		for (const tableName of allTables) {
			const origTable = origTables[tableName]
			const currTable = currTables[tableName]
			if (!origTable && currTable) {
				diffs.push({ schemaName, tableName, kind: 'added' })
			} else if (origTable && !currTable) {
				diffs.push({ schemaName, tableName, kind: 'removed' })
			} else if (origTable && currTable) {
				const currWithInitial: TableEditorValues = {
					...currTable,
					columns: currTable.columns.map((col) => ({
						...col,
						initialName: col.name,
						defaultValue: col.defaultValue ? `{${col.defaultValue}}` : undefined
					}))
				}
				const origTableTransformed: TableEditorValues = {
					...origTable,
					columns: origTable.columns.map((col) => ({
						...col,
						defaultValue: col.defaultValue ? `{${col.defaultValue}}` : undefined
					}))
				}
				const diff = diffTableEditorValues(origTableTransformed, currWithInitial)
				if (diff.operations.length > 0) {
					diffs.push({ schemaName, tableName, kind: 'modified', operations: diff })
				}
			}
		}
	}
	return diffs
}

export function computeDatatableDiff(
	datatableName: string,
	originalSchema: DatabaseSchema,
	parentSchema: DatabaseSchema,
	forkSchema: DatabaseSchema
): DatatableDiff {
	return {
		datatableName,
		behindChanges: diffDatabaseSchemas(originalSchema, parentSchema),
		aheadChanges: diffDatabaseSchemas(originalSchema, forkSchema),
		originalSchema,
		parentSchema,
		forkSchema
	}
}

/** Detect PostgreSQL auto-increment columns and return the serial type + cleaned props.
 *  e.g. bigint + nextval('seq'::regclass) → BIGSERIAL (no DEFAULT needed) */
function resolveColumnType(c: TableEditorValuesColumn): {
	datatype: string
	defaultValue: string | undefined
} {
	const dv = c.defaultValue ?? ''
	if (/^{?nextval\(/.test(dv)) {
		const dt = c.datatype?.toLowerCase() ?? ''
		if (dt === 'bigint') return { datatype: 'BIGSERIAL', defaultValue: undefined }
		if (dt === 'integer' || dt === 'int') return { datatype: 'SERIAL', defaultValue: undefined }
		if (dt === 'smallint') return { datatype: 'SMALLSERIAL', defaultValue: undefined }
	}
	return { datatype: c.datatype, defaultValue: c.defaultValue }
}

export function generateMigrationSql(change: TableDiff, sourceSchema: DatabaseSchema): string {
	if (change.kind === 'modified' && change.operations) {
		const queries = makeAlterTableQueries(change.operations, 'postgresql', change.schemaName)
		if (queries.length === 0) return ''
		return 'BEGIN;\n' + queries.join('\n') + '\nCOMMIT;'
	}
	if (change.kind === 'added') {
		const table = sourceSchema[change.schemaName]?.[change.tableName]
		if (!table) return ''
		const colDefs = table.columns
			.map((c) => {
				const { datatype, defaultValue } = resolveColumnType(c)
				let def = `"${c.name}" ${datatype}`
				if (c.nullable === false) def += ' NOT NULL'
				if (defaultValue) def += ` DEFAULT ${defaultValue}`
				return def
			})
			.join(',\n  ')
		const pkCols = table.columns.filter((c) => c.primaryKey).map((c) => `"${c.name}"`)
		const pkLine = pkCols.length > 0 ? `,\n  PRIMARY KEY (${pkCols.join(', ')})` : ''
		const qualifiedName = `"${change.schemaName}"."${change.tableName}"`
		let sql = `BEGIN;\nCREATE TABLE ${qualifiedName} (\n  ${colDefs}${pkLine}\n);`
		for (const fk of table.foreignKeys ?? []) {
			const fkSql = renderForeignKey(fk, {
				useSchema: true,
				dbType: 'postgresql',
				tableName: change.tableName
			})
			sql += `\nALTER TABLE ${qualifiedName} ADD ${fkSql};`
		}
		sql += '\nCOMMIT;'
		return sql
	}
	if (change.kind === 'removed') {
		return `BEGIN;\nDROP TABLE IF EXISTS "${change.schemaName}"."${change.tableName}";\nCOMMIT;`
	}
	return ''
}
