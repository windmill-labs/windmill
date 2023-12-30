<script lang="ts">
	import type { Schema, SchemaProperty } from '$lib/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { ColumnIdentity, type ColumnMetadata } from './utils'

	export let args: Record<string, any> = {}
	export let columnDefs: Array<
		{
			field: string
			ignored: boolean
			insert: boolean
			defaultValue: any
		} & ColumnMetadata
	> = []

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
			const shouldFilter = t.isidentity !== ColumnIdentity.Always && t?.insert === true

			return !shouldFilter
		})
		.map((column) => {
			const type = column.datatype
			const name = column.field
			const isPrimaryKey = column.isprimarykey
			const defaultValue = column.defaultvalue

			const baseType = type.split('(')[0]
			const validTextTypes = ['character varying', 'text']
			const validNumberTypes = ['integer', 'bigint', 'numeric', 'double precision']
			const validDateTypes = ['date', 'timestamp without time zone', 'timestamp with time zone']

			const fieldType = validTextTypes.includes(baseType)
				? 'text'
				: validNumberTypes.includes(baseType)
				? 'number'
				: baseType === 'boolean'
				? 'checkbox'
				: validDateTypes.includes(baseType)
				? 'date'
				: 'text'

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

	function extractDefaultValue(defaultValue: string | undefined): string | undefined {
		if (defaultValue && defaultValue.includes('::')) {
			const val = defaultValue.split('::')[0]
			if (val.startsWith("'") && val.endsWith("'")) {
				return val.slice(1, -1)
			}
			return val
		}
		return defaultValue
	}

	function parsePsqlDate(date: string | undefined): string | undefined {
		if (date && date.includes('::')) {
			const val = date.split('::')[0]
			if (val.startsWith("'") && val.endsWith("'")) {
				return val.slice(1, -1)
			}
			return val
		}

		if (date && date.includes('now()')) {
			// format is 2023-06-19 15:07:51.168209 UTC
			const now = new Date()
			const year = now.getFullYear()
			const month = now.getMonth() + 1
			const day = now.getDate()
			const hours = now.getHours()
			const minutes = now.getMinutes()
			const seconds = now.getSeconds()
			const milliseconds = now.getMilliseconds()

			return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}.${milliseconds} UTC`
		}

		return undefined
	}

	function builtSchema(fields: FieldMetadata[]): Schema {
		const properties: { [name: string]: SchemaProperty } = {}
		const required: string[] = []

		fields.forEach((field) => {
			const schemaProperty: SchemaProperty = {
				type: field.fieldType
			}

			if (field.nullable && field.isPrimaryKey) {
				schemaProperty.description = `If left empty, the value will be auto-generated`
			}

			switch (field.fieldType) {
				case 'number':
					schemaProperty.type = 'number'
					const extractedDefaultValue = extractDefaultValue(field.defaultValue)
					schemaProperty.default = extractedDefaultValue ? Number(extractedDefaultValue) : undefined
					break
				case 'checkbox':
					schemaProperty.type = 'boolean'
					schemaProperty.default = field.defaultValue === 'true'
					break
				case 'date':
					schemaProperty.type = 'string'
					schemaProperty.format = 'date'
					schemaProperty.default = parsePsqlDate(field.defaultValue)
					break
				case 'text':
				default:
					schemaProperty.type = 'string'
					schemaProperty.default = extractDefaultValue(field.defaultValue)
					break
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

	$: schema = builtSchema(fields ?? []) as Schema

	export let isInsertable: boolean = false

	$: if (schema) {
		const requiredFields = schema.required ?? []
		const filledFields = Object.keys(args).filter(
			(key) => args[key] !== undefined && args[key] !== null
		)
		isInsertable = requiredFields.every((field) => filledFields.includes(field))
	}
</script>

<SchemaForm {schema} bind:args />
