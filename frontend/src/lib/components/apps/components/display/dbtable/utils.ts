import type { AppInput, RunnableByName } from '../../../inputType'
import { JobService, Preview } from '$lib/gen'
import type { DBSchema, DBSchemas, GraphqlSchema, SQLSchema } from '$lib/stores'
import { buildClientSchema, getIntrospectionQuery, printSchema } from 'graphql'
import { tryEvery } from '$lib/utils'

export function makeQuery(
	table: string,
	tableMetadata: TableMetadata,
	whereClause: string | undefined
) {
	if (!table) throw new Error('Table name is required')

	const filteredColumns = tableMetadata
		.filter((x) => x != undefined)
		.map((column) => `"${column?.field}"::text`)

	let selectClause = filteredColumns.join(', ')

	let orderBy = `
	${tableMetadata
		.map(
			(column) =>
				`
(CASE WHEN $4 = '${column.field}' AND $5 IS false THEN "${column.field}"::text END),
(CASE WHEN $4 = '${column.field}' AND $5 IS true THEN "${column.field}"::text END) DESC`
		)
		.join(',\n')}`

	let query = `
-- $1 limit
-- $2 offset
-- $3 quicksearch
-- $4 orderBy
-- $5 is_desc

SELECT ${selectClause} FROM "${table}" WHERE `
	if (whereClause) {
		query += ` ${whereClause} AND `
	}
	query += ` ($3 = '' OR "${table}"::text ILIKE '%' || $3 || '%')`

	query += ` ORDER BY ${orderBy}`
	query += ` LIMIT $1::INT OFFSET $2::INT`

	return query
}

export function createPostgresInsert(
	table: string,
	columns: ColumnDef[],
	resource: string
): AppInput {
	return {
		runnable: {
			name: 'AppDbExplorer',
			type: 'runnableByName',
			inlineScript: {
				content: makeInsertQuery(table, columns),
				language: Preview.language.POSTGRESQL,
				schema: {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					properties: {},
					required: ['database'],
					type: 'object'
				}
			}
		},
		fields: {
			database: {
				type: 'static',
				value: resource,
				fieldType: 'object',
				format: 'resource-postgresql'
			}
		},
		type: 'runnable',
		fieldType: 'object'
	}
}

export function makeInsertQuery(table: string, columns: ColumnDef[]) {
	if (!table) throw new Error('Table name is required')

	const columnsInsert = columns.filter((x) => !x.hideInsert)
	const columnsDefault = columns.filter(
		(x) => x.hideInsert && (x.overrideDefaultValue || x.defaultvalue === null)
	)

	const allInsertColumns = columnsInsert.concat(columnsDefault)
	// Constructing the query
	const query = `
${columnsInsert.map((column, i) => `-- $${i + 1} ${column.field}`).join('\n')}

INSERT INTO ${table} (${allInsertColumns.map((c) => c.field).join(', ')}) 
VALUES (${columnsInsert.map((c, i) => `$${i + 1}::${c.datatype}`).join(', ')}${
		columnsDefault.length > 0 ? ',' : ''
	} ${columnsDefault
		.map((c) => (c.defaultValueNull ? 'NULL' : `${c.defaultUserValue}::${c.datatype}`))
		.join(', ')})`

	return query
}

export function getPrimaryKeys(tableMetadata?: TableMetadata): string[] {
	let r = tableMetadata?.filter((x) => x.isprimarykey)?.map((x) => x.field) ?? []
	if (r?.length === 0) {
		r = tableMetadata?.map((x) => x.field) ?? []
	}
	return r ?? []
}

export function createPostgresInput(
	resource: string,
	table: string | undefined,
	columns: TableMetadata,
	whereClause: string | undefined
): AppInput | undefined {
	if (!resource || !table || !columns) {
		// Return undefined if resource or table is not defined
		return undefined
	}

	const getRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: makeQuery(table, columns, whereClause),
			language: Preview.language.POSTGRESQL
		}
	}

	const getQuery: AppInput = {
		runnable: getRunnable,
		fields: {
			database: {
				type: 'static',
				value: resource,
				fieldType: 'object',
				format: 'resource-postgresql'
			}
		},
		type: 'runnable',
		fieldType: 'object'
	}

	return getQuery
}

export function createUpdatePostgresInput(
	resource: string,
	table: string,
	column: ColumnMetadata,
	columns: ColumnMetadata[]
): AppInput | undefined {
	if (!resource || !table) {
		return undefined
	}

	const query = updateWithAllValues()

	function updateWithAllValues() {
		let query = `
-- $1 valueToUpdate
${columns.map((c, i) => `-- $${i + 2} ${c.field}`).join('\n')}
		
UPDATE ${table} SET ${column.field} = $1::text::${column.datatype} WHERE 
${columns.map((c, i) => `${c.field} = $${i + 2}::text::${c.datatype} `).join(' AND ')}		
RETURNING 1`

		return query
	}

	const updateRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: query,
			language: Preview.language.POSTGRESQL,
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {},
				required: ['database'],
				type: 'object'
			}
		}
	}

	const updateQuery: AppInput = {
		runnable: updateRunnable,
		fields: {
			database: {
				type: 'static',
				value: resource,
				fieldType: 'object',
				format: 'resource-postgresql'
			}
		},
		type: 'runnable',
		fieldType: 'object'
	}

	return updateQuery
}

export function createDeletePostgresInput(
	resource: string,
	table: string,
	columns: ColumnMetadata[]
): AppInput | undefined {
	if (!resource || !table) {
		return undefined
	}

	const query = updateWithAllValues()

	function updateWithAllValues() {
		let query = `
${columns.map((c, i) => `-- $${i + 1} ${c.field}`).join('\n')}

DELETE FROM ${table} WHERE ${columns
			.map((c, i) => `${c.field} = $${i + 1}::text::${c.datatype}`)
			.join(' AND ')} RETURNING 1;`

		return query
	}

	const updateRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: query,
			language: Preview.language.POSTGRESQL,
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {},
				required: ['database'],
				type: 'object'
			}
		}
	}

	const updateQuery: AppInput = {
		runnable: updateRunnable,
		fields: {
			database: {
				type: 'static',
				value: resource,
				fieldType: 'object',
				format: 'resource-postgresql'
			}
		},
		type: 'runnable',
		fieldType: 'object'
	}

	return updateQuery
}

export function getCountPostgresql(resource: string, table: string): AppInput | undefined {
	if (!resource || !table) {
		return undefined
	}

	const query = `
-- $1 quicksearch
SELECT COUNT(*) FROM ${table} WHERE ($1 = '' OR ${table}::text ILIKE '%' || $1 || '%')`

	const updateRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: query,
			language: Preview.language.POSTGRESQL,
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {
					database: {
						description: 'Database name',
						type: 'object',
						format: 'resource-postgresql'
					}
				},
				required: ['database'],
				type: 'object'
			}
		}
	}

	const updateQuery: AppInput = {
		runnable: updateRunnable,
		fields: {
			database: {
				type: 'static',
				value: resource,
				fieldType: 'object',
				format: 'resource-postgresql'
			}
		},
		type: 'runnable',
		fieldType: 'object'
	}

	return updateQuery
}

export enum ColumnIdentity {
	ByDefault = 'By Default',
	Always = 'Always',
	No = 'No'
}

export type ColumnMetadata = {
	field: string
	datatype: string
	defaultvalue: string
	isprimarykey: boolean
	isidentity: ColumnIdentity
	isnullable: 'YES' | 'NO'
	isenum: boolean
}
export type TableMetadata = ColumnMetadata[]

export type ColumnDef = {
	minWidth: number
	hide: boolean
	flex: number
	sort: 'asc' | 'desc'
	sortIndex: number
	aggFunc: string
	pivot: boolean
	pivotIndex: number
	pinned: 'left' | 'right' | boolean
	rowGroup: boolean
	rowGroupIndex: number
	valueFormatter: string
	valueParser: string
	field: string
	headerName: string
	// DBExplorer
	ignored: boolean
	hideInsert: boolean
	editable: boolean
	overrideDefaultValue: boolean
	defaultUserValue: any
	defaultValueNull: boolean
} & ColumnMetadata

export async function loadTableMetaData(
	resource: string,
	workspace: string | undefined,
	table: string | undefined
): Promise<TableMetadata | undefined> {
	if (!resource || !table || !workspace) {
		return undefined
	}

	const code = `
	SELECT 
	a.attname as field,
	pg_catalog.format_type(a.atttypid, a.atttypmod) as DataType,
	(SELECT substring(pg_catalog.pg_get_expr(d.adbin, d.adrelid, true) for 128)
	 FROM pg_catalog.pg_attrdef d
	 WHERE d.adrelid = a.attrelid AND d.adnum = a.attnum AND a.atthasdef) as DefaultValue,
	(SELECT CASE WHEN i.indisprimary THEN true ELSE 'NO' END
	 FROM pg_catalog.pg_class tbl, pg_catalog.pg_class idx, pg_catalog.pg_index i, pg_catalog.pg_attribute att
	 WHERE tbl.oid = a.attrelid AND idx.oid = i.indexrelid AND att.attrelid = tbl.oid
							 AND i.indrelid = tbl.oid AND att.attnum = any(i.indkey) AND att.attname = a.attname LIMIT 1) as IsPrimaryKey,
	CASE a.attidentity
			WHEN 'd' THEN 'By Default'
			WHEN 'a' THEN 'Always'
			ELSE 'No'
	END as IsIdentity,
	CASE a.attnotnull
			WHEN false THEN 'YES'
			ELSE 'NO'
	END as IsNullable,
	(SELECT true
	 FROM pg_catalog.pg_enum e
	 WHERE e.enumtypid = a.atttypid FETCH FIRST ROW ONLY) as IsEnum
FROM pg_catalog.pg_attribute a
WHERE a.attrelid = (SELECT oid FROM pg_catalog.pg_class WHERE relname = '${table}') 
	AND a.attnum > 0 AND NOT a.attisdropped
ORDER BY a.attnum;

`

	const maxRetries = 3
	let attempts = 0

	while (attempts < maxRetries) {
		try {
			const job = await JobService.runScriptPreview({
				workspace: workspace,
				requestBody: {
					language: Preview.language.POSTGRESQL,
					content: code,
					args: {
						database: resource
					}
				}
			})

			await new Promise((resolve) => setTimeout(resolve, 3000))

			const testResult = await JobService.getCompletedJob({
				workspace: workspace,
				id: job
			})

			if (testResult.success) {
				attempts = maxRetries

				return testResult.result
			} else {
				attempts++
			}
		} catch (error) {
			attempts++
		}
		// Exponential back-off
		await new Promise((resolve) => setTimeout(resolve, 2000 * attempts))
	}

	console.error('Failed to load table metadata after maximum retries.')
	return undefined
}

export function resourceTypeToLang(rt: string) {
	if (rt === 'ms_sql_server') {
		return 'mssql'
	} else {
		return rt
	}
}

const scripts: Record<
	string,
	{
		code: string
		lang: string
		processingFn?: (any: any) => SQLSchema['schema']
		argName: string
	}
> = {
	postgresql: {
		code: `SELECT table_name, column_name, udt_name, column_default, is_nullable, table_schema FROM information_schema.columns WHERE table_schema != 'pg_catalog' AND table_schema != 'information_schema'`,
		processingFn: (rows) => {
			const schemas = rows.reduce((acc, a) => {
				const table_schema = a.table_schema
				delete a.table_schema
				acc[table_schema] = acc[table_schema] || []
				acc[table_schema].push(a)
				return acc
			}, {})
			const data = {}
			for (const key in schemas) {
				data[key] = schemas[key].reduce((acc, a) => {
					const table_name = a.table_name
					delete a.table_name
					acc[table_name] = acc[table_name] || {}
					const p: {
						type: string
						required: boolean
						default?: string
					} = {
						type: a.udt_name,
						required: a.is_nullable === 'NO'
					}
					if (a.column_default) {
						p.default = a.column_default
					}
					acc[table_name][a.column_name] = p
					return acc
				}, {})
			}
			return data
		},
		lang: 'postgresql',
		argName: 'database'
	},
	mysql: {
		code: "select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT from information_schema.columns where table_schema != 'information_schema'",
		processingFn: (rows) => {
			const schemas = rows.reduce((acc, a) => {
				const table_schema = a.TABLE_SCHEMA
				delete a.TABLE_SCHEMA
				acc[table_schema] = acc[table_schema] || []
				acc[table_schema].push(a)
				return acc
			}, {})
			const data = {}
			for (const key in schemas) {
				data[key] = schemas[key].reduce((acc, a) => {
					const table_name = a.TABLE_NAME
					delete a.TABLE_NAME
					acc[table_name] = acc[table_name] || {}
					const p: {
						type: string
						required: boolean
						default?: string
					} = {
						type: a.DATA_TYPE,
						required: a.is_nullable === 'NO'
					}
					if (a.column_default) {
						p.default = a.COLUMN_DEFAULT
					}
					acc[table_name][a.COLUMN_NAME] = p
					return acc
				}, {})
			}
			return data
		},
		lang: 'mysql',
		argName: 'database'
	},
	graphql: {
		code: getIntrospectionQuery(),
		lang: 'graphql',
		argName: 'api'
	},
	bigquery: {
		code: `import { BigQuery } from 'npm:@google-cloud/bigquery@7.2.0';
export async function main(args: bigquery) {
const bq = new BigQuery({
	credentials: args
})
const [datasets] = await bq.getDatasets();
const schema = {}
for (const dataset of datasets) {
	schema[dataset.id] = {}
	const query = "SELECT table_name, ARRAY_AGG(STRUCT(if(is_nullable = 'YES', true, false) AS required, column_name AS name, data_type AS type, if(column_default = 'NULL', null, column_default) AS \`default\`) ORDER BY ordinal_position) AS schema \
FROM \`{dataset.id}\`.INFORMATION_SCHEMA.COLUMNS \
GROUP BY table_name".replace('{dataset.id}', dataset.id)
	const [rows] = await bq.query(query)
	for (const row of rows) {
		schema[dataset.id][row.table_name] = {}
		for (const col of row.schema) {
			const colName = col.name
			delete col.name
			if (col.default === null) {
				delete col.default
			}
			schema[dataset.id][row.table_name][colName] = col
		}
	}
}
return schema
}`, // nested template literals
		lang: 'deno',
		argName: 'args'
	},
	snowflake: {
		code: `select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT, IS_NULLABLE from information_schema.columns where table_schema != 'INFORMATION_SCHEMA'`,
		lang: 'snowflake',
		processingFn: (rows) => {
			const schema = {}
			for (const row of rows) {
				if (!(row.TABLE_SCHEMA in schema)) {
					schema[row.TABLE_SCHEMA] = {}
				}
				if (!(row.TABLE_NAME in schema[row.TABLE_SCHEMA])) {
					schema[row.TABLE_SCHEMA][row.TABLE_NAME] = {}
				}
				schema[row.TABLE_SCHEMA][row.TABLE_NAME][row.COLUMN_NAME] = {
					type: row.DATA_TYPE,
					required: row.IS_NULLABLE === 'YES'
				}
				if (row.COLUMN_DEFAULT !== null) {
					schema[row.TABLE_SCHEMA][row.TABLE_NAME][row.COLUMN_NAME]['default'] = row.COLUMN_DEFAULT
				}
			}
			return schema
		},
		argName: 'database'
	},
	ms_sql_server: {
		argName: 'database',
		code: `select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT from information_schema.columns where table_schema != 'sys'`,
		lang: 'mssql',
		processingFn: (rows) => {
			const schemas = rows[0].reduce((acc, a) => {
				const table_schema = a.TABLE_SCHEMA
				delete a.TABLE_SCHEMA
				acc[table_schema] = acc[table_schema] || []
				acc[table_schema].push(a)
				return acc
			}, {})
			const data = {}
			for (const key in schemas) {
				data[key] = schemas[key].reduce((acc, a) => {
					const table_name = a.TABLE_NAME
					delete a.TABLE_NAME
					acc[table_name] = acc[table_name] || {}
					const p: {
						type: string
						required: boolean
						default?: string
					} = {
						type: a.DATA_TYPE,
						required: a.is_nullable === 'NO'
					}
					if (a.column_default) {
						p.default = a.COLUMN_DEFAULT
					}
					acc[table_name][a.COLUMN_NAME] = p
					return acc
				}, {})
			}
			return data
		}
	}
}

export { scripts }
export async function getDbSchemas(
	resourceType: string,
	resourcePath: string,
	workspace: string | undefined,
	dbSchemas: DBSchemas,
	errorCallback: (message: string) => void
): Promise<void> {
	return new Promise(async (resolve, reject) => {
		if (!resourceType || !resourcePath || !workspace) {
			resolve()
			return
		}

		const job = await JobService.runScriptPreview({
			workspace: workspace,
			requestBody: {
				language: scripts[resourceType].lang as Preview.language,
				content: scripts[resourceType].code,
				args: {
					[scripts[resourceType].argName]: '$res:' + resourcePath
				}
			}
		})

		tryEvery({
			tryCode: async () => {
				if (resourcePath) {
					const testResult = await JobService.getCompletedJob({
						workspace,
						id: job
					})
					if (!testResult.success) {
						console.error(testResult.result?.['error']?.['message'])
					} else {
						if (resourceType !== undefined) {
							if (resourceType !== 'graphql') {
								const { processingFn } = scripts[resourceType]
								const schema =
									processingFn !== undefined ? processingFn(testResult.result) : testResult.result
								dbSchemas[resourcePath] = {
									lang: resourceTypeToLang(resourceType) as SQLSchema['lang'],
									schema,
									publicOnly: !!schema.public || !!schema.PUBLIC || !!schema.dbo
								}
							} else {
								if (typeof testResult.result !== 'object' || !('__schema' in testResult.result)) {
									console.error('Invalid GraphQL schema')

									errorCallback('Invalid GraphQL schema')
								} else {
									dbSchemas[resourcePath] = {
										lang: 'graphql',
										schema: testResult.result
									}
								}
							}
						}
					}
					resolve()
				}
			},
			timeoutCode: async () => {
				console.error('Could not query schema within 5s')
				errorCallback('Could not query schema within 5s')
				try {
					await JobService.cancelQueuedJob({
						workspace,
						id: job,
						requestBody: {
							reason: 'Could not query schema within 5s'
						}
					})
				} catch (err) {
					console.error(err)
				}
				reject()
			},
			interval: 500,
			timeout: 5000
		})
	})
}

export function formatSchema(dbSchema: DBSchema) {
	if (dbSchema.lang !== 'graphql' && dbSchema.publicOnly) {
		return dbSchema.schema.public || dbSchema.schema.PUBLIC || dbSchema.schema.dbo || dbSchema
	} else if (dbSchema.lang === 'mysql' && Object.keys(dbSchema.schema).length === 1) {
		return dbSchema.schema[Object.keys(dbSchema.schema)[0]]
	} else {
		return dbSchema.schema
	}
}

export function formatGraphqlSchema(dbSchema: GraphqlSchema): string {
	return printSchema(buildClientSchema(dbSchema.schema))
}

/**
 * Base class for embedding a svelte component within an AGGrid call.
 * See: https://stackoverflow.com/a/72608215
 */
import type { ICellRendererComp, ICellRendererParams } from 'ag-grid-community'

/**
 * Class for defining a cell renderer.
 * If you don't need to define a separate class you could use cellRendererFactory
 * to create a component with the column definitions.
 */
export abstract class AbstractCellRenderer implements ICellRendererComp {
	eGui: any
	protected value: any
	protected params: any
	constructor(parentElement = 'span') {
		// create empty span (or other element) to place svelte component in
		this.eGui = document.createElement(parentElement)
	}

	init(params: ICellRendererParams & { onClick?: (data: any) => void }) {
		this.value = params.value
		this.createComponent(params)
		this.eGui.addEventListener('click', () => params.onClick?.(params.data))
		this.params = params
	}

	getGui() {
		return this.eGui
	}

	refresh(params: ICellRendererParams) {
		this.value = params.value
		this.eGui.innerHTML = ''

		return true
	}

	/**
	 * Define and create the svelte component to use in the cell
	 * @example
	 * // This is all you need to do within this method: create the component with new, specify the target
	 * // is the class, and pass in props via the params.
	 * new CampusIcon({
	 *    target: this.eGui,
	 *    props: {
	 *        color: params.data?.color,
	 *        name: params.data?.name
	 * }
	 * @param params params for rendering the call, including the value for the cell
	 */
	abstract createComponent(params: ICellRendererParams): void
}

/**
 * Creates a cell renderer using the given callback for how to initialise a svelte component.
 * See AbstractCellRenderer.createComponent
 * @param svelteComponent function for how to create the svelte component
 * @returns
 */
export function cellRendererFactory(
	svelteComponent: (cell: AbstractCellRenderer, params: ICellRendererParams) => void
) {
	class Renderer extends AbstractCellRenderer {
		createComponent(params: ICellRendererParams<any, any>): void {
			svelteComponent(this, params)
		}
	}
	return Renderer
}
