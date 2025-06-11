<script lang="ts">
	import type { Schema, SchemaProperty } from '$lib/common'
	import { ColumnIdentity, type ColumnMetadata, type DbType, type ColumnDef } from './utils'

	import init, {
		parse_sql,
		parse_mysql,
		parse_bigquery,
		parse_snowflake,
		parse_mssql
	} from 'windmill-sql-datatype-parser-wasm'
	import wasmUrl from 'windmill-sql-datatype-parser-wasm/windmill_sql_datatype_parser_wasm_bg.wasm?url'

	init(wasmUrl)

	import { argSigToJsonSchemaType } from '$lib/inferArgSig'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { untrack } from 'svelte'

	let schema: Schema | undefined = $state(undefined)

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
			default:
				throw new Error('Language not supported')
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
	async function builtSchema(fields: FieldMetadata[], dbType: DbType) {
		const properties: { [name: string]: SchemaProperty } = {}
		const required: string[] = []

		await init(wasmUrl)

		fields.forEach((field) => {
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

			properties[field.name] = schemaProperty

			const isRequired =
				(field.isPrimaryKey || field.defaultValue === undefined || field.defaultValue === null) &&
				field.nullable !== 'YES' &&
				![ColumnIdentity.Always, ColumnIdentity.ByDefault].includes(field.identity)

			if (isRequired) {
				required.push(field.name)
			}
		})

		schema = {
			$schema: 'http://json-schema.org/draft-07/schema#',
			type: 'object',
			properties,
			required
		} as Schema
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
				const shouldFilter = t.isidentity === ColumnIdentity.Always || t?.hideInsert === true

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
	$effect(() => {
		;[fields, dbType]
		untrack(() => builtSchema(fields ?? [], dbType))
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
	<SchemaForm onlyMaskPassword {schema} bind:args />
{/if}
