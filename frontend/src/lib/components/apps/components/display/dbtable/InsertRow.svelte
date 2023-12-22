<script lang="ts">
	import type { Schema, SchemaProperty } from '$lib/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import type { TableMetadata } from './utils'

	export let tableMetaData: TableMetadata | undefined = undefined
	export let args: Record<string, any> = {}

	const fields = tableMetaData?.map((column) => {
		const type = column.datatype
		const name = column.columnname
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
			fieldType
		}
	})

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

	function builtSchema(fields: any[]): Schema {
		const properties: { [name: string]: SchemaProperty } = {}
		const required: string[] = []

		fields.forEach((field) => {
			const schemaProperty: SchemaProperty = {
				type: field.fieldType
			}

			// Add specific properties based on field type
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

					debugger

					break
				case 'text':
				default:
					schemaProperty.type = 'string'
					schemaProperty.default = extractDefaultValue(field.defaultValue)
					break
			}

			properties[field.name] = schemaProperty

			if (field.isPrimaryKey || field.defaultValue === undefined) {
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

	const schema: Schema = builtSchema(fields ?? [])
</script>

<SchemaForm {schema} bind:args />
