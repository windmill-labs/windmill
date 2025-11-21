<script lang="ts">
	import type { Schema, SchemaProperty } from '$lib/common'
	import { ColumnIdentity, type ColumnMetadata, type ColumnDef } from './utils'

	import init, {
		parse_sql,
		parse_mysql,
		parse_bigquery,
		parse_snowflake,
		parse_mssql,
		parse_duckdb
	} from 'windmill-sql-datatype-parser-wasm'
	import wasmUrl from 'windmill-sql-datatype-parser-wasm/windmill_sql_datatype_parser_wasm_bg.wasm?url'

	init(wasmUrl)

	import { argSigToJsonSchemaType } from 'windmill-utils-internal'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { untrack } from 'svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import type { DbType } from '$lib/components/dbTypes'

	type FieldMetadata = {
		type: string
		name: string
		isPrimaryKey: boolean
		defaultValue: string | undefined
		identity: ColumnIdentity
		nullable: 'YES' | 'NO'
	}

	function parseSQLArgs(field: string, dbType: DbType): string {
		let rawType = ''
		switch (dbType) {
			case 'mysql':
				rawType = parse_mysql(field)
				break
			case 'postgresql':
				rawType = parse_sql(field)
				break
			case 'bigquery':
				rawType = parse_bigquery(field)
				break
			case 'snowflake':
				rawType = parse_snowflake(field)
				break
			case 'ms_sql_server':
				rawType = parse_mssql(field)
				break
			case 'duckdb':
				rawType = parse_duckdb(field)
				break
		}

		return rawType
	}

	function rawTypeToSchemaType(typ: string) {
		if (typ.startsWith('list-')) {
			return {
				list: rawTypeToSchemaType(typ.replace('list-', ''))
			}
		} else if (typ == 'str') {
			return {
				str: undefined
			}
		} else {
			return typ
		}
	}
	async function buildSchema(): Promise<Schema> {
		const properties: { [name: string]: SchemaProperty } = {}
		const required: string[] = []

		await init(wasmUrl)

		fields?.forEach((field) => {
			const schemaProperty: SchemaProperty = {
				type: 'string'
			}

			const parsedArg = columnDefs.find((arg) => arg.field === field.name)
			if (parsedArg) {
				let typ: any = rawTypeToSchemaType(
					parseSQLArgs(parsedArg.datatype.split('(')[0].trim(), dbType)
				)
				argSigToJsonSchemaType(typ, schemaProperty)
			}

			if (field.defaultValue) {
				if (schemaProperty.type === 'number' || schemaProperty.type === 'integer') {
					schemaProperty.default = field.defaultValue ? Number(field.defaultValue) : undefined
				} else if (schemaProperty.type === 'boolean') {
					schemaProperty.default = field.defaultValue?.toLocaleLowerCase() === 'true'
				} else {
					schemaProperty.default = field.defaultValue
				}
			}
			if (field.type === 'timestamp without time zone' || field.type === 'timestamp') {
				schemaProperty.format = 'naive-date-time'
			}
			if (field.type === 'timestamp with time zone' || field.type === 'timestamptz') {
				schemaProperty.format = 'date-time'
			}

			properties[field.name] = schemaProperty

			const isRequired =
				(field.isPrimaryKey || field.defaultValue === undefined || field.defaultValue === null) &&
				field.nullable !== 'YES' &&
				![ColumnIdentity.Always, ColumnIdentity.ByDefault].includes(field.identity)

			if (isRequired) {
				required.push(field.name)
			}
		})

		return {
			$schema: 'http://json-schema.org/draft-07/schema#',
			type: 'object',
			properties,
			required
		}
	}

	interface Props {
		args?: Record<string, any>
		dbType?: DbType
		columnDefs?: Array<ColumnDef & ColumnMetadata>
		isInsertable?: boolean
	}

	let {
		args = $bindable({}),
		dbType = 'postgresql',
		columnDefs = [],
		isInsertable = $bindable(false)
	}: Props = $props()

	let fields = $derived(
		columnDefs
			?.filter((t) => {
				const shouldFilter =
					t.isidentity === ColumnIdentity.Always ||
					t?.hideInsert === true ||
					t.defaultvalue?.startsWith('nextval(') // exclude postgres serial/auto increment fields
				return !shouldFilter
			})
			.map((column) => {
				const type = column.datatype
				const name = column.field
				const isPrimaryKey = column.isprimarykey
				const defaultValue = column.defaultValueNull ? null : column.defaultUserValue

				return {
					type,
					name,
					isPrimaryKey,
					defaultValue,
					identity: column.isidentity,
					nullable: column.isnullable
				}
			}) as FieldMetadata[] | undefined
	)

	let schemaPromise = usePromise(buildSchema)
	let schema = $derived(schemaPromise.value)
	$effect(() => {
		fields && dbType && untrack(() => schemaPromise.refresh())
	})
	$effect(() => {
		if (schema) {
			const requiredFields = schema.required ?? []
			const filledFields = Object.keys(args).filter(
				(key) => args[key] !== undefined && args[key] !== null && args[key] !== ''
			)

			isInsertable = requiredFields.every((field) => filledFields.includes(field))
		}
	})
</script>

{#if schema}
	<SchemaForm onlyMaskPassword {schema} bind:args>
		{#snippet actions({ item })}
			{@const disabled = fields?.[fields?.findIndex((f) => f.name === item.id)]?.nullable != 'YES'}
			{#if !disabled}
				<Toggle
					options={{ right: 'NULL' }}
					class="pl-2"
					textClass="text-primary"
					bind:checked={
						() => args[item.id] === null,
						(v) => {
							if (!schema?.properties[item.id]) return
							if (v) {
								schema.properties[item.id].nullable = true
								schema.properties[item.id].disabled = true
								args[item.id] = null
							} else {
								delete schema.properties[item.id].disabled
								delete schema.properties[item.id].nullable
								args[item.id] = schema.properties[item.id].default ?? ''
							}
						}
					}
				/>
			{/if}
		{/snippet}
	</SchemaForm>
{/if}
