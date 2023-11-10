<script lang="ts">
	import { JobService, Preview } from '$lib/gen'
	import {
		dbSchemas,
		workspaceStore,
		type DBSchema,
		type GraphqlSchema,
		type SQLSchema
	} from '$lib/stores'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import { sendUserToast, tryEvery } from '$lib/utils'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { buildClientSchema, getIntrospectionQuery, printSchema } from 'graphql'
	import GraphqlSchemaViewer from './GraphqlSchemaViewer.svelte'
	import { RefreshCcw } from 'lucide-svelte'

	export let resourceType: string | undefined
	export let resourcePath: string | undefined = undefined
	let dbSchema: DBSchema | undefined = undefined
	let loading = false

	let drawer: Drawer | undefined

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
			code: `import { BigQuery } from '@google-cloud/bigquery@7.2.0';
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
			lang: 'bun',
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
						schema[row.TABLE_SCHEMA][row.TABLE_NAME][row.COLUMN_NAME]['default'] =
							row.COLUMN_DEFAULT
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

	function resourceTypeToLang(rt: string) {
		if (rt === 'ms_sql_server') {
			return 'mssql'
		} else {
			return rt
		}
	}

	async function getSchema() {
		if (!resourceType || !resourcePath) return
		loading = true

		const job = await JobService.runScriptPreview({
			workspace: $workspaceStore!,
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
						workspace: $workspaceStore!,
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
								$dbSchemas[resourcePath] = {
									lang: resourceTypeToLang(resourceType) as SQLSchema['lang'],
									schema,
									publicOnly: !!schema.public || !!schema.PUBLIC || !!schema.dbo
								}
							} else {
								if (typeof testResult.result !== 'object' || !('__schema' in testResult.result)) {
									console.error('Invalid GraphQL schema')
									if (drawer?.isOpen()) {
										sendUserToast('Invalid GraphQL schema', true)
									}
								} else {
									$dbSchemas[resourcePath] = {
										lang: 'graphql',
										schema: testResult.result
									}
								}
							}
						}
					}
				}
				loading = false
			},
			timeoutCode: async () => {
				loading = false
				console.error('Could not query schema within 5s')
				if (drawer?.isOpen()) {
					sendUserToast('Could not query schema within 5s', true)
				}
				try {
					await JobService.cancelQueuedJob({
						workspace: $workspaceStore!,
						id: job,
						requestBody: {
							reason: 'Could not query schema within 5s'
						}
					})
				} catch (err) {
					console.error(err)
				}
			},
			interval: 500,
			timeout: 5000
		})
	}

	function formatSchema(dbSchema: DBSchema) {
		if (dbSchema.lang !== 'graphql' && dbSchema.publicOnly) {
			return dbSchema.schema.public || dbSchema.schema.PUBLIC || dbSchema.schema.dbo || dbSchema
		} else if (dbSchema.lang === 'mysql' && Object.keys(dbSchema.schema).length === 1) {
			return dbSchema.schema[Object.keys(dbSchema.schema)[0]]
		} else {
			return dbSchema.schema
		}
	}

	function formatGraphqlSchema(dbSchema: GraphqlSchema) {
		return printSchema(buildClientSchema(dbSchema.schema))
	}

	$: resourcePath &&
		Object.keys(scripts).includes(resourceType || '') &&
		!$dbSchemas[resourcePath] &&
		getSchema()

	$: dbSchema = resourcePath && resourcePath in $dbSchemas ? $dbSchemas[resourcePath] : undefined
</script>

{#if dbSchema}
	<Button
		size="xs"
		variant="border"
		color="blue"
		spacingSize="xs2"
		btnClasses="mt-1"
		on:click={drawer?.openDrawer}
	>
		Explore schema
	</Button>
	<Drawer bind:this={drawer}>
		<DrawerContent title="Schema Explorer" on:close={drawer.closeDrawer}>
			<svelte:fragment slot="actions">
				<Button
					on:click={getSchema}
					startIcon={{
						icon: RefreshCcw
					}}
					{loading}
					size="xs"
					color="light"
				>
					Refresh
				</Button>
			</svelte:fragment>
			{#if dbSchema.lang !== 'graphql' && (dbSchema.schema?.public || dbSchema.schema?.PUBLIC || dbSchema.schema?.dbo)}
				<ToggleButtonGroup class="mb-4" bind:selected={dbSchema.publicOnly}>
					<ToggleButton value={true} label={dbSchema.schema.dbo ? 'Dbo' : 'Public'} />
					<ToggleButton value={false} label="All" />
				</ToggleButtonGroup>
			{/if}
			{#if dbSchema.lang === 'graphql'}
				<GraphqlSchemaViewer code={formatGraphqlSchema(dbSchema)} class="h-full" />
			{:else}
				<ObjectViewer json={formatSchema(dbSchema)} pureViewer />
			{/if}
		</DrawerContent>
	</Drawer>
{/if}
