<script lang="ts">
	import type { Schema, SchemaProperty } from '$lib/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import type { TableMetadata } from './utils'

	export let tableMetaData: TableMetadata | undefined = undefined

	// Compute what kind of input fields we need to display
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

	function builtSchema(fields: any[]): Schema {
		const properties: { [name: string]: SchemaProperty } = {}
		const required: string[] = []

		fields.forEach((field) => {
			const schemaProperty: SchemaProperty = {
				type: field.fieldType,
				default: field.defaultValue
			}

			// Add specific properties based on field type
			switch (field.fieldType) {
				case 'number':
					schemaProperty.type = 'number'
					break
				case 'checkbox':
					schemaProperty.type = 'boolean'
					break
				case 'date':
					schemaProperty.type = 'string'
					schemaProperty.format = 'date'
					break
				case 'text':
				default:
					schemaProperty.type = 'string'
					break
			}

			properties[field.name] = schemaProperty

			// Add to required array if it's a primary key or doesn't have a default value
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

	let args: Record<string, any> = {}
</script>

<SchemaForm {schema} bind:args />

<Button
	color="dark"
	size="xs"
	on:click={() => {
		console.log(args)
	}}
>
	Submit
</Button>
