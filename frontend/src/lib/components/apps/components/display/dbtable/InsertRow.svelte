<script lang="ts">
	import type { Schema, SchemaProperty } from '$lib/common'
	import LightweightSchemaForm from '$lib/components/LightweightSchemaForm.svelte'
	import {
		ColumnIdentity,
		getFieldType,
		type ColumnMetadata,
		type DbType,
		type ColumnDef
	} from './utils'

	import wasmUrl from 'windmill-parser-wasm/windmill_parser_wasm_bg.wasm?url'
	import init, {
		parse_sql,
		parse_mysql,
		parse_bigquery,
		parse_snowflake,
		parse_mssql
	} from 'windmill-parser-wasm'
	import type { MainArgSignature } from '$lib/gen'
	import { makeInsertQuery } from './queries/insert'
	import { argSigToJsonSchemaType } from '$lib/infer'
	init(wasmUrl)

	export let args: Record<string, any> = {}
	export let dbType: DbType = 'postgresql'

	let schema: Schema | undefined = undefined

	export let columnDefs: Array<ColumnDef & ColumnMetadata> = []

	type FieldMetadata = {
		type: string
		name: string
		isPrimaryKey: boolean
		defaultValue: string | undefined
		fieldType: 'text' | 'number' | 'checkbox' | 'date'
		identity: ColumnIdentity
		nullable: 'YES' | 'NO'
	}

	$: fields = columnDefs
		?.filter((t) => {
			const shouldFilter = t.isidentity !== ColumnIdentity.Always && t?.hideInsert === true

			return !shouldFilter
		})
		.map((column) => {
			const type = column.datatype
			const name = column.field
			const isPrimaryKey = column.isprimarykey
			const defaultValue = column.defaultValueNull ? null : column.defaultUserValue
			const fieldType = getFieldType(type, dbType)

			return {
				type,
				name,
				isPrimaryKey,
				defaultValue,
				fieldType,
				identity: column.isidentity,
				nullable: column.isnullable
			}
		}) as FieldMetadata[] | undefined

	async function parseSQLArgs(code: string, dbType: DbType) {
		await init(wasmUrl)

		let rawSchema = ''
		switch (dbType) {
			case 'mysql':
				rawSchema = parse_mysql(code)
				break
			case 'postgresql':
				rawSchema = parse_sql(code)
				break
			case 'bigquery':
				rawSchema = parse_bigquery(code)
				break
			case 'snowflake':
				rawSchema = parse_snowflake(code)
				break
			case 'ms_sql_server':
				rawSchema = parse_mssql(code)
				break
			default:
				throw new Error('Language not supported')
		}

		const args: MainArgSignature = JSON.parse(rawSchema)

		return args
	}

	async function builtSchema(fields: FieldMetadata[], dbType: DbType) {
		const properties: { [name: string]: SchemaProperty } = {}
		const required: string[] = []

		const insertCode = makeInsertQuery('ignoredtable', columnDefs, dbType)
		const args = await parseSQLArgs(insertCode, dbType)

		fields.forEach((field) => {
			const schemaProperty: SchemaProperty = {
				type: 'string'
			}

			const parsedArg = args.args.find((arg) => arg.name === field.name)
			if (parsedArg) {
				argSigToJsonSchemaType(parsedArg.typ, schemaProperty)
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

	$: builtSchema(fields ?? [], dbType)

	export let isInsertable: boolean = false

	$: if (schema) {
		const requiredFields = schema.required ?? []
		const filledFields = Object.keys(args).filter(
			(key) => args[key] !== undefined && args[key] !== null
		)
		isInsertable = requiredFields.every((field) => filledFields.includes(field))
	}
</script>

{#if schema}
	<LightweightSchemaForm {schema} bind:args />
{/if}
